use crate::{Expr, Op, Program, Stmt, StructDef, UnitExpr};
use std::collections::{BTreeMap, HashMap};

pub type Dim = BTreeMap<String, i32>;

fn mul_dim(a: &Dim, b: &Dim) -> Dim {
    let mut out = a.clone();
    for (k, v) in b {
        let entry = out.entry(k.clone()).or_insert(0);
        *entry += v;
        if *entry == 0 {
            out.remove(k);
        }
    }
    out
}

fn div_dim(a: &Dim, b: &Dim) -> Dim {
    let mut out = a.clone();
    for (k, v) in b {
        let entry = out.entry(k.clone()).or_insert(0);
        *entry -= v;
        if *entry == 0 {
            out.remove(k);
        }
    }
    out
}

pub fn format_dim(d: &Dim) -> String {
    if d.is_empty() {
        return "scalar (einheitslos)".to_string();
    }
    d.iter()
        .map(|(k, exp)| {
            if *exp == 1 {
                k.clone()
            } else {
                format!("{}^{}", k, exp)
            }
        })
        .collect::<Vec<_>>()
        .join(" * ")
}

fn eval_unit_expr(expr: &UnitExpr, known: &HashMap<String, Dim>) -> Result<Dim, String> {
    match expr {
        UnitExpr::Name(n) => known
            .get(n)
            .cloned()
            .ok_or_else(|| format!("Unbekannte Einheit '{}'", n)),
        UnitExpr::Mul(l, r) => Ok(mul_dim(&eval_unit_expr(l, known)?, &eval_unit_expr(r, known)?)),
        UnitExpr::Div(l, r) => Ok(div_dim(&eval_unit_expr(l, known)?, &eval_unit_expr(r, known)?)),
    }
}

struct UnitEnv<'a> {
    known_units: &'a HashMap<String, Dim>,
    structs: &'a HashMap<String, StructDef>,
    fn_sigs: &'a HashMap<String, (Vec<(String, Option<Dim>)>, Option<Dim>)>,
}

fn infer_dim(
    expr: &Expr,
    vars: &HashMap<String, (Option<Dim>, Option<String>)>,
    env: &UnitEnv,
) -> Result<Option<Dim>, String> {
    match expr {
        Expr::UnitFloat(_, u) => {
            let d = env
                .known_units
                .get(u)
                .ok_or_else(|| format!("Unbekannte Einheit '{}'", u))?;
            Ok(Some(d.clone()))
        }
        Expr::Float(_) | Expr::Int(_) => Ok(Some(Dim::new())),
        Expr::Var(n) => Ok(vars.get(n).and_then(|(d, _)| d.clone())),
        Expr::FieldAccess { var, field } => {
            if let Some((_, Some(s_name))) = vars.get(var) {
                if let Some(sdef) = env.structs.get(s_name) {
                    if let Some((_, f_ty)) = sdef.fields.iter().find(|(f, _)| f == field) {
                        if let Some(d) = env.known_units.get(f_ty) {
                            return Ok(Some(d.clone()));
                        }
                        return Ok(Some(Dim::new()));
                    }
                }
            }
            Ok(None)
        }
        Expr::BinOp { left, op, right } => {
            let ld = infer_dim(left, vars, env)?;
            let rd = infer_dim(right, vars, env)?;
            match op {
                Op::Add | Op::Sub | Op::Eq | Op::NotEq | Op::Lt | Op::Gt | Op::LtEq | Op::GtEq => {
                    if let (Some(l), Some(r)) = (&ld, &rd) {
                        if l != r {
                            return Err(format!(
                                "Einheiten-Fehler: Kann '{}' nicht mit '{}' verrechnen/vergleichen!",
                                format_dim(l),
                                format_dim(r)
                            ));
                        }
                        return Ok(Some(l.clone()));
                    }
                    Ok(ld.or(rd))
                }
                Op::Mul => match (ld, rd) {
                    (Some(l), Some(r)) => Ok(Some(mul_dim(&l, &r))),
                    _ => Ok(None),
                },
                Op::Div => match (ld, rd) {
                    (Some(l), Some(r)) => Ok(Some(div_dim(&l, &r))),
                    _ => Ok(None),
                },
                _ => Ok(Some(Dim::new())),
            }
        }
        Expr::Call { name, args } => {
            for a in args {
                let _ = infer_dim(a, vars, env)?;
            }
            if let Some((param_dims, ret_dim)) = env.fn_sigs.get(name) {
                for (i, a) in args.iter().enumerate() {
                    if let Some((_, Some(expected))) = param_dims.get(i) {
                        if let Some(actual) = infer_dim(a, vars, env)? {
                            if &actual != expected {
                                return Err(format!(
                                    "Einheiten-Fehler in Aufruf '{}': Parameter {} erwartet '{}', erhielt '{}'",
                                    name,
                                    i + 1,
                                    format_dim(expected),
                                    format_dim(&actual)
                                ));
                            }
                        }
                    }
                }
                return Ok(ret_dim.clone());
            }
            Ok(None)
        }
        Expr::StructInit { name, fields } => {
            if let Some(sdef) = env.structs.get(name) {
                for (f_name, f_expr) in fields {
                    if let Some(actual) = infer_dim(f_expr, vars, env)? {
                        if let Some((_, f_ty)) = sdef.fields.iter().find(|(f, _)| f == f_name) {
                            if let Some(expected) = env.known_units.get(f_ty) {
                                if &actual != expected {
                                    return Err(format!(
                                        "Einheiten-Fehler in '{}.{}': erwartet '{}', erhielt '{}'",
                                        name,
                                        f_name,
                                        format_dim(expected),
                                        format_dim(&actual)
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn check_stmts(
    stmts: &[Stmt],
    vars: &mut HashMap<String, (Option<Dim>, Option<String>)>,
    ret_dim: &Option<Dim>,
    env: &UnitEnv,
) -> Result<(), String> {
    for s in stmts {
        match s {
            Stmt::Declare { name, ty, .. } => {
                if let Some(dim) = env.known_units.get(ty) {
                    vars.insert(name.clone(), (Some(dim.clone()), None));
                } else {
                    vars.insert(name.clone(), (None, Some(ty.clone())));
                }
            }
            Stmt::Let {
                name,
                ty_hint,
                value,
                ..
            } => {
                let val_dim = infer_dim(value, vars, env)?;
                if let Some(hint) = ty_hint {
                    if let Some(expected) = env.known_units.get(hint) {
                        if let Some(actual) = &val_dim {
                            if actual != expected {
                                return Err(format!(
                                    "Einheiten-Fehler bei '{}': erwartet '{}', erhielt '{}'",
                                    name,
                                    format_dim(expected),
                                    format_dim(actual)
                                ));
                            }
                        }
                    }
                }
                let s_name = match value {
                    Expr::StructInit { name, .. } => Some(name.clone()),
                    _ => ty_hint
                        .as_ref()
                        .filter(|t| env.structs.contains_key(*t))
                        .cloned(),
                };
                vars.insert(name.clone(), (val_dim, s_name));
            }
            Stmt::Return(expr) => {
                let actual = infer_dim(expr, vars, env)?;
                if let (Some(exp), Some(act)) = (ret_dim, &actual) {
                    if exp != act {
                        return Err(format!(
                            "Einheiten-Fehler bei `return`: erwartet '{}', erhielt '{}'",
                            format_dim(exp),
                            format_dim(act)
                        ));
                    }
                }
            }
            Stmt::Expr(e) | Stmt::Print(e) => {
                let _ = infer_dim(e, vars, env)?;
            }
            Stmt::Call { args, .. } => {
                for a in args {
                    let _ = infer_dim(a, vars, env)?;
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let _ = infer_dim(cond, vars, env)?;
                check_stmts(then_block, vars, ret_dim, env)?;
                check_stmts(else_block, vars, ret_dim, env)?;
            }
            Stmt::While { cond, body } => {
                let _ = infer_dim(cond, vars, env)?;
                check_stmts(body, vars, ret_dim, env)?;
            }
            Stmt::ForRange { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Concurrent(body)
            | Stmt::Lock { body, .. }
            | Stmt::Block(body) => {
                check_stmts(body, vars, ret_dim, env)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn erase_expr(expr: &mut Expr) {
    match expr {
        Expr::UnitFloat(v, _) => *expr = Expr::Float(*v),
        Expr::BinOp { left, right, .. } => {
            erase_expr(left);
            erase_expr(right);
        }
        Expr::Call { args, .. }
        | Expr::MethodCall { args, .. }
        | Expr::VecLit(args)
        | Expr::Tuple(args) => {
            for a in args {
                erase_expr(a);
            }
        }
        Expr::StructInit { fields, .. } => {
            for (_, e) in fields {
                erase_expr(e);
            }
        }
        Expr::Index { index, .. } => erase_expr(index),
        _ => {}
    }
}

fn erase_stmts(stmts: &mut [Stmt], known: &HashMap<String, Dim>) {
    for s in stmts {
        match s {
            Stmt::Declare { ty, .. } => {
                if known.contains_key(ty) {
                    *ty = "float".to_string();
                }
            }
            Stmt::Let { ty_hint, value, .. } => {
                if let Some(h) = ty_hint {
                    if known.contains_key(h) {
                        *h = "float".to_string();
                    }
                }
                erase_expr(value);
            }
            Stmt::Assign { value, .. }
            | Stmt::FieldAssign { value, .. }
            | Stmt::Return(value)
            | Stmt::Print(value)
            | Stmt::Expr(value) => erase_expr(value),
            Stmt::Call { args, .. } | Stmt::MethodCall { args, .. } => {
                for a in args {
                    erase_expr(a);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                erase_expr(cond);
                erase_stmts(then_block, known);
                erase_stmts(else_block, known);
            }
            Stmt::While { cond, body } => {
                erase_expr(cond);
                erase_stmts(body, known);
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                erase_expr(start);
                erase_expr(end);
                erase_stmts(body, known);
            }
            Stmt::ForEach { body, .. }
            | Stmt::Concurrent(body)
            | Stmt::Lock { body, .. }
            | Stmt::Block(body) => erase_stmts(body, known),
            Stmt::Match { target, arms } => {
                erase_expr(target);
                for (_, body) in arms {
                    erase_stmts(body, known);
                }
            }
            _ => {}
        }
    }
}

pub fn check_and_erase_units(prog: &mut Program) -> Result<(), String> {
    if prog.units.is_empty() {
        return Ok(());
    }

    let mut known_units: HashMap<String, Dim> = HashMap::new();
    for u in &prog.units {
        match &u.formula {
            None => {
                let mut d = Dim::new();
                d.insert(u.name.clone(), 1);
                known_units.insert(u.name.clone(), d);
            }
            Some(f) => {
                let d = eval_unit_expr(f, &known_units)?;
                known_units.insert(u.name.clone(), d);
            }
        }
    }

    let structs: HashMap<String, StructDef> = prog
        .structs
        .iter()
        .map(|s| (s.name.clone(), s.clone()))
        .collect();
    let mut fn_sigs = HashMap::new();
    for f in &prog.functions {
        let p_dims = f
            .params
            .iter()
            .map(|(n, ty, _)| (n.clone(), known_units.get(ty).cloned()))
            .collect();
        let r_dim = f
            .ret_type
            .as_ref()
            .and_then(|rt| known_units.get(rt).cloned());
        fn_sigs.insert(f.name.clone(), (p_dims, r_dim));
    }

    let env = UnitEnv {
        known_units: &known_units,
        structs: &structs,
        fn_sigs: &fn_sigs,
    };

    for f in &prog.functions {
        let mut vars = HashMap::new();
        for (p_name, p_ty, _) in &f.params {
            let d = known_units.get(p_ty).cloned();
            let s = if structs.contains_key(p_ty) {
                Some(p_ty.clone())
            } else {
                None
            };
            vars.insert(p_name.clone(), (d, s));
        }
        let ret_dim = f
            .ret_type
            .as_ref()
            .and_then(|rt| known_units.get(rt).cloned());
        check_stmts(&f.body, &mut vars, &ret_dim, &env)?;
    }

    for (_, body) in &prog.tests {
        let mut vars = HashMap::new();
        check_stmts(body, &mut vars, &None, &env)?;
    }

    // Zero-Cost Erasure: replace unit types with "float" and UnitFloat with Float
    for s in &mut prog.structs {
        for (_, f_ty) in &mut s.fields {
            if known_units.contains_key(f_ty) {
                *f_ty = "float".to_string();
            }
        }
    }
    for f in &mut prog.functions {
        for (_, p_ty, _) in &mut f.params {
            if known_units.contains_key(p_ty) {
                *p_ty = "float".to_string();
            }
        }
        if let Some(rt) = &mut f.ret_type {
            if known_units.contains_key(rt) {
                *rt = "float".to_string();
            }
        }
        erase_stmts(&mut f.body, &known_units);
    }
    for (_, body) in &mut prog.tests {
        erase_stmts(body, &known_units);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_mismatch_rejected() {
        let src = r#"
unit m
unit s
test "invalid addition" {
    let crash = 100.0_m + 5.0_s
}
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        let res = check_and_erase_units(&mut prog);
        assert!(res.is_err(), "Expected unit mismatch error, got Ok");
        let err = res.unwrap_err();
        assert!(err.contains("Einheiten-Fehler"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_unit_let_hint_mismatch() {
        let src = r#"
unit m
unit s
unit Velocity = m / s
test "bad hint" {
    let v: Velocity = 10.0_m
}
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        assert!(check_and_erase_units(&mut prog).is_err());
    }

    #[test]
    fn test_unit_param_mismatch() {
        let src = r#"
unit m
unit s
fn travel(dt: s) -> s { return dt }
test "bad param" {
    let x = travel(5.0_m)
}
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        assert!(check_and_erase_units(&mut prog).is_err());
    }

    #[test]
    fn test_unit_return_mismatch() {
        let src = r#"
unit m
unit s
unit Velocity = m / s
fn bad_vel(d: m) -> Velocity { return d }
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        assert!(check_and_erase_units(&mut prog).is_err());
    }

    #[test]
    fn test_open_enum_merge_ok() {
        let src = r#"
open enum Fuel { Liquid }
extend enum Fuel { Nuclear }
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        crate::merge_open_enums(&mut prog);
        assert_eq!(prog.enums.len(), 1);
        assert_eq!(prog.enums[0].variants.len(), 2);
    }
}
