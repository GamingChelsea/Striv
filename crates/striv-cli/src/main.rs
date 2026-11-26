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
                .expect("Projektname angeben: striv new space_game [--odin|--rust]");
            let is_odin = if args.iter().any(|a| a == "--odin") {
                true
            } else if args.iter().any(|a| a == "--rust") {
                false
            } else {
                println!("Welchen Syntax-Stil soll '{}' verwenden?", proj_name);
                println!("  [1] Rust-Style  (fn main(), struct Ship, let x = 10)");
                println!("  [2] Odin-Style  (main :: proc(), Ship :: struct, x := 10)");
                print!("Auswahl [1/2]: ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let mut input = String::new();
                let _ = std::io::stdin().read_line(&mut input);
                input.trim() == "2" || input.trim().eq_ignore_ascii_case("odin")
            };
            let is_allman = if args.iter().any(|a| a == "--allman") {
                true
            } else if args.iter().any(|a| a == "--kr") {
                false
            } else {
                println!("Welchen Klammer-Stil (Formatter) soll '{}' verwenden?", proj_name);
                println!("  [1] K&R-Style     (fn main() {{)");
                println!("  [2] Allman-Style  (fn main()\\n{{)");
                print!("Auswahl [1/2]: ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let mut b_in = String::new();
                let _ = std::io::stdin().read_line(&mut b_in);
                b_in.trim() == "2" || b_in.trim().eq_ignore_ascii_case("allman")
            };
            let root = std::path::Path::new(proj_name);
            std::fs::create_dir_all(root.join("src")).expect("Konnte src/ nicht anlegen");
            std::fs::create_dir_all(root.join("mods")).expect("Konnte mods/ nicht anlegen");
            std::fs::create_dir_all(root.join(".vscode")).expect("Konnte .vscode/ nicht anlegen");
            let syntax_line = if is_odin { "syntax = \"odin\"\n" } else { "syntax = \"rust\"\n" };
            let braces_line = if is_allman { "braces = \"allman\"\n" } else { "braces = \"kr\"\n" };
            std::fs::write(
                root.join("striv.toml"),
                format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\n{}{}\n[dependencies]\n", proj_name, syntax_line, braces_line),
            )
            .expect("Konnte striv.toml nicht schreiben");
            let main_src = if is_odin {
                "main :: proc() -> int {\n    msg := \"Hallo aus Striv im Odin-Modus!\"\n    print(msg)\n    return 0\n}\n"
            } else {
                "fn main() {\n    print(\"Willkommen in Striv!\")\n}\n"
            };
            std::fs::write(root.join("src/main.strv"), main_src)
                .expect("Konnte src/main.strv nicht schreiben");
            std::fs::write(root.join(".gitignore"), "target/\n.striv-release/\n")
                .expect("Konnte .gitignore nicht schreiben");
            std::fs::write(
                root.join(".vscode/settings.json"),
                "{\n    \"files.associations\": {\n        \"*.strv\": \"striv\"\n    }\n}\n",
            )
            .expect("Konnte .vscode/settings.json nicht schreiben");
            println!("[Striv] Neues Projekt '{}' ({}) erstellt!", proj_name, if is_odin { "Odin-Style" } else { "Rust-Style" });
        }
        "switch-syntax" => {
            let target = args
                .get(2)
                .expect("Dialekt angeben: striv switch-syntax <odin|rust> [datei.strv]");
            let target_dialect = if target.eq_ignore_ascii_case("odin") {
                striv_syntax::Dialect::Odin
            } else {
                striv_syntax::Dialect::Rust
            };
            let file_path = args.get(3).map(|s| s.as_str()).unwrap_or("src/main.strv");
            let p = std::path::Path::new(file_path);
            let cur_dialect = striv_syntax::detect_dialect_for_path(p);
            let src = std::fs::read_to_string(p).expect("Konnte .strv Datei nicht lesen");
            let prog = striv_syntax::Parser::new_with_dialect(&src, cur_dialect).parse_program();
            let target_braces = if args.iter().any(|a| a == "--allman") {
                striv_syntax::BraceStyle::Allman
            } else if args.iter().any(|a| a == "--kr") {
                striv_syntax::BraceStyle::Kr
            } else {
                striv_syntax::detect_format_options_for_path(p).1
            };
            let converted = striv_syntax::format_program_with_options(&prog, target_dialect, target_braces);
            std::fs::write(p, converted).expect("Konnte .strv Datei nicht schreiben");
            println!("[Striv] '{}' konvertiert zu {:?}!", file_path, target_dialect);
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
        "add" => {
            let spec = args.get(2).expect(
                "Paket angeben: striv add <crate[@version] | git:user/repo> [-F feat1,feat2]",
            );
            let toml_path = std::path::Path::new("striv.toml");

            let mut features: Vec<String> = Vec::new();
            let mut i = 3;
            while i < args.len() {
                if (args[i] == "-F" || args[i] == "--features") && i + 1 < args.len() {
                    features.extend(
                        args[i + 1]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }

            if spec.starts_with("git:")
                || spec.starts_with("https://")
                || spec.starts_with("http://")
            {
                let url = if let Some(short) = spec.strip_prefix("git:") {
                    format!("https://github.com/{}.git", short.trim_start_matches('/'))
                } else {
                    spec.clone()
                };
                let pkg_name = url
                    .trim_end_matches(".git")
                    .rsplit('/')
                    .next()
                    .expect("Ungültige Git-URL")
                    .to_string();

                let dest = std::path::Path::new(".striv").join("pkgs").join(&pkg_name);
                if dest.exists() {
                    let _ = fs::remove_dir_all(&dest);
                }
                fs::create_dir_all(dest.parent().unwrap()).unwrap();
                let status = std::process::Command::new("git")
                    .args(["clone", "--depth", "1", &url, dest.to_str().unwrap()])
                    .status()
                    .expect("Konnte `git clone` nicht starten");
                assert!(status.success(), "Git-Clone fehlgeschlagen für {}", url);

                upsert_toml_entry(
                    toml_path,
                    "striv-packages",
                    &pkg_name,
                    &format!("\"{}\"", url),
                );
                println!(
                    "[Striv] Striv-Paket '{}' installiert (.striv/pkgs/{})",
                    pkg_name, pkg_name
                );
            } else {
                let (crate_name, version) = if let Some((n, v)) = spec.split_once('@') {
                    (n.to_string(), v.to_string())
                } else {
                    let v = lookup_latest_crate_version(spec).unwrap_or_else(|e| panic!("{}", e));
                    (spec.to_string(), v)
                };

                let rhs = if features.is_empty() {
                    format!("\"{}\"", version)
                } else {
                    let feats: Vec<String> = features.iter().map(|f| format!("\"{}\"", f)).collect();
                    format!(
                        "{{ version = \"{}\", features = [{}] }}",
                        version,
                        feats.join(", ")
                    )
                };

                upsert_toml_entry(toml_path, "dependencies", &crate_name, &rhs);
                println!("[Striv] + {} v{} -> striv.toml", crate_name, version);

                if !args.iter().any(|a| a == "--no-sync") {
                    let (code, summary) = lang_bindgen::sync_project("striv.toml");
                    fs::write("crates/lang-runtime/src/generated_bridge.rs", code)
                        .expect("Could not write generated_bridge.rs");
                    println!(
                        "[Striv] Synced {} crate(s): {} struct vault(s), {} enum(s), {} function(s)",
                        summary.crate_count,
                        summary.struct_count,
                        summary.enum_count,
                        summary.fn_count
                    );
                }
            }
        }
        "remove" => {
            let name = args.get(2).expect("Paketname angeben: striv remove <name>");
            let toml_path = std::path::Path::new("striv.toml");
            let pkg_dir = std::path::Path::new(".striv").join("pkgs").join(name);
            if pkg_dir.exists() {
                let _ = fs::remove_dir_all(&pkg_dir);
            }
            if remove_toml_entry(toml_path, name) {
                println!("[Striv] - '{}' aus striv.toml entfernt", name);
            } else {
                println!("[Striv] Paket '{}' war nicht in striv.toml eingetragen", name);
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
                "Commands:\n  run <file.strv> - Compile & run immediately\n  add <crate|git:user/repo> - Add Rust crate or Striv package\n  remove <name> - Remove package\n  fmt <file.strv> - Format code automatically"
            );
        }
    }
}

fn lookup_latest_crate_version(crate_name: &str) -> Result<String, String> {
    let out = std::process::Command::new("cargo")
        .args(["search", crate_name, "--limit", "5"])
        .output()
        .map_err(|e| format!("Konnte `cargo search` nicht ausführen: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some((left, _)) = line.split_once('#') {
            if let Some((name_part, ver_part)) = left.split_once('=') {
                if name_part.trim() == crate_name {
                    let ver = ver_part.trim().trim_matches('"').to_string();
                    return Ok(ver);
                }
            }
        }
    }
    Err(format!(
        "Crate '{}' wurde auf crates.io nicht gefunden",
        crate_name
    ))
}

fn upsert_toml_entry(toml_path: &std::path::Path, section: &str, key: &str, value_rhs: &str) {
    let content =
        fs::read_to_string(toml_path).unwrap_or_else(|_| "[package]\nname = \"app\"\n".to_string());
    let header = format!("[{}]", section);
    let new_line = format!("{} = {}", key, value_rhs);

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut sec_start = None;
    let mut sec_end = lines.len();

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t == header {
            sec_start = Some(i);
        } else if sec_start.is_some() && t.starts_with('[') && t.ends_with(']') {
            sec_end = i;
            break;
        }
    }

    if let Some(start) = sec_start {
        for i in (start + 1)..sec_end {
            let t = lines[i].trim();
            if let Some((k, _)) = t.split_once('=') {
                if k.trim() == key {
                    lines[i] = new_line;
                    fs::write(toml_path, lines.join("\n") + "\n").unwrap();
                    return;
                }
            }
        }
        lines.insert(sec_end, new_line);
    } else {
        if !lines.last().is_none_or(|l| l.is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(new_line);
    }
    fs::write(toml_path, lines.join("\n") + "\n").unwrap();
}

fn remove_toml_entry(toml_path: &std::path::Path, key: &str) -> bool {
    let Ok(content) = fs::read_to_string(toml_path) else {
        return false;
    };
    let mut removed = false;
    let kept: Vec<&str> = content
        .lines()
        .filter(|l| {
            if let Some((k, _)) = l.trim().split_once('=') {
                if k.trim() == key {
                    removed = true;
                    return false;
                }
            }
            true
        })
        .collect();
    if removed {
        let _ = fs::write(toml_path, kept.join("\n") + "\n");
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_and_remove_toml_entry() {
        let tmp = std::env::temp_dir().join("striv_test_toml_pkg.toml");
        let _ = fs::remove_file(&tmp);

        upsert_toml_entry(&tmp, "dependencies", "glam", "\"0.29\"");
        upsert_toml_entry(
            &tmp,
            "dependencies",
            "wgpu",
            "{ version = \"24\", features = [\"wgsl\"] }",
        );
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("glam = \"0.29\""));
        assert!(content.contains("wgpu = { version = \"24\", features = [\"wgsl\"] }"));

        assert!(remove_toml_entry(&tmp, "glam"));
        let after = fs::read_to_string(&tmp).unwrap();
        assert!(!after.contains("glam ="));
        assert!(after.contains("wgpu ="));
        let _ = fs::remove_file(&tmp);
    }
}
