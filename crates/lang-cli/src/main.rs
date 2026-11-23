use lang_jit::JitCompiler;
use lang_syntax::Parser;
use std::{env, fs, mem, time::Instant};

fn main() {
    let source = env::args()
        .nth(1)
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_else(|| {
            r#"
            fn calculate_damage(mut hp: int, dmg: int, armor: int) -> int {
                let mut real_dmg = dmg - armor
                if real_dmg < 1 {
                    real_dmg = 1
                }
                hp = hp - real_dmg
                return hp
            }
        "#
            .to_string()
        });

    let start = Instant::now();

    let mut parser = Parser::new(&source);
    let func = parser.parse_function();

    let mut jit = JitCompiler::new();
    let ptr = jit.compile(&func);

    let elapsed = start.elapsed();

    let run_fn: fn(i64, i64, i64) -> i64 = unsafe { mem::transmute(ptr) };
    println!(
        "Compiled in {:?} -> Result: {}",
        elapsed,
        run_fn(100, 35, 10)
    );
}
