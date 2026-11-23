use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use striv_syntax::{format_program, Parser};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

pub struct StrivLsp {
    client: Client,
    // Stores the current text of each open document in the editor
    documents: RwLock<HashMap<Url, String>>,
}

impl StrivLsp {
    async fn validate(&self, uri: Url, text: &str) {
        let parse_errors = striv_syntax::check_diagnostics(text);
        let diags: Vec<Diagnostic> = parse_errors
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

        // Only format when the code currently parses without errors
        let parsed = catch_unwind(AssertUnwindSafe(|| Parser::new(src).parse_program()));
        if let Ok(prog) = parsed {
            let formatted = format_program(&prog);
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

        let parsed =
            catch_unwind(AssertUnwindSafe(|| Parser::new(&clean_src).parse_program())).ok();
        let mut items = Vec::new();

        // --- CASE 0: Path access (`Key::` or `Window::`) ---
        if let Some(colon_idx) = before_cursor.rfind("::") {
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
                    items.push(CompletionItem {
                        label: member.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        insert_text: Some(format!("{}($0)", member)),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        sort_text: Some(format!("1_{}", member)),
                        ..Default::default()
                    });
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // --- CASE 1: Dot access (e.g. `hero.` or `hero.he`) ---
        if let Some(dot_idx) = before_cursor.rfind('.') {
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
                return Ok(Some(CompletionResponse::Array(items)));
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
            for ty in ["int", "float", "bool", "String", "Vec"] {
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
        let keywords_with_space = [
            "tweak", "struct", "impl", "fn", "let", "mut", "if", "while", "for", "in", "return",
        ];
        for kw in keywords_with_space {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(format!("{} ", kw)),
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
        for (fn_name, snip, detail) in [
            ("clear", "clear($0)", "fn clear(win: int, color: int)"),
            (
                "draw_rect",
                "draw_rect($0)",
                "fn draw_rect(win: int, x: int, y: int, w: int, h: int, color: int)",
            ),
            (
                "key_down",
                "key_down($0)",
                "fn key_down(win: int, key: int) -> int",
            ),
        ] {
            items.push(CompletionItem {
                label: fn_name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(snip.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                detail: Some(detail.to_string()),
                sort_text: Some(format!("1_{}", fn_name)),
                ..Default::default()
            });
        }

        if let Some(prog) = &parsed {
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
                    sort_text: Some(format!("0_{}", func.name)),
                    ..Default::default()
                });
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

    // 2. Search for `let [mut] var = Struct` or parameter `var: Struct`
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
                let val = val_part.trim();
                let ty = val.split('{').next().unwrap_or("int").trim();
                let ty_display = if val.starts_with('"') {
                    "String".to_string()
                } else if val.starts_with('[') {
                    "Vec".to_string()
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
    vars
}
