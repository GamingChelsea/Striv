use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{
    Fields, FnArg, GenericArgument, ImplItem, Item, PathArguments, ReturnType, Type, Visibility,
};

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ArgKind {
    Int(String),             // i8..i64, u8..u64, isize, usize
    Bool,                    // bool
    Float(String),           // f32, f64
    StrRef,                  // &str
    U32Slice,                // &[u32]
    U8Slice,                 // &[u8]
    IntoStr,                 // impl Into<String> / AsRef<str>
    AsPath,                  // impl AsRef<Path>
    MutFloat(String),        // &mut f32 / &mut f64
    MutInt(String),          // &mut i32 / &mut i64 / &mut bool
    Enum(String),            // unit enum in same crate
    Handle(String),          // struct in same crate (by value or &T / &mut T)
    HandleRef(String, bool), // (&T, is_mut)
}

#[derive(Debug, Clone)]
pub enum RetKind {
    Unit,
    Int,
    Bool,
    Float,
    Handle(String),
    ResultHandle(String),
    ResultUnit,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub struct_name: Option<String>, // None for free functions
    pub name: String,
    pub receiver: Option<bool>, // None = assoc/free, Some(false) = &self, Some(true) = &mut self
    pub is_async: bool,
    pub args: Vec<ArgKind>,
    pub arg_metas: Vec<(String, String)>,
    pub ret: RetKind,
    pub ret_str: String,
    pub doc: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct CrateApi {
    pub crate_name: String,
    pub enums: Vec<EnumInfo>,
    pub structs: Vec<String>,
    pub methods: Vec<MethodInfo>,
}

pub fn parse_striv_toml(content: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if in_deps {
            if let Some((k, v)) = t.split_once('=') {
                let name = k.trim().to_string();
                let ver = v.trim().trim_matches('"').to_string();
                if !name.is_empty() && !ver.is_empty() {
                    deps.push((name, ver));
                }
            }
        }
    }
    deps
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    targets: Vec<CargoTarget>,
}

#[derive(Deserialize)]
struct CargoTarget {
    kind: Vec<String>,
    src_path: String,
}

pub fn resolve_crate_sources(deps: &[(String, String)]) -> Vec<(String, PathBuf)> {
    let host_dir = Path::new(".striv-host");
    let src_dir = host_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create .striv-host/src");

    let mut cargo_toml = String::from(
        "[workspace]\n\n[package]\nname = \"striv-host-cache\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n",
    );
    for (name, ver) in deps {
        cargo_toml.push_str(&format!("{} = \"{}\"\n", name, ver));
    }
    fs::write(host_dir.join("Cargo.toml"), cargo_toml)
        .expect("Failed to write .striv-host/Cargo.toml");
    let lib_file = src_dir.join("lib.rs");
    if !lib_file.exists() {
        fs::write(&lib_file, "").expect("Failed to write .striv-host/src/lib.rs");
    }

    let output = Command::new("cargo")
        .args([
            "metadata",
            "--manifest-path",
            ".striv-host/Cargo.toml",
            "--format-version",
            "1",
        ])
        .output()
        .expect("Failed to execute cargo metadata");

    if !output.status.success() {
        panic!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let meta: CargoMetadata =
        serde_json::from_slice(&output.stdout).expect("Failed to parse cargo metadata JSON");

    let mut resolved = Vec::new();
    for (dep_name, _) in deps {
        if let Some(pkg) = meta.packages.iter().find(|p| p.name == *dep_name) {
            if let Some(lib_target) = pkg
                .targets
                .iter()
                .find(|t| t.kind.iter().any(|k| k == "lib" || k == "rlib"))
            {
                resolved.push((
                    dep_name.replace('-', "_"),
                    PathBuf::from(&lib_target.src_path),
                ));
            }
        }
    }
    resolved
}

fn is_disabled_cfg(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cfg") {
            let s = attr
                .meta
                .require_list()
                .map(|l| l.tokens.to_string())
                .unwrap_or_default();
            if s.contains("not") && s.contains("windows") {
                return true;
            }
            if s.contains("not (") || s.contains("not(") {
                continue;
            }
            let forbidden = [
                "unix",
                "linux",
                "macos",
                "freebsd",
                "netbsd",
                "openbsd",
                "dragonfly",
                "wasm",
                "android",
                "ios",
                "redox",
                "test",
            ];
            if forbidden.iter().any(|k| s.contains(k)) && !s.contains("windows") {
                return true;
            }
        }
    }
    false
}

fn collect_use_names(tree: &syn::UseTree, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Name(n) => out.push(n.ident.to_string()),
        syn::UseTree::Rename(r) => out.push(r.rename.to_string()),
        syn::UseTree::Path(p) => collect_use_names(&p.tree, out),
        syn::UseTree::Group(g) => {
            for t in &g.items {
                collect_use_names(t, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn collect_all_items(
    file_path: &Path,
    items: &mut Vec<(PathBuf, Item)>,
    root_exports: &mut Vec<String>,
    is_root: bool,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    let Ok(source) = fs::read_to_string(file_path) else {
        return;
    };
    let Ok(parsed) = syn::parse_file(&source) else {
        return;
    };
    let dir = file_path.parent().unwrap_or(Path::new("."));
    let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut sub_paths = Vec::new();
    for item in parsed.items {
        if is_root {
            match &item {
                Item::Use(u) if matches!(u.vis, Visibility::Public(_)) => {
                    collect_use_names(&u.tree, root_exports);
                }
                Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                    root_exports.push(s.ident.to_string());
                }
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    root_exports.push(e.ident.to_string());
                }
                _ => {}
            }
        }
        if let Item::Mod(ref m) = item {
            if is_disabled_cfg(&m.attrs) {
                continue;
            }
            if m.content.is_none() {
                let mod_name = m.ident.to_string();
                let sub_dir = if stem == "lib" || stem == "mod" {
                    dir.to_path_buf()
                } else {
                    dir.join(stem)
                };
                let cand1 = sub_dir.join(format!("{}.rs", mod_name));
                let cand2 = sub_dir.join(&mod_name).join("mod.rs");
                if cand1.exists() {
                    sub_paths.push(cand1);
                } else if cand2.exists() {
                    sub_paths.push(cand2);
                }
            } else if let Some((_, ref inner)) = m.content {
                for sub in inner {
                    items.push((file_path.to_path_buf(), sub.clone()));
                }
            }
        }
        items.push((file_path.to_path_buf(), item));
    }
    for sub_path in sub_paths {
        collect_all_items(&sub_path, items, root_exports, false, depth + 1);
    }
}

pub fn scan_crate_api(crate_name: &str, lib_rs: &Path) -> CrateApi {
    let mut all_items = Vec::new();
    let mut root_exports = Vec::new();
    collect_all_items(lib_rs, &mut all_items, &mut root_exports, true, 0);

    let mut enums = Vec::new();
    let mut structs = Vec::new();
    let mut struct_files = std::collections::HashMap::<String, PathBuf>::new();
    let mut default_structs = Vec::new();

    // Pass 1: collect public unit-only enums and public structs reachable at crate root
    for (file_path, item) in &all_items {
        match item {
            Item::Enum(e)
                if matches!(e.vis, Visibility::Public(_))
                    && !is_disabled_cfg(&e.attrs)
                    && root_exports.contains(&e.ident.to_string()) =>
            {
                let name = e.ident.to_string();
                let all_unit = e.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
                if all_unit
                    && !e.variants.is_empty()
                    && !enums.iter().any(|x: &EnumInfo| x.name == name)
                {
                    let line = find_line_in_file(file_path, &format!("enum {}", name));
                    enums.push(EnumInfo {
                        name,
                        variants: e.variants.iter().map(|v| v.ident.to_string()).collect(),
                        file: file_path.to_string_lossy().to_string(),
                        line,
                    });
                }
            }
            Item::Struct(s)
                if matches!(s.vis, Visibility::Public(_))
                    && !is_disabled_cfg(&s.attrs)
                    && s.generics.params.is_empty()
                    && root_exports.contains(&s.ident.to_string()) =>
            {
                let s_name = s.ident.to_string();
                if !structs.contains(&s_name) {
                    structs.push(s_name.clone());
                    struct_files.insert(s_name.clone(), file_path.clone());
                    let has_default = s.attrs.iter().any(|a| {
                        a.path().is_ident("derive")
                            && a.meta
                                .require_list()
                                .map(|l| l.tokens.to_string().contains("Default"))
                                .unwrap_or(false)
                    });
                    if has_default {
                        default_structs.push(s_name);
                    }
                }
            }
            _ => {}
        }
    }

    let enum_names: Vec<String> = enums.iter().map(|e| e.name.clone()).collect();
    let mut methods = Vec::new();

    for s_name in &default_structs {
        let s_file = struct_files
            .get(s_name)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        methods.push(MethodInfo {
            struct_name: Some(s_name.clone()),
            name: "default".to_string(),
            receiver: None,
            is_async: false,
            args: Vec::new(),
            arg_metas: Vec::new(),
            ret: RetKind::Handle(s_name.clone()),
            ret_str: s_name.clone(),
            doc: format!("Returns the default `{}`.", s_name),
            file: s_file,
            line: 0,
        });
    }

    // Pass 2: collect public methods on structs (only in the struct's defining file or crate root)
    for (file_path, item) in &all_items {
        match item {
            Item::Impl(imp) if imp.generics.params.is_empty() && !is_disabled_cfg(&imp.attrs) => {
                let Some(struct_name) = type_ident(&imp.self_ty) else {
                    continue;
                };
                if !structs.contains(&struct_name) {
                    continue;
                }
                if struct_files.get(&struct_name) != Some(file_path) {
                    continue;
                }
                if let Some((_, trait_path, _)) = &imp.trait_ {
                    if trait_path
                        .segments
                        .last()
                        .map(|s| s.ident == "Default")
                        .unwrap_or(false)
                        && !methods.iter().any(|m| {
                            m.struct_name.as_deref() == Some(&struct_name) && m.name == "default"
                        })
                    {
                        methods.push(MethodInfo {
                            struct_name: Some(struct_name.clone()),
                            name: "default".to_string(),
                            receiver: None,
            is_async: false,
                            args: Vec::new(),
                            arg_metas: Vec::new(),
                            ret: RetKind::Handle(struct_name.clone()),
                            ret_str: struct_name.clone(),
                            doc: format!("Returns the default `{}`.", struct_name),
                            file: file_path.to_string_lossy().to_string(),
                            line: find_line_in_file(file_path, "fn default("),
                        });
                    }
                    continue;
                }
                for impl_item in &imp.items {
                    if let ImplItem::Fn(m) = impl_item {
                        if !matches!(m.vis, Visibility::Public(_))
                            || !m.sig.generics.params.is_empty()
                            || is_disabled_cfg(&m.attrs)
                        {
                            continue;
                        }
                        if let Some(mut info) =
                            parse_sig(Some(&struct_name), &m.sig, &enum_names, &structs)
                        {
                            info.doc = extract_docs(&m.attrs);
                            info.file = file_path.to_string_lossy().to_string();
                            info.line = find_line_in_file(file_path, &format!("fn {}(", info.name));
                            if !methods
                                .iter()
                                .any(|x| x.struct_name == info.struct_name && x.name == info.name)
                            {
                                methods.push(info);
                            }
                        }
                    }
                }
            }
            Item::Fn(f)
                if matches!(f.vis, Visibility::Public(_))
                    && f.sig.generics.params.is_empty()
                    && !is_disabled_cfg(&f.attrs) =>
            {
                if let Some(mut info) = parse_sig(None, &f.sig, &enum_names, &structs) {
                    info.doc = extract_docs(&f.attrs);
                    info.file = file_path.to_string_lossy().to_string();
                    info.line = find_line_in_file(file_path, &format!("fn {}(", info.name));
                    if !methods
                        .iter()
                        .any(|x| x.struct_name == info.struct_name && x.name == info.name)
                    {
                        methods.push(info);
                    }
                }
            }
            _ => {}
        }
    }

    CrateApi {
        crate_name: crate_name.to_string(),
        enums,
        structs,
        methods,
    }
}

fn type_ident(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

fn find_line_in_file(path: &Path, needle: &str) -> u32 {
    let Ok(src) = fs::read_to_string(path) else {
        return 0;
    };
    for (idx, line) in src.lines().enumerate() {
        if line.contains(needle) {
            return idx as u32;
        }
    }
    0
}

fn extract_docs(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(ls) = &el.lit {
                        lines.push(ls.value().trim().to_string());
                    }
                }
            }
        }
    }
    lines.join("\n")
}

fn striv_type_label(kind: &ArgKind) -> String {
    match kind {
        ArgKind::Int(_) => "int".to_string(),
        ArgKind::Bool => "bool".to_string(),
        ArgKind::Float(_) => "float".to_string(),
        ArgKind::StrRef => "String".to_string(),
        ArgKind::U32Slice => "Vec".to_string(),
        ArgKind::U8Slice => "Bytes".to_string(),
        ArgKind::IntoStr | ArgKind::AsPath => "String".to_string(),
        ArgKind::MutFloat(_) => "float".to_string(),
        ArgKind::MutInt(_) => "int".to_string(),
        ArgKind::Enum(e) => e.clone(),
        ArgKind::Handle(h) | ArgKind::HandleRef(h, _) => h.clone(),
    }
}

fn striv_ret_label(ret: &RetKind) -> String {
    match ret {
        RetKind::Unit | RetKind::ResultUnit => String::new(),
        RetKind::Int => "int".to_string(),
        RetKind::Bool => "bool".to_string(),
        RetKind::Float => "float".to_string(),
        RetKind::Handle(s) | RetKind::ResultHandle(s) => s.clone(),
    }
}

fn parse_sig(
    struct_name: Option<&str>,
    sig: &syn::Signature,
    enums: &[String],
    structs: &[String],
) -> Option<MethodInfo> {
    let mut receiver = None;
    let mut args = Vec::new();
    let mut arg_metas = Vec::new();

    for input in &sig.inputs {
        match input {
            FnArg::Receiver(r) => {
                r.reference.as_ref()?; // Skip consuming `self` for now
                receiver = Some(r.mutability.is_some());
            }
            FnArg::Typed(pt) => {
                let kind = classify_arg(&pt.ty, enums, structs)?;
                let p_name = match &*pt.pat {
                    syn::Pat::Ident(pi) => pi.ident.to_string(),
                    _ => format!("arg{}", args.len()),
                };
                arg_metas.push((p_name, striv_type_label(&kind)));
                args.push(kind);
            }
        }
    }

    let ret = classify_ret(&sig.output, struct_name, structs)?;
    let ret_str = striv_ret_label(&ret);
    Some(MethodInfo {
        struct_name: struct_name.map(|s| s.to_string()),
        name: sig.ident.to_string(),
        receiver,
        is_async: sig.asyncness.is_some(),
        args,
        arg_metas,
        ret,
        ret_str,
        doc: String::new(),
        file: String::new(),
        line: 0,
    })
}

fn classify_arg(ty: &Type, enums: &[String], structs: &[String]) -> Option<ArgKind> {
    match ty {
        Type::Path(tp) => {
            let ident = tp.path.segments.last()?.ident.to_string();
            match ident.as_str() {
                "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
                    Some(ArgKind::Int(ident))
                }
                "bool" => Some(ArgKind::Bool),
                "f32" | "f64" => Some(ArgKind::Float(ident)),
                other if enums.iter().any(|e| e == other) => Some(ArgKind::Enum(other.to_string())),
                other if structs.iter().any(|s| s == other) => {
                    Some(ArgKind::Handle(other.to_string()))
                }
                _ => None,
            }
        }
        Type::Reference(r) => {
            if let Type::Path(tp) = &*r.elem {
                let ident = tp.path.segments.last()?.ident.to_string();
                if ident == "str" && r.mutability.is_none() {
                    return Some(ArgKind::StrRef);
                }
            }
            if let Type::Slice(sl) = &*r.elem {
                if type_ident(&sl.elem).as_deref() == Some("u32") && r.mutability.is_none() {
                    return Some(ArgKind::U32Slice);
                }
                if type_ident(&sl.elem).as_deref() == Some("u8") && r.mutability.is_none() {
                    return Some(ArgKind::U8Slice);
                }
            }
            if r.mutability.is_some() {
                if let Some(id) = type_ident(&r.elem) {
                    if matches!(id.as_str(), "f32" | "f64") {
                        return Some(ArgKind::MutFloat(id));
                    }
                    if matches!(id.as_str(), "i32" | "i64" | "u32" | "u64" | "isize" | "usize" | "bool") {
                        return Some(ArgKind::MutInt(id));
                    }
                }
            }
            None
        }
        Type::ImplTrait(it) => {
            for b in &it.bounds {
                if let syn::TypeParamBound::Trait(tb) = b {
                    let s = tb
                        .path
                        .segments
                        .iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if s.contains("Path") {
                        return Some(ArgKind::AsPath);
                    } else if s.contains("Into") || s.contains("str") || s.contains("ToString") {
                        return Some(ArgKind::IntoStr);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn classify_ret(
    output: &ReturnType,
    self_struct: Option<&str>,
    structs: &[String],
) -> Option<RetKind> {
    let Type::Path(tp) = (match output {
        ReturnType::Default => return Some(RetKind::Unit),
        ReturnType::Type(_, ty) => &**ty,
    }) else {
        return None;
    };

    let seg = tp.path.segments.last()?;
    let ident = seg.ident.to_string();
    match ident.as_str() {
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            Some(RetKind::Int)
        }
        "bool" => Some(RetKind::Bool),
        "f32" | "f64" => Some(RetKind::Float),
        "Self" => self_struct.map(|s| RetKind::Handle(s.to_string())),
        "Result" => {
            if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(GenericArgument::Type(ok_ty)) = ab.args.first() {
                    if let Type::Tuple(t) = ok_ty {
                        if t.elems.is_empty() {
                            return Some(RetKind::ResultUnit);
                        }
                    }
                    if let Some(ok_name) = type_ident(ok_ty) {
                        if ok_name == "Self" {
                            return self_struct.map(|s| RetKind::ResultHandle(s.to_string()));
                        }
                        if structs.contains(&ok_name) {
                            return Some(RetKind::ResultHandle(ok_name));
                        }
                    }
                }
            }
            None
        }
        other if structs.iter().any(|s| s == other) => Some(RetKind::Handle(other.to_string())),
        _ => None,
    }
}
