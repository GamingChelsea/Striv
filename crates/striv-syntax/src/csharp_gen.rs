use crate::{fnv1a_32, fnv1a_64, Program};

pub fn generate_csharp_bindings(prog: &Program, dll_name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generierte Striv C# Modding-API\n");
    out.push_str("using System;\nusing System.Runtime.InteropServices;\n\n");
    out.push_str("namespace StrivModApi;\n\n");

    // 1. Enums & Open Enums (mit identischen 64-Bit FNV-1a Hashes!)
    for e in &prog.enums {
        out.push_str(&format!("public static class {} {{\n", e.name));
        for (idx, v) in e.variants.iter().enumerate() {
            let val = if e.is_open {
                fnv1a_64(&format!("{}::{}", e.name, v.name))
            } else {
                idx as i64
            };
            out.push_str(&format!("    public const long {} = {}L;\n", v.name, val));
        }
        out.push_str("}\n\n");
    }

    // 2. C-kompatible Structs (inkl. __dirty_mask an Offset 0 bei `sync`-Structs!)
    for s in &prog.structs {
        if !s.type_params.is_empty() {
            continue;
        }
        out.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        out.push_str(&format!("public struct {} {{\n", s.name));
        if s.sync_mask != 0 {
            out.push_str("    public ulong __dirty_mask;\n");
        }
        for (f_name, f_ty) in &s.fields {
            out.push_str(&format!("    public {} {};\n", map_cs_type(f_ty), f_name));
        }
        out.push_str("}\n\n");
    }

    // 3. RPC-IDs & Native Host-Imports
    out.push_str("public static partial class StrivHost {\n");
    out.push_str(&format!("    private const string DllName = \"{}\";\n\n", dll_name));

    for f in &prog.functions {
        if f.is_rpc {
            let hash = fnv1a_32(&f.name);
            out.push_str(&format!("    public const uint RpcId_{} = {}u;\n", f.name, hash));
        }
    }

    for f in &prog.functions {
        if f.name.contains("__") || !f.type_params.is_empty() || f.name == "main" {
            continue;
        }
        let params: Vec<String> = f
            .params
            .iter()
            .map(|(n, ty, is_mut)| {
                let cs_ty = map_cs_param_type(ty, *is_mut, prog);
                format!("{} {}", cs_ty, n)
            })
            .collect();
        let ret = f.ret_type.as_deref().map(map_cs_type).unwrap_or("long");

        out.push_str(&format!(
            "\n    [LibraryImport(DllName, EntryPoint = \"{}\")]\n",
            f.name
        ));
        out.push_str(&format!(
            "    public static partial {} {}({});\n",
            ret,
            f.name,
            params.join(", ")
        ));
    }

    out.push_str("}\n");
    out
}

fn map_cs_type(ty: &str) -> &'static str {
    match ty {
        "int" => "long",
        "float" => "double",
        "f32" => "float",
        "u32" => "uint",
        "i32" => "int",
        "u8" => "byte",
        "i8" => "sbyte",
        "bool" => "byte",
        _ => "IntPtr",
    }
}

fn map_cs_param_type(ty: &str, is_mut: bool, prog: &Program) -> String {
    let clean = ty.strip_prefix("shared:").unwrap_or(ty);
    if prog.structs.iter().any(|s| s.name == clean) {
        if is_mut {
            format!("ref {}", clean)
        } else {
            format!("in {}", clean)
        }
    } else {
        map_cs_type(clean).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csharp_codegen_open_enum_sync_rpc() {
        let src = r#"
open enum Fuel { Liquid, Solid }
struct Ship {
    sync x: float,
    sync shield: f32,
}
rpc fn fire_laser(mut s: Ship, power: float) {}
"#;
        let mut prog = crate::Parser::new(src).parse_program();
        crate::merge_open_enums(&mut prog);
        let cs = generate_csharp_bindings(&prog, "striv_game");
        assert!(cs.contains("public static class Fuel"));
        assert!(cs.contains("public ulong __dirty_mask;"));
        assert!(cs.contains("public float shield;"));
        assert!(cs.contains("public const uint RpcId_fire_laser"));
        assert!(cs.contains("public static partial long fire_laser(ref Ship s, double power);"));
    }
}
