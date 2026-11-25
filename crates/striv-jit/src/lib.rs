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
    for (i, (field_name, field_ty)) in def.fields.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        if field_ty == "float" {
            print!("{}: {}", field_name, f64::from_bits(values[i] as u64));
        } else {
            print!("{}: {}", field_name, values[i]);
        }
    }
    println!(" }}");
}

// 2. Variable kinds: integer, stack-allocated struct, or struct pointer
#[derive(Clone)]
#[allow(dead_code)]
enum VarInfo {
    Bytes {
        var: Variable,
        is_mut: bool,
    },
    Int {
        var: Variable,
        is_mut: bool,
    },
    Float {
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
    Handle {
        var: Variable,
        type_name: String,
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
    Tuple {
        slot: StackSlot,
        elem_is_float: Vec<bool>,
        is_mut: bool,
    },
    StructVec {
        var: Variable,
        struct_name: String,
        is_mut: bool,
    },
    SharedStruct {
        box_var: Variable,
        struct_name: String,
    },
    EnumVal {
        slot: StackSlot,
        enum_name: String,
        is_mut: bool,
    },
    EnumPtr {
        ptr_var: Variable,
        enum_name: String,
    },
}

struct CompileEnv<'a> {
    int_ty: Type,
    print_int_ref: cranelift::codegen::ir::FuncRef,
    print_struct_ref: cranelift::codegen::ir::FuncRef,
    str_new_ref: cranelift::codegen::ir::FuncRef,
    str_push_int_ref: cranelift::codegen::ir::FuncRef,
    str_push_float_ref: cranelift::codegen::ir::FuncRef,
    str_push_str_ref: cranelift::codegen::ir::FuncRef,
    str_print_ref: cranelift::codegen::ir::FuncRef,
    str_free_ref: cranelift::codegen::ir::FuncRef,
    tweak_get_str_ref: cranelift::codegen::ir::FuncRef,
    tweak_get_vec_ref: cranelift::codegen::ir::FuncRef,
    set_err_ref: cranelift::codegen::ir::FuncRef,
    take_err_ref: cranelift::codegen::ir::FuncRef,
    vec_new_ref: cranelift::codegen::ir::FuncRef,
    vec_push_ref: cranelift::codegen::ir::FuncRef,
    vec_get_ref: cranelift::codegen::ir::FuncRef,
    vec_set_ref: cranelift::codegen::ir::FuncRef,
    vec_len_ref: cranelift::codegen::ir::FuncRef,
    vec_free_ref: cranelift::codegen::ir::FuncRef,
    svec_new_ref: cranelift::codegen::ir::FuncRef,
    svec_push_ref: cranelift::codegen::ir::FuncRef,
    svec_get_ptr_ref: cranelift::codegen::ir::FuncRef,
    svec_len_ref: cranelift::codegen::ir::FuncRef,
    svec_bytes_ptr_ref: cranelift::codegen::ir::FuncRef,
    svec_byte_len_ref: cranelift::codegen::ir::FuncRef,
    pack_struct_ref: cranelift::codegen::ir::FuncRef,
    apply_struct_ref: cranelift::codegen::ir::FuncRef,
    bytes_len_ref: cranelift::codegen::ir::FuncRef,
    bytes_write_u32_ref: cranelift::codegen::ir::FuncRef,
    bytes_write_i64_ref: cranelift::codegen::ir::FuncRef,
    bytes_write_f64_ref: cranelift::codegen::ir::FuncRef,
    bytes_read_u32_ref: cranelift::codegen::ir::FuncRef,
    bytes_read_i64_ref: cranelift::codegen::ir::FuncRef,
    bytes_read_f64_ref: cranelift::codegen::ir::FuncRef,
    fn_ret_handles: &'a HashMap<String, String>,
    fn_ret_floats: &'a std::collections::HashSet<String>,
    sandbox_mode: bool,
    check_gas_ref: cranelift::codegen::ir::FuncRef,
    ecs_set_ref: cranelift::codegen::ir::FuncRef,
    ecs_query_run_ref: cranelift::codegen::ir::FuncRef,
    sync_layouts: &'a HashMap<String, usize>,
    svec_clear_ref: cranelift::codegen::ir::FuncRef,
    group_begin_ref: cranelift::codegen::ir::FuncRef,
    group_spawn_ref: cranelift::codegen::ir::FuncRef,
    group_wait_ref: cranelift::codegen::ir::FuncRef,
    active_group: Option<Value>,
    shared_new_ref: cranelift::codegen::ir::FuncRef,
    shared_lock_ref: cranelift::codegen::ir::FuncRef,
    shared_unlock_ref: cranelift::codegen::ir::FuncRef,
    active_locks: Vec<Value>,
    par_for_svec_ref: cranelift::codegen::ir::FuncRef,
    par_for_vec_ref: cranelift::codegen::ir::FuncRef,
    select_recv_ref: cranelift::codegen::ir::FuncRef,
    par_helpers: &'a HashMap<String, (String, Vec<String>)>,
    current_fn: &'a str,
    deferred: Vec<Stmt>,
    fn_ret_structs: &'a HashMap<String, Option<String>>,
    ret_struct_ptr: Option<(Variable, String)>,
    loop_targets: Vec<(Block, Block)>,
    literals: &'a mut Vec<Box<[u8]>>,
    user_enums: &'a HashMap<String, i64>,
    enum_defs: &'a HashMap<String, striv_syntax::EnumDef>,
    structs: &'a HashMap<String, Box<StructDef>>,
    funcs: HashMap<String, (cranelift::codegen::ir::FuncRef, Vec<(String, bool)>)>,
}

pub struct JitCompiler {
    module: JITModule,
    // Box keeps the StructDef memory address stable for host_print_struct
    kept_structs: HashMap<String, Box<StructDef>>,
    kept_literals: Vec<Box<[u8]>>,
    pub compiled_fns: HashMap<String, *const u8>,
    pub sandbox_mode: bool,
}

fn cached_isa() -> std::sync::Arc<dyn cranelift::codegen::isa::TargetIsa> {
    static ISA: std::sync::OnceLock<std::sync::Arc<dyn cranelift::codegen::isa::TargetIsa>> =
        std::sync::OnceLock::new();
    ISA.get_or_init(|| {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(flag_builder))
            .unwrap()
    })
    .clone()
}

impl JitCompiler {
    pub fn new() -> Self {
        let mut builder = JITBuilder::with_isa(cached_isa(), cranelift_module::default_libcall_names());
        builder.symbol("host_print_int", host_print_int as *const u8);
        builder.symbol("host_print_struct", host_print_struct as *const u8);
        builder.symbol("striv_str_new", lang_runtime::striv_str_new as *const u8);
        builder.symbol(
            "striv_str_push_int",
            lang_runtime::striv_str_push_int as *const u8,
        );
        builder.symbol(
            "striv_str_push_float",
            lang_runtime::striv_str_push_float as *const u8,
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
        builder.symbol("striv_svec_new", lang_runtime::striv_svec_new as *const u8);
        builder.symbol(
            "striv_svec_push",
            lang_runtime::striv_svec_push as *const u8,
        );
        builder.symbol(
            "striv_svec_get_ptr",
            lang_runtime::striv_svec_get_ptr as *const u8,
        );
        builder.symbol("striv_svec_len", lang_runtime::striv_svec_len as *const u8);
        builder.symbol(
            "striv_svec_as_bytes_ptr",
            lang_runtime::striv_svec_as_bytes_ptr as *const u8,
        );
        builder.symbol(
            "striv_svec_byte_len",
            lang_runtime::striv_svec_byte_len as *const u8,
        );
        builder.symbol(
            "striv_pack_struct",
            lang_runtime::striv_pack_struct as *const u8,
        );
        builder.symbol(
            "striv_apply_struct",
            lang_runtime::striv_apply_struct as *const u8,
        );
        builder.symbol("striv_bytes_len", lang_runtime::striv_bytes_len as *const u8);
        builder.symbol("striv_bytes_write_u32", lang_runtime::striv_bytes_write_u32 as *const u8);
        builder.symbol("striv_bytes_write_i64", lang_runtime::striv_bytes_write_i64 as *const u8);
        builder.symbol("striv_bytes_write_f64", lang_runtime::striv_bytes_write_f64 as *const u8);
        builder.symbol("striv_bytes_read_u32", lang_runtime::striv_bytes_read_u32 as *const u8);
        builder.symbol("striv_bytes_read_i64", lang_runtime::striv_bytes_read_i64 as *const u8);
        builder.symbol("striv_bytes_read_f64", lang_runtime::striv_bytes_read_f64 as *const u8);
        builder.symbol("striv_check_gas", lang_runtime::striv_check_gas as *const u8);
        builder.symbol("striv_ecs_set", lang_runtime::striv_ecs_set as *const u8);
        builder.symbol(
            "striv_ecs_query_run",
            lang_runtime::striv_ecs_query_run as *const u8,
        );
        builder.symbol(
            "striv_svec_clear",
            lang_runtime::striv_svec_clear as *const u8,
        );
        builder.symbol(
            "striv_svec_free",
            lang_runtime::striv_svec_free as *const u8,
        );
        builder.symbol(
            "striv_group_begin",
            lang_runtime::striv_group_begin as *const u8,
        );
        builder.symbol(
            "striv_group_spawn",
            lang_runtime::striv_group_spawn as *const u8,
        );
        builder.symbol(
            "striv_group_wait",
            lang_runtime::striv_group_wait as *const u8,
        );
        builder.symbol(
            "striv_shared_new",
            lang_runtime::striv_shared_new as *const u8,
        );
        builder.symbol(
            "striv_shared_lock",
            lang_runtime::striv_shared_lock as *const u8,
        );
        builder.symbol(
            "striv_shared_unlock",
            lang_runtime::striv_shared_unlock as *const u8,
        );
        builder.symbol(
            "striv_shared_free",
            lang_runtime::striv_shared_free as *const u8,
        );
        builder.symbol("striv_assert", lang_runtime::striv_assert as *const u8);
        builder.symbol("striv_sin", lang_runtime::striv_sin as *const u8);
        builder.symbol("striv_cos", lang_runtime::striv_cos as *const u8);
        builder.symbol("striv_tan", lang_runtime::striv_tan as *const u8);
        builder.symbol("striv_atan2", lang_runtime::striv_atan2 as *const u8);
        builder.symbol("striv_sqrt", lang_runtime::striv_sqrt as *const u8);
        builder.symbol(
            "striv_par_for_svec",
            lang_runtime::striv_par_for_svec as *const u8,
        );
        builder.symbol(
            "striv_par_for_vec",
            lang_runtime::striv_par_for_vec as *const u8,
        );
        builder.symbol(
            "striv_select_recv",
            lang_runtime::striv_select_recv as *const u8,
        );
        builder.symbol("striv_set_err", lang_runtime::striv_set_err as *const u8);
        builder.symbol("striv_take_err", lang_runtime::striv_take_err as *const u8);
        for &(_, c_sym, ptr, _) in lang_runtime::generated_bridge::exported_symbols() {
            builder.symbol(c_sym, ptr);
        }
        for (s_name, ptr, _, _) in lang_runtime::std_collection_symbols() {
            let sym = format!("striv_std_{}", s_name.replace("::", "__"));
            builder.symbol(&sym, ptr);
        }

        Self {
            module: JITModule::new(builder),
            kept_structs: HashMap::new(),
            kept_literals: Vec::new(),
            compiled_fns: HashMap::new(),
            sandbox_mode: false,
        }
    }

    pub fn new_sandboxed() -> Self {
        let mut jit = Self::new();
        jit.sandbox_mode = true;
        jit
    }

    pub fn compile_program(&mut self, prog: &Program) -> *const u8 {
        let mut sync_layouts: HashMap<String, usize> = HashMap::new();
        for s in &prog.structs {
            if s.sync_mask != 0 {
                let mut layout = Vec::with_capacity(3 + s.fields.len() * 2);
                layout.push(s.fields.len() as u32);
                layout.push(s.sync_mask as u32);
                layout.push((s.sync_mask >> 32) as u32);
                for (f_name, f_ty) in &s.fields {
                    let off = field_offset(s, f_name) as u32;
                    let (sz, _) = field_type_info(f_ty);
                    layout.push(off);
                    layout.push(sz);
                }
                let leaked = Box::leak(layout.into_boxed_slice());
                sync_layouts.insert(s.name.clone(), leaked.as_ptr() as usize);
            }
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
        let mut sig_push_float = self.module.make_signature();
        sig_push_float.params.push(AbiParam::new(int_ty));
        sig_push_float.params.push(AbiParam::new(types::F64));
        sig_push_float.returns.push(AbiParam::new(int_ty));
        let str_push_float_id = self
            .module
            .declare_function("striv_str_push_float", Linkage::Import, &sig_push_float)
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
        let svec_new_id = self
            .module
            .declare_function("striv_svec_new", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let svec_push_id = self
            .module
            .declare_function("striv_svec_push", Linkage::Import, &sig_push)
            .unwrap();
        let svec_get_ptr_id = self
            .module
            .declare_function("striv_svec_get_ptr", Linkage::Import, &sig_push)
            .unwrap();
        let svec_len_id = self
            .module
            .declare_function("striv_svec_len", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let svec_bytes_ptr_id = self
            .module
            .declare_function("striv_svec_as_bytes_ptr", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let svec_byte_len_id = self
            .module
            .declare_function("striv_svec_byte_len", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let mut sig_3i = self.module.make_signature();
        for _ in 0..3 {
            sig_3i.params.push(AbiParam::new(int_ty));
        }
        sig_3i.returns.push(AbiParam::new(int_ty));
        let pack_struct_id = self
            .module
            .declare_function("striv_pack_struct", Linkage::Import, &sig_3i)
            .unwrap();
        let apply_struct_id = self
            .module
            .declare_function("striv_apply_struct", Linkage::Import, &sig_3i)
            .unwrap();
        let bytes_len_id = self
            .module
            .declare_function("striv_bytes_len", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let mut sig_ii_i = self.module.make_signature();
        sig_ii_i.params.push(AbiParam::new(int_ty));
        sig_ii_i.params.push(AbiParam::new(int_ty));
        sig_ii_i.returns.push(AbiParam::new(int_ty));
        let bytes_write_u32_id = self
            .module
            .declare_function("striv_bytes_write_u32", Linkage::Import, &sig_ii_i)
            .unwrap();
        let bytes_write_i64_id = self
            .module
            .declare_function("striv_bytes_write_i64", Linkage::Import, &sig_ii_i)
            .unwrap();
        let mut sig_if_i = self.module.make_signature();
        sig_if_i.params.push(AbiParam::new(int_ty));
        sig_if_i.params.push(AbiParam::new(types::F64));
        sig_if_i.returns.push(AbiParam::new(int_ty));
        let bytes_write_f64_id = self
            .module
            .declare_function("striv_bytes_write_f64", Linkage::Import, &sig_if_i)
            .unwrap();
        let bytes_read_u32_id = self
            .module
            .declare_function("striv_bytes_read_u32", Linkage::Import, &sig_ii_i)
            .unwrap();
        let bytes_read_i64_id = self
            .module
            .declare_function("striv_bytes_read_i64", Linkage::Import, &sig_ii_i)
            .unwrap();
        let mut sig_ii_f = self.module.make_signature();
        sig_ii_f.params.push(AbiParam::new(int_ty));
        sig_ii_f.params.push(AbiParam::new(int_ty));
        sig_ii_f.returns.push(AbiParam::new(types::F64));
        let bytes_read_f64_id = self
            .module
            .declare_function("striv_bytes_read_f64", Linkage::Import, &sig_ii_f)
            .unwrap();
        let svec_clear_id = self
            .module
            .declare_function("striv_svec_clear", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let group_begin_id = self
            .module
            .declare_function("striv_group_begin", Linkage::Import, &sig_vec_new)
            .unwrap();
        let mut sig_gspawn = self.module.make_signature();
        for _ in 0..4 {
            sig_gspawn.params.push(AbiParam::new(int_ty));
        }
        let group_spawn_id = self
            .module
            .declare_function("striv_group_spawn", Linkage::Import, &sig_gspawn)
            .unwrap();
        let mut sig_gwait = self.module.make_signature();
        sig_gwait.params.push(AbiParam::new(int_ty));
        let group_wait_id = self
            .module
            .declare_function("striv_group_wait", Linkage::Import, &sig_gwait)
            .unwrap();
        let shared_new_id = self
            .module
            .declare_function("striv_shared_new", Linkage::Import, &sig_push)
            .unwrap();
        let shared_lock_id = self
            .module
            .declare_function("striv_shared_lock", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let shared_unlock_id = self
            .module
            .declare_function("striv_shared_unlock", Linkage::Import, &sig_gwait)
            .unwrap();
        let par_for_svec_id = self
            .module
            .declare_function("striv_par_for_svec", Linkage::Import, &sig_gspawn)
            .unwrap();
        let par_for_vec_id = self
            .module
            .declare_function("striv_par_for_vec", Linkage::Import, &sig_gspawn)
            .unwrap();
        let mut sig_sel = self.module.make_signature();
        for _ in 0..4 {
            sig_sel.params.push(AbiParam::new(int_ty));
        }
        sig_sel.returns.push(AbiParam::new(int_ty));
        let select_recv_id = self
            .module
            .declare_function("striv_select_recv", Linkage::Import, &sig_sel)
            .unwrap();
        let assert_id = self
            .module
            .declare_function("striv_assert", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let mut sig_f1 = self.module.make_signature();
        sig_f1.params.push(AbiParam::new(types::F64));
        sig_f1.returns.push(AbiParam::new(types::F64));
        let sin_id = self
            .module
            .declare_function("striv_sin", Linkage::Import, &sig_f1)
            .unwrap();
        let cos_id = self
            .module
            .declare_function("striv_cos", Linkage::Import, &sig_f1)
            .unwrap();
        let tan_id = self
            .module
            .declare_function("striv_tan", Linkage::Import, &sig_f1)
            .unwrap();
        let sqrt_id = self
            .module
            .declare_function("striv_sqrt", Linkage::Import, &sig_f1)
            .unwrap();
        let mut sig_f2 = self.module.make_signature();
        sig_f2.params.push(AbiParam::new(types::F64));
        sig_f2.params.push(AbiParam::new(types::F64));
        sig_f2.returns.push(AbiParam::new(types::F64));
        let atan2_id = self
            .module
            .declare_function("striv_atan2", Linkage::Import, &sig_f2)
            .unwrap();

        let set_err_id = self
            .module
            .declare_function("striv_set_err", Linkage::Import, &sig_tweak_str)
            .unwrap();
        let mut sig_take_err = self.module.make_signature();
        sig_take_err.returns.push(AbiParam::new(int_ty));
        let take_err_id = self
            .module
            .declare_function("striv_take_err", Linkage::Import, &sig_take_err)
            .unwrap();
        let check_gas_id = self
            .module
            .declare_function("striv_check_gas", Linkage::Import, &sig_take_err)
            .unwrap();
        let mut sig_5i_i = self.module.make_signature();
        for _ in 0..5 {
            sig_5i_i.params.push(AbiParam::new(int_ty));
        }
        sig_5i_i.returns.push(AbiParam::new(int_ty));
        let ecs_set_id = self
            .module
            .declare_function("striv_ecs_set", Linkage::Import, &sig_5i_i)
            .unwrap();
        let mut sig_7i_v = self.module.make_signature();
        for _ in 0..7 {
            sig_7i_v.params.push(AbiParam::new(int_ty));
        }
        let ecs_query_run_id = self
            .module
            .declare_function("striv_ecs_query_run", Linkage::Import, &sig_7i_v)
            .unwrap();

        let mut user_enums = HashMap::new();
        let mut enum_defs = HashMap::new();
        for e in &prog.enums {
            enum_defs.insert(e.name.clone(), e.clone());
            for (idx, v) in e.variants.iter().enumerate() {
                let val = if e.is_open {
                    striv_syntax::fnv1a_64(&format!("{}::{}", e.name, v.name))
                } else {
                    idx as i64
                };
                user_enums.insert(format!("{}__{}", e.name, v.name), val);
            }
        }
        if !prog.consts.is_empty() {
            for (c_name, c_val) in eval_comptime_consts(prog) {
                user_enums.insert(c_name, c_val);
            }
        }

        let expanded_funcs = expand_trailing_blocks(&prog.functions);
        let (all_functions, par_helpers) = extract_parallel_bodies(&expanded_funcs);

        // 1. Pre-register all functions so they can call each other
        let mut func_ids = Vec::new();
        let mut fn_ret_structs: HashMap<String, Option<String>> = HashMap::new();
        let mut fn_ret_handles: HashMap<String, String> = HashMap::new();
        let mut fn_ret_floats: std::collections::HashSet<String> = std::collections::HashSet::new();
        for func in &all_functions {
            let ret_struct = func
                .ret_type
                .as_ref()
                .filter(|rt| self.kept_structs.contains_key(*rt))
                .cloned();
            fn_ret_structs.insert(func.name.clone(), ret_struct.clone());
            if let Some(rt) = &func.ret_type {
                if rt == "Bytes" {
                    fn_ret_handles.insert(func.name.clone(), rt.clone());
                } else if rt == "float" {
                    fn_ret_floats.insert(func.name.clone());
                }
            }

            let mut sig = self.module.make_signature();
            if ret_struct.is_some() {
                sig.params.push(AbiParam::new(int_ty)); // hidden __ret_ptr
            }
            for (_, p_ty, _) in &func.params {
                let abi_ty = if p_ty == "float" { types::F64 } else { int_ty };
                sig.params.push(AbiParam::new(abi_ty)); // Integer, float, or 8-byte struct pointer
            }
            let ret_ty = if func.ret_type.as_deref() == Some("float") {
                types::F64
            } else {
                int_ty
            };
            sig.returns.push(AbiParam::new(ret_ty));
            let id = self
                .module
                .declare_function(&func.name, Linkage::Export, &sig)
                .unwrap();
            func_ids.push(id);
        }

        // 2. Compile each function with Cranelift
        let mut func_ctx = FunctionBuilderContext::new();
        for (i, func) in all_functions.iter().enumerate() {
            let ret_struct = fn_ret_structs.get(&func.name).cloned().flatten();
            let mut ctx = self.module.make_context();
            if ret_struct.is_some() {
                ctx.func.signature.params.push(AbiParam::new(int_ty));
            }
            for (_, p_ty, _) in &func.params {
                let abi_ty = if p_ty == "float" { types::F64 } else { int_ty };
                ctx.func.signature.params.push(AbiParam::new(abi_ty));
            }
            let ret_ty = if func.ret_type.as_deref() == Some("float") {
                types::F64
            } else {
                int_ty
            };
            ctx.func.signature.returns.push(AbiParam::new(ret_ty));

            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let entry = builder.create_block();
                builder.append_block_params_for_function_params(entry);
                builder.switch_to_block(entry);
                builder.seal_block(entry);

                let mut funcs_map = HashMap::new();
                for (j, f) in all_functions.iter().enumerate() {
                    let f_ref = self.module.declare_func_in_func(func_ids[j], builder.func);
                    let param_meta = f.params.iter().map(|(_, ty, m)| (ty.clone(), *m)).collect();
                    funcs_map.insert(f.name.clone(), (f_ref, param_meta));
                }
                let assert_ref = self.module.declare_func_in_func(assert_id, builder.func);
                funcs_map.insert("assert".to_string(), (assert_ref, vec![("int".into(), false)]));
                let sin_ref = self.module.declare_func_in_func(sin_id, builder.func);
                funcs_map.insert("sin".to_string(), (sin_ref, vec![("float".into(), false)]));
                let cos_ref = self.module.declare_func_in_func(cos_id, builder.func);
                funcs_map.insert("cos".to_string(), (cos_ref, vec![("float".into(), false)]));
                let tan_ref = self.module.declare_func_in_func(tan_id, builder.func);
                funcs_map.insert("tan".to_string(), (tan_ref, vec![("float".into(), false)]));
                let sqrt_ref = self.module.declare_func_in_func(sqrt_id, builder.func);
                funcs_map.insert("sqrt".to_string(), (sqrt_ref, vec![("float".into(), false)]));
                let atan2_ref = self.module.declare_func_in_func(atan2_id, builder.func);
                funcs_map.insert(
                    "atan2".to_string(),
                    (
                        atan2_ref,
                        vec![("float".into(), false), ("float".into(), false)],
                    ),
                );
                for &(striv_name, c_sym, _, sig_str) in
                    lang_runtime::generated_bridge::exported_symbols()
                {
                    let (params_s, ret_s) = sig_str.split_once("->").unwrap_or((sig_str, "i"));
                    let mut sig_ext = self.module.make_signature();
                    for ch in params_s.chars() {
                        let ty = if ch == 'f' { types::F64 } else { int_ty };
                        sig_ext.params.push(AbiParam::new(ty));
                    }
                    let ret_ty = if ret_s.starts_with('f') {
                        types::F64
                    } else {
                        int_ty
                    };
                    sig_ext.returns.push(AbiParam::new(ret_ty));
                    let ext_id = self
                        .module
                        .declare_function(c_sym, Linkage::Import, &sig_ext)
                        .unwrap();
                    let ext_ref = self.module.declare_func_in_func(ext_id, builder.func);
                    let param_types = params_s
                        .chars()
                        .map(|ch| {
                            if ch == 'f' {
                                ("float".into(), false)
                            } else {
                                ("int".into(), false)
                            }
                        })
                        .collect();
                    funcs_map.insert(striv_name.to_string(), (ext_ref, param_types));
                }
                for (s_name, _, sig_str, mut_self) in lang_runtime::std_collection_symbols() {
                    let sym = format!("striv_std_{}", s_name.replace("::", "__"));
                    let (params_s, ret_s) = sig_str.split_once("->").unwrap_or((sig_str, "i"));
                    let mut sig_std = self.module.make_signature();
                    for ch in params_s.chars() {
                        let ty = if ch == 'f' { types::F64 } else { int_ty };
                        sig_std.params.push(AbiParam::new(ty));
                    }
                    let ret_ty = if ret_s.starts_with('f') {
                        types::F64
                    } else {
                        int_ty
                    };
                    sig_std.returns.push(AbiParam::new(ret_ty));
                    let std_id = self
                        .module
                        .declare_function(&sym, Linkage::Import, &sig_std)
                        .unwrap();
                    let std_ref = self.module.declare_func_in_func(std_id, builder.func);
                    let mut param_types: Vec<(String, bool)> = params_s
                        .chars()
                        .map(|ch| {
                            if ch == 'f' {
                                ("float".into(), false)
                            } else {
                                ("int".into(), false)
                            }
                        })
                        .collect();
                    if mut_self && !param_types.is_empty() {
                        param_types[0].1 = true;
                    }
                    funcs_map.insert(s_name.replace("::", "__"), (std_ref, param_types));
                }
                funcs_map.insert(
                    "Bytes__write_u32".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_write_u32_id, builder.func),
                        vec![("int".into(), false), ("int".into(), false)],
                    ),
                );
                funcs_map.insert(
                    "Bytes__write_i64".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_write_i64_id, builder.func),
                        vec![("int".into(), false), ("int".into(), false)],
                    ),
                );
                funcs_map.insert(
                    "Bytes__write_f64".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_write_f64_id, builder.func),
                        vec![("int".into(), false), ("float".into(), false)],
                    ),
                );
                funcs_map.insert(
                    "Bytes__read_u32".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_read_u32_id, builder.func),
                        vec![("int".into(), false), ("int".into(), false)],
                    ),
                );
                funcs_map.insert(
                    "Bytes__read_i64".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_read_i64_id, builder.func),
                        vec![("int".into(), false), ("int".into(), false)],
                    ),
                );
                funcs_map.insert(
                    "Bytes__read_f64".into(),
                    (
                        self.module
                            .declare_func_in_func(bytes_read_f64_id, builder.func),
                        vec![("int".into(), false), ("int".into(), false)],
                    ),
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
                    str_push_float_ref: self
                        .module
                        .declare_func_in_func(str_push_float_id, builder.func),
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
                    set_err_ref: self.module.declare_func_in_func(set_err_id, builder.func),
                    take_err_ref: self.module.declare_func_in_func(take_err_id, builder.func),
                    vec_new_ref: self.module.declare_func_in_func(vec_new_id, builder.func),
                    vec_push_ref: self.module.declare_func_in_func(vec_push_id, builder.func),
                    vec_get_ref: self.module.declare_func_in_func(vec_get_id, builder.func),
                    vec_set_ref: self.module.declare_func_in_func(vec_set_id, builder.func),
                    vec_len_ref: self.module.declare_func_in_func(vec_len_id, builder.func),
                    vec_free_ref: self.module.declare_func_in_func(vec_free_id, builder.func),
                    svec_new_ref: self.module.declare_func_in_func(svec_new_id, builder.func),
                    svec_push_ref: self.module.declare_func_in_func(svec_push_id, builder.func),
                    svec_get_ptr_ref: self
                        .module
                        .declare_func_in_func(svec_get_ptr_id, builder.func),
                    svec_len_ref: self.module.declare_func_in_func(svec_len_id, builder.func),
                    svec_bytes_ptr_ref: self
                        .module
                        .declare_func_in_func(svec_bytes_ptr_id, builder.func),
                    svec_byte_len_ref: self
                        .module
                        .declare_func_in_func(svec_byte_len_id, builder.func),
                    pack_struct_ref: self
                        .module
                        .declare_func_in_func(pack_struct_id, builder.func),
                    apply_struct_ref: self
                        .module
                        .declare_func_in_func(apply_struct_id, builder.func),
                    bytes_len_ref: self
                        .module
                        .declare_func_in_func(bytes_len_id, builder.func),
                    bytes_write_u32_ref: self
                        .module
                        .declare_func_in_func(bytes_write_u32_id, builder.func),
                    bytes_write_i64_ref: self
                        .module
                        .declare_func_in_func(bytes_write_i64_id, builder.func),
                    bytes_write_f64_ref: self
                        .module
                        .declare_func_in_func(bytes_write_f64_id, builder.func),
                    bytes_read_u32_ref: self
                        .module
                        .declare_func_in_func(bytes_read_u32_id, builder.func),
                    bytes_read_i64_ref: self
                        .module
                        .declare_func_in_func(bytes_read_i64_id, builder.func),
                    bytes_read_f64_ref: self
                        .module
                        .declare_func_in_func(bytes_read_f64_id, builder.func),
                    fn_ret_handles: &fn_ret_handles,
                    fn_ret_floats: &fn_ret_floats,
                    sandbox_mode: self.sandbox_mode,
                    check_gas_ref: self.module.declare_func_in_func(check_gas_id, builder.func),
                    ecs_set_ref: self.module.declare_func_in_func(ecs_set_id, builder.func),
                    ecs_query_run_ref: self
                        .module
                        .declare_func_in_func(ecs_query_run_id, builder.func),
                    sync_layouts: &sync_layouts,
                    svec_clear_ref: self
                        .module
                        .declare_func_in_func(svec_clear_id, builder.func),
                    group_begin_ref: self
                        .module
                        .declare_func_in_func(group_begin_id, builder.func),
                    group_spawn_ref: self
                        .module
                        .declare_func_in_func(group_spawn_id, builder.func),
                    group_wait_ref: self
                        .module
                        .declare_func_in_func(group_wait_id, builder.func),
                    active_group: None,
                    shared_new_ref: self
                        .module
                        .declare_func_in_func(shared_new_id, builder.func),
                    shared_lock_ref: self
                        .module
                        .declare_func_in_func(shared_lock_id, builder.func),
                    shared_unlock_ref: self
                        .module
                        .declare_func_in_func(shared_unlock_id, builder.func),
                    active_locks: Vec::new(),
                    par_for_svec_ref: self
                        .module
                        .declare_func_in_func(par_for_svec_id, builder.func),
                    par_for_vec_ref: self
                        .module
                        .declare_func_in_func(par_for_vec_id, builder.func),
                    select_recv_ref: self
                        .module
                        .declare_func_in_func(select_recv_id, builder.func),
                    par_helpers: &par_helpers,
                    current_fn: &func.name,
                    deferred: Vec::new(),
                    fn_ret_structs: &fn_ret_structs,
                    ret_struct_ptr: None,
                    loop_targets: Vec::new(),
                    literals: &mut self.kept_literals,
                    user_enums: &user_enums,
                    enum_defs: &enum_defs,
                    structs: &self.kept_structs,
                    funcs: funcs_map,
                };

                let mut vars: HashMap<String, VarInfo> = HashMap::new();
                let param_offset = if let Some(rs_name) = &ret_struct {
                    let rp_var = builder.declare_var(int_ty);
                    let rp_val = builder.block_params(entry)[0];
                    builder.def_var(rp_var, rp_val);
                    env.ret_struct_ptr = Some((rp_var, rs_name.clone()));
                    1
                } else {
                    0
                };
                for (idx, (name, ty_name, is_mut)) in func.params.iter().enumerate() {
                    let var_ty = if ty_name == "float" {
                        types::F64
                    } else {
                        int_ty
                    };
                    let var = builder.declare_var(var_ty);
                    let val = builder.block_params(entry)[idx + param_offset];
                    builder.def_var(var, val);
                    if ty_name == "__par_f64" {
                        let fval = builder.ins().bitcast(types::F64, MemFlagsData::new(), val);
                        let fvar = builder.declare_var(types::F64);
                        builder.def_var(fvar, fval);
                        vars.insert(
                            name.clone(),
                            VarInfo::Float {
                                var: fvar,
                                is_mut: false,
                            },
                        );
                    } else if ty_name == "String" {
                        vars.insert(
                            name.clone(),
                            VarInfo::Str {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    } else if let Some(s_name) = ty_name.strip_prefix("shared:") {
                        vars.insert(
                            name.clone(),
                            VarInfo::SharedStruct {
                                box_var: var,
                                struct_name: s_name.to_string(),
                            },
                        );
                    } else if let Some(inner_s) = ty_name
                        .strip_prefix("Vec<")
                        .and_then(|s| s.strip_suffix('>'))
                    {
                        vars.insert(
                            name.clone(),
                            VarInfo::StructVec {
                                var,
                                struct_name: inner_s.to_string(),
                                is_mut: *is_mut,
                            },
                        );
                    } else if ty_name == "Vec" {
                        vars.insert(
                            name.clone(),
                            VarInfo::Vec {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    } else if ty_name == "float" {
                        vars.insert(
                            name.clone(),
                            VarInfo::Float {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    } else if lang_runtime::std_drop_symbols()
                        .iter()
                        .any(|(n, _)| *n == ty_name)
                    {
                        vars.insert(
                            name.clone(),
                            VarInfo::Handle {
                                var,
                                type_name: ty_name.clone(),
                                is_mut: *is_mut,
                            },
                        );
                    } else if enum_defs
                        .get(ty_name)
                        .map(|ed| ed.variants.iter().any(|v| !v.field_types.is_empty()))
                        .unwrap_or(false)
                    {
                        vars.insert(
                            name.clone(),
                            VarInfo::EnumPtr {
                                ptr_var: var,
                                enum_name: ty_name.clone(),
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
                    let to_run: Vec<Stmt> = env.deferred.drain(..).rev().collect();
                    for d_stmt in &to_run {
                        compile_stmt(
                            d_stmt,
                            &mut builder,
                            &mut vars,
                            &mut env,
                            &mut owned_strings,
                            &mut owned_vecs,
                        );
                    }
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
        for (idx, func) in all_functions.iter().enumerate() {
            let ptr = self.module.get_finalized_function(func_ids[idx]);
            self.compiled_fns.insert(func.name.clone(), ptr);
        }
        let entry_idx = all_functions
            .iter()
            .position(|f| f.name == "main")
            .unwrap_or(func_ids.len() - 1);
        self.module.get_finalized_function(func_ids[entry_idx])
    }

    pub fn compile_tests(&mut self, base_prog: &Program) -> Vec<(String, fn() -> i64)> {
        let mut prog = base_prog.clone();
        let mut test_names = Vec::new();
        for (i, (t_name, t_body)) in prog.tests.drain(..).enumerate() {
            let fn_name = format!("__striv_test_{}", i);
            test_names.push((t_name, fn_name.clone()));
            prog.functions.push(striv_syntax::Function {
                name: fn_name,
                is_rpc: false,
            rust_body: None,
                type_params: Vec::new(),
                params: Vec::new(),
                ret_type: Some("int".to_string()),
                body: t_body,
            });
        }
        let _ = self.compile_program(&prog);
        test_names
            .into_iter()
            .map(|(label, fn_name)| {
                let ptr = self.compiled_fns[&fn_name];
                let f: fn() -> i64 = unsafe { std::mem::transmute(ptr) };
                (label, f)
            })
            .collect()
    }
}

// Helper: computes the byte offset of a field (field 0 -> 0, field 1 -> 8, ...)
fn field_type_info(ty: &str) -> (u32, u32) {
    match ty {
        "f32" | "u32" | "i32" => (4, 4),
        "u8" | "i8" | "bool" => (1, 1),
        _ => (8, 8),
    }
}

fn struct_byte_size(def: &StructDef) -> u32 {
    let mut offset = if def.sync_mask != 0 { 8u32 } else { 0u32 };
    let mut max_align = if def.sync_mask != 0 { 8u32 } else { 1u32 };
    for (_, ty) in &def.fields {
        let (size, align) = field_type_info(ty);
        max_align = max_align.max(align);
        offset = (offset + align - 1) & !(align - 1);
        offset += size;
    }
    ((offset + max_align - 1) & !(max_align - 1)).max(1)
}

fn field_offset(def: &StructDef, field: &str) -> i32 {
    let mut offset = if def.sync_mask != 0 { 8u32 } else { 0u32 };
    for (f, ty) in &def.fields {
        let (size, align) = field_type_info(ty);
        offset = (offset + align - 1) & !(align - 1);
        if f == field {
            return offset as i32;
        }
        offset += size;
    }
    panic!("Unknown field '{}' in struct '{}'", field, def.name);
}

fn field_type<'a>(def: &'a StructDef, field: &str) -> &'a str {
    def.fields
        .iter()
        .find(|(f, _)| f == field)
        .map(|(_, ty)| ty.as_str())
        .unwrap_or("int")
}

fn store_struct_field(
    builder: &mut FunctionBuilder,
    val: Value,
    ptr: Value,
    offset: i32,
    field_ty: &str,
) {
    match field_ty {
        "f32" => {
            let f = to_f64(val, builder);
            let f32v = builder.ins().fdemote(types::F32, f);
            builder.ins().store(MemFlagsData::new(), f32v, ptr, offset);
        }
        "u32" | "i32" => {
            let i32v = builder.ins().ireduce(types::I32, val);
            builder.ins().store(MemFlagsData::new(), i32v, ptr, offset);
        }
        "u8" | "i8" | "bool" => {
            let i8v = if builder.func.dfg.value_type(val) == types::I8 {
                val
            } else {
                builder.ins().ireduce(types::I8, val)
            };
            builder.ins().store(MemFlagsData::new(), i8v, ptr, offset);
        }
        "float" => {
            let f = to_f64(val, builder);
            builder.ins().store(MemFlagsData::new(), f, ptr, offset);
        }
        _ => {
            builder.ins().store(MemFlagsData::new(), val, ptr, offset);
        }
    }
}

fn load_struct_field(
    builder: &mut FunctionBuilder,
    ptr: Value,
    offset: i32,
    field_ty: &str,
    int_ty: Type,
) -> Value {
    match field_ty {
        "f32" => {
            let f32v = builder.ins().load(types::F32, MemFlagsData::new(), ptr, offset);
            builder.ins().fpromote(types::F64, f32v)
        }
        "u32" => {
            let i32v = builder.ins().load(types::I32, MemFlagsData::new(), ptr, offset);
            builder.ins().uextend(int_ty, i32v)
        }
        "i32" => {
            let i32v = builder.ins().load(types::I32, MemFlagsData::new(), ptr, offset);
            builder.ins().sextend(int_ty, i32v)
        }
        "u8" | "bool" => {
            let i8v = builder.ins().load(types::I8, MemFlagsData::new(), ptr, offset);
            builder.ins().uextend(int_ty, i8v)
        }
        "i8" => {
            let i8v = builder.ins().load(types::I8, MemFlagsData::new(), ptr, offset);
            builder.ins().sextend(int_ty, i8v)
        }
        "float" => builder.ins().load(types::F64, MemFlagsData::new(), ptr, offset),
        _ => builder.ins().load(int_ty, MemFlagsData::new(), ptr, offset),
    }
}

fn field_is_float(def: &StructDef, field: &str) -> bool {
    def.fields
        .iter()
        .find(|(f, _)| f == field)
        .map(|(_, ty)| ty == "float" || ty == "f32")
        .unwrap_or(false)
}

fn is_float_expr(expr: &Expr, vars: &HashMap<String, VarInfo>, env: &CompileEnv) -> bool {
    match expr {
        Expr::Float(_) | Expr::UnitFloat(_, _) => true,
        Expr::MethodCall { method, .. } if method == "read_f64" => true,
        Expr::Call { name, .. } if env.fn_ret_floats.contains(name) => true,
        Expr::Var(n) => matches!(vars.get(n), Some(VarInfo::Float { .. })),
        Expr::FieldAccess { var, field } => match vars.get(var) {
            Some(VarInfo::Struct { struct_name, .. })
            | Some(VarInfo::StructPtr { struct_name, .. }) => {
                field_is_float(env.structs.get(struct_name).unwrap(), field)
            }
            _ => false,
        },
        Expr::BinOp { left, op, right } => {
            !matches!(op, Op::Lt | Op::Gt | Op::Eq)
                && (is_float_expr(left, vars, env) || is_float_expr(right, vars, env))
        }
        _ => false,
    }
}

fn to_f64(val: Value, builder: &mut FunctionBuilder) -> Value {
    if builder.func.dfg.value_type(val) == types::F64 {
        val
    } else {
        builder.ins().fcvt_from_sint(types::F64, val)
    }
}

fn emit_sandbox_gas_guard(builder: &mut FunctionBuilder, exit_bb: Block, env: &CompileEnv) {
    if !env.sandbox_mode {
        return;
    }
    let call = builder.ins().call(env.check_gas_ref, &[]);
    let out_of_gas = builder.inst_results(call)[0];
    let cont_bb = builder.create_block();
    builder.ins().brif(out_of_gas, exit_bb, &[], cont_bb, &[]);
    builder.switch_to_block(cont_bb);
    builder.seal_block(cont_bb);
}

fn compile_call(
    name: &str,
    args: &[Expr],
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, VarInfo>,
    env: &mut CompileEnv,
) -> Value {
    if env.sandbox_mode
        && (name == "Bytes::read_file"
            || name == "Bytes__read_file"
            || name.contains("write_file"))
    {
        panic!(
            "Sandbox-Verstoß: Dateizugriff '{}' ist in Mods verboten!",
            name
        );
    }
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
    let ret_slot = if let Some(Some(rs_name)) = env.fn_ret_structs.get(name) {
        let def = env.structs.get(rs_name).unwrap();
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            struct_byte_size(def),
            3,
        ));
        arg_vals.push(builder.ins().stack_addr(env.int_ty, slot, 0));
        Some(slot)
    } else {
        None
    };

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
                Some(VarInfo::EnumVal { slot, .. }) => {
                    arg_vals.push(builder.ins().stack_addr(env.int_ty, *slot, 0));
                    continue;
                }
                Some(VarInfo::EnumPtr { ptr_var, .. }) => {
                    arg_vals.push(builder.use_var(*ptr_var));
                    continue;
                }
                _ => {}
            }
        }
        let mut val = compile_expr(arg, builder, vars, env);
        let expects_float = param_meta
            .get(i)
            .map(|(ty, _)| ty == "float")
            .unwrap_or(false);
        if expects_float {
            val = to_f64(val, builder);
        } else if builder.func.dfg.value_type(val) == types::F64 {
            val = builder.ins().fcvt_to_sint_sat(env.int_ty, val);
        } else if builder.func.dfg.value_type(val) == types::I8 {
            val = builder.ins().uextend(env.int_ty, val);
        }
        arg_vals.push(val);
    }

    let call = builder.ins().call(func_ref, &arg_vals);
    if let Some(slot) = ret_slot {
        builder.ins().stack_addr(env.int_ty, slot, 0)
    } else {
        builder.inst_results(call)[0]
    }
}

fn compile_method_call(
    receiver: &str,
    method: &str,
    args: &[Expr],
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, VarInfo>,
    env: &mut CompileEnv,
) -> Value {
    if matches!(
        method,
        "is_dirty" | "clear_dirty" | "pack_delta" | "pack_full" | "apply_delta"
    ) {
        let (base_ptr, struct_name) = match vars.get(receiver) {
            Some(VarInfo::Struct {
                slot, struct_name, ..
            }) => (
                builder.ins().stack_addr(env.int_ty, *slot, 0),
                struct_name.clone(),
            ),
            Some(VarInfo::StructPtr {
                ptr_var,
                struct_name,
                ..
            }) => (builder.use_var(*ptr_var), struct_name.clone()),
            _ => panic!("'{}' is not a struct for .{}()", receiver, method),
        };
        let layout_addr = *env
            .sync_layouts
            .get(&struct_name)
            .unwrap_or_else(|| panic!("Struct '{}' has no `sync` fields", struct_name));
        let layout_ptr = builder.ins().iconst(env.int_ty, layout_addr as i64);
        return match method {
            "is_dirty" => {
                let mask = builder
                    .ins()
                    .load(env.int_ty, MemFlagsData::new(), base_ptr, 0);
                let b = builder.ins().icmp_imm(IntCC::NotEqual, mask, 0);
                builder.ins().uextend(env.int_ty, b)
            }
            "clear_dirty" => {
                let zero = builder.ins().iconst(env.int_ty, 0);
                builder.ins().store(MemFlagsData::new(), zero, base_ptr, 0);
                zero
            }
            "pack_delta" => {
                let full = builder.ins().iconst(env.int_ty, 0);
                let c = builder
                    .ins()
                    .call(env.pack_struct_ref, &[base_ptr, layout_ptr, full]);
                builder.inst_results(c)[0]
            }
            "pack_full" => {
                let full = builder.ins().iconst(env.int_ty, 1);
                let c = builder
                    .ins()
                    .call(env.pack_struct_ref, &[base_ptr, layout_ptr, full]);
                builder.inst_results(c)[0]
            }
            "apply_delta" => {
                let pkt = compile_expr(&args[0], builder, vars, env);
                let c = builder
                    .ins()
                    .call(env.apply_struct_ref, &[base_ptr, layout_ptr, pkt]);
                builder.inst_results(c)[0]
            }
            _ => unreachable!(),
        };
    }

    if let Some(VarInfo::Handle { var, type_name, .. }) = vars.get(receiver).cloned() {
        if type_name == "Ecs" && method == "set" {
            let ecs_ptr = builder.use_var(var);
            let ent_val = compile_expr(&args[0], builder, vars, env);
            let (s_name, fields) = match &args[1] {
                Expr::StructInit { name, fields } => (name.clone(), fields),
                _ => panic!("ecs.set(ent, Struct {{ ... }}) expects a struct literal"),
            };
            let def = env.structs.get(&s_name).unwrap().clone();
            let byte_size = struct_byte_size(&def);
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                byte_size,
                3,
            ));
            let base = builder.ins().stack_addr(env.int_ty, slot, 0);
            if def.sync_mask != 0 {
                let zero = builder.ins().iconst(env.int_ty, 0);
                builder.ins().store(MemFlagsData::new(), zero, base, 0);
            }
            for (f_name, f_expr) in fields {
                let v = compile_expr(f_expr, builder, vars, env);
                let off = field_offset(&def, f_name);
                let f_ty = field_type(&def, f_name);
                store_struct_field(builder, v, base, off, f_ty);
            }
            let tid = builder
                .ins()
                .iconst(env.int_ty, striv_syntax::fnv1a_64(&s_name));
            let esz = builder.ins().iconst(env.int_ty, byte_size as i64);
            let c = builder
                .ins()
                .call(env.ecs_set_ref, &[ecs_ptr, ent_val, tid, esz, base]);
            return builder.inst_results(c)[0];
        }
    }

    if let Some(VarInfo::Bytes { var, .. }) = vars.get(receiver).cloned() {
        let b_ptr = builder.use_var(var);
        return match method {
            "len" => {
                let c = builder.ins().call(env.bytes_len_ref, &[b_ptr]);
                builder.inst_results(c)[0]
            }
            "write_u32" => {
                let v = compile_expr(&args[0], builder, vars, env);
                let c = builder.ins().call(env.bytes_write_u32_ref, &[b_ptr, v]);
                builder.inst_results(c)[0]
            }
            "write_i64" => {
                let v = compile_expr(&args[0], builder, vars, env);
                let c = builder.ins().call(env.bytes_write_i64_ref, &[b_ptr, v]);
                builder.inst_results(c)[0]
            }
            "write_f64" => {
                let v = to_f64(compile_expr(&args[0], builder, vars, env), builder);
                let c = builder.ins().call(env.bytes_write_f64_ref, &[b_ptr, v]);
                builder.inst_results(c)[0]
            }
            "read_u32" => {
                let off = compile_expr(&args[0], builder, vars, env);
                let c = builder.ins().call(env.bytes_read_u32_ref, &[b_ptr, off]);
                builder.inst_results(c)[0]
            }
            "read_i64" => {
                let off = compile_expr(&args[0], builder, vars, env);
                let c = builder.ins().call(env.bytes_read_i64_ref, &[b_ptr, off]);
                builder.inst_results(c)[0]
            }
            "read_f64" => {
                let off = compile_expr(&args[0], builder, vars, env);
                let c = builder.ins().call(env.bytes_read_f64_ref, &[b_ptr, off]);
                builder.inst_results(c)[0]
            }
            _ => panic!("Unknown method '.{}()' on Bytes", method),
        };
    }

    if let Some(VarInfo::StructVec {
        var,
        struct_name,
        is_mut,
    }) = vars.get(receiver).cloned()
    {
        let vec_ptr = builder.use_var(var);
        return match method {
            "len" => {
                let c = builder.ins().call(env.svec_len_ref, &[vec_ptr]);
                builder.inst_results(c)[0]
            }
            "as_bytes_ptr" => {
                let c = builder.ins().call(env.svec_bytes_ptr_ref, &[vec_ptr]);
                builder.inst_results(c)[0]
            }
            "byte_len" => {
                let c = builder.ins().call(env.svec_byte_len_ref, &[vec_ptr]);
                builder.inst_results(c)[0]
            }
            "clear" => {
                assert!(
                    is_mut,
                    "Error: '{}' must be `mut` for `.clear()`!",
                    receiver
                );
                let c = builder.ins().call(env.svec_clear_ref, &[vec_ptr]);
                builder.inst_results(c)[0]
            }
            "push" => {
                assert!(is_mut, "Error: '{}' must be `mut` for `.push()`!", receiver);
                let arg = &args[0];
                let src_addr = match arg {
                    Expr::StructInit {
                        name: s_name,
                        fields,
                    } => {
                        assert_eq!(s_name, &struct_name, "Wrong struct type in push()");
                        let def = env.structs.get(s_name).unwrap().clone();
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            struct_byte_size(&def),
                            3,
                        ));
                        if def.sync_mask != 0 {
                    let base = builder.ins().stack_addr(env.int_ty, slot, 0);
                    let zero = builder.ins().iconst(env.int_ty, 0);
                    builder.ins().store(MemFlagsData::new(), zero, base, 0);
                }
                for (f_name, f_expr) in fields {
                            let v = compile_expr(f_expr, builder, vars, env);
                            let off = field_offset(&def, f_name);
                            let f_ty = field_type(&def, f_name);
                            let base = builder.ins().stack_addr(env.int_ty, slot, 0);
                            store_struct_field(builder, v, base, off, f_ty);
                        }
                        builder.ins().stack_addr(env.int_ty, slot, 0)
                    }
                    Expr::Var(vn) => match vars.get(vn) {
                        Some(VarInfo::Struct { slot, .. }) => {
                            builder.ins().stack_addr(env.int_ty, *slot, 0)
                        }
                        Some(VarInfo::StructPtr { ptr_var, .. }) => builder.use_var(*ptr_var),
                        _ => panic!("Expected '{}' in push()", struct_name),
                    },
                    _ => panic!("Expected struct '{}' in push()", struct_name),
                };
                let c = builder.ins().call(env.svec_push_ref, &[vec_ptr, src_addr]);
                builder.inst_results(c)[0]
            }
            _ => panic!("Unknown method '.{}()' on Vec<{}>", method, struct_name),
        };
    }
    if let Some(VarInfo::Handle { var, type_name, .. }) = vars.get(receiver) {
        let full_name = format!("{}__{}", type_name, method);
        let (func_ref, param_meta) = env
            .funcs
            .get(&full_name)
            .unwrap_or_else(|| panic!("Unknown method '{}' on '{}'", method, type_name))
            .clone();
        let mut arg_vals = vec![builder.use_var(*var)];
        for (i, arg) in args.iter().enumerate() {
            let mut val = compile_expr(arg, builder, vars, env);
            let expects_float = param_meta
                .get(i + 1)
                .map(|(ty, _)| ty == "float")
                .unwrap_or(false);
            if expects_float {
                val = to_f64(val, builder);
            } else if builder.func.dfg.value_type(val) == types::F64 {
                val = builder.ins().fcvt_to_sint_sat(env.int_ty, val);
            }
            arg_vals.push(val);
        }
        let call = builder.ins().call(func_ref, &arg_vals);
        return builder.inst_results(call)[0];
    }
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
        Stmt::LetTuple { names, value } => {
            if let Expr::Tuple(items) = value {
                assert_eq!(names.len(), items.len(), "Tuple length mismatch");
                for ((n, is_mut), item) in names.iter().zip(items.iter()) {
                    let val = compile_expr(item, builder, vars, env);
                    let ty = builder.func.dfg.value_type(val);
                    let var = builder.declare_var(ty);
                    builder.def_var(var, val);
                    if ty == types::F64 {
                        vars.insert(
                            n.clone(),
                            VarInfo::Float {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    } else {
                        vars.insert(
                            n.clone(),
                            VarInfo::Int {
                                var,
                                is_mut: *is_mut,
                            },
                        );
                    }
                }
            } else {
                let base_ptr = compile_expr(value, builder, vars, env);
                for (i, (n, is_mut)) in names.iter().enumerate() {
                    let val = builder.ins().load(
                        env.int_ty,
                        MemFlagsData::new(),
                        base_ptr,
                        (i * 8) as i32,
                    );
                    let var = builder.declare_var(env.int_ty);
                    builder.def_var(var, val);
                    vars.insert(
                        n.clone(),
                        VarInfo::Int {
                            var,
                            is_mut: *is_mut,
                        },
                    );
                }
            }
            false
        }
        Stmt::Let {
            name,
            ty_hint,
            is_mut,
            value,
        } => {
            if let Some(inner_s) = ty_hint
                .as_deref()
                .and_then(|t| t.strip_prefix("Vec<"))
                .and_then(|t| t.strip_suffix('>'))
            {
                let def = env
                    .structs
                    .get(inner_s)
                    .unwrap_or_else(|| panic!("Unknown struct in Vec<{}>", inner_s));
                let elem_bytes = struct_byte_size(&def) as i64;
                let size_val = builder.ins().iconst(env.int_ty, elem_bytes);
                let call = builder.ins().call(env.svec_new_ref, &[size_val]);
                let ptr = builder.inst_results(call)[0];
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, ptr);
                vars.insert(
                    name.clone(),
                    VarInfo::StructVec {
                        var,
                        struct_name: inner_s.to_string(),
                        is_mut: *is_mut,
                    },
                );
                return false;
            }
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
                let byte_size = struct_byte_size(def);

                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    byte_size,
                    3, // Alignment 2^3 = 8 Bytes
                ));

                if def.sync_mask != 0 {
                    let base = builder.ins().stack_addr(env.int_ty, slot, 0);
                    let zero = builder.ins().iconst(env.int_ty, 0);
                    builder.ins().store(MemFlagsData::new(), zero, base, 0);
                }
                for (f_name, f_expr) in fields {
                    let val = compile_expr(f_expr, builder, vars, env);
                    let offset = field_offset(def, f_name);
                    let f_ty = field_type(def, f_name);
                    let base = builder.ins().stack_addr(env.int_ty, slot, 0);
                    store_struct_field(builder, val, base, offset, f_ty);
                }

                vars.insert(
                    name.clone(),
                    VarInfo::Struct {
                        slot,
                        struct_name: s_name.clone(),
                        is_mut: *is_mut,
                    },
                );
            } else if let Expr::Tuple(items) = value {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    (items.len() * 8) as u32,
                    3,
                ));
                let mut elem_is_float = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let val = compile_expr(item, builder, vars, env);
                    let is_f = builder.func.dfg.value_type(val) == types::F64;
                    elem_is_float.push(is_f);
                    builder
                        .ins()
                        .stack_store(env.int_ty, val, slot, (i * 8) as i32);
                }
                vars.insert(
                    name.clone(),
                    VarInfo::Tuple {
                        slot,
                        elem_is_float,
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
            } else if let Expr::Call {
                name: fn_name,
                args,
            } = value
                && let Some((en, vn)) = fn_name.split_once("__")
                && let Some(ed) = env.enum_defs.get(en).cloned()
                && let Some((tag_idx, variant)) =
                    ed.variants.iter().enumerate().find(|(_, v)| v.name == vn)
            {
                let max_fields = ed
                    .variants
                    .iter()
                    .map(|v| v.field_types.len())
                    .max()
                    .unwrap_or(0);
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    ((1 + max_fields) * 8) as u32,
                    3,
                ));
                let tag_val = builder.ins().iconst(env.int_ty, tag_idx as i64);
                builder.ins().stack_store(env.int_ty, tag_val, slot, 0);
                for (idx, arg) in args.iter().enumerate() {
                    let mut v = compile_expr(arg, builder, vars, env);
                    if variant
                        .field_types
                        .get(idx)
                        .map(|t| t == "float")
                        .unwrap_or(false)
                    {
                        v = to_f64(v, builder);
                    }
                    builder
                        .ins()
                        .stack_store(env.int_ty, v, slot, ((idx + 1) * 8) as i32);
                }
                vars.insert(
                    name.clone(),
                    VarInfo::EnumVal {
                        slot,
                        enum_name: en.to_string(),
                        is_mut: *is_mut,
                    },
                );
            } else if let Expr::PathConst { ty, member } = value
                && let Some(ed) = env.enum_defs.get(ty).cloned()
                && ed.variants.iter().any(|v| !v.field_types.is_empty())
                && let Some((tag_idx, _)) = ed
                    .variants
                    .iter()
                    .enumerate()
                    .find(|(_, v)| &v.name == member)
            {
                let max_fields = ed
                    .variants
                    .iter()
                    .map(|v| v.field_types.len())
                    .max()
                    .unwrap_or(0);
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    ((1 + max_fields) * 8) as u32,
                    3,
                ));
                let tag_val = builder.ins().iconst(env.int_ty, tag_idx as i64);
                builder.ins().stack_store(env.int_ty, tag_val, slot, 0);
                vars.insert(
                    name.clone(),
                    VarInfo::EnumVal {
                        slot,
                        enum_name: ty.clone(),
                        is_mut: *is_mut,
                    },
                );
            } else if let Some(rs_name) = expr_struct_type(value, vars, env) {
                let ptr = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, ptr);
                vars.insert(
                    name.clone(),
                    VarInfo::StructPtr {
                        ptr_var: var,
                        struct_name: rs_name,
                        is_mut: *is_mut,
                    },
                );
            } else if matches!(value, Expr::Call { name, .. } if name.contains("__") && !name.contains("__G_")) {
                let Expr::Call { name: fn_name, .. } = value else {
                    unreachable!()
                };
                let type_name = fn_name.split("__").next().unwrap().to_string();
                let ptr = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, ptr);
                vars.insert(
                    name.clone(),
                    VarInfo::Handle {
                        var,
                        type_name,
                        is_mut: *is_mut,
                    },
                );
            } else if matches!(value, Expr::MethodCall { method, .. } if method == "pack_delta" || method == "pack_full")
                || matches!(value, Expr::Call { name, .. } if name == "Bytes__new" || name == "Bytes::new" || env.fn_ret_handles.get(name).map(|s| s == "Bytes").unwrap_or(false))
            {
                let val = compile_expr(value, builder, vars, env);
                let var = builder.declare_var(env.int_ty);
                builder.def_var(var, val);
                vars.insert(
                    name.clone(),
                    VarInfo::Bytes {
                        var,
                        is_mut: *is_mut,
                    },
                );
            } else if is_float_expr(value, vars, env) {
                let val = to_f64(compile_expr(value, builder, vars, env), builder);
                let var = builder.declare_var(types::F64);
                builder.def_var(var, val);
                vars.insert(
                    name.clone(),
                    VarInfo::Float {
                        var,
                        is_mut: *is_mut,
                    },
                );
            } else {
                // Case B: Regular integer or float-returning call
                let val = compile_expr(value, builder, vars, env);
                let is_f = builder.func.dfg.value_type(val) == types::F64;
                let var = builder.declare_var(if is_f { types::F64 } else { env.int_ty });
                builder.def_var(var, val);
                vars.insert(
                    name.clone(),
                    if is_f {
                        VarInfo::Float {
                            var,
                            is_mut: *is_mut,
                        }
                    } else {
                        VarInfo::Int {
                            var,
                            is_mut: *is_mut,
                        }
                    },
                );
            }
            false
        }
        Stmt::Assign { name, value } => match vars.get(name).cloned() {
            Some(VarInfo::Int { var, is_mut }) => {
                assert!(is_mut, "Error: '{}' is not `mut`!", name);
                let val = compile_expr(value, builder, vars, env);
                builder.def_var(var, val);
                false
            }
            Some(VarInfo::Float { var, is_mut }) => {
                assert!(is_mut, "Error: '{}' is not `mut`!", name);
                let val = to_f64(compile_expr(value, builder, vars, env), builder);
                builder.def_var(var, val);
                false
            }
            _ => panic!("'{}' is not a regular number variable", name),
        },
        Stmt::FieldAssign { var, field, value } => {
            let mut val = compile_expr(value, builder, vars, env);
            match vars.get(var).cloned() {
                Some(VarInfo::Tuple { slot, is_mut, .. }) => {
                    assert!(is_mut, "Error: tuple '{}' is not `mut`!", var);
                    let idx = tuple_field_index(field)
                        .unwrap_or_else(|| panic!("Unknown tuple field '.{}'", field));
                    builder
                        .ins()
                        .stack_store(env.int_ty, val, slot, (idx * 8) as i32);
                }
                Some(VarInfo::Struct {
                    slot,
                    struct_name,
                    is_mut,
                }) => {
                    assert!(is_mut, "Error: '{}' is not `mut`!", var);
                    let def = env.structs.get(&struct_name).unwrap();
                    let offset = field_offset(def, field);
                    let f_ty = field_type(def, field);
                    let base = builder.ins().stack_addr(env.int_ty, slot, 0);
                    if let Some(fi) = def.fields.iter().position(|(f, _)| f == field) {
                        if fi < 64 && (def.sync_mask & (1u64 << fi)) != 0 {
                            let old_v = load_struct_field(builder, base, offset, f_ty, env.int_ty);
                            let changed = if f_ty == "float" || f_ty == "f32" {
                                let nf = to_f64(val, builder);
                                builder.ins().fcmp(FloatCC::NotEqual, old_v, nf)
                            } else {
                                builder.ins().icmp(IntCC::NotEqual, old_v, val)
                            };
                            let mask = builder.ins().load(env.int_ty, MemFlagsData::new(), base, 0);
                            let new_mask = builder.ins().bor_imm(mask, (1u64 << fi) as i64);
                            let updated = builder.ins().select(changed, new_mask, mask);
                            builder.ins().store(MemFlagsData::new(), updated, base, 0);
                        }
                    }
                    store_struct_field(builder, val, base, offset, f_ty);
                }
                Some(VarInfo::StructPtr {
                    ptr_var,
                    struct_name,
                    is_mut,
                }) => {
                    assert!(is_mut, "Error: parameter '{}' is not `mut`!", var);
                    let def = env.structs.get(&struct_name).unwrap();
                    let offset = field_offset(def, field);
                    let f_ty = field_type(def, field);
                    let addr = builder.use_var(ptr_var);
                    if let Some(fi) = def.fields.iter().position(|(f, _)| f == field) {
                        if fi < 64 && (def.sync_mask & (1u64 << fi)) != 0 {
                            let old_v = load_struct_field(builder, addr, offset, f_ty, env.int_ty);
                            let changed = if f_ty == "float" || f_ty == "f32" {
                                let nf = to_f64(val, builder);
                                builder.ins().fcmp(FloatCC::NotEqual, old_v, nf)
                            } else {
                                builder.ins().icmp(IntCC::NotEqual, old_v, val)
                            };
                            let mask = builder.ins().load(env.int_ty, MemFlagsData::new(), addr, 0);
                            let new_mask = builder.ins().bor_imm(mask, (1u64 << fi) as i64);
                            let updated = builder.ins().select(changed, new_mask, mask);
                            builder.ins().store(MemFlagsData::new(), updated, addr, 0);
                        }
                    }
                    store_struct_field(builder, val, addr, offset, f_ty);
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
            let mut val = compile_expr(expr, builder, vars, env);
            if builder.func.dfg.value_type(val) == types::F64 {
                val = builder.ins().fcvt_to_sint_sat(env.int_ty, val);
            }
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
            emit_sandbox_gas_guard(builder, exit_bb, env);

            let cond_val = compile_expr(cond, builder, vars, env);
            builder.ins().brif(cond_val, body_bb, &[], exit_bb, &[]);

            builder.switch_to_block(body_bb);
            builder.seal_block(body_bb);
            env.loop_targets.push((header_bb, exit_bb));
            let mut body_ret = false;
            for s in body {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    body_ret = true;
                    break;
                }
            }
            env.loop_targets.pop();
            if !body_ret {
                builder.ins().jump(header_bb, &[]);
            }

            builder.seal_block(header_bb);

            builder.switch_to_block(exit_bb);
            builder.seal_block(exit_bb);
            false
        }
        Stmt::Match { target, arms } => {
            let (target_val, enum_slot_or_ptr) = if let Expr::Var(vn) = target {
                match vars.get(vn) {
                    Some(VarInfo::EnumVal { slot, .. }) => {
                        let tag = builder.ins().stack_load(env.int_ty, env.int_ty, *slot, 0);
                        (tag, Some((*slot, None)))
                    }
                    Some(VarInfo::EnumPtr { ptr_var, .. }) => {
                        let ptr = builder.use_var(*ptr_var);
                        let tag = builder.ins().load(env.int_ty, MemFlagsData::new(), ptr, 0);
                        (
                            tag,
                            Some((
                                builder.create_sized_stack_slot(StackSlotData::new(
                                    StackSlotKind::ExplicitSlot,
                                    8,
                                    3,
                                )),
                                Some(ptr),
                            )),
                        )
                    }
                    _ => (compile_expr(target, builder, vars, env), None),
                }
            } else {
                (compile_expr(target, builder, vars, env), None)
            };
            let exit_bb = builder.create_block();

            for (pat, body) in arms {
                match pat {
                    striv_syntax::MatchPat::Wildcard => {
                        let mut ret = false;
                        for s in body {
                            if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                                ret = true;
                                break;
                            }
                        }
                        if !ret {
                            builder.ins().jump(exit_bb, &[]);
                        }
                        let after_wild = builder.create_block();
                        builder.switch_to_block(after_wild);
                        builder.seal_block(after_wild);
                        break;
                    }
                    striv_syntax::MatchPat::Expr(pat_expr) => {
                        let pat_val = compile_expr(pat_expr, builder, vars, env);
                        let cond = builder.ins().icmp(IntCC::Equal, target_val, pat_val);

                        let arm_bb = builder.create_block();
                        let next_bb = builder.create_block();
                        builder.ins().brif(cond, arm_bb, &[], next_bb, &[]);

                        builder.switch_to_block(arm_bb);
                        builder.seal_block(arm_bb);
                        let mut ret = false;
                        for s in body {
                            if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                                ret = true;
                                break;
                            }
                        }
                        if !ret {
                            builder.ins().jump(exit_bb, &[]);
                        }

                        builder.switch_to_block(next_bb);
                        builder.seal_block(next_bb);
                    }
                    striv_syntax::MatchPat::EnumDestructure {
                        enum_name,
                        variant,
                        bindings,
                    } => {
                        let ed = env
                            .enum_defs
                            .get(enum_name)
                            .unwrap_or_else(|| panic!("Unknown enum '{}'", enum_name))
                            .clone();
                        let (tag_idx, ev) = ed
                            .variants
                            .iter()
                            .enumerate()
                            .find(|(_, v)| &v.name == variant)
                            .unwrap_or_else(|| {
                                panic!("Unknown variant '{}::{}'", enum_name, variant)
                            });

                        let cond =
                            builder
                                .ins()
                                .icmp_imm_s(IntCC::Equal, target_val, tag_idx as i64);
                        let arm_bb = builder.create_block();
                        let next_bb = builder.create_block();
                        builder.ins().brif(cond, arm_bb, &[], next_bb, &[]);

                        builder.switch_to_block(arm_bb);
                        builder.seal_block(arm_bb);

                        if let Some((slot, ptr_opt)) = enum_slot_or_ptr {
                            for (idx, b_name) in bindings.iter().enumerate() {
                                let is_f = ev
                                    .field_types
                                    .get(idx)
                                    .map(|t| t == "float")
                                    .unwrap_or(false);
                                let ty = if is_f { types::F64 } else { env.int_ty };
                                let off = ((idx + 1) * 8) as i32;
                                let val = if let Some(ptr) = ptr_opt {
                                    builder.ins().load(ty, MemFlagsData::new(), ptr, off)
                                } else {
                                    builder.ins().stack_load(env.int_ty, ty, slot, off)
                                };
                                let v_var = builder.declare_var(ty);
                                builder.def_var(v_var, val);
                                if is_f {
                                    vars.insert(
                                        b_name.clone(),
                                        VarInfo::Float {
                                            var: v_var,
                                            is_mut: false,
                                        },
                                    );
                                } else {
                                    vars.insert(
                                        b_name.clone(),
                                        VarInfo::Int {
                                            var: v_var,
                                            is_mut: false,
                                        },
                                    );
                                }
                            }
                        }

                        let mut ret = false;
                        for s in body {
                            if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                                ret = true;
                                break;
                            }
                        }
                        if !ret {
                            builder.ins().jump(exit_bb, &[]);
                        }

                        builder.switch_to_block(next_bb);
                        builder.seal_block(next_bb);
                    }
                }
            }

            builder.ins().jump(exit_bb, &[]);
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
        Stmt::CompoundAssign { target, op, value } => {
            let desugared = match target {
                striv_syntax::AssignTarget::Var(name) => Stmt::Assign {
                    name: name.clone(),
                    value: Expr::BinOp {
                        left: Box::new(Expr::Var(name.clone())),
                        op: *op,
                        right: Box::new(value.clone()),
                    },
                },
                striv_syntax::AssignTarget::Field { var, field } => Stmt::FieldAssign {
                    var: var.clone(),
                    field: field.clone(),
                    value: Expr::BinOp {
                        left: Box::new(Expr::FieldAccess {
                            var: var.clone(),
                            field: field.clone(),
                        }),
                        op: *op,
                        right: Box::new(value.clone()),
                    },
                },
                striv_syntax::AssignTarget::Index { var, index } => Stmt::IndexAssign {
                    var: var.clone(),
                    index: index.clone(),
                    value: Expr::BinOp {
                        left: Box::new(Expr::Index {
                            var: var.clone(),
                            index: Box::new(index.clone()),
                        }),
                        op: *op,
                        right: Box::new(value.clone()),
                    },
                },
            };
            compile_stmt(&desugared, builder, vars, env, owned_strings, owned_vecs)
        }
        Stmt::Break => {
            let (_, break_bb) = *env
                .loop_targets
                .last()
                .expect("`break` can only be used inside a loop!");
            builder.ins().jump(break_bb, &[]);
            true
        }
        Stmt::Query {
            terms,
            ecs_var,
            is_parallel,
            ..
        } => {
            let ecs_ptr = match vars.get(ecs_var) {
                Some(VarInfo::Handle { var, type_name, .. }) if type_name == "Ecs" => {
                    builder.use_var(*var)
                }
                _ => panic!("'{}' is not an Ecs instance", ecs_var),
            };
            let key = format!(
                "query:{}:{}:{}",
                env.current_fn,
                ecs_var,
                terms
                    .iter()
                    .map(|(v, t, _)| format!("{}:{}", v, t))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let (helper_name, captures) = env.par_helpers.get(&key).unwrap().clone();
            let (helper_ref, _) = env.funcs.get(&helper_name).unwrap().clone();

            let tids_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (terms.len().max(1) * 8) as u32,
                3,
            ));
            for (i, (_, s_name, _)) in terms.iter().enumerate() {
                let tid = builder
                    .ins()
                    .iconst(env.int_ty, striv_syntax::fnv1a_64(s_name));
                builder
                    .ins()
                    .stack_store(env.int_ty, tid, tids_slot, (i * 8) as i32);
            }

            let cap_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (captures.len().max(1) * 8) as u32,
                3,
            ));
            for (i, cap_name) in captures.iter().enumerate() {
                let v = compile_expr(&Expr::Var(cap_name.clone()), builder, vars, env);
                let v_i64 = if builder.func.dfg.value_type(v) == types::F64 {
                    builder.ins().bitcast(env.int_ty, MemFlagsData::new(), v)
                } else {
                    v
                };
                builder
                    .ins()
                    .stack_store(env.int_ty, v_i64, cap_slot, (i * 8) as i32);
            }

            let tids_addr = builder.ins().stack_addr(env.int_ty, tids_slot, 0);
            let comp_count = builder.ins().iconst(env.int_ty, terms.len() as i64);
            let fn_addr = builder.ins().func_addr(env.int_ty, helper_ref);
            let caps_addr = builder.ins().stack_addr(env.int_ty, cap_slot, 0);
            let cap_count = builder.ins().iconst(env.int_ty, captures.len() as i64);
            let par_flag = builder
                .ins()
                .iconst(env.int_ty, if *is_parallel { 1 } else { 0 });
            builder.ins().call(
                env.ecs_query_run_ref,
                &[
                    ecs_ptr,
                    tids_addr,
                    comp_count,
                    fn_addr,
                    caps_addr,
                    cap_count,
                    par_flag,
                ],
            );
            false
        }
        Stmt::Continue => {
            let (cont_bb, _) = *env
                .loop_targets
                .last()
                .expect("`continue` can only be used inside a loop!");
            builder.ins().jump(cont_bb, &[]);
            true
        }
        Stmt::Defer(inner) => {
            env.deferred.push((**inner).clone());
            false
        }
        Stmt::Block(stmts) => {
            for s in stmts {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    return true;
                }
            }
            false
        }
        Stmt::Yield | Stmt::BlockCall { .. } => false,
        Stmt::Select { arms, timeout } => {
            let chans_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (arms.len().max(1) * 8) as u32,
                3,
            ));
            let val_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                3,
            ));
            for (i, arm) in arms.iter().enumerate() {
                let ch_ptr = compile_expr(&Expr::Var(arm.channel_var.clone()), builder, vars, env);
                builder
                    .ins()
                    .stack_store(env.int_ty, ch_ptr, chans_slot, (i * 8) as i32);
            }
            let timeout_val = if let Some((ms_expr, _)) = timeout {
                compile_expr(ms_expr, builder, vars, env)
            } else {
                builder.ins().iconst(env.int_ty, -1)
            };
            let chans_addr = builder.ins().stack_addr(env.int_ty, chans_slot, 0);
            let out_val_addr = builder.ins().stack_addr(env.int_ty, val_slot, 0);
            let count_val = builder.ins().iconst(env.int_ty, arms.len() as i64);

            let call = builder.ins().call(
                env.select_recv_ref,
                &[chans_addr, count_val, timeout_val, out_val_addr],
            );
            let winner_idx = builder.inst_results(call)[0];
            let recv_val = builder
                .ins()
                .stack_load(env.int_ty, env.int_ty, val_slot, 0);

            let exit_bb = builder.create_block();
            for (i, arm) in arms.iter().enumerate() {
                let is_win = builder.ins().icmp_imm_s(IntCC::Equal, winner_idx, i as i64);
                let arm_bb = builder.create_block();
                let next_bb = builder.create_block();
                builder.ins().brif(is_win, arm_bb, &[], next_bb, &[]);

                builder.switch_to_block(arm_bb);
                builder.seal_block(arm_bb);

                let v_var = builder.declare_var(env.int_ty);
                builder.def_var(v_var, recv_val);
                vars.insert(
                    arm.bind_var.clone(),
                    VarInfo::Int {
                        var: v_var,
                        is_mut: false,
                    },
                );

                let mut ret = false;
                for s in &arm.body {
                    if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                        ret = true;
                        break;
                    }
                }
                if !ret {
                    builder.ins().jump(exit_bb, &[]);
                }

                builder.switch_to_block(next_bb);
                builder.seal_block(next_bb);
            }

            if let Some((_, t_body)) = timeout {
                let mut ret = false;
                for s in t_body {
                    if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                        ret = true;
                        break;
                    }
                }
                if !ret {
                    builder.ins().jump(exit_bb, &[]);
                }
            } else {
                builder.ins().jump(exit_bb, &[]);
            }

            builder.switch_to_block(exit_bb);
            builder.seal_block(exit_bb);
            false
        }
        Stmt::Shared { name, value } => {
            let Expr::StructInit {
                name: s_name,
                fields,
            } = value
            else {
                panic!("`shared {}` requires a struct initializer", name);
            };
            let def = env
                .structs
                .get(s_name)
                .unwrap_or_else(|| panic!("Unknown struct: {}", s_name))
                .clone();
            let byte_size = struct_byte_size(&def);
            let tmp_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                byte_size,
                3,
            ));
            for (f_name, f_expr) in fields {
                let val = compile_expr(f_expr, builder, vars, env);
                let off = field_offset(&def, f_name);
                let f_ty = field_type(&def, f_name);
                let base = builder.ins().stack_addr(env.int_ty, tmp_slot, 0);
                store_struct_field(builder, val, base, off, f_ty);
            }
            let src_addr = builder.ins().stack_addr(env.int_ty, tmp_slot, 0);
            let size_val = builder.ins().iconst(env.int_ty, byte_size as i64);
            let call = builder
                .ins()
                .call(env.shared_new_ref, &[src_addr, size_val]);
            let box_ptr = builder.inst_results(call)[0];
            let box_var = builder.declare_var(env.int_ty);
            builder.def_var(box_var, box_ptr);
            vars.insert(
                name.clone(),
                VarInfo::SharedStruct {
                    box_var,
                    struct_name: s_name.clone(),
                },
            );
            false
        }
        Stmt::Lock { var, body } => {
            let Some(VarInfo::SharedStruct {
                box_var,
                struct_name,
            }) = vars.get(var).cloned()
            else {
                panic!("`lock` can only be used on a `shared` variable ('{}')", var);
            };
            let box_ptr = builder.use_var(box_var);
            let c_lock = builder.ins().call(env.shared_lock_ref, &[box_ptr]);
            let data_ptr = builder.inst_results(c_lock)[0];
            let ptr_var = builder.declare_var(env.int_ty);
            builder.def_var(ptr_var, data_ptr);

            let saved_info = vars.insert(
                var.clone(),
                VarInfo::StructPtr {
                    ptr_var,
                    struct_name,
                    is_mut: true,
                },
            );
            env.active_locks.push(box_ptr);

            let mut body_ret = false;
            for s in body {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    body_ret = true;
                    break;
                }
            }

            env.active_locks.pop();
            if let Some(prev) = saved_info {
                vars.insert(var.clone(), prev);
            }
            if !body_ret {
                builder.ins().call(env.shared_unlock_ref, &[box_ptr]);
            }
            false
        }
        Stmt::Concurrent(body) => {
            let prev_group = env.active_group;
            let c_begin = builder.ins().call(env.group_begin_ref, &[]);
            let group_ptr = builder.inst_results(c_begin)[0];
            env.active_group = Some(group_ptr);

            for s in body {
                compile_stmt(s, builder, vars, env, owned_strings, owned_vecs);
            }

            builder.ins().call(env.group_wait_ref, &[group_ptr]);
            env.active_group = prev_group;
            false
        }
        Stmt::Spawn { name, args } => {
            let group_ptr = env
                .active_group
                .expect("`spawn` must be inside a `concurrent { ... }` block!");
            let (func_ref, param_meta) = env
                .funcs
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("Unknown function in `spawn`: {}", name));
            assert_eq!(
                args.len(),
                param_meta.len(),
                "Wrong argument count for `spawn {}`",
                name
            );

            for (ty, is_mut) in &param_meta {
                if *is_mut && env.structs.contains_key(ty) {
                    panic!(
                        "Data-race guard: `spawn {}` cannot take unprotected `mut {}` (use `Channel` or `Atomic`)",
                        name, ty
                    );
                }
            }

            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (args.len().max(1) * 8) as u32,
                3,
            ));
            for (i, arg) in args.iter().enumerate() {
                let v = compile_expr(arg, builder, vars, env);
                builder
                    .ins()
                    .stack_store(env.int_ty, v, slot, (i * 8) as i32);
            }

            let fn_addr = builder.ins().func_addr(env.int_ty, func_ref);
            let args_ptr = builder.ins().stack_addr(env.int_ty, slot, 0);
            let count_val = builder.ins().iconst(env.int_ty, args.len() as i64);
            builder.ins().call(
                env.group_spawn_ref,
                &[group_ptr, fn_addr, args_ptr, count_val],
            );
            false
        }
        Stmt::ForEach {
            var,
            is_mut,
            is_parallel,
            collection,
            body,
        } => {
            if *is_parallel {
                let key = format!("{}:{}:{}", env.current_fn, collection, var);
                let (helper_name, captures) = env
                    .par_helpers
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| panic!("Missing parallel helper for {}", key));
                let (helper_ref, _) = env
                    .funcs
                    .get(&helper_name)
                    .cloned()
                    .unwrap_or_else(|| panic!("Missing helper func {}", helper_name));

                let coll_info = vars
                    .get(collection)
                    .cloned()
                    .unwrap_or_else(|| panic!("Unknown list '{}' in `parallel for`", collection));
                let (vec_ptr, is_svec) = match &coll_info {
                    VarInfo::StructVec {
                        var: v_var,
                        is_mut: coll_mut,
                        ..
                    } => {
                        if *is_mut {
                            assert!(
                                *coll_mut,
                                "Error: list '{}' must be `mut` for `parallel for mut {}`!",
                                collection, var
                            );
                        }
                        (builder.use_var(*v_var), true)
                    }
                    VarInfo::Vec { var: v_var, .. } => (builder.use_var(*v_var), false),
                    _ => panic!("'{}' is not an iterable Vec list!", collection),
                };

                let cap_slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    (captures.len().max(1) * 8) as u32,
                    3,
                ));
                for (i, cap_name) in captures.iter().enumerate() {
                    let v = compile_expr(&Expr::Var(cap_name.clone()), builder, vars, env);
                    let v_i64 = if builder.func.dfg.value_type(v) == types::F64 {
                        builder.ins().bitcast(env.int_ty, MemFlagsData::new(), v)
                    } else {
                        v
                    };
                    builder
                        .ins()
                        .stack_store(env.int_ty, v_i64, cap_slot, (i * 8) as i32);
                }

                let fn_addr = builder.ins().func_addr(env.int_ty, helper_ref);
                let caps_ptr = builder.ins().stack_addr(env.int_ty, cap_slot, 0);
                let cap_count = builder.ins().iconst(env.int_ty, captures.len() as i64);
                let runner = if is_svec {
                    env.par_for_svec_ref
                } else {
                    env.par_for_vec_ref
                };
                builder
                    .ins()
                    .call(runner, &[vec_ptr, fn_addr, caps_ptr, cap_count]);
                return false;
            }
            let coll_info = vars
                .get(collection)
                .cloned()
                .unwrap_or_else(|| panic!("Unknown list '{}' in `for`", collection));

            let (vec_ptr, len_val, is_struct_elem) = match &coll_info {
                VarInfo::StructVec {
                    var: v_var,
                    struct_name,
                    is_mut: coll_mut,
                } => {
                    if *is_mut {
                        assert!(
                            *coll_mut,
                            "Error: list '{}' must be `mut` for `for mut {}`!",
                            collection, var
                        );
                    }
                    let vp = builder.use_var(*v_var);
                    let c = builder.ins().call(env.svec_len_ref, &[vp]);
                    (vp, builder.inst_results(c)[0], Some(struct_name.clone()))
                }
                VarInfo::Vec { var: v_var, .. } => {
                    let vp = builder.use_var(*v_var);
                    let c = builder.ins().call(env.vec_len_ref, &[vp]);
                    (vp, builder.inst_results(c)[0], None)
                }
                _ => panic!("'{}' is not an iterable Vec list!", collection),
            };

            let idx_var = builder.declare_var(env.int_ty);
            let zero = builder.ins().iconst(env.int_ty, 0);
            builder.def_var(idx_var, zero);

            let elem_var = builder.declare_var(env.int_ty);
            if let Some(s_name) = &is_struct_elem {
                vars.insert(
                    var.clone(),
                    VarInfo::StructPtr {
                        ptr_var: elem_var,
                        struct_name: s_name.clone(),
                        is_mut: *is_mut,
                    },
                );
            } else {
                vars.insert(
                    var.clone(),
                    VarInfo::Int {
                        var: elem_var,
                        is_mut: *is_mut,
                    },
                );
            }

            let header_bb = builder.create_block();
            let body_bb = builder.create_block();
            let exit_bb = builder.create_block();

            builder.ins().jump(header_bb, &[]);
            builder.switch_to_block(header_bb);
            emit_sandbox_gas_guard(builder, exit_bb, env);

            let curr_idx = builder.use_var(idx_var);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, curr_idx, len_val);
            builder.ins().brif(cond, body_bb, &[], exit_bb, &[]);

            builder.switch_to_block(body_bb);
            builder.seal_block(body_bb);

            let elem_val = if is_struct_elem.is_some() {
                let c = builder
                    .ins()
                    .call(env.svec_get_ptr_ref, &[vec_ptr, curr_idx]);
                builder.inst_results(c)[0]
            } else {
                let c = builder.ins().call(env.vec_get_ref, &[vec_ptr, curr_idx]);
                builder.inst_results(c)[0]
            };
            builder.def_var(elem_var, elem_val);

            let mut body_ret = false;
            for s in body {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    body_ret = true;
                    break;
                }
            }
            if !body_ret {
                let cur = builder.use_var(idx_var);
                let next_idx = builder.ins().iadd_imm_s(cur, 1);
                builder.def_var(idx_var, next_idx);
                builder.ins().jump(header_bb, &[]);
            }
            builder.seal_block(header_bb);

            builder.switch_to_block(exit_bb);
            builder.seal_block(exit_bb);
            false
        }
        Stmt::ForRange {
            var,
            start,
            end,
            body,
        } => {
            let start_val = compile_expr(start, builder, vars, env);
            let end_val = compile_expr(end, builder, vars, env);

            let loop_var = builder.declare_var(env.int_ty);
            builder.def_var(loop_var, start_val);
            vars.insert(
                var.clone(),
                VarInfo::Int {
                    var: loop_var,
                    is_mut: false,
                },
            );

            let header_bb = builder.create_block();
            let body_bb = builder.create_block();
            let step_bb = builder.create_block();
            let exit_bb = builder.create_block();

            builder.ins().jump(header_bb, &[]);
            builder.switch_to_block(header_bb);
            emit_sandbox_gas_guard(builder, exit_bb, env);

            let curr = builder.use_var(loop_var);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, curr, end_val);
            builder.ins().brif(cond, body_bb, &[], exit_bb, &[]);

            builder.switch_to_block(body_bb);
            builder.seal_block(body_bb);
            env.loop_targets.push((step_bb, exit_bb));
            let mut body_ret = false;
            for s in body {
                if compile_stmt(s, builder, vars, env, owned_strings, owned_vecs) {
                    body_ret = true;
                    break;
                }
            }
            env.loop_targets.pop();
            if !body_ret {
                builder.ins().jump(step_bb, &[]);
            }
            builder.switch_to_block(step_bb);
            builder.seal_block(step_bb);
            let cur_v = builder.use_var(loop_var);
            let next_val = builder.ins().iadd_imm_s(cur_v, 1);
            builder.def_var(loop_var, next_val);
            builder.ins().jump(header_bb, &[]);
            builder.seal_block(header_bb);

            builder.switch_to_block(exit_bb);
            builder.seal_block(exit_bb);
            false
        }
        Stmt::Return(expr) => {
            let val = if let Some((ret_ptr_var, rs_name)) = env.ret_struct_ptr.clone() {
                let ret_ptr = builder.use_var(ret_ptr_var);
                let def = env.structs.get(&rs_name).unwrap().clone();
                match expr {
                    Expr::StructInit { fields, .. } => {
                        if def.sync_mask != 0 {
                            let zero = builder.ins().iconst(env.int_ty, 0);
                            builder.ins().store(MemFlagsData::new(), zero, ret_ptr, 0);
                        }
                        for (f_name, f_expr) in fields {
                            let v = compile_expr(f_expr, builder, vars, env);
                            let off = field_offset(&def, f_name);
                            let f_ty = field_type(&def, f_name);
                            store_struct_field(builder, v, ret_ptr, off, f_ty);
                        }
                    }
                    Expr::Var(vn) => match vars.get(vn) {
                        Some(VarInfo::Struct { slot, .. }) => {
                            let src = builder.ins().stack_addr(env.int_ty, *slot, 0);
                            for (f_name, f_ty) in &def.fields {
                                let off = field_offset(&def, f_name);
                                let v = load_struct_field(builder, src, off, f_ty, env.int_ty);
                                store_struct_field(builder, v, ret_ptr, off, f_ty);
                            }
                        }
                        Some(VarInfo::StructPtr { ptr_var, .. }) => {
                            let src = builder.use_var(*ptr_var);
                            for (f_name, f_ty) in &def.fields {
                                let off = field_offset(&def, f_name);
                                let v = load_struct_field(builder, src, off, f_ty, env.int_ty);
                                store_struct_field(builder, v, ret_ptr, off, f_ty);
                            }
                        }
                        _ => panic!("Expected struct '{}' in return", rs_name),
                    },
                    other => {
                        let src = compile_expr(other, builder, vars, env);
                        for (f_name, f_ty) in &def.fields {
                            let off = field_offset(&def, f_name);
                            let v = load_struct_field(builder, src, off, f_ty, env.int_ty);
                            store_struct_field(builder, v, ret_ptr, off, f_ty);
                        }
                    }
                }
                ret_ptr
            } else {
                compile_expr(expr, builder, vars, env)
            };
            let to_run: Vec<Stmt> = env.deferred.drain(..).rev().collect();
            for d_stmt in &to_run {
                compile_stmt(d_stmt, builder, vars, env, owned_strings, owned_vecs);
            }
            for lp in env.active_locks.iter().rev() {
                builder.ins().call(env.shared_unlock_ref, &[*lp]);
            }
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
        Stmt::Expr(expr) => {
            compile_expr(expr, builder, vars, env);
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
        Expr::Int(v) | Expr::Hex(v) => builder.ins().iconst(env.int_ty, *v),
        Expr::Ok(inner) | Expr::Some(inner) => compile_expr(inner, builder, vars, env),
        Expr::None => {
            let one = builder.ins().iconst(env.int_ty, 1);
            builder.ins().call(env.set_err_ref, &[one]);
            builder.ins().iconst(env.int_ty, 0)
        }
        Expr::Err(inner) => {
            let code = compile_expr(inner, builder, vars, env);
            builder.ins().call(env.set_err_ref, &[code]);
            builder.ins().iconst(env.int_ty, 0)
        }
        Expr::Try(inner) => {
            let val = compile_expr(inner, builder, vars, env);
            let c_err = builder.ins().call(env.take_err_ref, &[]);
            let err_tag = builder.inst_results(c_err)[0];

            let ok_bb = builder.create_block();
            let err_bb = builder.create_block();
            builder.ins().brif(err_tag, err_bb, &[], ok_bb, &[]);

            builder.switch_to_block(err_bb);
            builder.seal_block(err_bb);
            builder.ins().call(env.set_err_ref, &[err_tag]);
            let zero = builder.ins().iconst(env.int_ty, 0);
            builder.ins().return_(&[zero]);

            builder.switch_to_block(ok_bb);
            builder.seal_block(ok_bb);
            val
        }
        Expr::Float(v) | Expr::UnitFloat(v, _) => builder.ins().f64const(*v),
        Expr::PathConst { ty, member } => {
            let key = format!("{}__{}", ty, member);
            let val = env
                .user_enums
                .get(&key)
                .copied()
                .or_else(|| {
                    lang_runtime::generated_bridge::exported_constants()
                        .iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| *v)
                })
                .unwrap_or_else(|| panic!("Unknown constant `{}::{}`", ty, member));
            builder.ins().iconst(env.int_ty, val)
        }
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
                        let c = if builder.func.dfg.value_type(num) == types::F64 {
                            builder.ins().call(env.str_push_float_ref, &[s_ptr, num])
                        } else {
                            builder.ins().call(env.str_push_int_ref, &[s_ptr, num])
                        };
                        s_ptr = builder.inst_results(c)[0];
                    }
                }
            }
            s_ptr
        }
        Expr::Var(name) => match vars.get(name) {
            Some(VarInfo::Int { var, .. })
            | Some(VarInfo::Float { var, .. })
            | Some(VarInfo::Str { var, .. })
            | Some(VarInfo::Vec { var, .. })
            | Some(VarInfo::Handle { var, .. }) => builder.use_var(*var),
            Some(VarInfo::SharedStruct { box_var, .. }) => builder.use_var(*box_var),
            Some(VarInfo::Tweak { addr }) => {
                let ptr_val = builder.ins().iconst(env.int_ty, *addr);
                builder
                    .ins()
                    .load(env.int_ty, MemFlagsData::new(), ptr_val, 0)
            }
            Some(VarInfo::Bytes { var, .. }) => builder.use_var(*var),
            _ => {
                if let Some(&c_val) = env.user_enums.get(name) {
                    builder.ins().iconst(env.int_ty, c_val)
                } else {
                    panic!("Unknown integer variable: {}", name)
                }
            }
        },
        Expr::FieldAccess { var, field } => match vars.get(var) {
            Some(VarInfo::Struct {
                slot, struct_name, ..
            }) => {
                let def = env.structs.get(struct_name).unwrap();
                let ty = if field_is_float(def, field) {
                    types::F64
                } else {
                    env.int_ty
                };
                let offset = field_offset(def, field);
                let f_ty = field_type(def, field);
                let base = builder.ins().stack_addr(env.int_ty, *slot, 0);
                load_struct_field(builder, base, offset, f_ty, env.int_ty)
            }
            Some(VarInfo::StructPtr {
                ptr_var,
                struct_name,
                ..
            }) => {
                let def = env.structs.get(struct_name).unwrap();
                let offset = field_offset(def, field);
                let f_ty = field_type(def, field);
                let addr = builder.use_var(*ptr_var);
                load_struct_field(builder, addr, offset, f_ty, env.int_ty)
            }
            Some(VarInfo::Tuple {
                slot,
                elem_is_float,
                ..
            }) => {
                let idx = tuple_field_index(field).unwrap_or_else(|| {
                    panic!(
                        "Unknown tuple field '.{}' (use .x/.y/.z/.w or .0/.1)",
                        field
                    )
                });
                let ty = if *elem_is_float.get(idx).unwrap_or(&false) {
                    types::F64
                } else {
                    env.int_ty
                };
                builder
                    .ins()
                    .stack_load(env.int_ty, ty, *slot, (idx * 8) as i32)
            }
            _ => panic!("'{}' is not a struct!", var),
        },
        Expr::StructInit { .. } => {
            panic!("StructInit must currently be assigned to a `let` variable")
        }
        Expr::Tuple(_) => {
            panic!("Tuple literal must currently be assigned to a `let` variable")
        }
        Expr::TupleIndex { tuple, idx } => {
            if let Expr::Var(v_name) = &**tuple {
                if let Some(VarInfo::Tuple {
                    slot,
                    elem_is_float,
                    ..
                }) = vars.get(v_name)
                {
                    let ty = if *elem_is_float.get(*idx).unwrap_or(&false) {
                        types::F64
                    } else {
                        env.int_ty
                    };
                    return builder
                        .ins()
                        .stack_load(env.int_ty, ty, *slot, (*idx * 8) as i32);
                }
            }
            panic!(
                "Tuple access `.{}` is only supported on tuple variables",
                idx
            );
        }
        Expr::BinOp { left, op, right } => {
            if let Some(s_name) = expr_struct_type(left, vars, env) {
                let op_method = match op {
                    Op::Add => "op_add",
                    Op::Sub => "op_sub",
                    Op::Mul => "op_mul",
                    Op::Div => "op_div",
                    Op::Eq => "op_eq",
                    _ => panic!("Unsupported operator on struct '{}'", s_name),
                };
                let full_name = format!("{}__{}", s_name, op_method);
                return compile_call(&full_name, &[(*left.clone()), (*right.clone())], builder, vars, env);
            }
            let l = compile_expr(left, builder, vars, env);
            let r = compile_expr(right, builder, vars, env);
            let is_float = builder.func.dfg.value_type(l) == types::F64
                || builder.func.dfg.value_type(r) == types::F64;
            if is_float {
                let lf = to_f64(l, builder);
                let rf = to_f64(r, builder);
                match op {
                    Op::Add => builder.ins().fadd(lf, rf),
                    Op::Sub => builder.ins().fsub(lf, rf),
                    Op::Mul => builder.ins().fmul(lf, rf),
                    Op::Div => builder.ins().fdiv(lf, rf),
                    Op::Lt => builder.ins().fcmp(FloatCC::LessThan, lf, rf),
                    Op::Gt => builder.ins().fcmp(FloatCC::GreaterThan, lf, rf),
                    Op::LtEq => builder.ins().fcmp(FloatCC::LessThanOrEqual, lf, rf),
                    Op::GtEq => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lf, rf),
                    Op::Eq => builder.ins().fcmp(FloatCC::Equal, lf, rf),
                    Op::NotEq => builder.ins().fcmp(FloatCC::NotEqual, lf, rf),
                    Op::And | Op::Or => panic!("Cannot use && or || on floats"),
                    Op::BitOr | Op::BitAnd | Op::BitXor | Op::Shl | Op::Shr => {
                        panic!("Cannot use bitwise operators on floats")
                    }
                }
            } else {
                match op {
                    Op::Add => builder.ins().iadd(l, r),
                    Op::Sub => builder.ins().isub(l, r),
                    Op::Mul => builder.ins().imul(l, r),
                    Op::Div => builder.ins().sdiv(l, r),
                    Op::Lt => builder.ins().icmp(IntCC::SignedLessThan, l, r),
                    Op::Gt => builder.ins().icmp(IntCC::SignedGreaterThan, l, r),
                    Op::LtEq => builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r),
                    Op::GtEq => builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r),
                    Op::Eq => builder.ins().icmp(IntCC::Equal, l, r),
                    Op::NotEq => builder.ins().icmp(IntCC::NotEqual, l, r),
                    Op::And => {
                        let lb = if builder.func.dfg.value_type(l) == types::I8 {
                            l
                        } else {
                            builder.ins().icmp_imm_s(IntCC::NotEqual, l, 0)
                        };
                        let rb = if builder.func.dfg.value_type(r) == types::I8 {
                            r
                        } else {
                            builder.ins().icmp_imm_s(IntCC::NotEqual, r, 0)
                        };
                        builder.ins().band(lb, rb)
                    }
                    Op::Or => {
                        let lb = if builder.func.dfg.value_type(l) == types::I8 {
                            l
                        } else {
                            builder.ins().icmp_imm_s(IntCC::NotEqual, l, 0)
                        };
                        let rb = if builder.func.dfg.value_type(r) == types::I8 {
                            r
                        } else {
                            builder.ins().icmp_imm_s(IntCC::NotEqual, r, 0)
                        };
                        builder.ins().bor(lb, rb)
                    }
                    Op::BitOr => builder.ins().bor(l, r),
                    Op::BitAnd => builder.ins().band(l, r),
                    Op::BitXor => builder.ins().bxor(l, r),
                    Op::Shl => builder.ins().ishl(l, r),
                    Op::Shr => builder.ins().sshr(l, r),
                }
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

fn expand_trailing_blocks(funcs: &[striv_syntax::Function]) -> Vec<striv_syntax::Function> {
    let mut templates: HashMap<String, striv_syntax::Function> = HashMap::new();
    let mut normal = Vec::new();
    for f in funcs {
        if f.params
            .last()
            .map(|(_, ty, _)| ty == "block")
            .unwrap_or(false)
        {
            templates.insert(f.name.clone(), f.clone());
        } else {
            normal.push(f.clone());
        }
    }
    for f in &mut normal {
        f.body = expand_stmts(&f.body, &templates);
    }
    normal
}

fn expand_stmts(stmts: &[Stmt], templates: &HashMap<String, striv_syntax::Function>) -> Vec<Stmt> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::BlockCall { name, args, body } => {
                let tpl = templates
                    .get(name)
                    .unwrap_or_else(|| panic!("Unknown block function '{}'", name));
                let expanded_body = expand_stmts(body, templates);
                let mut inlined = Vec::new();
                for ((p_name, p_ty, _), arg_expr) in tpl.params.iter().zip(args.iter()) {
                    if p_ty != "block" {
                        inlined.push(Stmt::Let {
                            name: p_name.clone(),
                            ty_hint: Some(p_ty.clone()),
                            is_mut: false,
                            value: arg_expr.clone(),
                        });
                    }
                }
                let tpl_expanded = expand_stmts(&tpl.body, templates);
                for s in replace_yield(&tpl_expanded, &expanded_body) {
                    inlined.push(s);
                }
                out.push(Stmt::Block(inlined));
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => out.push(Stmt::If {
                cond: cond.clone(),
                then_block: expand_stmts(then_block, templates),
                else_block: expand_stmts(else_block, templates),
            }),
            Stmt::While { cond, body } => out.push(Stmt::While {
                cond: cond.clone(),
                body: expand_stmts(body, templates),
            }),
            Stmt::ForRange {
                var,
                start,
                end,
                body,
            } => out.push(Stmt::ForRange {
                var: var.clone(),
                start: start.clone(),
                end: end.clone(),
                body: expand_stmts(body, templates),
            }),
            Stmt::ForEach {
                var,
                is_mut,
                is_parallel,
                collection,
                body,
            } => out.push(Stmt::ForEach {
                var: var.clone(),
                is_mut: *is_mut,
                is_parallel: *is_parallel,
                collection: collection.clone(),
                body: expand_stmts(body, templates),
            }),
            Stmt::Block(inner) => out.push(Stmt::Block(expand_stmts(inner, templates))),
            other => out.push(other.clone()),
        }
    }
    out
}

fn replace_yield(stmts: &[Stmt], replacement: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Yield => out.push(Stmt::Block(replacement.to_vec())),
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => out.push(Stmt::If {
                cond: cond.clone(),
                then_block: replace_yield(then_block, replacement),
                else_block: replace_yield(else_block, replacement),
            }),
            Stmt::While { cond, body } => out.push(Stmt::While {
                cond: cond.clone(),
                body: replace_yield(body, replacement),
            }),
            Stmt::ForRange {
                var,
                start,
                end,
                body,
            } => out.push(Stmt::ForRange {
                var: var.clone(),
                start: start.clone(),
                end: end.clone(),
                body: replace_yield(body, replacement),
            }),
            Stmt::Block(inner) => out.push(Stmt::Block(replace_yield(inner, replacement))),
            other => out.push(other.clone()),
        }
    }
    out
}

fn extract_parallel_bodies(
    prog_funcs: &[striv_syntax::Function],
) -> (
    Vec<striv_syntax::Function>,
    HashMap<String, (String, Vec<String>)>,
) {
    let mut out_funcs = prog_funcs.to_vec();
    let mut helpers = HashMap::new();
    let mut next_id = 0usize;

    for func in prog_funcs {
        let mut local_types: HashMap<String, (String, bool)> = HashMap::new();
        for (p_name, p_ty, p_mut) in &func.params {
            local_types.insert(p_name.clone(), (p_ty.clone(), *p_mut));
        }
        collect_parallel_in_stmts(
            &func.name,
            &func.body,
            &mut local_types,
            &mut out_funcs,
            &mut helpers,
            &mut next_id,
        );
    }

    (out_funcs, helpers)
}

fn collect_parallel_in_stmts(
    fn_name: &str,
    stmts: &[Stmt],
    local_types: &mut HashMap<String, (String, bool)>,
    out_funcs: &mut Vec<striv_syntax::Function>,
    helpers: &mut HashMap<String, (String, Vec<String>)>,
    next_id: &mut usize,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                ty_hint,
                is_mut,
                value,
            } => {
                let ty = if let Some(th) = ty_hint {
                    th.clone()
                } else {
                    match value {
                        Expr::Float(_) | Expr::UnitFloat(_, _) => "float".to_string(),
                        Expr::StrInterp(_) => "String".to_string(),
                        Expr::VecLit(_) => "Vec".to_string(),
                        Expr::StructInit { name, .. } => name.clone(),
                        Expr::Call { name, .. } if name.contains("__") => {
                            name.split("__").next().unwrap_or("int").to_string()
                        }
                        _ => "int".to_string(),
                    }
                };
                local_types.insert(name.clone(), (ty, *is_mut));
            }
            Stmt::Shared { name, value } => {
                if let Expr::StructInit { name: s_name, .. } = value {
                    local_types.insert(name.clone(), (format!("shared:{}", s_name), false));
                }
            }
            Stmt::ForEach {
                var,
                is_mut,
                is_parallel,
                collection,
                body,
            } if *is_parallel => {
                let elem_ty = local_types
                    .get(collection)
                    .and_then(|(t, _)| t.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')))
                    .unwrap_or("int")
                    .to_string();

                let mut free_vars = Vec::new();
                let mut inner_locals = std::collections::HashSet::new();
                inner_locals.insert(var.clone());
                collect_free_vars_in_stmts(body, &inner_locals, &mut free_vars);

                let mut params = vec![(var.clone(), elem_ty, *is_mut)];
                let mut cap_names = Vec::new();
                for cap in free_vars {
                    if let Some((cap_ty, cap_mut)) = local_types.get(&cap) {
                        if *cap_mut && !cap_ty.starts_with("shared:") {
                            panic!(
                                "Data-race guard: outer `mut` variable '{}' cannot be captured in `parallel for` (use `Atomic` or `shared`)",
                                cap
                            );
                        }
                        let param_ty = if cap_ty == "float" {
                            "__par_f64".to_string()
                        } else {
                            cap_ty.clone()
                        };
                        params.push((cap.clone(), param_ty, false));
                        cap_names.push(cap);
                    }
                }

                let helper_name = format!("__par_body_{}", *next_id);
                *next_id += 1;
                out_funcs.push(striv_syntax::Function {
                    is_rpc: false,
            rust_body: None,
                    name: helper_name.clone(),
                    type_params: Vec::new(),
                    params,
                    ret_type: None,
                    body: body.clone(),
                });
                helpers.insert(
                    format!("{}:{}:{}", fn_name, collection, var),
                    (helper_name, cap_names),
                );
            }
            Stmt::Query {
                terms,
                ecs_var,
                body,
                ..
            } => {
                let mut bound = std::collections::HashSet::new();
                for (v, _, _) in terms {
                    bound.insert(v.clone());
                }
                let mut free = Vec::new();
                collect_free_vars_in_stmts(body, &bound, &mut free);
                let mut params: Vec<(String, String, bool)> = terms.clone();
                let mut cap_names = Vec::new();
                for cap in free {
                    let (cap_ty, _) = local_types
                        .get(&cap)
                        .cloned()
                        .unwrap_or_else(|| ("int".to_string(), false));
                    let param_ty = if cap_ty == "float" {
                        "__par_f64".to_string()
                    } else {
                        cap_ty
                    };
                    params.push((cap.clone(), param_ty, false));
                    cap_names.push(cap);
                }
                let helper_name = format!("__ecs_query_{}", *next_id);
                *next_id += 1;
                out_funcs.push(striv_syntax::Function {
                    is_rpc: false,
                    rust_body: None,
                    name: helper_name.clone(),
                    type_params: Vec::new(),
                    params,
                    ret_type: None,
                    body: body.clone(),
                });
                let key = format!(
                    "query:{}:{}:{}",
                    fn_name,
                    ecs_var,
                    terms
                        .iter()
                        .map(|(v, t, _)| format!("{}:{}", v, t))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                helpers.insert(key, (helper_name, cap_names));
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_parallel_in_stmts(
                    fn_name,
                    then_block,
                    local_types,
                    out_funcs,
                    helpers,
                    next_id,
                );
                collect_parallel_in_stmts(
                    fn_name,
                    else_block,
                    local_types,
                    out_funcs,
                    helpers,
                    next_id,
                );
            }
            Stmt::While { body, .. }
            | Stmt::ForRange { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Concurrent(body)
            | Stmt::Lock { body, .. } => {
                collect_parallel_in_stmts(fn_name, body, local_types, out_funcs, helpers, next_id);
            }
            _ => {}
        }
    }
}

fn collect_free_vars_in_stmts(
    stmts: &[Stmt],
    bound: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    let mut local_bound = bound.clone();
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, value, .. } => {
                collect_free_vars_in_expr(value, &local_bound, out);
                local_bound.insert(name.clone());
            }
            Stmt::Assign { name, value }
            | Stmt::FieldAssign {
                var: name, value, ..
            } => {
                if !local_bound.contains(name) && !out.contains(name) {
                    out.push(name.clone());
                }
                collect_free_vars_in_expr(value, &local_bound, out);
            }
            Stmt::CompoundAssign { value, .. } => {
                collect_free_vars_in_expr(value, &local_bound, out);
            }
            Stmt::Lock { var, body } => {
                if !local_bound.contains(var) && !out.contains(var) {
                    out.push(var.clone());
                }
                collect_free_vars_in_stmts(body, &local_bound, out);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                collect_free_vars_in_expr(cond, &local_bound, out);
                collect_free_vars_in_stmts(then_block, &local_bound, out);
                collect_free_vars_in_stmts(else_block, &local_bound, out);
            }
            Stmt::Print(e) | Stmt::Return(e) | Stmt::Expr(e) => {
                collect_free_vars_in_expr(e, &local_bound, out);
            }
            Stmt::While { cond, body } => {
                collect_free_vars_in_expr(cond, &local_bound, out);
                collect_free_vars_in_stmts(body, &local_bound, out);
            }
            _ => {}
        }
    }
}

fn collect_free_vars_in_expr(
    expr: &Expr,
    bound: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    match expr {
        Expr::Var(n) => {
            if !bound.contains(n) && !out.contains(n) {
                out.push(n.clone());
            }
        }
        Expr::FieldAccess { var, .. } => {
            if !bound.contains(var) && !out.contains(var) {
                out.push(var.clone());
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_free_vars_in_expr(left, bound, out);
            collect_free_vars_in_expr(right, bound, out);
        }
        Expr::MethodCall { receiver, args, .. } => {
            if !bound.contains(receiver) && !out.contains(receiver) {
                out.push(receiver.clone());
            }
            for a in args {
                collect_free_vars_in_expr(a, bound, out);
            }
        }
        Expr::Call { args, .. } => {
            for a in args {
                collect_free_vars_in_expr(a, bound, out);
            }
        }
        _ => {}
    }
}

fn expr_struct_type(
    expr: &Expr,
    vars: &HashMap<String, VarInfo>,
    env: &CompileEnv,
) -> Option<String> {
    match expr {
        Expr::Var(vn) => match vars.get(vn) {
            Some(VarInfo::Struct { struct_name, .. })
            | Some(VarInfo::StructPtr { struct_name, .. }) => Some(struct_name.clone()),
            _ => None,
        },
        Expr::Call { name, .. } => env.fn_ret_structs.get(name).cloned().flatten(),
        Expr::BinOp { left, .. } => expr_struct_type(left, vars, env),
        _ => None,
    }
}

fn tuple_field_index(field: &str) -> Option<usize> {
    match field {
        "x" | "r" | "width" | "min" => Some(0),
        "y" | "g" | "height" | "max" => Some(1),
        "z" | "b" | "depth" => Some(2),
        "w" | "a" => Some(3),
        _ => None,
    }
}

pub fn eval_comptime_consts(prog: &Program) -> HashMap<String, i64> {
    let mut const_vals = HashMap::new();
    for (c_name, c_expr) in &prog.consts {
        let mut funcs: Vec<striv_syntax::Function> = prog
            .functions
            .iter()
            .filter(|f| f.name != "main")
            .cloned()
            .collect();
        funcs.push(striv_syntax::Function {
            is_rpc: false,
            rust_body: None,
            name: format!("__comptime_{}", c_name),
            type_params: Vec::new(),
            params: Vec::new(),
            ret_type: Some("int".to_string()),
            body: vec![Stmt::Return(c_expr.clone())],
        });
        let mini_prog = Program {
            imports: Vec::new(),
            tweaks: prog.tweaks.clone(),
            consts: Vec::new(),
            units: Vec::new(),
            rust_blocks: Vec::new(),
            enums: prog.enums.clone(),
            structs: prog.structs.clone(),
            functions: funcs,
            tests: Vec::new(),
        };
        let mut jit = JitCompiler::new();
        let ptr = jit.compile_program(&mini_prog);
        let eval_fn: fn() -> i64 = unsafe { std::mem::transmute(ptr) };
        const_vals.insert(c_name.clone(), eval_fn());
    }
    const_vals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_stops_infinite_loop() {
        let src = r#"
fn main() -> int {
    let mut x = 0
    while true {
        x += 1
    }
    return x
}
"#;
        let prog = striv_syntax::Parser::new(src).parse_program();
        let mut jit = JitCompiler::new_sandboxed();
        let ptr = jit.compile_program(&prog);
        let f: fn() -> i64 = unsafe { std::mem::transmute(ptr) };
        lang_runtime::set_sandbox_gas(5000);
        let _ = f();
        assert!(lang_runtime::take_sandbox_aborted());
    }

    #[test]
    #[should_panic(expected = "Sandbox")]
    fn test_sandbox_blocks_file_io() {
        let src = r#"
fn main() -> int {
    let b = Bytes::read_file("secret.txt")
    return 0
}
"#;
        let prog = striv_syntax::Parser::new(src).parse_program();
        let mut jit = JitCompiler::new_sandboxed();
        let _ = jit.compile_program(&prog);
    }
}
