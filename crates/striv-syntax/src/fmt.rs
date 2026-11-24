use crate::{Expr, Function, Op, Program, Stmt};

pub fn format_program(prog: &Program) -> String {
    let mut out = String::new();

    for imp in &prog.imports {
        out.push_str(&format!("use {}\n", imp));
    }
    if !prog.imports.is_empty() {
        out.push('\n');
    }

    for (n, v) in &prog.tweaks {
        match v {
            crate::TweakVal::Int(i) => out.push_str(&format!("tweak {} = {}\n", n, i)),
            crate::TweakVal::Str(s) => out.push_str(&format!("tweak {} = \"{}\"\n", n, s)),
            crate::TweakVal::Vec(v) => {
                let s: Vec<String> = v.iter().map(|x| x.to_string()).collect();
                out.push_str(&format!("tweak {} = [{}]\n", n, s.join(", ")));
            }
        }
    }
    if !prog.tweaks.is_empty() {
        out.push('\n');
    }

    for e in &prog.enums {
        out.push_str(&format!("enum {} {{\n", e.name));
        for v in &e.variants {
            out.push_str(&format!("    {},\n", v));
        }
        out.push_str("}\n\n");
    }

    // 1. Print all structs
    for s in &prog.structs {
        out.push_str(&format!("struct {} {{\n", s.name));
        for (f, ty) in &s.fields {
            out.push_str(&format!("    {}: {},\n", f, ty));
        }
        out.push_str("}\n\n");
    }

    // 2. Group impl methods (identified by `Struct__method`) per struct
    for s in &prog.structs {
        let prefix = format!("{}__", s.name);
        let methods: Vec<&Function> = prog
            .functions
            .iter()
            .filter(|f| f.name.starts_with(&prefix))
            .collect();
        if !methods.is_empty() {
            out.push_str(&format!("impl {} {{\n", s.name));
            for (i, m) in methods.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let short_name = &m.name[prefix.len()..];
                format_fn(m, short_name, 1, &mut out);
            }
            out.push_str("}\n\n");
        }
    }

    // 3. Print free functions
    let free_funcs: Vec<&Function> = prog
        .functions
        .iter()
        .filter(|f| !f.name.contains("__"))
        .collect();
    for (i, f) in free_funcs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        format_fn(f, &f.name, 0, &mut out);
    }

    out.trim_end().to_string() + "\n"
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn format_fn(f: &Function, display_name: &str, level: usize, out: &mut String) {
    let ind = indent(level);
    let params_str: Vec<String> = f
        .params
        .iter()
        .map(|(name, ty, is_mut)| {
            let m = if *is_mut { "mut " } else { "" };
            if name == "self" {
                format!("{}self", m)
            } else {
                format!("{}{}: {}", m, name, ty)
            }
        })
        .collect();
    let ret_str = match &f.ret_type {
        Some(rt) => format!(" -> {}", rt),
        None => String::new(),
    };
    out.push_str(&format!(
        "{}fn {}({}){} {{\n",
        ind,
        display_name,
        params_str.join(", "),
        ret_str
    ));
    for stmt in &f.body {
        format_stmt(stmt, level + 1, out);
    }
    out.push_str(&format!("{}}}\n", ind));
}

fn format_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    let ind = indent(level);
    match stmt {
        Stmt::Let {
            name,
            is_mut,
            value,
        } => {
            let m = if *is_mut { "mut " } else { "" };
            out.push_str(&format!(
                "{}let {}{} = {}\n",
                ind,
                m,
                name,
                format_expr(value)
            ));
        }
        Stmt::Assign { name, value } => {
            out.push_str(&format!("{}{} = {}\n", ind, name, format_expr(value)));
        }
        Stmt::FieldAssign { var, field, value } => {
            out.push_str(&format!(
                "{}{}.{} = {}\n",
                ind,
                var,
                field,
                format_expr(value)
            ));
        }
        Stmt::Print(e) => {
            out.push_str(&format!("{}print({})\n", ind, format_expr(e)));
        }
        Stmt::IndexAssign { var, index, value } => {
            out.push_str(&format!(
                "{}{}[{}] = {}\n",
                ind,
                var,
                format_expr(index),
                format_expr(value)
            ));
        }
        Stmt::Call { name, args } => {
            out.push_str(&format!("{}{}({})\n", ind, name, format_args(args)));
        }
        Stmt::Expr(e) => {
            out.push_str(&format!("{}{}\n", ind, format_expr(e)));
        }
        Stmt::MethodCall {
            receiver,
            method,
            args,
        } => {
            out.push_str(&format!(
                "{}{}.{}({})\n",
                ind,
                receiver,
                method,
                format_args(args)
            ));
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            out.push_str(&format!("{}if {} {{\n", ind, format_expr(cond)));
            for s in then_block {
                format_stmt(s, level + 1, out);
            }
            if !else_block.is_empty() {
                out.push_str(&format!("{}}} else {{\n", ind));
                for s in else_block {
                    format_stmt(s, level + 1, out);
                }
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{}while {} {{\n", ind, format_expr(cond)));
            for s in body {
                format_stmt(s, level + 1, out);
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
                "{}for {} in {}..{} {{\n",
                ind,
                var,
                format_expr(start),
                format_expr(end)
            ));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::CompoundAssign { target, op, value } => {
            let op_str = if *op == Op::Add { "+=" } else { "-=" };
            let lhs = match target {
                crate::AssignTarget::Var(n) => n.clone(),
                crate::AssignTarget::Field { var, field } => format!("{}.{}", var, field),
                crate::AssignTarget::Index { var, index } => {
                    format!("{}[{}]", var, format_expr(index))
                }
            };
            out.push_str(&format!(
                "{}{} {} {}\n",
                ind,
                lhs,
                op_str,
                format_expr(value)
            ));
        }
        Stmt::Match { target, arms } => {
            out.push_str(&format!("{}match {} {{\n", ind, format_expr(target)));
            for (pat, body) in arms {
                let pat_str = match pat {
                    crate::MatchPat::Wildcard => "_".to_string(),
                    crate::MatchPat::Expr(e) => format_expr(e),
                };
                out.push_str(&format!("{}    {} => {{\n", ind, pat_str));
                for s in body {
                    format_stmt(s, level + 2, out);
                }
                out.push_str(&format!("{}    }}\n", ind));
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Return(e) => {
            out.push_str(&format!("{}return {}\n", ind, format_expr(e)));
        }
    }
}

fn format_args(args: &[Expr]) -> String {
    args.iter().map(format_expr).collect::<Vec<_>>().join(", ")
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Int(v) => v.to_string(),
        Expr::Hex(v) => format!("0x{:X}", v),
        Expr::Ok(e) => format!("Ok({})", format_expr(e)),
        Expr::Err(e) => format!("Err({})", format_expr(e)),
        Expr::Some(e) => format!("Some({})", format_expr(e)),
        Expr::None => "None".to_string(),
        Expr::Try(e) => format!("{}?", format_expr(e)),
        Expr::Float(v) => format!("{:?}", v),
        Expr::PathConst { ty, member } => format!("{}::{}", ty, member),
        Expr::VecLit(items) => format!("[{}]", format_args(items)),
        Expr::Index { var, index } => format!("{}[{}]", var, format_expr(index)),
        Expr::StrInterp(parts) => {
            let mut s = String::from("\"");
            for p in parts {
                match p {
                    crate::StringPart::Literal(lit) => s.push_str(lit),
                    crate::StringPart::Expr(e) => s.push_str(&format!("{{{}}}", format_expr(e))),
                }
            }
            s.push('"');
            s
        }
        Expr::Var(n) => n.clone(),
        Expr::FieldAccess { var, field } => format!("{}.{}", var, field),
        Expr::Call { name, args } => {
            format!("{}({})", name.replace("__", "::"), format_args(args))
        }
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => format!("{}.{}({})", receiver, method, format_args(args)),
        Expr::StructInit { name, fields } => {
            let f_str: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr(v)))
                .collect();
            format!("{} {{ {} }}", name, f_str.join(", "))
        }
        Expr::BinOp { left, op, right } => {
            if *op == Op::Sub && matches!(&**left, Expr::Int(0)) {
                return format!("-{}", format_expr(right));
            }
            if *op == Op::Sub {
                if let Expr::Float(f) = &**left {
                    if *f == 0.0 {
                        return format!("-{}", format_expr(right));
                    }
                }
            }
            let op_str = match op {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div => "/",
                Op::Lt => "<",
                Op::Gt => ">",
                Op::Eq => "==",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
    }
}
