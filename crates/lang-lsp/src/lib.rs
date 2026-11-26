use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use striv_syntax::Parser;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

pub struct StrivLsp {
    client: Client,
    // Stores the current text of each open document in the editor
    documents: RwLock<HashMap<Url, String>>,
}

fn options_from_uri(uri: &Url) -> (striv_syntax::Dialect, striv_syntax::BraceStyle) {
    let s = uri.as_str();
    let clean = s.strip_prefix("file:///").unwrap_or(s).replace("%3A", ":");
    striv_syntax::detect_format_options_for_path(std::path::Path::new(&clean))
}

fn dialect_from_uri(uri: &Url) -> striv_syntax::Dialect {
    options_from_uri(uri).0
}

impl StrivLsp {
    async fn validate(&self, uri: Url, text: &str) {
        let dialect = dialect_from_uri(&uri);
        let parse_errors = striv_syntax::check_diagnostics_with_dialect(text, dialect);
        let mut diags: Vec<Diagnostic> = parse_errors
            .into_iter()
            .map(|err| Diagnostic {
                range: Range::new(
                    offset_to_position(text, err.span.start),
                    offset_to_position(text, err.span.end.max(err.span.start + 1)),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("striv".to_string()),
                message: err.message,
                ..Default::default()
            })
            .collect();
        if diags.is_empty() {
            let mut prog = striv_syntax::Parser::new_with_dialect(text, dialect).parse_program();
            if let Err(null_err) = striv_syntax::check_container_nulls(&prog) {
                diags.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("striv-null".to_string()),
                    message: null_err,
                    ..Default::default()
                });
            }
            if let Err(unit_err) = striv_syntax::units::check_and_erase_units(&mut prog) {
                diags.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("striv-units".to_string()),
                    message: unit_err,
                    ..Default::default()
                });
            }
        }
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

fn offset_to_position(src: &str, offset: usize) -> Position {
    let safe_offset = offset.min(src.len());
    let before = &src[..safe_offset];
    let line = before.bytes().filter(|&b| b == b'\n').count() as u32;
    let col = before
        .rfind('\n')
        .map(|nl| safe_offset - nl - 1)
        .unwrap_or(safe_offset) as u32;
    Position::new(line, col)
}

#[tower_lsp::async_trait]
impl LanguageServer for StrivLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // VS Code sends the full document text on every change
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // Enables Format-on-Save (Ctrl+S / Shift+Alt+F)
                document_formatting_provider: Some(OneOf::Left(true)),
                // Enables autocompletion (especially after `.` and `:`)
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        " ".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Striv Language Server ready!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.validate(uri.clone(), &text).await;
        self.documents.write().await.insert(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let uri = params.text_document.uri;
            let text = change.text;
            self.validate(uri.clone(), &text).await;
            self.documents.write().await.insert(uri, text);
        }
    }

    // 1. AUTO-FORMATTER ON SAVE
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let docs = self.documents.read().await;
        let Some(src) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };

        let (dialect, braces) = options_from_uri(&params.text_document.uri);
        // Only format when the code currently parses without errors
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            Parser::new_with_dialect(src, dialect).parse_program()
        }));
        if let Ok(prog) = parsed {
            let formatted = striv_syntax::format_program_with_options(&prog, dialect, braces);
            let edit = TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                new_text: formatted,
            };
            return Ok(Some(vec![edit]));
        }
        Ok(None)
    }

    // 2. CONTEXT-SENSITIVE AUTOCOMPLETION
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let docs = self.documents.read().await;
        let pos = params.text_document_position.position;
        let Some(src) = docs.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };

        let lines: Vec<&str> = src.lines().collect();
        let current_line = lines.get(pos.line as usize).copied().unwrap_or("");
        let col = (pos.character as usize).min(current_line.len());
        let before_cursor = &current_line[..col];

        // Blank out the current line so the parser can still parse all structs & impls
        let clean_src: String = lines
            .iter()
            .enumerate()
            .map(|(i, l)| if i == pos.line as usize { "" } else { *l })
            .collect::<Vec<_>>()
            .join("\n");

        let uri = &params.text_document_position.text_document.uri;
        let mut items = Vec::new();

        // --- CASE -1: `use ` module completion ---
        if let Some(after_use) = before_cursor.trim_start().strip_prefix("use ") {
            if !after_use.contains("::") {
                if let Ok(file_path) = uri.to_file_path() {
                    if let Some(dir) = file_path.parent() {
                        if let Ok(entries) = std::fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.extension().and_then(|e| e.to_str()) == Some("strv")
                                    && p != file_path
                                {
                                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                        items.push(CompletionItem {
                                            label: stem.to_string(),
                                            kind: Some(CompletionItemKind::MODULE),
                                            detail: Some("Local Striv module".to_string()),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                for crate_name in ["minifb", "fastrand"] {
                    items.push(CompletionItem {
                        label: crate_name.to_string(),
                        kind: Some(CompletionItemKind::MODULE),
                        detail: Some("Rust crate (from striv.toml)".to_string()),
                        ..Default::default()
                    });
                }
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        let mut parsed =
            catch_unwind(AssertUnwindSafe(|| Parser::new(&clean_src).parse_program())).ok();
        let mut unimported_fns: Vec<(String, String, String)> = Vec::new();
        if let (Some(prog), Ok(file_path)) = (&mut parsed, uri.to_file_path()) {
            if let Some(dir) = file_path.parent() {
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("strv");
                for imp in &prog.imports {
                    let dep_path = dir.join(format!("{}.{}", imp, ext));
                    if let Ok(dep_src) = std::fs::read_to_string(&dep_path) {
                        if let Ok(dep_prog) =
                            catch_unwind(AssertUnwindSafe(|| Parser::new(&dep_src).parse_program()))
                        {
                            prog.structs.extend(dep_prog.structs);
                            prog.functions.extend(dep_prog.functions);
                        }
                    }
                }
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().and_then(|e| e.to_str()) == Some(ext) && p != file_path {
                            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                if !prog.imports.iter().any(|i| i == stem) {
                                    if let Ok(dep_src) = std::fs::read_to_string(&p) {
                                        if let Ok(dep_prog) = catch_unwind(AssertUnwindSafe(|| {
                                            Parser::new(&dep_src).parse_program()
                                        })) {
                                            for f in dep_prog
                                                .functions
                                                .iter()
                                                .filter(|f| !f.name.contains("__"))
                                            {
                                                unimported_fns.push((
                                                    f.name.clone(),
                                                    format_fn_sig(f),
                                                    stem.to_string(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // --- CASE 0: Path access (`Key::` or `Window::`) ---
        if let Some(colon_idx) = before_cursor.rfind("::").filter(|&i| {
            before_cursor[i + 2..]
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
        }) {
            let left = &before_cursor[..colon_idx];
            let ty_name: String = left
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let prefix = format!("{}__", ty_name);
            for (k, _) in lang_runtime::generated_bridge::exported_constants() {
                if let Some(member) = k.strip_prefix(&prefix) {
                    items.push(CompletionItem {
                        label: member.to_string(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        sort_text: Some(format!("0_{}", member)),
                        ..Default::default()
                    });
                }
            }
            for (s_name, _, _, _) in lang_runtime::generated_bridge::exported_symbols() {
                if let Some(member) = s_name.strip_prefix(&prefix) {
                    let doc_entry = lang_runtime::generated_bridge::exported_docs()
                        .iter()
                        .find(|(n, _, _)| n == s_name);
                    items.push(CompletionItem {
                        label: member.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        insert_text: Some(format!("{}($0)", member)),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        detail: doc_entry.map(|(_, sig, _)| sig.to_string()),
                        documentation: doc_entry.and_then(|(_, _, doc)| {
                            if doc.is_empty() {
                                None
                            } else {
                                Some(Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: doc.to_string(),
                                }))
                            }
                        }),
                        sort_text: Some(format!("1_{}", member)),
                        ..Default::default()
                    });
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // --- CASE 1: Dot access (e.g. `hero.` or `hero.he`) ---
        if let Some(dot_idx) = before_cursor.rfind('.').filter(|&i| {
            before_cursor[i + 1..]
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
        }) {
            let left = &before_cursor[..dot_idx];
            let var_name: String = left
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();

            let visible = collect_visible_vars(&lines, pos.line as usize);
            if visible.iter().any(|(n, ty)| n == &var_name && ty == "Vec") {
                items.push(CompletionItem {
                    label: "push".to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    insert_text: Some("push($0)".to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    detail: Some("fn push(val: int)".to_string()),
                    sort_text: Some("0_push".to_string()),
                    ..Default::default()
                });
                items.push(CompletionItem {
                    label: "len".to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    insert_text: Some("len()".to_string()),
                    detail: Some("fn len() -> int".to_string()),
                    sort_text: Some("1_len".to_string()),
                    ..Default::default()
                });
                items.push(CompletionItem {
                    label: "as_bytes_ptr".to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    insert_text: Some("as_bytes_ptr()".to_string()),
                    detail: Some("fn as_bytes_ptr() -> int (Zero-Copy pointer)".to_string()),
                    sort_text: Some("2_as_bytes_ptr".to_string()),
                    ..Default::default()
                });
                items.push(CompletionItem {
                    label: "byte_len".to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    insert_text: Some("byte_len()".to_string()),
                    detail: Some("fn byte_len() -> int".to_string()),
                    sort_text: Some("3_byte_len".to_string()),
                    ..Default::default()
                });
                items.push(CompletionItem {
                    label: "to_bytes".to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    insert_text: Some("to_bytes()".to_string()),
                    detail: Some("fn to_bytes() -> Bytes".to_string()),
                    sort_text: Some("4_to_bytes".to_string()),
                    ..Default::default()
                });
                return Ok(Some(CompletionResponse::Array(items)));
            }
            if visible
                .iter()
                .any(|(n, ty)| n == &var_name && ty == "Bytes")
            {
                for (m, snip, det) in [
                    ("len", "len()", "fn len() -> int"),
                    ("is_valid", "is_valid()", "fn is_valid() -> bool"),
                    ("write_u32", "write_u32(${1:val})", "fn write_u32(v: int)"),
                    ("write_i64", "write_i64(${1:val})", "fn write_i64(v: int)"),
                    ("write_f64", "write_f64(${1:val})", "fn write_f64(v: float)"),
                    ("read_u32", "read_u32(${1:offset})", "fn read_u32(offset: int) -> int"),
                    ("read_i64", "read_i64(${1:offset})", "fn read_i64(offset: int) -> int"),
                    ("read_f64", "read_f64(${1:offset})", "fn read_f64(offset: int) -> float"),
                ] {
                    items.push(CompletionItem {
                        label: m.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        insert_text: Some(snip.to_string()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        detail: Some(det.to_string()),
                        sort_text: Some(format!("0_{}", m)),
                        ..Default::default()
                    });
                }
                return Ok(Some(CompletionResponse::Array(items)));
            }
            if visible
                .iter()
                .any(|(n, ty)| n == &var_name && ty == "Tuple")
            {
                for (i, (swiz, idx)) in [("x", 0), ("y", 1), ("z", 2), ("w", 3)].iter().enumerate()
                {
                    items.push(CompletionItem {
                        label: swiz.to_string(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(format!("tuple .{}", idx)),
                        sort_text: Some(format!("0_{}_{}", i, swiz)),
                        ..Default::default()
                    });
                }
                for idx in 0..2 {
                    items.push(CompletionItem {
                        label: idx.to_string(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(format!("tuple element .{}", idx)),
                        sort_text: Some(format!("1_{}", idx)),
                        ..Default::default()
                    });
                }
                for (i, (swiz, idx)) in [("r", 0), ("g", 1), ("b", 2), ("a", 3)].iter().enumerate()
                {
                    items.push(CompletionItem {
                        label: swiz.to_string(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(format!("color .{}", idx)),
                        sort_text: Some(format!("2_{}_{}", i, swiz)),
                        ..Default::default()
                    });
                }
                return Ok(Some(CompletionResponse::Array(items)));
            }
            if let Some((_, ty)) = visible.iter().find(|(n, _)| n == &var_name) {
                let prefix = format!("{}__", ty);
                let mut found_std = false;
                for (s_name, _, sig_str, _) in lang_runtime::std_collection_symbols() {
                    if let Some(m_name) = s_name.strip_prefix(&prefix) {
                        found_std = true;
                        let param_count =
                            sig_str.split_once("->").map(|(p, _)| p.len()).unwrap_or(0);
                        let snip = if param_count <= 1 {
                            format!("{}()", m_name)
                        } else {
                            format!("{}($0)", m_name)
                        };
                        items.push(CompletionItem {
                            label: m_name.to_string(),
                            kind: Some(CompletionItemKind::METHOD),
                            insert_text: Some(snip),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            detail: Some(format!(
                                "std::{} ({})",
                                s_name.replace("__", "."),
                                sig_str
                            )),
                            sort_text: Some(format!("0_{}", m_name)),
                            ..Default::default()
                        });
                    }
                }
                if found_std {
                    return Ok(Some(CompletionResponse::Array(items)));
                }
            }

            if let Some(prog) = &parsed {
                if let Some(struct_name) = infer_var_struct(src, &var_name, prog) {
                    // 1. Struct fields first ("0_...")
                    if let Some(s) = prog.structs.iter().find(|s| s.name == struct_name) {
                        for (f, ty) in &s.fields {
                            items.push(CompletionItem {
                                label: f.clone(),
                                kind: Some(CompletionItemKind::FIELD),
                                detail: Some(ty.clone()),
                                sort_text: Some(format!("0_{}", f)),
                                ..Default::default()
                            });
                        }
                    }

                    // 2. Methods from `impl Struct` right below ("1_...")
                    let prefix = format!("{}__", struct_name);
                    for func in &prog.functions {
                        if let Some(m_name) = func.name.strip_prefix(&prefix) {
                            items.push(CompletionItem {
                                label: m_name.to_string(),
                                kind: Some(CompletionItemKind::METHOD),
                                insert_text: Some(format!("{}($0)", m_name)),
                                insert_text_format: Some(InsertTextFormat::SNIPPET),
                                detail: Some(format!("fn {}(...)", m_name)),
                                sort_text: Some(format!("1_{}", m_name)),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            // Also suggest methods for auto-bindgen handle types (e.g. `rng.` -> `bool()`, `f64()`)
            if let Some((_, ty_name)) = visible.iter().find(|(n, _)| n == &var_name) {
                let prefix = format!("{}__", ty_name);
                if let Some(prog) = &parsed {
                    for e in &prog.enums {
                        if e.name == *ty_name {
                            for v in &e.variants {
                                let (snip, fmt, detail) = if v.field_types.is_empty() {
                                    (None, None, None)
                                } else {
                                    (
                                        Some(format!("{}($0)", v.name)),
                                        Some(InsertTextFormat::SNIPPET),
                                        Some(format!("{}({})", v.name, v.field_types.join(", "))),
                                    )
                                };
                                items.push(CompletionItem {
                                    label: v.name.clone(),
                                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                                    insert_text: snip,
                                    insert_text_format: fmt,
                                    detail,
                                    sort_text: Some(format!("0_{}", v.name)),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                for (s_name, _, _, sig_str) in lang_runtime::generated_bridge::exported_symbols() {
                    if let Some(m_name) = s_name.strip_prefix(&prefix) {
                        let param_count =
                            sig_str.split_once("->").map(|(p, _)| p.len()).unwrap_or(0);
                        let snip = if param_count <= 1 {
                            format!("{}()", m_name)
                        } else {
                            format!("{}($0)", m_name)
                        };
                        let doc_entry = lang_runtime::generated_bridge::exported_docs()
                            .iter()
                            .find(|(n, _, _)| n == s_name);
                        items.push(CompletionItem {
                            label: m_name.to_string(),
                            kind: Some(CompletionItemKind::METHOD),
                            insert_text: Some(snip),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            detail: doc_entry.map(|(_, sig, _)| sig.to_string()),
                            documentation: doc_entry.and_then(|(_, _, doc)| {
                                if doc.is_empty() {
                                    None
                                } else {
                                    Some(Documentation::MarkupContent(MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: doc.to_string(),
                                    }))
                                }
                            }),
                            sort_text: Some(format!("1_{}", m_name)),
                            ..Default::default()
                        });
                    }
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // --- CASE 2: Type context after `:` or `->` (e.g. `hp: ` or `fn foo() -> `) ---
        let trimmed = before_cursor.trim_end();
        let after_colon = trimmed.ends_with(':')
            || trimmed
                .rfind(':')
                .map(|idx| {
                    !before_cursor[idx + 1..]
                        .chars()
                        .any(|c| matches!(c, ',' | ')' | '{' | '='))
                })
                .unwrap_or(false);
        let after_arrow = trimmed.ends_with("->");

        if after_colon || after_arrow {
            for ty in ["int", "float", "bool", "String", "Vec", "Result", "Option"] {
                items.push(CompletionItem {
                    label: ty.to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    sort_text: Some(format!("0_{}", ty)),
                    ..Default::default()
                });
            }
            if let Some(prog) = &parsed {
                for s in &prog.structs {
                    items.push(CompletionItem {
                        label: s.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        sort_text: Some(format!("1_{}", s.name)),
                        ..Default::default()
                    });
                }
            } else {
                for line in src.lines() {
                    if let Some(rest) = line.trim_start().strip_prefix("struct ") {
                        let name = rest
                            .split(|c: char| !c.is_alphanumeric() && c != '_')
                            .next()
                            .unwrap_or("");
                        if !name.is_empty() {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::STRUCT),
                                sort_text: Some(format!("1_{}", name)),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // Do not force popup on plain space unless we are after `:` or `->`
        if let Some(ctx) = &params.context {
            if ctx.trigger_character.as_deref() == Some(" ") {
                return Ok(None);
            }
        }

        // --- CASE 3: General context (keywords with trailing space + functions) ---
        let dialect = dialect_from_uri(&params.text_document_position.text_document.uri);
        if dialect == striv_syntax::Dialect::Odin {
            items.push(CompletionItem {
                label: "proc".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                insert_text: Some("${1:name} :: proc(${2}) {\n\t$0\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                detail: Some("Odin-Style procedure".to_string()),
                sort_text: Some("0_proc".to_string()),
                ..Default::default()
            });
            items.push(CompletionItem {
                label: "struct".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                insert_text: Some("${1:Name} :: struct {\n\t$0\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                detail: Some("Odin-Style struct".to_string()),
                sort_text: Some("0_struct".to_string()),
                ..Default::default()
            });
        }
        let keywords_with_space = [
            "use",
            "tweak",
            "const",
            "enum",
            "match",
            "struct",
            "impl",
            "fn",
            "let",
            "mut",
            "if",
            "while",
            "for",
            "in",
            "return",
            "concurrent",
            "spawn",
            "shared",
            "lock",
            "parallel",
            "select",
            "timeout",
            "defer",
            "yield",
            "break",
            "continue",
            "test",
            "sync",
            "rpc",
            "open",
            "extend",
            "unit",
            "rust",
            "query",
        ];
        for kw in keywords_with_space {
            if dialect == striv_syntax::Dialect::Odin && matches!(kw, "fn" | "let") {
                continue;
            }
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(format!("{} ", kw)),
                sort_text: Some(format!("2_{}", kw)),
                ..Default::default()
            });
        }
        for kw in ["true", "false", "null"] {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                sort_text: Some(format!("2_{}", kw)),
                ..Default::default()
            });
        }

        items.push(CompletionItem {
            label: "print".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some("print($0)".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            sort_text: Some("1_print".to_string()),
            ..Default::default()
        });
        for (kw, snip, detail) in [
            ("Ok", "Ok($0)", "Result::Ok(val)"),
            ("Err", "Err($0)", "Result::Err(err)"),
            ("Some", "Some($0)", "Option::Some(val)"),
            ("None", "None", "Option::None"),
        ] {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                insert_text: Some(snip.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                detail: Some(detail.to_string()),
                sort_text: Some(format!("1_{}", kw)),
                ..Default::default()
            });
        }
        if let Some(prog) = &parsed {
            for e in &prog.enums {
                items.push(CompletionItem {
                    label: e.name.clone(),
                    kind: Some(CompletionItemKind::ENUM),
                    sort_text: Some(format!("1_{}", e.name)),
                    ..Default::default()
                });
                for v in &e.variants {
                    let full = format!("{}::{}", e.name, v.name);
                    let (snip, fmt, detail) = if v.field_types.is_empty() {
                        (None, None, None)
                    } else {
                        (
                            Some(format!("{}($0)", full)),
                            Some(InsertTextFormat::SNIPPET),
                            Some(format!("{}({})", full, v.field_types.join(", "))),
                        )
                    };
                    items.push(CompletionItem {
                        label: full.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        insert_text: snip,
                        insert_text_format: fmt,
                        detail,
                        sort_text: Some(format!("0_{}", full)),
                        ..Default::default()
                    });
                }
            }
            for (c_name, _) in &prog.consts {
                items.push(CompletionItem {
                    label: c_name.clone(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some("const (comptime)".to_string()),
                    sort_text: Some(format!("0_{}", c_name)),
                    ..Default::default()
                });
            }
            for (t_name, t_val) in &prog.tweaks {
                let detail = match t_val {
                    striv_syntax::TweakVal::Int(v) => format!("tweak = {}", v),
                    striv_syntax::TweakVal::Str(s) => format!("tweak = \"{}\"", s),
                    striv_syntax::TweakVal::Vec(v) => format!("tweak = {:?}", v),
                };
                items.push(CompletionItem {
                    label: t_name.clone(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some(detail),
                    sort_text: Some(format!("0_{}", t_name)),
                    ..Default::default()
                });
            }
            for s in &prog.structs {
                items.push(CompletionItem {
                    label: s.name.clone(),
                    kind: Some(CompletionItemKind::STRUCT),
                    sort_text: Some(format!("1_{}", s.name)),
                    ..Default::default()
                });
            }
            for func in prog.functions.iter().filter(|f| !f.name.contains("__")) {
                items.push(CompletionItem {
                    label: func.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some(format!("{}($0)", func.name)),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    detail: Some(format_fn_sig(func)),
                    sort_text: Some(format!("0_{}", func.name)),
                    ..Default::default()
                });
            }
        }
        for (fn_name, sig, mod_name) in unimported_fns {
            items.push(CompletionItem {
                label: fn_name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(format!("{}($0)", fn_name)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                detail: Some(format!("{} (auto-import `use {}`)", sig, mod_name)),
                additional_text_edits: Some(vec![TextEdit {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    new_text: format!("use {}\n", mod_name),
                }]),
                sort_text: Some(format!("1_{}", fn_name)),
                ..Default::default()
            });
        }

        // Suggest auto-bindgen type names (e.g. `Rng`, `Window`, `Key`) with auto-import if missing
        let imported_set: std::collections::HashSet<&str> = parsed
            .as_ref()
            .map(|p| p.imports.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        let mut seen_types = std::collections::HashSet::new();
        for (s_name, _, _, _) in lang_runtime::generated_bridge::exported_symbols() {
            if let Some((ty, _)) = s_name.split_once("__") {
                if seen_types.insert(ty) {
                    let crate_name = lang_runtime::generated_bridge::exported_crates()
                        .iter()
                        .find(|(n, _)| *n == ty)
                        .map(|(_, c)| *c);
                    let mut item = CompletionItem {
                        label: ty.to_string(),
                        kind: Some(CompletionItemKind::STRUCT),
                        sort_text: Some(format!("1_{}", ty)),
                        ..Default::default()
                    };
                    if let Some(cr) = crate_name {
                        if !imported_set.contains(cr) {
                            item.detail = Some(format!("{} (auto-import `use {}`)", cr, cr));
                            item.additional_text_edits = Some(vec![TextEdit {
                                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                                new_text: format!("use {}\n", cr),
                            }]);
                        } else {
                            item.detail = Some(format!("from `{}`", cr));
                        }
                    }
                    items.push(item);
                }
            }
        }
        for (ty, _) in lang_runtime::std_drop_symbols() {
            if seen_types.insert(ty) {
                items.push(CompletionItem {
                    label: ty.to_string(),
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("std collection".to_string()),
                    sort_text: Some(format!("1_{}", ty)),
                    ..Default::default()
                });
            }
        }
        for (c_name, _) in lang_runtime::generated_bridge::exported_constants() {
            if let Some((ty, _)) = c_name.split_once("__") {
                if seen_types.insert(ty) {
                    items.push(CompletionItem {
                        label: ty.to_string(),
                        kind: Some(CompletionItemKind::ENUM),
                        sort_text: Some(format!("1_{}", ty)),
                        ..Default::default()
                    });
                }
            }
        }

        // Suggest local variables & parameters of the current function at the top ("0_...")
        for (var_name, var_ty) in collect_visible_vars(&lines, pos.line as usize) {
            items.push(CompletionItem {
                label: var_name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(var_ty),
                sort_text: Some(format!("0_{}", var_name)),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    // 3. HOVER DOCUMENTATION
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(src) = docs.get(uri) else {
            return Ok(None);
        };
        let lines: Vec<&str> = src.lines().collect();
        let line = lines.get(pos.line as usize).copied().unwrap_or("");
        let col = (pos.character as usize).min(line.len());

        let token = extract_symbol_at(line, col);
        if token.is_empty() {
            return Ok(None);
        }

        let lookup_key = if let Some((ty, member)) = token.split_once("::") {
            format!("{}__{}", ty, member)
        } else if let Some((recv, method)) = token.split_once('.') {
            let visible = collect_visible_vars(&lines, pos.line as usize);
            if let Some((_, ty)) = visible.iter().find(|(n, _)| n == recv) {
                format!("{}__{}", ty, method)
            } else {
                token.clone()
            }
        } else {
            token.clone()
        };

        if let Some((_, sig, doc)) = lang_runtime::generated_bridge::exported_docs()
            .iter()
            .find(|(n, _, _)| *n == lookup_key)
        {
            let mut md = format!("```striv\n{}\n```", sig);
            if !doc.is_empty() {
                md.push_str("\n---\n");
                md.push_str(doc);
            }
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: None,
            }));
        }

        if let Some((c_name, val)) = lang_runtime::generated_bridge::exported_constants()
            .iter()
            .find(|(n, _)| *n == lookup_key)
        {
            let disp = c_name.replace("__", "::");
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```striv\nconst {}: int = {}\n```", disp, val),
                }),
                range: None,
            }));
        }

        if let Some(prog) = parse_with_imports(src, uri) {
            if let Some(func) = prog
                .functions
                .iter()
                .find(|f| f.name == lookup_key || f.name == token)
            {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```striv\n{}\n```", format_fn_sig(func)),
                    }),
                    range: None,
                }));
            }
            if let Some((c_name, c_expr)) = prog.consts.iter().find(|(n, _)| n == &token) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "```striv\nconst {} = {}\n```\n*(comptime constant)*",
                            c_name,
                            striv_syntax::format_expr(c_expr)
                        ),
                    }),
                    range: None,
                }));
            }
            if let Some((t_name, t_val)) = prog.tweaks.iter().find(|(n, _)| n == &token) {
                let val_s = match t_val {
                    striv_syntax::TweakVal::Int(i) => i.to_string(),
                    striv_syntax::TweakVal::Str(s) => format!("\"{}\"", s),
                    striv_syntax::TweakVal::Vec(v) => format!("{:?}", v),
                };
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "```striv\ntweak {} = {}\n```\n*(live tweak)*",
                            t_name, val_s
                        ),
                    }),
                    range: None,
                }));
            }
            if let Some((ty, member)) = token.split_once("::") {
                if let Some(e) = prog.enums.iter().find(|e| e.name == ty) {
                    if let Some(idx) = e.variants.iter().position(|v| &v.name == member) {
                        return Ok(Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: format!("```striv\n{}::{} = {}\n```", e.name, member, idx),
                            }),
                            range: None,
                        }));
                    }
                }
            }
            if let Some(e) = prog.enums.iter().find(|e| e.name == token) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "```striv\nenum {} {{ {} }}\n```",
                            e.name,
                            e.variants
                                .iter()
                                .map(|v| v.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    }),
                    range: None,
                }));
            }
            if let Some(s) = prog.structs.iter().find(|s| s.name == token) {
                let fields: Vec<String> = s
                    .fields
                    .iter()
                    .map(|(f, ty)| format!("{}: {}", f, ty))
                    .collect();
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "```striv\nstruct {} {{ {} }}\n```",
                            s.name,
                            fields.join(", ")
                        ),
                    }),
                    range: None,
                }));
            }
            if let Some((recv, field)) = token.split_once('.') {
                let visible = collect_visible_vars(&lines, pos.line as usize);
                if let Some((_, ty)) = visible.iter().find(|(n, _)| n == recv) {
                    if let Some(s) = prog.structs.iter().find(|s| &s.name == ty) {
                        if let Some((_, f_ty)) = s.fields.iter().find(|(f, _)| f == field) {
                            return Ok(Some(Hover {
                                contents: HoverContents::Markup(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!("```striv\n{}.{}: {}\n```", s.name, field, f_ty),
                                }),
                                range: None,
                            }));
                        }
                    }
                }
            }
        }

        let visible = collect_visible_vars(&lines, pos.line as usize);
        if let Some((v_name, v_ty)) = visible.iter().find(|(n, _)| n == &token) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```striv\nlet {}: {}\n```", v_name, v_ty),
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    // 4. SIGNATURE HELP (active parameter hint inside `(...)`)
    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(src) = docs.get(uri) else {
            return Ok(None);
        };
        let lines: Vec<&str> = src.lines().collect();
        let line = lines.get(pos.line as usize).copied().unwrap_or("");
        let col = (pos.character as usize).min(line.len());
        let before = &line[..col];

        // Find matching '(' before cursor and count commas at depth 0
        let mut depth = 0i32;
        let mut open_idx = None;
        let mut comma_count = 0u32;
        for (idx, ch) in before.char_indices().rev() {
            match ch {
                ')' => depth += 1,
                '(' => {
                    if depth == 0 {
                        open_idx = Some(idx);
                        break;
                    }
                    depth -= 1;
                }
                ',' if depth == 0 => comma_count += 1,
                _ => {}
            }
        }

        let Some(open) = open_idx else {
            return Ok(None);
        };
        let fn_expr = before[..open]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':' || *c == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        let lookup_key = if let Some((ty, member)) = fn_expr.split_once("::") {
            format!("{}__{}", ty, member)
        } else if let Some((recv, method)) = fn_expr.split_once('.') {
            let visible = collect_visible_vars(&lines, pos.line as usize);
            if let Some((_, ty)) = visible.iter().find(|(n, _)| n == recv) {
                format!("{}__{}", ty, method)
            } else {
                fn_expr.clone()
            }
        } else {
            fn_expr.clone()
        };

        if let Some((_, sig, doc)) = lang_runtime::generated_bridge::exported_docs()
            .iter()
            .find(|(n, _, _)| *n == lookup_key)
        {
            return Ok(Some(build_signature_help(sig, doc, comma_count)));
        }

        if let Some(prog) = parse_with_imports(src, uri) {
            if let Some(func) = prog
                .functions
                .iter()
                .find(|f| f.name == lookup_key || f.name == fn_expr)
            {
                let sig = format_fn_sig(func);
                return Ok(Some(build_signature_help(&sig, "", comma_count)));
            }
        }

        Ok(None)
    }

    // 5. GO TO DEFINITION (Ctrl+Click)
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(src) = docs.get(uri) else {
            return Ok(None);
        };
        let lines: Vec<&str> = src.lines().collect();
        let line = lines.get(pos.line as usize).copied().unwrap_or("");
        let col = (pos.character as usize).min(line.len());
        let sym = extract_symbol_at(line, col);
        let name = sym.split('.').last().unwrap_or(&sym);
        if name.is_empty() {
            return Ok(None);
        }

        if let Some(loc) = find_def_in_source(uri, src, name) {
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }

        // Check Rust crate locations (`Window::new`, `win.is_key_down`, `Key::Left`, `Window`)
        let lookup_key = if let Some((ty, member)) = sym.split_once("::") {
            format!("{}__{}", ty, member)
        } else if let Some((recv, method)) = sym.split_once('.') {
            let visible = collect_visible_vars(&lines, pos.line as usize);
            if let Some((_, ty)) = visible.iter().find(|(n, _)| n == recv) {
                format!("{}__{}", ty, method)
            } else {
                sym.clone()
            }
        } else {
            sym.clone()
        };
        if let Some((_, rs_file, rs_line)) = lang_runtime::generated_bridge::exported_locations()
            .iter()
            .find(|(n, _, _)| *n == lookup_key || *n == sym)
        {
            if let Ok(rs_uri) = Url::from_file_path(rs_file) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                    rs_uri,
                    Range::new(Position::new(*rs_line, 0), Position::new(*rs_line, 0)),
                ))));
            }
        }

        if let (Some(prog), Ok(file_path)) = (
            catch_unwind(AssertUnwindSafe(|| Parser::new(src).parse_program())).ok(),
            uri.to_file_path(),
        ) {
            if let Some(dir) = file_path.parent() {
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("strv");
                for imp in &prog.imports {
                    let dep_path = dir.join(format!("{}.{}", imp, ext));
                    if let Ok(dep_src) = std::fs::read_to_string(&dep_path) {
                        if let Ok(dep_uri) = Url::from_file_path(&dep_path) {
                            if let Some(loc) = find_def_in_source(&dep_uri, &dep_src, name) {
                                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    // 6. DOCUMENT SYMBOLS (Outline / Breadcrumbs / Ctrl+Shift+O)
    #[allow(deprecated)]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document.uri;
        let Some(src) = docs.get(uri) else {
            return Ok(None);
        };
        let mut syms = Vec::new();
        for (idx, line) in src.lines().enumerate() {
            let t = line.trim_start();
            let line_u32 = idx as u32;
            let range = Range::new(
                Position::new(line_u32, 0),
                Position::new(line_u32, line.len() as u32),
            );
            let (kind, name) = if let Some(r) = t.strip_prefix("fn ") {
                (
                    SymbolKind::FUNCTION,
                    r.split('(').next().unwrap_or("").trim(),
                )
            } else if let Some(r) = t.strip_prefix("struct ") {
                (
                    SymbolKind::STRUCT,
                    r.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or(""),
                )
            } else if let Some(r) = t.strip_prefix("enum ") {
                (
                    SymbolKind::ENUM,
                    r.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or(""),
                )
            } else if let Some(r) = t.strip_prefix("const ") {
                (
                    SymbolKind::CONSTANT,
                    r.split('=').next().unwrap_or("").trim(),
                )
            } else if let Some(r) = t.strip_prefix("tweak ") {
                (
                    SymbolKind::CONSTANT,
                    r.split('=').next().unwrap_or("").trim(),
                )
            } else {
                continue;
            };
            if !name.is_empty() {
                syms.push(SymbolInformation {
                    name: name.to_string(),
                    kind,
                    tags: None,
                    deprecated: None,
                    location: Location::new(uri.clone(), range),
                    container_name: None,
                });
            }
        }
        Ok(Some(DocumentSymbolResponse::Flat(syms)))
    }

    // 7. INLAY HINTS (`: Type` after `let` and `for` variables, like rust-analyzer)
    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document.uri;
        let Some(src) = docs.get(uri) else {
            return Ok(None);
        };

        let prog = parse_with_imports(src, uri);
        let mut hints = Vec::new();
        let start_line = params.range.start.line as usize;
        let end_line = params.range.end.line as usize;

        for (line_idx, line) in src.lines().enumerate() {
            if line_idx < start_line || line_idx > end_line {
                continue;
            }
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();

            if let Some(after_let) = trimmed.strip_prefix("let ") {
                let is_mut = after_let.starts_with("mut ");
                let var_start_offset = indent_len + 4 + if is_mut { 4 } else { 0 };
                let rest = if is_mut { &after_let[4..] } else { after_let };
                if let Some((lhs, rhs)) = rest.split_once('=') {
                    if lhs.contains(':') {
                        continue;
                    }
                    let var_name = lhs.trim_end();
                    if var_name.is_empty()
                        || !var_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        continue;
                    }
                    let col = (var_start_offset + lhs.len() - (lhs.len() - var_name.len())) as u32;
                    let ty = infer_expr_type(rhs.trim(), prog.as_ref());
                    hints.push(InlayHint {
                        position: Position::new(line_idx as u32, col),
                        label: InlayHintLabel::String(format!(": {}", ty)),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: None,
                        padding_left: None,
                        padding_right: Some(false),
                        data: None,
                    });
                }
            } else if let Some(after_for) = trimmed.strip_prefix("for ") {
                if let Some((var_part, _)) = after_for.split_once(" in ") {
                    let var_name = var_part.trim();
                    if !var_name.is_empty()
                        && var_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        let col = (indent_len + 4 + var_part.trim_end().len()) as u32;
                        hints.push(InlayHint {
                            position: Position::new(line_idx as u32, col),
                            label: InlayHintLabel::String(": int".to_string()),
                            kind: Some(InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: None,
                            padding_right: Some(false),
                            data: None,
                        });
                    }
                }
            }
        }

        Ok(Some(hints))
    }
}

fn infer_expr_type(rhs: &str, prog: Option<&striv_syntax::Program>) -> String {
    let has_try = rhs.ends_with('?');
    let expr = rhs.trim_end_matches('?').trim();

    if expr.starts_with('"') {
        return "String".to_string();
    }
    if expr.starts_with('[') {
        return "Vec".to_string();
    }
    if expr == "true" || expr == "false" || expr == "null" {
        return "bool".to_string();
    }
    if expr.starts_with('(') && expr.contains(',') {
        return "Tuple".to_string();
    }
    if expr.starts_with("Ok(") || expr.starts_with("Err(") {
        return "Result".to_string();
    }
    if expr.starts_with("Some(") || expr == "None" {
        return "Option".to_string();
    }
    if expr.contains('.')
        && expr
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return "float".to_string();
    }
    if expr.starts_with("0x") || expr.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return "int".to_string();
    }
    if let Some((before_brace, _)) = expr.split_once('{') {
        let s = before_brace.trim();
        if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return s.to_string();
        }
    }
    if let Some((before_paren, _)) = expr.split_once('(') {
        let call = before_paren.trim();
        let lookup = call.replace("::", "__");
        if let Some((_, sig, _)) = lang_runtime::generated_bridge::exported_docs()
            .iter()
            .find(|(n, _, _)| *n == lookup)
        {
            if let Some((_, ret)) = sig.split_once("->") {
                return ret.trim().to_string();
            }
        }
        if let Some(p) = prog {
            if let Some(func) = p.functions.iter().find(|f| f.name == lookup) {
                if let Some(rt) = &func.ret_type {
                    if has_try && (rt == "Result" || rt == "Option") {
                        return "int".to_string();
                    }
                    return rt.clone();
                }
            }
        }
        if let Some((ty, _)) = call.split_once("::") {
            return ty.trim().to_string();
        }
    }
    "int".to_string()
}

fn extract_symbol_at(line: &str, col: usize) -> String {
    let bytes = line.as_bytes();
    let is_sym = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b':' || b == b'.';
    let mut start = col;
    while start > 0 && is_sym(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_sym(bytes[end]) {
        end += 1;
    }
    line[start..end]
        .trim_matches('.')
        .trim_matches(':')
        .to_string()
}

fn build_signature_help(sig: &str, doc: &str, active_param: u32) -> SignatureHelp {
    let mut parameters = Vec::new();
    if let (Some(open), Some(close)) = (sig.find('('), sig.rfind(')')) {
        let inner = &sig[open + 1..close];
        if !inner.trim().is_empty() {
            for p in inner.split(',') {
                parameters.push(ParameterInformation {
                    label: ParameterLabel::Simple(p.trim().to_string()),
                    documentation: None,
                });
            }
        }
    }
    SignatureHelp {
        signatures: vec![SignatureInformation {
            label: sig.to_string(),
            documentation: if doc.is_empty() {
                None
            } else {
                Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                }))
            },
            parameters: Some(parameters),
            active_parameter: Some(active_param),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_param),
    }
}

fn infer_var_struct(src: &str, var_name: &str, prog: &striv_syntax::Program) -> Option<String> {
    // 1. If `self.` is typed inside an `impl Struct` block:
    if var_name == "self" {
        for line in src.lines().rev() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("impl ") {
                let s_name = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('{');
                if !s_name.is_empty() {
                    return Some(s_name.to_string());
                }
            }
        }
    }

    // 2. Search for `for [mut] var in list` -> `list: Vec<Struct>`
    let for_a = format!("for {} in ", var_name);
    let for_b = format!("for mut {} in ", var_name);
    for line in src.lines() {
        let t = line.trim();
        let rest = t.strip_prefix(&for_a).or_else(|| t.strip_prefix(&for_b));
        if let Some(after_in) = rest {
            let coll = after_in
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or("");
            if !coll.is_empty() {
                for l2 in src.lines() {
                    if let Some((_, rhs)) = l2.split_once(&format!("{}: Vec<", coll)) {
                        if let Some(s_name) = rhs.split('>').next() {
                            return Some(s_name.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    // 3. Search for `let [mut] var = Struct` or parameter `var: Struct`
    for line in src.lines() {
        for s in &prog.structs {
            if line.contains(&format!("{}: {}", var_name, s.name))
                || line.contains(&format!("{}:{}", var_name, s.name))
            {
                return Some(s.name.clone());
            }
            if (line.contains(&format!("let {} =", var_name))
                || line.contains(&format!("let mut {} =", var_name)))
                && line.contains(&s.name)
            {
                return Some(s.name.clone());
            }
        }
    }
    None
}

pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| StrivLsp {
        client,
        documents: RwLock::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn collect_visible_vars(lines: &[&str], cursor_line: usize) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    let mut fn_start = 0;
    for i in (0..=cursor_line.min(lines.len().saturating_sub(1))).rev() {
        if lines[i].trim_start().starts_with("fn ") {
            fn_start = i;
            break;
        }
    }

    // 1. Collect parameters from the `fn name(p: Player, mut x: int)` line
    if let Some(fn_line) = lines.get(fn_start) {
        if let (Some(open), Some(close)) = (fn_line.find('('), fn_line.find(')')) {
            let params_str = &fn_line[open + 1..close];
            for part in params_str.split(',') {
                let part = part.trim().trim_start_matches("mut ").trim();
                if part == "self" {
                    vars.push(("self".to_string(), "Self".to_string()));
                } else if let Some((name, ty)) = part.split_once(':') {
                    vars.push((name.trim().to_string(), ty.trim().to_string()));
                }
            }
        }
    }

    // 2. Collect `let [mut] x = ...` bindings between function start and cursor
    for line in &lines[fn_start..cursor_line.min(lines.len())] {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("let ") {
            let rest = rest.trim_start_matches("mut ").trim();
            if let Some((name_part, val_part)) = rest.split_once('=') {
                let name = name_part.split(':').next().unwrap_or("").trim();
                if name.starts_with('(') && name.ends_with(')') {
                    let inner = &name[1..name.len() - 1];
                    for part in inner.split(',') {
                        let v = part.trim().trim_start_matches("mut ").trim();
                        if !v.is_empty() {
                            vars.push((v.to_string(), "int".to_string()));
                        }
                    }
                } else {
                    let val = val_part.trim();
                    let ty = val.split('{').next().unwrap_or("int").trim();
                    let ty_display = if val.starts_with('"') {
                        "String".to_string()
                    } else if val.starts_with('[') {
                        "Vec".to_string()
                    } else if val.starts_with('(') && val.contains(',') {
                        "Tuple".to_string()
                    } else if let Some((handle_ty, _)) = val.split_once("::") {
                        handle_ty.trim().to_string()
                    } else if ty.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        ty.to_string()
                    } else {
                        "int".to_string()
                    };
                    if !name.is_empty() {
                        vars.push((name.to_string(), ty_display));
                    }
                }
            }
        }
    }
    vars
}

fn format_fn_sig(func: &striv_syntax::Function) -> String {
    let params_str: Vec<String> = func
        .params
        .iter()
        .map(|(n, ty, is_mut)| {
            let m = if *is_mut { "mut " } else { "" };
            if n == "self" {
                format!("{}self", m)
            } else {
                format!("{}{}: {}", m, n, ty)
            }
        })
        .collect();
    let ret_str = func
        .ret_type
        .as_ref()
        .map(|r| format!(" -> {}", r))
        .unwrap_or_default();
    format!(
        "fn {}({}){}",
        func.name.replace("__", "."),
        params_str.join(", "),
        ret_str
    )
}

fn parse_with_imports(src: &str, uri: &Url) -> Option<striv_syntax::Program> {
    let mut prog = catch_unwind(AssertUnwindSafe(|| Parser::new(src).parse_program())).ok()?;
    if let Ok(file_path) = uri.to_file_path() {
        if let Some(dir) = file_path.parent() {
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("strv");
            for imp in &prog.imports {
                let dep_path = dir.join(format!("{}.{}", imp, ext));
                if let Ok(dep_src) = std::fs::read_to_string(&dep_path) {
                    if let Ok(dep_prog) =
                        catch_unwind(AssertUnwindSafe(|| Parser::new(&dep_src).parse_program()))
                    {
                        prog.structs.extend(dep_prog.structs);
                        prog.functions.extend(dep_prog.functions);
                    }
                }
            }
        }
    }
    Some(prog)
}

fn find_def_in_source(uri: &Url, src: &str, name: &str) -> Option<Location> {
    let fn_pat = format!("fn {}(", name);
    let fn_pat_sp = format!("fn {} (", name);
    let st_pat = format!("struct {} ", name);
    let st_pat_br = format!("struct {}{{", name);
    let en_pat = format!("enum {} ", name);
    let en_pat_br = format!("enum {}{{", name);
    let const_pat = format!("const {} ", name);
    let const_pat_eq = format!("const {}=", name);
    let tweak_pat = format!("tweak {} ", name);
    let tweak_pat_eq = format!("tweak {}=", name);
    let let_pat = format!("let {} ", name);
    let let_mut_pat = format!("let mut {} ", name);
    for (idx, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with(&fn_pat)
            || t.starts_with(&fn_pat_sp)
            || t.starts_with(&st_pat)
            || t.starts_with(&st_pat_br)
            || t.starts_with(&en_pat)
            || t.starts_with(&en_pat_br)
            || t.starts_with(&const_pat)
            || t.starts_with(&const_pat_eq)
            || t.starts_with(&tweak_pat)
            || t.starts_with(&tweak_pat_eq)
            || t.starts_with(&let_pat)
            || t.starts_with(&let_mut_pat)
        {
            let col = line.find(name).unwrap_or(0) as u32;
            return Some(Location::new(
                uri.clone(),
                Range::new(
                    Position::new(idx as u32, col),
                    Position::new(idx as u32, col + name.len() as u32),
                ),
            ));
        }
    }
    None
}
