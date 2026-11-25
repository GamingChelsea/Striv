use crate::{Expr, Function, Op, Program, Stmt};

/// Source-preserving formatter: normalizes indentation (4 spaces per `{}` block),
/// preserves `//` comments and single blank lines, and ensures a trailing newline.
pub fn format_source(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut indent: usize = 0;
    let mut prev_blank = false;

    for raw_line in src.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if !out.is_empty() && !prev_blank {
                out.push('\n');
                prev_blank = true;
            }
            continue;
        }
        prev_blank = false;

        let leading_closes = trimmed.chars().take_while(|&c| c == '}').count();
        let line_indent = indent.saturating_sub(leading_closes);
        for _ in 0..line_indent {
            out.push_str("    ");
        }
        out.push_str(trimmed);
        out.push('\n');

        let (opens, closes) = count_braces_outside_strings_and_comments(trimmed);
        indent = indent.saturating_add(opens).saturating_sub(closes);
    }

    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

fn count_braces_outside_strings_and_comments(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            break;
        }
        match c {
            '"' => in_str = true,
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

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

    for (n, expr) in &prog.consts {
        out.push_str(&format!("const {} = {}\n", n, format_expr(expr)));
    }
    if !prog.consts.is_empty() {
        out.push('\n');
    }

    for e in &prog.enums {
        out.push_str(&format!("enum {} {{\n", e.name));
        for v in &e.variants {
            if v.field_types.is_empty() {
                out.push_str(&format!("    {},\n", v.name));
            } else {
                out.push_str(&format!("    {}({}),\n", v.name, v.field_types.join(", ")));
            }
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
            ty_hint,
            is_mut,
            value,
        } => {
            let m = if *is_mut { "mut " } else { "" };
            let ty_s = ty_hint
                .as_ref()
                .map(|t| format!(": {}", t))
                .unwrap_or_default();
            out.push_str(&format!(
                "{}let {}{}{} = {}\n",
                ind,
                m,
                name,
                ty_s,
                format_expr(value)
            ));
        }
        Stmt::Break => out.push_str(&format!("{}break\n", ind)),
        Stmt::Continue => out.push_str(&format!("{}continue\n", ind)),
        Stmt::Query { .. } => out.push_str(&format!("{}// query\n", ind)),
        Stmt::Defer(inner) => {
            out.push_str(&format!("{}defer ", ind));
            if let Stmt::Block(stmts) = &**inner {
                out.push_str("{\n");
                for s in stmts {
                    format_stmt(s, level + 1, out);
                }
                out.push_str(&format!("{}}}\n", ind));
            } else {
                let mut tmp = String::new();
                format_stmt(inner, 0, &mut tmp);
                out.push_str(tmp.trim_start());
            }
        }
        Stmt::Block(stmts) => {
            out.push_str(&format!("{}{{\n", ind));
            for s in stmts {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Yield => {
            out.push_str(&format!("{}yield()\n", ind));
        }
        Stmt::BlockCall { name, args, body } => {
            if args.is_empty() {
                out.push_str(&format!("{}{} {{\n", ind, name));
            } else {
                out.push_str(&format!("{}{}({}) {{\n", ind, name, format_args(args)));
            }
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Select { arms, timeout } => {
            out.push_str(&format!("{}select {{\n", ind));
            for arm in arms {
                out.push_str(&format!(
                    "{}    {} = {}.recv() => {{\n",
                    ind, arm.bind_var, arm.channel_var
                ));
                for s in &arm.body {
                    format_stmt(s, level + 2, out);
                }
                out.push_str(&format!("{}    }}\n", ind));
            }
            if let Some((ms_expr, t_body)) = timeout {
                out.push_str(&format!(
                    "{}    timeout({}) => {{\n",
                    ind,
                    format_expr(ms_expr)
                ));
                for s in t_body {
                    format_stmt(s, level + 2, out);
                }
                out.push_str(&format!("{}    }}\n", ind));
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Shared { name, value } => {
            out.push_str(&format!(
                "{}shared {} = {}\n",
                ind,
                name,
                format_expr(value)
            ));
        }
        Stmt::Lock { var, body } => {
            out.push_str(&format!("{}lock {} {{\n", ind, var));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Concurrent(body) => {
            out.push_str(&format!("{}concurrent {{\n", ind));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
        Stmt::Spawn { name, args } => {
            out.push_str(&format!("{}spawn {}({})\n", ind, name, format_args(args)));
        }
        Stmt::ForEach {
            var,
            is_mut,
            is_parallel,
            collection,
            body,
        } => {
            let p = if *is_parallel { "parallel " } else { "" };
            let m = if *is_mut { "mut " } else { "" };
            out.push_str(&format!(
                "{}{}for {}{} in {} {{\n",
                ind, p, m, var, collection
            ));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}}}\n", ind));
        }
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
                "{}let ({}) = {}\n",
                ind,
                parts.join(", "),
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
                    crate::MatchPat::EnumDestructure {
                        enum_name,
                        variant,
                        bindings,
                    } => format!("{}::{}({})", enum_name, variant, bindings.join(", ")),
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

pub fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Int(v) => v.to_string(),
        Expr::Hex(v) => format!("0x{:X}", v),
        Expr::Ok(e) => format!("Ok({})", format_expr(e)),
        Expr::Err(e) => format!("Err({})", format_expr(e)),
        Expr::Some(e) => format!("Some({})", format_expr(e)),
        Expr::None => "None".to_string(),
        Expr::Try(e) => format!("{}?", format_expr(e)),
        Expr::Float(v) | Expr::UnitFloat(v, _) => format!("{:?}", v),
        Expr::PathConst { ty, member } => format!("{}::{}", ty, member),
        Expr::VecLit(items) => format!("[{}]", format_args(items)),
        Expr::Tuple(items) => format!("({})", format_args(items)),
        Expr::TupleIndex { tuple, idx } => format!("{}.{}", format_expr(tuple), idx),
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
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
    }
}
