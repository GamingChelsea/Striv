use std::{env, fs, mem, time::Instant};
use striv_jit::JitCompiler;
use striv_syntax::{Parser, format_program};

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "run" => {
            let path = args
                .get(2)
                .expect("Please provide a file: cargo run -p striv-cli -- run test.strv");
            let source = fs::read_to_string(path).expect("Could not read file");

            let start = Instant::now();
            let prog = Parser::new(&source).parse_program();
            let mut jit = JitCompiler::new();
            let ptr = jit.compile_program(&prog);
            let elapsed = start.elapsed();

            let main_fn: fn() -> i64 = unsafe { mem::transmute(ptr) };
            let res = main_fn();
            println!("Compiled in {:?} -> Result: {}", elapsed, res);
        }
        "fmt" => {
            let path = args
                .get(2)
                .expect("Please provide a file: cargo run -p striv-cli -- fmt test.strv");
            let source = fs::read_to_string(path).expect("Could not read file");

            let start = Instant::now();
            let prog = Parser::new(&source).parse_program();
            let formatted = format_program(&prog);
            fs::write(path, formatted).expect("Could not save file");
            println!("Formatted '{}' in {:?}", path, start.elapsed());
        }
        "dev" => {
            let path = args
                .get(2)
                .expect("Please provide a file: striv-cli dev test.strv");

            lang_runtime::start_tweak_server();

            // Persistent state owned by the host (up to 64 i64 fields, preserved across reloads)
            let mut persistent_state = vec![0i64; 64];
            let mut last_modified = fs::metadata(path).and_then(|m| m.modified()).ok();

            let compile_fresh = |p: &str| -> Option<(JitCompiler, fn(*mut i64, i64) -> i64)> {
                let src = fs::read_to_string(p).ok()?;
                let errs = striv_syntax::check_diagnostics(&src);
                if !errs.is_empty() {
                    println!("[Striv] Waiting for fix: {}", errs[0].message);
                    return None;
                }
                let start = Instant::now();
                let prog = Parser::new(&src).parse_program();
                let mut jit = JitCompiler::new();
                let ptr = jit.compile_program(&prog);
                println!("[Striv] Hot-Reloaded in {:?}", start.elapsed());
                let func: fn(*mut i64, i64) -> i64 = unsafe { mem::transmute(ptr) };
                Some((jit, func))
            };

            let mut current = compile_fresh(path);
            if current.is_none() {
                println!("Please fix syntax errors in '{}' and save...", path);
            }

            let win_handle = lang_runtime::create_host_window("Striv Live Engine", 800, 600);
            persistent_state[0] = 350.0_f64.to_bits() as i64;
            persistent_state[1] = 250.0_f64.to_bits() as i64;

            println!(
                "[Striv] Dev-Loop running! Edit '{}' in VS Code and press Ctrl+S!",
                path
            );
            while lang_runtime::present_host_window(win_handle) {
                let now_mod = fs::metadata(path).and_then(|m| m.modified()).ok();
                if now_mod != last_modified {
                    last_modified = now_mod;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    if let Some(new_compiled) = compile_fresh(path) {
                        current = Some(new_compiled);
                    }
                }

                if let Some((_jit_guard, update_fn)) = &current {
                    update_fn(persistent_state.as_mut_ptr(), win_handle);
                }

                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }
        "live" => {
            let path = args
                .get(2)
                .expect("Please provide a file: striv-cli live test.strv");
            let source = fs::read_to_string(path).expect("Could not read file");

            lang_runtime::start_tweak_server();
            let prog = Parser::new(&source).parse_program();
            let mut jit = JitCompiler::new();
            let ptr = jit.compile_program(&prog);
            let main_fn: fn() -> i64 = unsafe { mem::transmute(ptr) };

            println!("Starting live loop (Ctrl+C to stop)...");
            loop {
                main_fn();
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
        }
        "sync" => {
            let toml_path = args.get(2).map(|s| s.as_str()).unwrap_or("striv.toml");
            let (code, summary) = lang_bindgen::sync_project(toml_path);
            fs::write("crates/lang-runtime/src/generated_bridge.rs", code)
                .expect("Could not write generated_bridge.rs");
            println!(
                "[Striv] Synced {} crate(s): {} struct vault(s), {} enum(s), {} function(s) -> crates/lang-runtime/src/generated_bridge.rs",
                summary.crate_count, summary.struct_count, summary.enum_count, summary.fn_count
            );
        }
        "bindgen" => {
            let input = args
                .get(2)
                .expect("Please provide a Rust file: striv-cli bindgen bridge.rs");
            let rust_src = fs::read_to_string(input).expect("Could not read file");
            let (code, funcs) = lang_bindgen::generate_rust_bridge(&rust_src);

            fs::write("crates/lang-runtime/src/generated_bridge.rs", code)
                .expect("Could not write generated_bridge.rs");

            println!("[Striv] Exported {} Rust functions for Striv:", funcs.len());
            for f in funcs {
                println!(
                    "  -> {} ({} params) => {}",
                    f.striv_name, f.param_count, f.c_symbol
                );
            }
        }
        "lsp" => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(lang_lsp::run_lsp_server());
        }
        _ => {
            println!(
                "Commands:\n  run <file.strv> - Compile & run immediately\n  fmt <file.strv> - Format code automatically"
            );
        }
    }
}
