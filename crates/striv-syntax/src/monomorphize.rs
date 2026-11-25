use crate::{Expr, Function, Op, Program, Stmt, StructDef};
use std::collections::{HashMap, HashSet};

pub fn monomorphize_program(prog: &mut Program) {
    let mut struct_templates: HashMap<String, StructDef> = HashMap::new();
    let mut concrete_structs: Vec<StructDef> = Vec::new();
    for s in prog.structs.drain(..) {
        if s.type_params.is_empty() {
            concrete_structs.push(s);
        } else {
            struct_templates.insert(s.name.clone(), s);
        }
    }

    let mut fn_templates: HashMap<String, Function> = HashMap::new();
    let mut concrete_fns: Vec<Function> = Vec::new();
    for f in prog.functions.drain(..) {
        if f.type_params.is_empty() {
            concrete_fns.push(f);
        } else {
            fn_templates.insert(f.name.clone(), f);
        }
    }

    if struct_templates.is_empty() && fn_templates.is_empty() {
        prog.structs = concrete_structs;
        prog.functions = concrete_fns;
        return;
    }

    let mut emitted_structs: HashSet<String> =
        concrete_structs.iter().map(|s| s.name.clone()).collect();
    let mut emitted_fns: HashSet<String> = concrete_fns.iter().map(|f| f.name.clone()).collect();

    loop {
        let mut new_structs = Vec::new();
        let mut new_fns = Vec::new();

        let struct_map: HashMap<String, StructDef> = concrete_structs
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect();
        let fn_ret_map: HashMap<String, Option<String>> = concrete_fns
            .iter()
            .map(|f| (f.name.clone(), f.ret_type.clone()))
            .collect();

        for func in &mut concrete_fns {
            let mut var_types: HashMap<String, String> = HashMap::new();
            for (p_name, p_ty, _) in &func.params {
                var_types.insert(p_name.clone(), p_ty.clone());
            }
            let mut ctx = MonoCtx {
                struct_templates: &struct_templates,
                fn_templates: &fn_templates,
                struct_map: &struct_map,
                fn_ret_map: &fn_ret_map,
                emitted_structs: &mut emitted_structs,
                emitted_fns: &mut emitted_fns,
                new_structs: &mut new_structs,
                new_fns: &mut new_fns,
            };
            mono_stmts(&mut func.body, &mut var_types, &mut ctx);
        }
        for (_test_name, test_body) in &mut prog.tests {
            let mut vars = HashMap::new();
            let mut ctx = MonoCtx {
                struct_templates: &struct_templates,
                fn_templates: &fn_templates,
                struct_map: &struct_map,
                fn_ret_map: &fn_ret_map,
                emitted_structs: &mut emitted_structs,
                emitted_fns: &mut emitted_fns,
                new_structs: &mut new_structs,
                new_fns: &mut new_fns,
            };
            mono_stmts(test_body, &mut vars, &mut ctx);
        }

        let changed = !new_structs.is_empty() || !new_fns.is_empty();
        concrete_structs.extend(new_structs);
        concrete_fns.extend(new_fns);
        if !changed {
            break;
        }
    }

    prog.structs = concrete_structs;
    prog.functions = concrete_fns;
}

struct MonoCtx<'a> {
    struct_templates: &'a HashMap<String, StructDef>,
    fn_templates: &'a HashMap<String, Function>,
    struct_map: &'a HashMap<String, StructDef>,
    fn_ret_map: &'a HashMap<String, Option<String>>,
    emitted_structs: &'a mut HashSet<String>,
    emitted_fns: &'a mut HashSet<String>,
    new_structs: &'a mut Vec<StructDef>,
    new_fns: &'a mut Vec<Function>,
}

fn mangle(base: &str, args: &[String]) -> String {
    format!("{}__G_{}", base, args.join("_"))
}

fn infer_type(
    expr: &Expr,
    var_types: &HashMap<String, String>,
    ctx: &MonoCtx,
) -> String {
    match expr {
        Expr::Int(_) | Expr::Hex(_) => "int".to_string(),
        Expr::Float(_) | Expr::UnitFloat(_, _) => "float".to_string(),
        Expr::StrInterp(_) => "String".to_string(),
        Expr::VecLit(_) => "Vec".to_string(),
        Expr::Var(n) => var_types.get(n).cloned().unwrap_or_else(|| "int".to_string()),
        Expr::StructInit { name, .. } => name.clone(),
        Expr::FieldAccess { var, field } => {
            if let Some(s_name) = var_types.get(var) {
                if let Some(sd) = ctx.struct_map.get(s_name) {
                    if let Some((_, f_ty)) = sd.fields.iter().find(|(f, _)| f == field) {
                        return f_ty.clone();
                    }
                }
            }
            "int".to_string()
        }
        Expr::Call { name, .. } => ctx
            .fn_ret_map
            .get(name)
            .and_then(|r| r.clone())
            .or_else(|| {
                ctx.new_fns
                    .iter()
                    .find(|f| &f.name == name)
                    .and_then(|f| f.ret_type.clone())
            })
            .unwrap_or_else(|| "int".to_string()),
        Expr::BinOp { left, op, right } => {
            if matches!(
                op,
                Op::Lt | Op::Gt | Op::LtEq | Op::GtEq | Op::Eq | Op::NotEq | Op::And | Op::Or
            ) {
                return "int".to_string();
            }
            let lt = infer_type(left, var_types, ctx);
            if lt != "int" && lt != "float" {
                return lt;
            }
            let rt = infer_type(right, var_types, ctx);
            if lt == "float" || rt == "float" {
                "float".to_string()
            } else {
                "int".to_string()
            }
        }
        _ => "int".to_string(),
    }
}

fn mono_stmts(
    stmts: &mut [Stmt],
    var_types: &mut HashMap<String, String>,
    ctx: &mut MonoCtx,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                ty_hint,
                value,
                ..
            } => {
                mono_expr(value, var_types, ctx);
                let ty = ty_hint
                    .clone()
                    .unwrap_or_else(|| infer_type(value, var_types, ctx));
                var_types.insert(name.clone(), ty);
            }
            Stmt::Assign { value, .. }
            | Stmt::FieldAssign { value, .. }
            | Stmt::Return(value)
            | Stmt::Print(value)
            | Stmt::Expr(value) => {
                mono_expr(value, var_types, ctx);
            }
            Stmt::Call { args, .. } => {
                for a in args.iter_mut() {
                    mono_expr(a, var_types, ctx);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                mono_expr(cond, var_types, ctx);
                mono_stmts(then_block, var_types, ctx);
                mono_stmts(else_block, var_types, ctx);
            }
            Stmt::While { cond, body } => {
                mono_expr(cond, var_types, ctx);
                mono_stmts(body, var_types, ctx);
            }
            Stmt::ForRange {
                var,
                start,
                end,
                body,
            } => {
                mono_expr(start, var_types, ctx);
                mono_expr(end, var_types, ctx);
                var_types.insert(var.clone(), "int".to_string());
                mono_stmts(body, var_types, ctx);
            }
            Stmt::ForEach { body, .. }
            | Stmt::Concurrent(body)
            | Stmt::Lock { body, .. }
            | Stmt::Block(body) => {
                mono_stmts(body, var_types, ctx);
            }
            Stmt::Defer(inner) => {
                mono_stmts(std::slice::from_mut(&mut **inner), var_types, ctx);
            }
            Stmt::Match { target, arms } => {
                mono_expr(target, var_types, ctx);
                for (_, body) in arms {
                    mono_stmts(body, var_types, ctx);
                }
            }
            _ => {}
        }
    }
}

fn mono_expr(
    expr: &mut Expr,
    var_types: &HashMap<String, String>,
    ctx: &mut MonoCtx,
) {
    match expr {
        Expr::StructInit { name, fields } => {
            for (_, f_expr) in fields.iter_mut() {
                mono_expr(f_expr, var_types, ctx);
            }
            if let Some(tpl) = ctx.struct_templates.get(name.as_str()).cloned() {
                let mut subst: HashMap<String, String> = HashMap::new();
                for (f_name, f_expr) in fields.iter() {
                    if let Some((_, tpl_ty)) = tpl.fields.iter().find(|(fnm, _)| fnm == f_name) {
                        if tpl.type_params.contains(tpl_ty) {
                            subst.insert(tpl_ty.clone(), infer_type(f_expr, var_types, ctx));
                        }
                    }
                }
                let concrete_args: Vec<String> = tpl
                    .type_params
                    .iter()
                    .map(|tp| subst.get(tp).cloned().unwrap_or_else(|| "int".to_string()))
                    .collect();
                let mangled = mangle(&tpl.name, &concrete_args);
                if ctx.emitted_structs.insert(mangled.clone()) {
                    let new_fields = tpl
                        .fields
                        .iter()
                        .map(|(fnm, fty)| {
                            (
                                fnm.clone(),
                                subst.get(fty).cloned().unwrap_or_else(|| fty.clone()),
                            )
                        })
                        .collect();
                    ctx.new_structs.push(StructDef {
                        name: mangled.clone(),
                        type_params: Vec::new(),
                        fields: new_fields,
                        sync_mask: tpl.sync_mask,
                    });
                }
                *name = mangled;
            }
        }
        Expr::Call { name, args } => {
            for a in args.iter_mut() {
                mono_expr(a, var_types, ctx);
            }
            if let Some(tpl) = ctx.fn_templates.get(name.as_str()).cloned() {
                let mut subst: HashMap<String, String> = HashMap::new();
                for ((_, p_ty, _), arg_expr) in tpl.params.iter().zip(args.iter()) {
                    if tpl.type_params.contains(p_ty) {
                        subst.insert(p_ty.clone(), infer_type(arg_expr, var_types, ctx));
                    }
                }
                let concrete_args: Vec<String> = tpl
                    .type_params
                    .iter()
                    .map(|tp| subst.get(tp).cloned().unwrap_or_else(|| "int".to_string()))
                    .collect();
                let mangled = mangle(&tpl.name, &concrete_args);
                if ctx.emitted_fns.insert(mangled.clone()) {
                    let new_params = tpl
                        .params
                        .iter()
                        .map(|(pn, pt, pm)| {
                            (
                                pn.clone(),
                                subst.get(pt).cloned().unwrap_or_else(|| pt.clone()),
                                *pm,
                            )
                        })
                        .collect();
                    let new_ret = tpl
                        .ret_type
                        .as_ref()
                        .map(|rt| subst.get(rt).cloned().unwrap_or_else(|| rt.clone()));
                    let mut new_body = tpl.body.clone();
                    subst_stmts(&mut new_body, &subst);
                    ctx.new_fns.push(Function {
                        name: mangled.clone(),
                        is_rpc: false,
                        type_params: Vec::new(),
                        params: new_params,
                        ret_type: new_ret,
                        body: new_body,
                    });
                }
                *name = mangled;
            }
        }
        Expr::BinOp { left, right, .. } => {
            mono_expr(left, var_types, ctx);
            mono_expr(right, var_types, ctx);
        }
        Expr::MethodCall { args, .. } | Expr::VecLit(args) | Expr::Tuple(args) => {
            for a in args.iter_mut() {
                mono_expr(a, var_types, ctx);
            }
        }
        Expr::Try(inner)
        | Expr::Ok(inner)
        | Expr::Err(inner)
        | Expr::Some(inner)
        | Expr::TupleIndex { tuple: inner, .. } => {
            mono_expr(inner, var_types, ctx);
        }
        _ => {}
    }
}

fn subst_stmts(stmts: &mut [Stmt], subst: &HashMap<String, String>) {
    for stmt in stmts {
        if let Stmt::Let { ty_hint, .. } = stmt {
            if let Some(th) = ty_hint {
                if let Some(rep) = subst.get(th) {
                    *th = rep.clone();
                }
            }
        }
        match stmt {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                subst_stmts(then_block, subst);
                subst_stmts(else_block, subst);
            }
            Stmt::While { body, .. }
            | Stmt::ForRange { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Concurrent(body)
            | Stmt::Lock { body, .. }
            | Stmt::Block(body) => subst_stmts(body, subst),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monomorphize_in_test_block() {
        let src = r#"
fn clamp<T>(val: T, min: T, max: T) -> T {
    if val < min { return min }
    if val > max { return max }
    return val
}

test "Generics mit int und float" {
    assert(clamp(150, 0, 100) == 100)
    assert(clamp(-3.5, 0.0, 10.0) == 0.0)
}
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        monomorphize_program(&mut prog);
        let fn_names: Vec<&str> = prog.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            fn_names.contains(&"clamp__G_int"),
            "Expected clamp__G_int in {:?}",
            fn_names
        );
        assert!(
            fn_names.contains(&"clamp__G_float"),
            "Expected clamp__G_float in {:?}",
            fn_names
        );
    }
}
