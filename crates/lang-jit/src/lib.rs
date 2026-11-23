use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use lang_syntax::{Expr, Function, Op, Stmt};
use std::collections::HashMap;

pub struct JitCompiler {
    module: JITModule,
}

impl JitCompiler {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa = cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        Self {
            module: JITModule::new(builder),
        }
    }

    pub fn compile(&mut self, func: &Function) -> *const u8 {
        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();
        let target_config = self.module.target_config();
        let int_ty = target_config.pointer_type();

        for _ in &func.params {
            ctx.func.signature.params.push(AbiParam::new(int_ty));
        }
        ctx.func.signature.returns.push(AbiParam::new(int_ty));

        let func_id = self
            .module
            .declare_function(&func.name, Linkage::Export, &ctx.func.signature)
            .unwrap();

        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let entry = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            builder.seal_block(entry);

            let mut vars: HashMap<String, (Variable, bool)> = HashMap::new();
            for (idx, (name, is_mut)) in func.params.iter().enumerate() {
                let var = builder.declare_var(int_ty);
                let val = builder.block_params(entry)[idx];
                builder.def_var(var, val);
                vars.insert(name.clone(), (var, *is_mut));
            }

            let mut terminated = false;
            for stmt in &func.body {
                if compile_stmt(stmt, &mut builder, &mut vars, int_ty) {
                    terminated = true;
                    break;
                }
            }
            if !terminated {
                let zero = builder.ins().iconst(int_ty, 0);
                builder.ins().return_(&[zero]);
            }

            builder.finalize(target_config);
        }

        self.module.define_function(func_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().unwrap();
        self.module.get_finalized_function(func_id)
    }
}

fn compile_stmt(
    stmt: &Stmt,
    builder: &mut FunctionBuilder,
    vars: &mut HashMap<String, (Variable, bool)>,
    int_ty: Type,
) -> bool {
    match stmt {
        Stmt::Let {
            name,
            is_mut,
            value,
        } => {
            let val = compile_expr(value, builder, vars, int_ty);
            let var = builder.declare_var(int_ty);
            builder.def_var(var, val);
            vars.insert(name.clone(), (var, *is_mut));
            false
        }
        Stmt::Assign { name, value } => {
            let (var, is_mut) = *vars
                .get(name)
                .unwrap_or_else(|| panic!("Unknown Variable: {}", name));
            assert!(is_mut, "Error: '{}' is not `mut`!", name);
            let val = compile_expr(value, builder, vars, int_ty);
            builder.def_var(var, val);
            false
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            let cond_val = compile_expr(cond, builder, vars, int_ty);
            let then_bb = builder.create_block();
            let else_bb = builder.create_block();
            let merge_bb = builder.create_block();

            builder.ins().brif(cond_val, then_bb, &[], else_bb, &[]);

            builder.switch_to_block(then_bb);
            builder.seal_block(then_bb);
            let mut then_ret = false;
            for s in then_block {
                if compile_stmt(s, builder, vars, int_ty) {
                    then_ret = true;
                    break;
                }
            }
            if !then_ret {
                builder.ins().jump(merge_bb, &[]);
            }

            builder.switch_to_block(else_bb);
            builder.seal_block(else_bb);
            let mut else_ret = false;
            for s in else_block {
                if compile_stmt(s, builder, vars, int_ty) {
                    else_ret = true;
                    break;
                }
            }
            if !else_ret {
                builder.ins().jump(merge_bb, &[]);
            }

            builder.switch_to_block(merge_bb);
            builder.seal_block(merge_bb);
            false
        }
        Stmt::Return(expr) => {
            let val = compile_expr(expr, builder, vars, int_ty);
            builder.ins().return_(&[val]);
            true
        }
    }
}

fn compile_expr(
    expr: &Expr,
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, (Variable, bool)>,
    int_ty: Type,
) -> Value {
    match expr {
        Expr::Int(v) => builder.ins().iconst(int_ty, *v),
        Expr::Var(name) => {
            let (var, _) = vars
                .get(name)
                .unwrap_or_else(|| panic!("Unknown Variable: {}", name));
            builder.use_var(*var)
        }
        Expr::BinOp { left, op, right } => {
            let l = compile_expr(left, builder, vars, int_ty);
            let r = compile_expr(right, builder, vars, int_ty);
            match op {
                Op::Add => builder.ins().iadd(l, r),
                Op::Sub => builder.ins().isub(l, r),
                Op::Mul => builder.ins().imul(l, r),
                Op::Lt => builder.ins().icmp(IntCC::SignedLessThan, l, r),
                Op::Gt => builder.ins().icmp(IntCC::SignedGreaterThan, l, r),
                Op::Eq => builder.ins().icmp(IntCC::Equal, l, r),
            }
        }
    }
}
