use crate::{AssignTarget, Expr, Function, MatchPat, Op, Program, Stmt, StringPart, TweakVal};
use std::collections::{HashMap, HashSet};

pub fn transpile_to_rust(prog: &Program) -> String {
    transpile_to_rust_with_consts(prog, &HashMap::new())
}

pub fn transpile_to_rust_with_consts(
    prog: &Program,
    comptime_consts: &HashMap<String, i64>,
) -> String {
    let mut out = String::from(
        "#![allow(unused_variables, unused_mut, non_snake_case, non_upper_case_globals, dead_code, unused_imports)]\n\n",
    );

    let struct_names: HashSet<String> = prog.structs.iter().map(|s| s.name.clone()).collect();

    // 1. Tweaks become zero-cost constants in release builds
    for (name, val) in &prog.tweaks {
        match val {
            TweakVal::Int(i) => out.push_str(&format!("const {}: i64 = {};\n", name, i)),
            TweakVal::Str(s) => out.push_str(&format!("const {}: &str = {:?};\n", name, s)),
            TweakVal::Vec(v) => {
                let items: Vec<String> = v.iter().map(|x| x.to_string()).collect();
                out.push_str(&format!(
                    "const {}: &[i64] = &[{}];\n",
                    name,
                    items.join(", ")
                ));
            }
        }
    }
    if !prog.tweaks.is_empty() {
        out.push('\n');
    }

    for (name, expr) in &prog.consts {
        if let Some(&val) = comptime_consts.get(name) {
            out.push_str(&format!("const {}: i64 = {}i64;\n", name, val));
        } else {
            out.push_str(&format!(
                "const {}: i64 = {};\n",
                name,
                emit_expr(expr, &struct_names)
            ));
        }
    }
    if !prog.consts.is_empty() {
        out.push('\n');
    }

    if !prog.extern_fns.is_empty() {
        let mut by_lib: HashMap<&str, Vec<&crate::ExternFn>> = HashMap::new();
        for ext in &prog.extern_fns {
            by_lib.entry(ext.lib.as_str()).or_default().push(ext);
        }
        for (lib, fns) in by_lib {
            let link_name = lib
                .strip_suffix(".dll")
                .or_else(|| lib.strip_suffix(".so"))
                .or_else(|| lib.strip_suffix(".dylib"))
                .unwrap_or(lib);
            let link_name = link_name.strip_prefix("lib").unwrap_or(link_name);
            out.push_str(&format!("#[link(name = \"{}\")]\nunsafe extern \"C\" {{\n", link_name));
            for ext in fns {
                let params: Vec<String> = ext
                    .params
                    .iter()
                    .map(|(p, t)| format!("{}: {}", p, map_type(t)))
                    .collect();
                let ret = ext
                    .ret_type
                    .as_ref()
                    .map(|r| format!(" -> {}", map_type(r)))
                    .unwrap_or_default();
                out.push_str(&format!("    fn {}({}){};\n", ext.name, params.join(", "), ret));
            }
            out.push_str("}\n\n");
        }
    }

    // 2. User-defined enums -> constant i64 modules
    for e in &prog.enums {
        out.push_str(&format!("pub mod {} {{\n", e.name));
        for (i, v) in e.variants.iter().enumerate() {
            out.push_str(&format!("    pub const {}: i64 = {};\n", v.name, i));
        }
        out.push_str("}\n\n");
    }

    // 3. Structs
    for s in &prog.structs {
        out.push_str("#[derive(Debug, Clone)]\n");
        out.push_str(&format!("pub struct {} {{\n", s.name));
        for (f_name, f_ty) in &s.fields {
            out.push_str(&format!("    pub {}: {},\n", f_name, map_type(f_ty)));
        }
        out.push_str("}\n\n");
    }

    // 4. Impl blocks
    for s in &prog.structs {
        let prefix = format!("{}__", s.name);
        let methods: Vec<&Function> = prog
            .functions
            .iter()
            .filter(|f| f.name.starts_with(&prefix))
            .collect();

        if !methods.is_empty() {
            out.push_str(&format!("impl {} {{\n", s.name));
            for m in methods {
                let short_name = &m.name[prefix.len()..];
                emit_fn(m, short_name, &struct_names, 1, &mut out);
            }
            out.push_str("}\n\n");
        }
    }

    // 5. Free functions (`main` gets a Rust entry point)
    for f in prog.functions.iter().filter(|f| !f.name.contains("__")) {
        if f.name == "main" {
            emit_fn(f, "striv_main", &struct_names, 0, &mut out);
            out.push_str("\nfn main() {\n    let _ = striv_main();\n}\n");
        } else {
            emit_fn(f, &f.name, &struct_names, 0, &mut out);
        }
    }

    out
}

fn map_type(ty: &str) -> String {
    match ty {
        "int" => "i64".to_string(),
        "float" => "f64".to_string(),
        "bool" => "bool".to_string(),
        "cstr" => "*const u8".to_string(),
        "String" => "String".to_string(),
        "Vec" => "Vec<i64>".to_string(),
        "Result" => "Result<i64, i64>".to_string(),
        "Option" => "Option<i64>".to_string(),
        other => other.to_string(),
    }
}

fn emit_fn(f: &Function, name: &str, structs: &HashSet<String>, level: usize, out: &mut String) {
    let ind = "    ".repeat(level);
    let params: Vec<String> = f
        .params
        .iter()
        .map(|(p_name, p_ty, is_mut)| {
            if p_name == "self" {
                if *is_mut {
                    "&mut self".into()
                } else {
                    "&self".into()
                }
            } else if structs.contains(p_ty) || p_ty == "Vec" {
                let m = if *is_mut { "&mut " } else { "&" };
                format!("{}: {}{}", p_name, m, map_type(p_ty))
            } else {
                let m = if *is_mut { "mut " } else { "" };
                format!("{}{}: {}", m, p_name, map_type(p_ty))
            }
        })
        .collect();

    let ret = f
        .ret_type
        .as_ref()
        .map(|rt| format!(" -> {}", map_type(rt)))
        .unwrap_or_default();

    out.push_str(&format!(
        "{}pub fn {}({}){} {{\n",
        ind,
        name,
        params.join(", "),
        ret
    ));
    for stmt in &f.body {
        emit_stmt(stmt, structs, level + 1, out);
    }
    out.push_str(&format!("{}}}\n", ind));
}

fn emit_stmt(stmt: &Stmt, structs: &HashSet<String>, level: usize, out: &mut String) {
    let ind = "    ".repeat(level);
    match stmt {
        Stmt::LetTuple { names, value } => {
            let parts: Vec<String> = names
                .iter()
                .map(|(n, is_mut)| {
                    if *is_mut {
                        format!("mut {}", n)
                    } else {
                        n.clone()
                    }
                })
                .collect();
            out.push_str(&format!(
                "{}let ({}) = {};\n",
                ind,
                parts.join(", "),
                emit_expr(value, structs)
            ));
        }
        Stmt::Let {
            name,
            ty_hint: _,
            is_mut,
            value,
        } => {
            let m = if *is_mut { "mut " } else { "" };
            if matches!(value, Expr::VecLit(items) if items.is_empty()) {
                out.push_str(&format!(
                    "{}let {}{}: Vec<i64> = Vec::new();\n",
                    ind, m, name
                ));
            } else {
                out.push_str(&format!(
                    "{}let {}{} = {};\n",
                    ind,
                    m,
                    name,
                    emit_expr(value, structs)
                ));
            }
        }
        Stmt::Assign { name, value } => {
            out.push_str(&format!(
                "{}{} = {};\n",
                ind,
                name,
                emit_expr(value, structs)
            ));
        }
        Stmt::FieldAssign { var, field, value } => {
            out.push_str(&format!(
                "{}{}.{} = {};\n",
                ind,
                var,
                field,
                emit_expr(value, structs)
            ));
        }
        Stmt::IndexAssign { var, index, value } => {
            out.push_str(&format!(
                "{}{}[({}) as usize] = {};\n",
                ind,
                var,
                emit_expr(index, structs),
                emit_expr(value, structs)
            ));
        }
        Stmt::CompoundAssign { target, op, value } => {
            let op_s = if *op == Op::Add { "+=" } else { "-=" };
            let lhs = match target {
                AssignTarget::Var(n) => n.clone(),
                AssignTarget::Field { var, field } => format!("{}.{}", var, field),
                AssignTarget::Index { var, index } => {
                    format!("{}[({}) as usize]", var, emit_expr(index, structs))
                }
            };
            out.push_str(&format!(
                "{}{} {} {};\n",
                ind,
                lhs,
                op_s,
                emit_expr(value, structs)
            ));
        }
        Stmt::Print(e) => {
            if let Expr::StrInterp(parts) = e {
                out.push_str(&format!(
                    "{}println!({});\n",
                    ind,
                    emit_interp_args(">> ", parts, structs)
                ));
            } else {
                out.push_str(&format!(
                    "{}println!(\">> {{:?}}\", {});\n",
                    ind,
                    emit_expr(e, structs)
                ));
            }
        }
        Stmt::Call { name, args } => {
            out.push_str(&format!(
                "{}{}({});\n",
                ind,
                name.replace("__", "::"),
                emit_call_args(args, structs)
            ));
        }
        Stmt::MethodCall {
            receiver,
            method,
            args,
        } => {
            if method == "len" && args.is_empty() {
                out.push_str(&format!("{}({}.len() as i64);\n", ind, receiver));
            } else {
                out.push_str(&format!(
                    "{}{}.{}({});\n",
                    ind,
                    receiver,
                    method,
                    emit_call_args(args, structs)
                ));
            }
        }
        Stmt::Expr(e) => {
            out.push_str(&format!("{}{};\n", ind, emit_expr(e, structs)));
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            out.push_str(&format!("{}if {} {{\n", ind, emit_expr(cond, structs)));
            for s in then_block {
                emit_stmt(s, structs, level + 1, out);
            }
            if !else_block.is_empty() {
                out.push_str(&format!("{}}} else {{\n", ind));
                for s in else_block {
                    emit_stmt(s, structs, level + 1, out);
                }
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{}while {} {{\n", ind, emit_expr(cond, structs)));
            for s in body {
                emit_stmt(s, structs, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Break => out.push_str(&format!("{}break;\n", ind)),
        Stmt::Continue => out.push_str(&format!("{}continue;\n", ind)),
        Stmt::Query { .. } => {}
        Stmt::Defer(_) | Stmt::Block(_) | Stmt::Yield | Stmt::BlockCall { .. } => {
            out.push_str(&format!(
                "{}// defer/block not yet lowered in release_gen\n",
                ind
            ));
        }
        Stmt::Select { .. } => {
            out.push_str(&format!(
                "{}// select not yet lowered in release_gen\n",
                ind
            ));
        }
        Stmt::Shared { name, value } => {
            out.push_str(&format!(
                "{}let {} = std::sync::Arc::new(std::sync::Mutex::new({}));\n",
                ind,
                name,
                emit_expr(value, structs)
            ));
        }
        Stmt::Lock { var, body } => {
            out.push_str(&format!(
                "{}{{\n{}    let mut {} = {}.lock().unwrap();\n",
                ind, ind, var, var
            ));
            for s in body {
                emit_stmt(s, structs, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Concurrent(body) => {
            out.push_str(&format!("{}{{\n", ind));
            for s in body {
                emit_stmt(s, structs, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Spawn { name, args } => {
            out.push_str(&format!(
                "{}{}({});\n",
                ind,
                name.replace("__", "::"),
                emit_call_args(args, structs)
            ));
        }
        Stmt::ForEach {
            var,
            is_mut,
            is_parallel: _,
            collection,
            body,
        } => {
            let iter_fn = if *is_mut { "iter_mut" } else { "iter" };
            out.push_str(&format!(
                "{}for {} in {}.{}() {{\n",
                ind, var, collection, iter_fn
            ));
            for s in body {
                emit_stmt(s, structs, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::ForRange {
            var,
            start,
            end,
            body,
        } => {
            out.push_str(&format!(
                "{}for {} in ({})..({}) {{\n",
                ind,
                var,
                emit_expr(start, structs),
                emit_expr(end, structs)
            ));
            for s in body {
                emit_stmt(s, structs, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Match { target, arms } => {
            out.push_str(&format!("{}match {} {{\n", ind, emit_expr(target, structs)));
            for (pat, body) in arms {
                let p_str = match pat {
                    MatchPat::Wildcard => "_".to_string(),
                    MatchPat::Expr(e) => emit_expr(e, structs),
                    MatchPat::EnumDestructure { .. } => "_".to_string(),
                };
                out.push_str(&format!("{}    {} => {{\n", ind, p_str));
                for s in body {
                    emit_stmt(s, structs, level + 2, out);
                }
                out.push_str(&format!("{}    }}\n", ind));
            }
            out.push_str(&format!("{}    _ => {{}}\n{}}}\n", ind, ind));
        }
        Stmt::Return(e) => {
            out.push_str(&format!("{}return {};\n", ind, emit_expr(e, structs)));
        }
    }
}

fn emit_bool_expr(expr: &Expr, structs: &HashSet<String>) -> String {
    if let Expr::BinOp { op, .. } = expr {
        if matches!(
            op,
            Op::Lt | Op::Gt | Op::LtEq | Op::GtEq | Op::Eq | Op::NotEq | Op::And | Op::Or
        ) {
            return emit_expr(expr, structs);
        }
    }
    format!("({} != 0)", emit_expr(expr, structs))
}

fn emit_call_args(args: &[Expr], structs: &HashSet<String>) -> String {
    args.iter()
        .map(|a| emit_expr(a, structs))
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_interp_args(prefix: &str, parts: &[StringPart], structs: &HashSet<String>) -> String {
    let mut fmt_str = String::from(prefix);
    let mut fmt_args = Vec::new();
    for p in parts {
        match p {
            StringPart::Literal(lit) => fmt_str.push_str(lit),
            StringPart::Expr(e) => {
                fmt_str.push_str("{}");
                fmt_args.push(emit_expr(e, structs));
            }
        }
    }
    if fmt_args.is_empty() {
        format!("{:?}", fmt_str)
    } else {
        format!("{:?}, {}", fmt_str, fmt_args.join(", "))
    }
}

fn emit_expr(expr: &Expr, structs: &HashSet<String>) -> String {
    match expr {
        Expr::Int(v) => format!("{}i64", v),
        Expr::Hex(v) => format!("0x{:X}i64", v),
        Expr::Float(v) | Expr::UnitFloat(v, _) => format!("{:?}f64", v),
        Expr::Var(n) => n.clone(),
        Expr::PathConst { ty, member } => format!("{}::{}", ty, member),
        Expr::FieldAccess { var, field } => format!("{}.{}", var, field),
        Expr::Index { var, index } => format!("{}[({}) as usize]", var, emit_expr(index, structs)),
        Expr::VecLit(items) => format!("vec![{}]", emit_call_args(items, structs)),
        Expr::Tuple(items) => format!("({})", emit_call_args(items, structs)),
        Expr::TupleIndex { tuple, idx } => format!("{}.{}", emit_expr(tuple, structs), idx),
        Expr::Call { name, args } => {
            format!(
                "{}({})",
                name.replace("__", "::"),
                emit_call_args(args, structs)
            )
        }
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            if method == "len" && args.is_empty() {
                format!("({}.len() as i64)", receiver)
            } else {
                format!("{}.{}({})", receiver, method, emit_call_args(args, structs))
            }
        }
        Expr::StrInterp(parts) => format!("format!({})", emit_interp_args("", parts, structs)),
        Expr::StructInit { name, fields } => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, emit_expr(v, structs)))
                .collect();
            format!("{} {{ {} }}", name, fs.join(", "))
        }
        Expr::Ok(e) => format!("Ok({})", emit_expr(e, structs)),
        Expr::Err(e) => format!("Err({})", emit_expr(e, structs)),
        Expr::Some(e) => format!("Some({})", emit_expr(e, structs)),
        Expr::None => "None".to_string(),
        Expr::Try(e) => format!("{}?", emit_expr(e, structs)),
        Expr::BinOp { left, op, right } => {
            if matches!(op, Op::And | Op::Or) {
                let op_s = if *op == Op::And { "&&" } else { "||" };
                return format!(
                    "({} {} {})",
                    emit_bool_expr(left, structs),
                    op_s,
                    emit_bool_expr(right, structs)
                );
            }
            let op_s = match op {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div => "/",
                Op::Lt => "<",
                Op::Gt => ">",
                Op::LtEq => "<=",
                Op::GtEq => ">=",
                Op::Eq => "==",
                Op::NotEq => "!=",
                Op::And => "&&",
                Op::Or => "||",
                Op::BitOr => "|",
                Op::BitAnd => "&",
                Op::BitXor => "^",
                Op::Shl => "<<",
                Op::Shr => ">>",
            };
            format!(
                "({} {} {})",
                emit_expr(left, structs),
                op_s,
                emit_expr(right, structs)
            )
        }
    }
}
