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
}

#[derive(Debug, Clone)]
pub enum ArgKind {
    Int(String),             // i8..i64, u8..u64, isize, usize
    Bool,                    // bool
    Float(String),           // f32, f64
    StrRef,                  // &str
    U32Slice,                // &[u32]
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
    pub args: Vec<ArgKind>,
    pub ret: RetKind,
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

pub fn scan_crate_api(crate_name: &str, lib_rs: &Path) -> CrateApi {
    let source = fs::read_to_string(lib_rs)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", lib_rs.display(), e));
    let file = syn::parse_file(&source)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", lib_rs.display(), e));

    let mut enums = Vec::new();
    let mut structs = Vec::new();

    // Pass 1: collect public unit-only enums and public structs
    for item in &file.items {
        match item {
            Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                let all_unit = e.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
                if all_unit && !e.variants.is_empty() {
                    enums.push(EnumInfo {
                        name: e.ident.to_string(),
                        variants: e.variants.iter().map(|v| v.ident.to_string()).collect(),
                    });
                }
            }
            Item::Struct(s)
                if matches!(s.vis, Visibility::Public(_)) && s.generics.params.is_empty() =>
            {
                structs.push(s.ident.to_string());
            }
            _ => {}
        }
    }

    let enum_names: Vec<String> = enums.iter().map(|e| e.name.clone()).collect();
    let mut methods = Vec::new();

    // Pass 2: collect public methods on structs and public free functions
    for item in &file.items {
        match item {
            Item::Impl(imp) if imp.trait_.is_none() && imp.generics.params.is_empty() => {
                let Some(struct_name) = type_ident(&imp.self_ty) else {
                    continue;
                };
                if !structs.contains(&struct_name) {
                    continue;
                }
                for impl_item in &imp.items {
                    if let ImplItem::Fn(m) = impl_item {
                        if !matches!(m.vis, Visibility::Public(_))
                            || !m.sig.generics.params.is_empty()
                        {
                            continue;
                        }
                        if let Some(info) =
                            parse_sig(Some(&struct_name), &m.sig, &enum_names, &structs)
                        {
                            methods.push(info);
                        }
                    }
                }
            }
            Item::Fn(f)
                if matches!(f.vis, Visibility::Public(_)) && f.sig.generics.params.is_empty() =>
            {
                if let Some(info) = parse_sig(None, &f.sig, &enum_names, &structs) {
                    methods.push(info);
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

fn parse_sig(
    struct_name: Option<&str>,
    sig: &syn::Signature,
    enums: &[String],
    structs: &[String],
) -> Option<MethodInfo> {
    let mut receiver = None;
    let mut args = Vec::new();

    for input in &sig.inputs {
        match input {
            FnArg::Receiver(r) => {
                r.reference.as_ref()?; // Skip consuming `self` for now
                receiver = Some(r.mutability.is_some());
            }
            FnArg::Typed(pt) => {
                args.push(classify_arg(&pt.ty, enums, structs)?);
            }
        }
    }

    let ret = classify_ret(&sig.output, struct_name, structs)?;
    Some(MethodInfo {
        struct_name: struct_name.map(|s| s.to_string()),
        name: sig.ident.to_string(),
        receiver,
        args,
        ret,
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
