use cranelift::codegen::ir::{MemFlagsData, StackSlot};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::collections::HashMap;
use striv_syntax::{Expr, Op, Program, Stmt, StringPart, StructDef, TweakVal};

// 1. Rust host functions for printing (integers and structs without derive)
extern "C" fn host_print_int(val: i64) {
    println!(">> {}", val);
}

extern "C" fn host_print_struct(data_ptr: *const i64, def_ptr: *const StructDef) {
    let def = unsafe { &*def_ptr };
    let values = unsafe { std::slice::from_raw_parts(data_ptr, def.fields.len()) };

    print!(">> {} {{ ", def.name);
    for (i, field_name) in def.fields.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{}: {}", field_name, values[i]);
    }
    println!(" }}");
}

// 2. Variable kinds: integer, stack-allocated struct, or struct pointer
#[derive(Clone)]
enum VarInfo {
    Int {
        var: Variable,
        is_mut: bool,
    },
    Str {
        var: Variable,
        is_mut: bool,
    },
    Vec {
        var: Variable,
        is_mut: bool,
    },
    Tweak {
        addr: i64,
    },
    TweakStr {
        id: usize,
    },
    TweakVec {
        id: usize,
    },
    Struct {
        slot: StackSlot,
        struct_name: String,
        is_mut: bool,
    },
    StructPtr {
        ptr_var: Variable,
        struct_name: String,
        is_mut: bool,
    },
}

struct CompileEnv<'a> {
    int_ty: Type,
    print_int_ref: cranelift::codegen::ir::FuncRef,
    print_struct_ref: cranelift::codegen::ir::FuncRef,
    str_new_ref: cranelift::codegen::ir::FuncRef,
    str_push_int_ref: cranelift::codegen::ir::FuncRef,
    str_push_str_ref: cranelift::codegen::ir::FuncRef,
    str_print_ref: cranelift::codegen::ir::FuncRef,
    str_free_ref: cranelift::codegen::ir::FuncRef,
    tweak_get_str_ref: cranelift::codegen::ir::FuncRef,
    tweak_get_vec_ref: cranelift::codegen::ir::FuncRef,
    vec_new_ref: cranelift::codegen::ir::FuncRef,
    vec_push_ref: cranelift::codegen::ir::FuncRef,
    vec_get_ref: cranelift::codegen::ir::FuncRef,
    vec_set_ref: cranelift::codegen::ir::FuncRef,
    vec_len_ref: cranelift::codegen::ir::FuncRef,
    vec_free_ref: cranelift::codegen::ir::FuncRef,
    literals: &'a mut Vec<Box<[u8]>>,
    structs: &'a HashMap<String, Box<StructDef>>,
    funcs: HashMap<String, (cranelift::codegen::ir::FuncRef, Vec<(String, bool)>)>,
}

pub struct JitCompiler {
    module: JITModule,
    // Box keeps the StructDef memory address stable for host_print_struct
    kept_structs: HashMap<String, Box<StructDef>>,
    kept_literals: Vec<Box<[u8]>>,
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

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbol("host_print_int", host_print_int as *const u8);
        builder.symbol("host_print_struct", host_print_struct as *const u8);
        builder.symbol("striv_str_new", lang_runtime::striv_str_new as *const u8);
        builder.symbol(
            "striv_str_push_int",
            lang_runtime::striv_str_push_int as *const u8,
        );
        builder.symbol(
            "striv_str_push_str",
            lang_runtime::striv_str_push_str as *const u8,
        );
        builder.symbol(
            "striv_str_print",
            lang_runtime::striv_str_print as *const u8,
        );
        builder.symbol("striv_str_free", lang_runtime::striv_str_free as *const u8);
        builder.symbol(
            "striv_tweak_get_str",
            lang_runtime::striv_tweak_get_str as *const u8,
        );
        builder.symbol(
            "striv_tweak_get_vec",
            lang_runtime::striv_tweak_get_vec as *const u8,
        );
        builder.symbol("striv_vec_new", lang_runtime::striv_vec_new as *const u8);
        builder.symbol("striv_vec_push", lang_runtime::striv_vec_push as *const u8);
        builder.symbol("striv_vec_get", lang_runtime::striv_vec_get as *const u8);
        builder.symbol("striv_vec_set", lang_runtime::striv_vec_set as *const u8);
        builder.symbol("striv_vec_len", lang_runtime::striv_vec_len as *const u8);
        builder.symbol("striv_vec_free", lang_runtime::striv_vec_free as *const u8);
        builder.symbol("striv_clear", lang_runtime::striv_clear as *const u8);
        builder.symbol("striv_rect", lang_runtime::striv_rect as *const u8);
        builder.symbol("striv_key_down", lang_runtime::striv_key_down as *const u8);

        Self {
            module: JITModule::new(builder),
            kept_structs: HashMap::new(),
            kept_literals: Vec::new(),
        }
    }

    pub fn compile_program(&mut self, prog: &Program) -> *const u8 {
        for s in &prog.structs {
            self.kept_structs
                .insert(s.name.clone(), Box::new(s.clone()));
        }

        let target_config = self.module.target_config();
        let int_ty = target_config.pointer_type();

        // Declare print imports
        let mut sig_int = self.module.make_signature();
        sig_int.params.push(AbiParam::new(int_ty));
        let print_int_id = self
            .module
            .declare_function("host_print_int", Linkage::Import, &sig_int)
            .unwrap();

        let mut sig_struct = self.module.make_signature();
        sig_struct.params.push(AbiParam::new(int_ty));
        sig_struct.params.push(AbiParam::new(int_ty));
        let print_struct_id = self
            .module
            .declare_function("host_print_struct", Linkage::Import, &sig_struct)
            .unwrap();

        let mut sig_str_new = self.module.make_signature();
        sig_str_new.params.push(AbiParam::new(int_ty));
        sig_str_new.params.push(AbiParam::new(int_ty));
        sig_str_new.returns.push(AbiParam::new(int_ty));
        let str_new_id = self
            .module
            .declare_function("striv_str_new", Linkage::Import, &sig_str_new)
            .unwrap();

        let mut sig_push = self.module.make_signature();
        sig_push.params.push(AbiParam::new(int_ty));
        sig_push.params.push(AbiParam::new(int_ty));
        sig_push.returns.push(AbiParam::new(int_ty));
        let str_push_int_id = self
            .module
            .declare_function("striv_str_push_int", Linkage::Import, &sig_push)
            .unwrap();
        let str_push_str_id = self
            .module
            .declare_function("striv_str_push_str", Linkage::Import, &sig_push)
            .unwrap();

        let str_print_id = self
            .module
            .declare_function("striv_str_print", Linkage::Import, &sig_int)
            .unwrap();
        let str_free_id = self
            .module
            .declare_function("striv_str_free", Linkage::Import, &sig_int)
            .unwrap();

        let mut sig_tweak_str = self.module.make_signature();
        sig_tweak_str.params.push(AbiParam::new(int_ty));
        sig_tweak_str.returns.push(AbiParam::new(int_ty));
        let tweak_get_str_id = self
            .module
            .declare_function("striv_tweak_get_str", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let tweak_get_vec_id = self
            .module
            .declare_function("striv_tweak_get_vec", Linkage::Import, &sig_tweak_str)
            .unwrap();

        let mut sig_vec_new = self.module.make_signature();
        sig_vec_new.returns.push(AbiParam::new(int_ty));
        let vec_new_id = self
            .module
            .declare_function("striv_vec_new", Linkage::Import, &sig_vec_new)
            .unwrap();

        let mut sig_vec_push = self.module.make_signature();
        sig_vec_push.params.push(AbiParam::new(int_ty));
        sig_vec_push.params.push(AbiParam::new(int_ty));
        let vec_push_id = self
            .module
            .declare_function("striv_vec_push", Linkage::Import, &sig_vec_push)
            .unwrap();

        let mut sig_vec_get = self.module.make_signature();
        sig_vec_get.params.push(AbiParam::new(int_ty));
        sig_vec_get.params.push(AbiParam::new(int_ty));
        sig_vec_get.returns.push(AbiParam::new(int_ty));
        let vec_get_id = self
            .module
            .declare_function("striv_vec_get", Linkage::Import, &sig_vec_get)
            .unwrap();

        let mut sig_vec_set = self.module.make_signature();
        sig_vec_set.params.push(AbiParam::new(int_ty));
        sig_vec_set.params.push(AbiParam::new(int_ty));
        sig_vec_set.params.push(AbiParam::new(int_ty));
        let vec_set_id = self
            .module
            .declare_function("striv_vec_set", Linkage::Import, &sig_vec_set)
            .unwrap();

        let mut sig_vec_len = self.module.make_signature();
        sig_vec_len.params.push(AbiParam::new(int_ty));
        sig_vec_len.returns.push(AbiParam::new(int_ty));
        let vec_len_id = self
            .module
            .declare_function("striv_vec_len", Linkage::Import, &sig_vec_len)
            .unwrap();

        let vec_free_id = self
            .module
            .declare_function("striv_vec_free", Linkage::Import, &sig_int)
            .unwrap();

        let clear_id = self
            .module
            .declare_function("striv_clear", Linkage::Import, &sig_push)
            .unwrap();
        let key_id = self
            .module
            .declare_function("striv_key_down", Linkage::Import, &sig_push)
            .unwrap();

        let mut sig_rect = self.module.make_signature();
        for _ in 0..6 {
            sig_rect.params.push(AbiParam::new(int_ty));
        }
        sig_rect.returns.push(AbiParam::new(int_ty));
        let rect_id = self
            .module
            .declare_function("striv_rect", Linkage::Import, &sig_rect)
            .unwrap();

        // 1. Pre-register all functions so they can call each other
        let mut func_ids = Vec::new();
        for func in &prog.functions {
            let mut sig = self.module.make_signature();
            for _ in &func.params {
                sig.params.push(AbiParam::new(int_ty)); // Integer or 8-byte struct pointer
            }
            sig.returns.push(AbiParam::new(int_ty));
            let id = self
                .module
                .declare_function(&func.name, Linkage::Export, &sig)
                .unwrap();
            func_ids.push(id);
        }

        // 2. Compile each function with Cranelift
        let mut func_ctx = FunctionBuilderContext::new();
        for (i, func) in prog.functions.iter().enumerate() {
            let mut ctx = self.module.make_context();
            for _ in &func.params {
                ctx.func.signature.params.push(AbiParam::new(int_ty));
            }
            ctx.func.signature.returns.push(AbiParam::new(int_ty));

            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let entry = builder.create_block();
                builder.append_block_params_for_function_params(entry);
                builder.switch_to_block(entry);
                builder.seal_block(entry);

                let mut funcs_map = HashMap::new();
                for (j, f) in prog.functions.iter().enumerate() {
                    let f_ref = self.module.declare_func_in_func(func_ids[j], builder.func);
                    let param_meta = f.params.iter().map(|(_, ty, m)| (ty.clone(), *m)).collect();
                    funcs_map.insert(f.name.clone(), (f_ref, param_meta));
                }
                let clear_ref = self.module.declare_func_in_func(clear_id, builder.func);
                let key_ref = self.module.declare_func_in_func(key_id, builder.func);
                let rect_ref = self.module.declare_func_in_func(rect_id, builder.func);
                funcs_map.insert(
                    "clear".to_string(),
                    (clear_ref, vec![("int".into(), false); 2]),
                );
                funcs_map.insert(
                    "key_down".to_string(),
                    (key_ref, vec![("int".into(), false); 2]),
                );
                funcs_map.insert(
                    "draw_rect".to_string(),
                    (rect_ref, vec![("int".into(), false); 6]),
                );

                let mut env = CompileEnv {
                    int_ty,
                    print_int_ref: self.module.declare_func_in_func(print_int_id, builder.func),
                    print_struct_ref: self
                        .module
                        .declare_func_in_func(print_struct_id, builder.func),
                    str_new_ref: self.module.declare_func_in_func(str_new_id, builder.func),
                    str_push_int_ref: self
                        .module
                        .declare_func_in_func(str_push_int_id, builder.func),
                    str_push_str_ref: self
                        .module
                        .declare_func_in_func(str_push_str_id, builder.func),
                    str_print_ref: self.module.declare_func_in_func(str_print_id, builder.func),
                    str_free_ref: self.module.declare_func_in_func(str_free_id, builder.func),
                    tweak_get_str_ref: self
                        .module
                        .declare_func_in_func(tweak_get_str_id, builder.func),
                    tweak_get_vec_ref: self
                        .module
                        .declare_func_in_func(tweak_get_vec_id, builder.func),
                    vec_new_ref: self.module.declare_func_in_func(vec_new_id, builder.func),
                    vec_push_ref: self.module.declare_func_in_func(vec_push_id, builder.func),
                    vec_get_ref: self.module.declare_func_in_func(vec_get_id, builder.func),
                    vec_set_ref: self.module.declare_func_in_func(vec_set_id, builder.func),
                    vec_len_ref: self.module.declare_func_in_func(vec_len_id, builder.func),
                    vec_free_ref: self.module.declare_func_in_func(vec_free_id, builder.func),
                    literals: &mut self.kept_literals,
                    structs: &self.kept_structs,
                    funcs: funcs_map,
                };

                let mut vars: HashMap<String, VarInfo> = HashMap::new();
                for (idx, (name, ty_name, is_mut)) in func.params.iter().enumerate() {
                    let var = builder.declare_var(int_ty);
                    let val = builder.block_params(entry)[idx];
                    builder.def_var(var, val);
                    if ty_name == "String" {
                        vars.insert(
                            name.clone(),
                            VarInfo::Str {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    } else if self.kept_structs.contains_key(ty_name) {
                        vars.insert(
                            name.clone(),
                            VarInfo::StructPtr {
                                ptr_var: var,
                                struct_name: ty_name.clone(),
                                is_mut: *is_mut,
                            },
                        );
                    } else {
                        vars.insert(
                            name.clone(),
                            VarInfo::Int {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    }
                }
                for (t_name, init_val) in &prog.tweaks {
                    match init_val {
                        TweakVal::Int(v) => {
                            let ptr = lang_runtime::register_tweak(t_name, *v) as i64;
                            vars.insert(t_name.clone(), VarInfo::Tweak { addr: ptr });
                        }
                        TweakVal::Str(s) => {
                            let id = lang_runtime::register_str_tweak(t_name, s);
                            vars.insert(t_name.clone(), VarInfo::TweakStr { id });
                        }
                        TweakVal::Vec(v) => {
                            let id = lang_runtime::register_vec_tweak(t_name, v);
                            vars.insert(t_name.clone(), VarInfo::TweakVec { id });
                        }
                    }
                }

                let mut owned_strings: Vec<Variable> = Vec::new();
                let mut owned_vecs: Vec<Variable> = Vec::new();
                let mut terminated = false;
                for stmt in &func.body {
                    if compile_stmt(
                        stmt,
                        &mut builder,
                        &mut vars,
                        &mut env,
                        &mut owned_strings,
                        &mut owned_vecs,
                    ) {
                        terminated = true;
                        break;
                    }
                }
                if !terminated {
                    for s_var in &owned_strings {
                        let ptr = builder.use_var(*s_var);
                        builder.ins().call(env.str_free_ref, &[ptr]);
                    }
                    for v_var in &owned_vecs {
                        let ptr = builder.use_var(*v_var);
                        builder.ins().call(env.vec_free_ref, &[ptr]);
                    }
                    let zero = builder.ins().iconst(int_ty, 0);
                    builder.ins().return_(&[zero]);
                }
                builder.finalize(target_config);
            }

            self.module.define_function(func_ids[i], &mut ctx).unwrap();
            self.module.clear_context(&mut ctx);
        }

        self.module.finalize_definitions().unwrap();
        let entry_idx = prog
            .functions
            .iter()
            .position(|f| f.name == "main")
            .unwrap_or(func_ids.len() - 1);
        self.module.get_finalized_function(func_ids[entry_idx])
    }
}

// Helper: computes the byte offset of a field (field 0 -> 0, field 1 -> 8, ...)
fn field_offset(def: &StructDef, field: &str) -> i32 {
    let idx = def
        .fields
        .iter()
        .position(|f| f == field)
        .unwrap_or_else(|| panic!("Unknown field '{}' in struct '{}'", field, def.name));
    (idx * 8) as i32
}

fn compile_call(
    name: &str,
    args: &[Expr],
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, VarInfo>,
    env: &mut CompileEnv,
) -> Value {
    let (func_ref, param_meta) = env
        .funcs
        .get(name)
        .unwrap_or_else(|| panic!("Unknown function: {}", name))
        .clone();
    assert_eq!(
        args.len(),
        param_meta.len(),
        "Wrong number of arguments for '{}'",
        name
    );

    let mut arg_vals = Vec::new();
    let mut mut_borrowed = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let (_expected_ty, expects_mut) = &param_meta[i];
        if let Expr::Var(v_name) = arg {
            match vars.get(v_name) {
                Some(VarInfo::Struct { slot, is_mut, .. }) => {
                    if *expects_mut {
                        assert!(
                            *is_mut,
                            "Error: '{}' must be `mut` to call '{}'!",
                            v_name, name
                        );
                        assert!(
                            !mut_borrowed.contains(&v_name),
                            "Exclusivity error: '{}' passed twice as `mut`!",
                            v_name
                        );
                        mut_borrowed.push(v_name);
                    }
                    // Zero-copy: pass the 8-byte stack slot address
                    arg_vals.push(builder.ins().stack_addr(env.int_ty, *slot, 0));
                    continue;
                }
                Some(VarInfo::StructPtr {
                    ptr_var, is_mut, ..
                }) => {
                    if *expects_mut {
                        assert!(*is_mut, "Error: '{}' must be `mut`!", v_name);
                    }
                    arg_vals.push(builder.use_var(*ptr_var));
                    continue;
                }
                _ => {}
            }
        }
        arg_vals.push(compile_expr(arg, builder, vars, env));
    }

    let call = builder.ins().call(func_ref, &arg_vals);
    builder.inst_results(call)[0]
}

fn compile_method_call(
    receiver: &str,
    method: &str,
    args: &[Expr],
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, VarInfo>,
    env: &mut CompileEnv,
) -> Value {
    if let Some(VarInfo::TweakVec { id }) = vars.get(receiver) {
        if method == "len" {
            let id_val = builder.ins().iconst(env.int_ty, *id as i64);
            let c_get = builder.ins().call(env.tweak_get_vec_ref, &[id_val]);
            let v_ptr = builder.inst_results(c_get)[0];
            let c_len = builder.ins().call(env.vec_len_ref, &[v_ptr]);
            let len_val = builder.inst_results(c_len)[0];
            builder.ins().call(env.vec_free_ref, &[v_ptr]);
            return len_val;
        }
        panic!("Tweak Vec only supports `.len()`");
    }
    if let Some(VarInfo::Vec { var, is_mut }) = vars.get(receiver) {
        let v_ptr = builder.use_var(*var);
        return match method {
            "push" => {
                assert!(*is_mut, "Error: '{}' must be `mut` to push!", receiver);
                let val = compile_expr(&args[0], builder, vars, env);
                builder.ins().call(env.vec_push_ref, &[v_ptr, val]);
                builder.ins().iconst(env.int_ty, 0)
            }
            "len" => {
                let c = builder.ins().call(env.vec_len_ref, &[v_ptr]);
                builder.inst_results(c)[0]
            }
            _ => panic!("Unknown Vec method: {}", method),
        };
    }
    let struct_name = match vars.get(receiver) {
        Some(VarInfo::Struct { struct_name, .. })
        | Some(VarInfo::StructPtr { struct_name, .. }) => struct_name,
        _ => panic!(
            "'{}' is not a struct for method call '.{}()'!",
            receiver, method
        ),
    };
    let full_name = format!("{}__{}", struct_name, method);

    let mut full_args = vec![Expr::Var(receiver.to_string())];
    full_args.extend_from_slice(args);

    compile_call(&full_name, &full_args, builder, vars, env)
}

fn compile_stmt(
    stmt: &Stmt,
    builder: &mut FunctionBuilder,
    vars: &mut HashMap<String, VarInfo>,
    env: &mut CompileEnv,
    owned_strings: &mut Vec<Variable>,
    owned_vecs: &mut Vec<Variable>,
) -> bool {
    match stmt {
        Stmt::Let {
            name,
            is_mut,
            value,
        } => {
            // Case A: Allocate a new struct on the stack
            if let Expr::StructInit {
                name: s_name,
                fields,
            } = value
            {
                let def = env
                    .structs
                    .get(s_name)
                    .unwrap_or_else(|| panic!("Unknown struct: {}", s_name));
                let byte_size = (def.fields.len() * 8) as u32;

                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    byte_size,
                    3, // Alignment 2^3 = 8 Bytes
                ));

                for (f_name, f_expr) in fields {
                    let val = compile_expr(f_expr, builder, vars, env);
                    let offset = field_offset(def, f_name);
                    builder.ins().stack_store(env.int_ty, val, slot, offset);
                }

                vars.insert(
                    name.clone(),
                    VarInfo::Struct {
                        slot,
                        struct_name: s_name.clone(),
                        is_mut: *is_mut,
                    },
                );
            } else if matches!(value, Expr::StrInterp(_)) {
                let ptr = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, ptr);
                owned_strings.push(var);
                vars.insert(
                    name.clone(),
                    VarInfo::Str {
                        var,
                        is_mut: *is_mut,
                    },
                );
            } else if matches!(value, Expr::VecLit(_)) {
                let ptr = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, ptr);
                owned_vecs.push(var);
                vars.insert(
                    name.clone(),
                    VarInfo::Vec {
                        var,
                        is_mut: *is_mut,
                    },
                );
            } else {
                // Case B: Regular integer
                let val = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, val);
                vars.insert(
                    name.clone(),
                    VarInfo::Int {
                        var,
                        is_mut: *is_mut,
                    },
                );
            }
            false
        }
        Stmt::Assign { name, value } => {
            let Some(VarInfo::Int { var, is_mut }) = vars.get(name).cloned() else {
                panic!("'{}' is not a regular integer variable", name);
            };
            assert!(is_mut, "Error: '{}' is not `mut`!", name);
            let val = compile_expr(value, builder, vars, env);
            builder.def_var(var, val);
            false
        }
        Stmt::FieldAssign { var, field, value } => {
            let val = compile_expr(value, builder, vars, env);
            match vars.get(var).cloned() {
                Some(VarInfo::Struct {
                    slot,
                    struct_name,
                    is_mut,
                }) => {
                    assert!(is_mut, "Error: '{}' is not `mut`!", var);
                    let offset = field_offset(env.structs.get(&struct_name).unwrap(), field);
                    builder.ins().stack_store(env.int_ty, val, slot, offset);
                }
                Some(VarInfo::StructPtr {
                    ptr_var,
                    struct_name,
                    is_mut,
                }) => {
                    assert!(is_mut, "Error: parameter '{}' is not `mut`!", var);
                    let offset = field_offset(env.structs.get(&struct_name).unwrap(), field);
                    let addr = builder.use_var(ptr_var);
                    builder.ins().store(MemFlagsData::new(), val, addr, offset);
                }
                _ => panic!("'{}' is not a struct!", var),
            }
            false
        }
        Stmt::Print(expr) => {
            if matches!(expr, Expr::StrInterp(_)) {
                let tmp_ptr = compile_expr(expr, builder, vars, env);
                builder.ins().call(env.str_print_ref, &[tmp_ptr]);
                builder.ins().call(env.str_free_ref, &[tmp_ptr]);
                return false;
            }
            if let Expr::Var(v_name) = expr {
                match vars.get(v_name) {
                    Some(VarInfo::Str { var, .. }) => {
                        let ptr = builder.use_var(*var);
                        builder.ins().call(env.str_print_ref, &[ptr]);
                        return false;
                    }
                    Some(VarInfo::Struct {
                        slot, struct_name, ..
                    }) => {
                        let data_addr = builder.ins().stack_addr(env.int_ty, *slot, 0i32);
                        let def_ptr =
                            &**env.structs.get(struct_name).unwrap() as *const StructDef as i64;
                        let def_addr = builder.ins().iconst(env.int_ty, def_ptr);
                        builder
                            .ins()
                            .call(env.print_struct_ref, &[data_addr, def_addr]);
                        return false;
                    }
                    Some(VarInfo::StructPtr {
                        ptr_var,
                        struct_name,
                        ..
                    }) => {
                        let data_addr = builder.use_var(*ptr_var);
                        let def_ptr =
                            &**env.structs.get(struct_name).unwrap() as *const StructDef as i64;
                        let def_addr = builder.ins().iconst(env.int_ty, def_ptr);
                        builder
                            .ins()
                            .call(env.print_struct_ref, &[data_addr, def_addr]);
                        return false;
                    }
                    _ => {}
                }
            }
            let val = compile_expr(expr, builder, vars, env);
            builder.ins().call(env.print_int_ref, &[val]);
            false
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            let cond_val = compile_expr(cond, builder, vars, env);
            let then_bb = builder.create_block();
            let else_bb = builder.create_block();
            let merge_bb = builder.create_block();

            builder.ins().brif(cond_val, then_bb, &[], else_bb, &[]);

            builder.switch_to_block(then_bb);
            builder.seal_block(then_bb);
            let mut then_ret = false;
            for s in then_block {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
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
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
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
        Stmt::While { cond, body } => {
            let header_bb = builder.create_block();
            let body_bb = builder.create_block();
            let exit_bb = builder.create_block();

            builder.ins().jump(header_bb, &[]);
            builder.switch_to_block(header_bb);

            let cond_val = compile_expr(cond, builder, vars, env);
            builder.ins().brif(cond_val, body_bb, &[], exit_bb, &[]);

            builder.switch_to_block(body_bb);
            builder.seal_block(body_bb);
            let mut body_ret = false;
            for s in body {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    body_ret = true;
                    break;
                }
            }
            if !body_ret {
                builder.ins().jump(header_bb, &[]);
            }

            builder.seal_block(header_bb);

            builder.switch_to_block(exit_bb);
            builder.seal_block(exit_bb);
            false
        }
        Stmt::IndexAssign { var, index, value } => {
            let Some(VarInfo::Vec { var: v_var, is_mut }) = vars.get(var) else {
                panic!("'{}' is not a Vec!", var);
            };
            assert!(*is_mut, "Error: '{}' must be `mut`!", var);
            let v_ptr = builder.use_var(*v_var);
            let idx_val = compile_expr(index, builder, vars, env);
            let val = compile_expr(value, builder, vars, env);
            builder.ins().call(env.vec_set_ref, &[v_ptr, idx_val, val]);
            false
        }
        Stmt::Return(expr) => {
            let val = compile_expr(expr, builder, vars, env);
            for s_var in owned_strings.iter() {
                let ptr = builder.use_var(*s_var);
                builder.ins().call(env.str_free_ref, &[ptr]);
            }
            for v_var in owned_vecs.iter() {
                let ptr = builder.use_var(*v_var);
                builder.ins().call(env.vec_free_ref, &[ptr]);
            }
            builder.ins().return_(&[val]);
            true
        }
        Stmt::Call { name, args } => {
            compile_call(name, args, builder, vars, env);
            false
        }
        Stmt::MethodCall {
            receiver,
            method,
            args,
        } => {
            compile_method_call(receiver, method, args, builder, vars, env);
            false
        }
    }
}

fn compile_expr(
    expr: &Expr,
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, VarInfo>,
    env: &mut CompileEnv,
) -> Value {
    match expr {
        Expr::Int(v) => builder.ins().iconst(env.int_ty, *v),
        Expr::VecLit(items) => {
            let c_new = builder.ins().call(env.vec_new_ref, &[]);
            let v_ptr = builder.inst_results(c_new)[0];
            for item in items {
                let val = compile_expr(item, builder, vars, env);
                builder.ins().call(env.vec_push_ref, &[v_ptr, val]);
            }
            v_ptr
        }
        Expr::Index { var, index } => {
            if let Some(VarInfo::TweakVec { id }) = vars.get(var) {
                let id_val = builder.ins().iconst(env.int_ty, *id as i64);
                let c_get_vec = builder.ins().call(env.tweak_get_vec_ref, &[id_val]);
                let v_ptr = builder.inst_results(c_get_vec)[0];
                let idx_val = compile_expr(index, builder, vars, env);
                let c_get = builder.ins().call(env.vec_get_ref, &[v_ptr, idx_val]);
                let val = builder.inst_results(c_get)[0];
                builder.ins().call(env.vec_free_ref, &[v_ptr]);
                return val;
            }
            let Some(VarInfo::Vec { var: v_var, .. }) = vars.get(var) else {
                panic!("'{}' is not a Vec!", var);
            };
            let v_ptr = builder.use_var(*v_var);
            let idx_val = compile_expr(index, builder, vars, env);
            let c_get = builder.ins().call(env.vec_get_ref, &[v_ptr, idx_val]);
            builder.inst_results(c_get)[0]
        }
        Expr::StrInterp(parts) => {
            let zero = builder.ins().iconst(env.int_ty, 0);
            let call_init = builder.ins().call(env.str_new_ref, &[zero, zero]);
            let mut s_ptr = builder.inst_results(call_init)[0];

            for part in parts {
                match part {
                    StringPart::Literal(text) => {
                        let bytes: Box<[u8]> = text.as_bytes().into();
                        let raw_ptr = bytes.as_ptr() as i64;
                        let raw_len = bytes.len() as i64;
                        env.literals.push(bytes);

                        let p = builder.ins().iconst(env.int_ty, raw_ptr);
                        let l = builder.ins().iconst(env.int_ty, raw_len);
                        let c_tmp = builder.ins().call(env.str_new_ref, &[p, l]);
                        let tmp = builder.inst_results(c_tmp)[0];

                        let c_push = builder.ins().call(env.str_push_str_ref, &[s_ptr, tmp]);
                        s_ptr = builder.inst_results(c_push)[0];
                        builder.ins().call(env.str_free_ref, &[tmp]);
                    }
                    StringPart::Expr(inner) => {
                        if let Expr::Var(vn) = inner {
                            if let Some(VarInfo::Str { var, .. }) = vars.get(vn) {
                                let other = builder.use_var(*var);
                                let c = builder.ins().call(env.str_push_str_ref, &[s_ptr, other]);
                                s_ptr = builder.inst_results(c)[0];
                                continue;
                            }
                            if let Some(VarInfo::TweakStr { id }) = vars.get(vn) {
                                let id_val = builder.ins().iconst(env.int_ty, *id as i64);
                                let c_get = builder.ins().call(env.tweak_get_str_ref, &[id_val]);
                                let tmp = builder.inst_results(c_get)[0];
                                let c = builder.ins().call(env.str_push_str_ref, &[s_ptr, tmp]);
                                s_ptr = builder.inst_results(c)[0];
                                builder.ins().call(env.str_free_ref, &[tmp]);
                                continue;
                            }
                        }
                        let num = compile_expr(inner, builder, vars, env);
                        let c = builder.ins().call(env.str_push_int_ref, &[s_ptr, num]);
                        s_ptr = builder.inst_results(c)[0];
                    }
                }
            }
            s_ptr
        }
        Expr::Var(name) => match vars.get(name) {
            Some(VarInfo::Int { var, .. })
            | Some(VarInfo::Str { var, .. })
            | Some(VarInfo::Vec { var, .. }) => builder.use_var(*var),
            Some(VarInfo::Tweak { addr }) => {
                let ptr_val = builder.ins().iconst(env.int_ty, *addr);
                builder
                    .ins()
                    .load(env.int_ty, MemFlagsData::new(), ptr_val, 0)
            }
            _ => panic!("Unknown integer variable: {}", name),
        },
        Expr::FieldAccess { var, field } => match vars.get(var) {
            Some(VarInfo::Struct {
                slot, struct_name, ..
            }) => {
                let offset = field_offset(env.structs.get(struct_name).unwrap(), field);
                builder
                    .ins()
                    .stack_load(env.int_ty, env.int_ty, *slot, offset)
            }
            Some(VarInfo::StructPtr {
                ptr_var,
                struct_name,
                ..
            }) => {
                let offset = field_offset(env.structs.get(struct_name).unwrap(), field);
                let addr = builder.use_var(*ptr_var);
                builder
                    .ins()
                    .load(env.int_ty, MemFlagsData::new(), addr, offset)
            }
            _ => panic!("'{}' is not a struct!", var),
        },
        Expr::StructInit { .. } => {
            panic!("StructInit must currently be assigned to a `let` variable")
        }
        Expr::BinOp { left, op, right } => {
            let l = compile_expr(left, builder, vars, env);
            let r = compile_expr(right, builder, vars, env);
            match op {
                Op::Add => builder.ins().iadd(l, r),
                Op::Sub => builder.ins().isub(l, r),
                Op::Mul => builder.ins().imul(l, r),
                Op::Lt => builder.ins().icmp(IntCC::SignedLessThan, l, r),
                Op::Gt => builder.ins().icmp(IntCC::SignedGreaterThan, l, r),
                Op::Eq => builder.ins().icmp(IntCC::Equal, l, r),
            }
        }
        Expr::Call { name, args } => compile_call(name, args, builder, vars, env),
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => compile_method_call(receiver, method, args, builder, vars, env),
    }
}
