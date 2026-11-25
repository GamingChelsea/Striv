use std::{env, fs, mem, time::Instant};
use striv_jit::JitCompiler;
use striv_syntax::format_source;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "run" => {
            let path = args
                .get(2)
                .expect("Please provide a file: cargo run -p striv-cli -- run test.strv");

            let start = Instant::now();
            let prog = striv_syntax::load_program_with_imports(std::path::Path::new(path))
                .expect("Failed to load program");
            let mut jit = JitCompiler::new();
            let ptr = jit.compile_program(&prog);
            let elapsed = start.elapsed();

            let main_fn: fn() -> i64 = unsafe { mem::transmute(ptr) };
            let exec_start = Instant::now();
            let res = main_fn();
            let exec_elapsed = exec_start.elapsed();
            println!(
                "Compiled in {:?} | Executed in {:?} -> Result: {}",
                elapsed, exec_elapsed, res
            );
        }
        "new" => {
            let proj_name = args
                .get(2)
                .expect("Projektname angeben: striv new space_game");
            let root = std::path::Path::new(proj_name);
            std::fs::create_dir_all(root.join("src")).expect("Konnte src/ nicht anlegen");
            std::fs::create_dir_all(root.join("mods")).expect("Konnte mods/ nicht anlegen");
            std::fs::create_dir_all(root.join(".vscode")).expect("Konnte .vscode/ nicht anlegen");
            std::fs::write(
                root.join("striv.toml"),
                format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[dependencies]\n", proj_name),
            )
            .expect("Konnte striv.toml nicht schreiben");
            std::fs::write(
                root.join("src/main.strv"),
                "fn main() {\n    print(\"Willkommen in Striv!\")\n}\n",
            )
            .expect("Konnte src/main.strv nicht schreiben");
            std::fs::write(root.join(".gitignore"), "target/\n.striv-release/\n")
                .expect("Konnte .gitignore nicht schreiben");
            std::fs::write(
                root.join(".vscode/settings.json"),
                "{\n    \"files.associations\": {\n        \"*.strv\": \"striv\"\n    }\n}\n",
            )
            .expect("Konnte .vscode/settings.json nicht schreiben");
            println!("[Striv] Neues Projekt '{}' erstellt!", proj_name);
        }
        "bindgen-cs" => {
            let path = args.get(2).expect("Datei angeben: striv bindgen-cs api.strv");
            let out_path = args.get(3).map(|s| s.as_str()).unwrap_or("StrivModApi.cs");
            let src = std::fs::read_to_string(path).expect("Konnte Datei nicht lesen");
            let mut prog = striv_syntax::Parser::new(&src).parse_program();
            striv_syntax::merge_open_enums(&mut prog);
            let cs = striv_syntax::csharp_gen::generate_csharp_bindings(&prog, "striv_game");
            std::fs::write(out_path, cs).expect("Konnte .cs Datei nicht schreiben");
            println!("[Striv] C#-Modding-Bindings erzeugt: {}", out_path);
        }
        "test" => {
            let path = args
                .get(2)
                .expect("Please provide a file: striv-cli test suite.strv");
            let compile_start = Instant::now();
            let prog = striv_syntax::load_program_with_imports(std::path::Path::new(path))
                .expect("Failed to load program");

            let mut jit = JitCompiler::new();
            let tests = jit.compile_tests(&prog);
            let compile_dur = compile_start.elapsed();

            let mut passed = 0;
            let mut failed = 0;
            for (name, test_fn) in tests {
                let _ = lang_runtime::take_assert_failed();
                let t0 = Instant::now();
                test_fn();
                let dur = t0.elapsed();
                if lang_runtime::take_assert_failed() {
                    println!("  FAIL  {} ({:?})", name, dur);
                    failed += 1;
                } else {
                    println!("  PASS  {} ({:?})", name, dur);
                    passed += 1;
                }
            }
            println!("--------------------------------------------------");
            println!(
                "{} passed, {} failed (compiled in {:?})",
                passed, failed, compile_dur
            );
            if failed > 0 {
                std::process::exit(1);
            }
        }
        "fmt" => {
            let path = args
                .get(2)
                .expect("Please provide a file: cargo run -p striv-cli -- fmt test.strv");

            let start = Instant::now();
            let source = fs::read_to_string(path).expect("Could not read file");
            let formatted = format_source(&source);
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
                let prog = striv_syntax::load_program_with_imports(std::path::Path::new(p)).ok()?;
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

            lang_runtime::start_tweak_server();
            let prog = striv_syntax::load_program_with_imports(std::path::Path::new(path))
                .expect("Failed to load program");
            let mut jit = JitCompiler::new();
            let ptr = jit.compile_program(&prog);
            let main_fn: fn() -> i64 = unsafe { mem::transmute(ptr) };

            println!("Starting live loop (Ctrl+C to stop)...");
            loop {
                main_fn();
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
        }
        "build" => {
            let path = args
                .get(2)
                .expect("Please provide a file: striv-cli build main.strv");
            let entry = std::path::Path::new(path);
            let app_name = entry
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("striv_app");

            let start = Instant::now();
            let prog =
                striv_syntax::load_program_with_imports(entry).expect("Failed to load modules");
            let comptime_consts = striv_jit::eval_comptime_consts(&prog);
            let rust_code =
                striv_syntax::release_gen::transpile_to_rust_with_consts(&prog, &comptime_consts);

            let _ = fs::create_dir_all(".striv-release/src");
            let deps_section =
                fs::read_to_string("striv.toml").unwrap_or_else(|_| "[dependencies]\n".into());
            let cargo_toml = format!(
                "[workspace]\n\n[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{}\n",
                app_name, deps_section
            );

            fs::write(".striv-release/Cargo.toml", cargo_toml).unwrap();
            fs::write(".striv-release/src/main.rs", rust_code).unwrap();

            println!("[Striv] Compiling optimized release EXE with LLVM...");
            let status = std::process::Command::new("cargo")
                .args([
                    "build",
                    "--release",
                    "--manifest-path",
                    ".striv-release/Cargo.toml",
                ])
                .status()
                .expect("Failed to launch cargo");

            if status.success() {
                let built_exe = format!(".striv-release/target/release/{}.exe", app_name);
                let out_exe = format!("{}.exe", app_name);
                fs::copy(&built_exe, &out_exe).expect("Failed to copy built executable");
                println!(
                    "[Striv] Done in {:?}! Standalone executable created: {}",
                    start.elapsed(),
                    out_exe
                );
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
