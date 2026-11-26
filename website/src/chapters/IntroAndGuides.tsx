import { CodeBlock } from '../components/CodeBlock'
import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocH2, DocH3, DocP, DocTable } from './DocPrimitives'

export function OverviewChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Striv 1.0 — Language Documentation', 'Striv 1.0 — Sprachdokumentation')}</DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'Striv is a statically compiled systems programming language with a two-stage execution model: during development, a Cranelift JIT (striv run, striv dev, striv test) compiles code in milliseconds directly to native x86_64 machine code. For distribution, striv build transpiles the program to Rust and builds an optimized release executable via Cargo/LLVM.',
          'Striv ist eine statisch kompilierte Systemsprache mit einem zweistufigen Ausführungsmodell: Während der Entwicklung übersetzt ein Cranelift-JIT (striv run, striv dev, striv test) Quellcode in wenigen Millisekunden direkt in nativen x86_64-Maschinencode. Für Auslieferungen transpiliert striv build das Programm nach Rust und baut über Cargo/LLVM eine optimierte Release-Executable.',
        )}
      </DocP>

      <DocH2 id="what-is-striv" theme={theme}>
        {tr(lang, 'What is Striv?', 'Was ist Striv?')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'The language combines C-compatible struct layouts, automatic scope RAII without lifetime annotations, a built-in multi-core sparse-set Ecs, compile-time dimensional units (unit), bitmask delta networking (sync & rpc), and direct Rust crate and C DLL interop.',
          'Die Sprache kombiniert C-kompatible Struct-Layouts, automatisches Scope-RAII ohne Lifetime-Annotationen, ein eingebautes Multi-Core Sparse-Set Ecs, Compile-Time-Einheitenprüfung (unit), Bitmask-Delta-Networking (sync & rpc) sowie direkte Rust-Crate- und C-DLL-Anbindung.',
        )}
      </DocP>

      <DocH2 id="dual-syntax" theme={theme}>
        {tr(lang, 'Dual Syntax (Rust & Odin)', 'Dual-Syntax (Rust-Stil & Odin-Stil)')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Setting syntax = "rust" or syntax = "odin" in striv.toml switches the parser, formatter (fmt.rs), and LSP simultaneously. Both dialects produce the exact same AST. Use the toggle in the top right to switch every example on this site.',
          'Über syntax = "rust" oder syntax = "odin" in striv.toml schalten Parser, Formatter (fmt.rs) und LSP gleichzeitig um. Beide Dialekte erzeugen exakt denselben AST. Über den Schalter oben rechts kannst du alle Beispiele in dieser Dokumentation sofort umschalten.',
        )}
      </DocP>

      <DocH2 id="benchmarks" theme={theme}>
        {tr(lang, 'Measured Compile & Runtime Timings', 'Gemessene Compile- und Laufzeiten')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'All numbers below are measured directly with striv-cli test in release mode on x86_64 Windows (Cranelift 0.138):',
          'Alle folgenden Werte stammen direkt aus striv-cli test im Release-Profil auf x86_64 Windows (Cranelift 0.138):',
        )}
      </DocP>
      <DocTable
        theme={theme}
        headers={
          lang === 'de'
            ? ['Test-Suite', 'Tests', 'JIT-Compile', 'Ausführung', 'Inhalt']
            : ['Test Suite', 'Tests', 'JIT Compile', 'Execution', 'Scope']
        }
        rows={[
          ['comprehensive_suite.strv', '12', '8.58 ms', '1.37 ms', 'Generics, Structs, Open Enums, Sync, RPC, Units, Parallel'],
          ['ecs_and_rust_test.strv', '1', '2.93 ms', '1.55 ms', 'Sparse-Set ECS parallel query (1,000 entities)'],
          ['json_and_bindgen3_test.strv', '1', '2.20 ms', '30.5 µs', 'Struct::to_json() & from_json() with default fields'],
          ['delta_sync_test.strv', '1', '2.51 ms', '3.4 µs', 'Bitmask dirty check, pack_delta() & pack_full()'],
          ['units_ksp_test.strv', '1', '2.62 ms', '100 ns', 'Physical units (kg, m, s, Force, Energy)'],
          ['open_enum_mod_test.strv', '1', '2.50 ms', '100 ns', 'FNV-1a 64-bit Open & Extend Enums'],
        ]}
      />

      <DocH2 id="first-program" theme={theme}>
        {tr(lang, 'First Program', 'Erstes Programm')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="src/main.strv"
        code={`struct Ship {
    x: float,
    hp: int = 100,
}

fn main() -> int {
    let mut s = Ship { x: 10.0 }
    s.hp -= 15
    print("Ship HP: {s.hp}")
    return 0
}`}
        odinCode={`Ship :: struct {
    x: float,
    hp: int = 100,
}

main :: proc() -> int {
    s := Ship { x: 10.0 }
    s.hp -= 15
    print("Ship HP: {s.hp}")
    return 0
}`}
      />
    </div>
  )
}

export function CliChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Installation & CLI', 'Installation & CLI')}</DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'The CLI is built from crates/striv-cli and bundles project scaffolding, JIT execution, hot-reload dev server, test runner, release transpiler, bindgen sync, formatter, and LSP.',
          'Die CLI wird über crates/striv-cli gebaut und bündelt Projekt-Scaffolding, JIT-Ausführung, Hot-Reload-Dev-Server, Test-Runner, Release-Transpiler, Bindgen-Sync, Formatter und LSP.',
        )}
      </DocP>

      <DocH2 id="project-setup" theme={theme}>
        striv new &amp; striv.toml
      </DocH2>
      <CodeBlock
        lang="bash"
        theme={theme}
        filename="terminal"
        code={`# Create a new project (interactive or via flags)
striv new space_game --rust --kr
striv new space_game --odin --allman`}
      />
      <CodeBlock
        lang="toml"
        theme={theme}
        filename="striv.toml"
        code={`[package]
name = "space_game"
version = "0.1.0"
syntax = "${dialect}"
braces = "kr"

[dependencies]
# For striv sync (lang-bindgen 3.0)`}
      />

      <DocH2 id="jit-commands" theme={theme}>
        run, dev (:4040) &amp; test
      </DocH2>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Command', 'Befehl'), tr(lang, 'Behavior', 'Funktion')]}
        rows={[
          [
            'striv run <file.strv>',
            tr(
              lang,
              'Loads imports, compiles via Cranelift JIT, and executes main() with compile/exec timings.',
              'Lädt Imports, kompiliert per Cranelift-JIT und führt main() aus (inkl. Compile-/Exec-Zeit).',
            ),
          ],
          [
            'striv dev <file.strv>',
            tr(
              lang,
              'Starts host window, file-watcher hot-reload (64 i64 persistent_state slots), and Tweak HTTP server on port :4040.',
              'Startet Host-Fenster, File-Watcher-Hot-Reload (64 i64 persistent_state Slots) und den Tweak-Webserver auf Port :4040.',
            ),
          ],
          [
            'striv live <file.strv>',
            tr(
              lang,
              'Starts the Tweak server (:4040) and runs main() in an 800 ms loop.',
              'Startet den Tweak-Server (:4040) und führt main() alle 800 ms in einer Endlosschleife aus.',
            ),
          ],
          [
            'striv test <file.strv>',
            tr(
              lang,
              'Compiles all test "..." blocks via compile_tests() and prints PASS/FAIL with timings.',
              'Kompiliert alle test "..."-Blöcke via compile_tests() und meldet PASS/FAIL samt Laufzeit.',
            ),
          ],
        ]}
      />

      <DocH2 id="build-sync" theme={theme}>
        build, sync &amp; bindgen-cs
      </DocH2>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Command', 'Befehl'), tr(lang, 'Behavior', 'Funktion')]}
        rows={[
          [
            'striv build <file.strv>',
            tr(
              lang,
              'Evaluates comptime consts, transpiles to .striv-release/src/main.rs, and builds <app>.exe via cargo build --release.',
              'Wertet Comptime-const aus, transpiliert nach .striv-release/src/main.rs und baut <app>.exe via cargo build --release.',
            ),
          ],
          [
            'striv sync [striv.toml]',
            tr(
              lang,
              'Scans [dependencies] via cargo metadata and generates crates/lang-runtime/src/generated_bridge.rs.',
              'Liest [dependencies] via cargo metadata und generiert crates/lang-runtime/src/generated_bridge.rs.',
            ),
          ],
          [
            'striv bindgen <bridge.rs>',
            tr(
              lang,
              'Exports manually annotated Rust functions from a .rs file into generated_bridge.rs.',
              'Exportiert manuell markierte Rust-Funktionen aus einer .rs-Datei nach generated_bridge.rs.',
            ),
          ],
          [
            'striv bindgen-cs <api.strv> [Out.cs]',
            tr(
              lang,
              'Generates C# modding bindings (StructLayout, FNV-1a Open Enums, RpcId_*, LibraryImport).',
              'Generiert C#-Modding-Bindings (StructLayout, FNV-1a Open Enums, RpcId_*, LibraryImport).',
            ),
          ],
        ]}
      />

      <DocH2 id="fmt-lsp" theme={theme}>
        fmt, switch-syntax &amp; lsp
      </DocH2>
      <CodeBlock
        lang="bash"
        theme={theme}
        filename="terminal"
        code={`# Format source according to striv.toml
striv fmt src/main.strv

# Convert project between Rust and Odin dialects
striv switch-syntax odin src/main.strv --kr

# Start Language Server for VS Code & Zed
striv lsp`}
      />

      <DocH3>
        {tr(
          lang,
          'Downloads, VS Code Extension & Zed Editor',
          'Downloads, VS Code Extension & Zed Editor',
        )}
      </DocH3>
      <DocP theme={theme}>
        {tr(
          lang,
          'Prebuilt binaries and editor extensions are published in GitHub Release v1.1.0 (striv.exe, striv-vscode-ext.zip, striv-zed-ext.zip). Place striv.exe on your system PATH so editors can launch striv lsp.',
          'Fertige Binaries und Editor-Extensions liegen im GitHub Release v1.1.0 (striv.exe, striv-vscode-ext.zip, striv-zed-ext.zip). Lege striv.exe in deinen System-PATH, damit Editoren striv lsp starten können.',
        )}
      </DocP>
      <DocTable
        theme={theme}
        headers={[
          tr(lang, 'Editor', 'Editor'),
          tr(lang, 'Package / Setup', 'Paket / Einrichtung'),
        ]}
        rows={[
          [
            'VS Code',
            tr(
              lang,
              'Extract striv-vscode-ext.zip into %USERPROFILE%\\.vscode\\extensions\\striv-lang\\ and run npm install --omit=dev once.',
              'striv-vscode-ext.zip nach %USERPROFILE%\\.vscode\\extensions\\striv-lang\\ entpacken und einmalig npm install --omit=dev ausführen.',
            ),
          ],
          [
            'Zed',
            tr(
              lang,
              'Extract striv-zed-ext.zip and use Extensions → Install Dev Extension, or point lsp.striv.binary in settings.json to striv lsp.',
              'striv-zed-ext.zip entpacken und in Zed Extensions → Install Dev Extension wählen (oder lsp.striv.binary in settings.json auf striv lsp setzen).',
            ),
          ],
        ]}
      />
    </div>
  )
}

export function FromRustChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Guide: Coming from Rust', 'Guide: Umstieg von Rust')}</DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'Striv adopts Rust value semantics, struct, impl, enum, and match, while omitting lifetime annotations (\'a) and Arc<Mutex<T>> boilerplate in the JIT.',
          'Striv übernimmt aus Rust die klare Wert-Semantik, struct, impl, enum und match, verzichtet aber im JIT bewusst auf Lifetime-Annotationen (\'a) und Arc<Mutex<T>>-Boilerplate.',
        )}
      </DocP>

      <DocH2 id="values-raii" theme={theme}>
        {tr(lang, 'Values & Automatic Scope RAII', 'Werte & automatisches Scope-RAII')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Structs live on Cranelift StackSlots and are passed by 8-byte pointer without copying. Runtime handles (Vec, Map, Ecs, Bytes, Channel) are automatically freed at scope exit via std_drop_symbols().',
          'Structs liegen auf Cranelift-StackSlots und werden bei Funktionsaufrufen per 8-Byte-Zeiger ohne Kopie übergeben. Runtime-Handles (Vec, Map, Ecs, Bytes, Channel) werden am Scope-Ende automatisch über std_drop_symbols() freigegeben.',
        )}
      </DocP>

      <DocH2 id="concurrency-diff" theme={theme}>
        {tr(lang, 'concurrent & shared vs. Arc<Mutex<T>>', 'concurrent & shared statt Arc<Mutex<T>>')}
      </DocH2>
      <CodeBlock
        lang="rust"
        theme={theme}
        filename="rust_comparison.rs"
        code={`// Rust: Arc + Mutex + move closure
let counter = Arc::new(Mutex::new(0));
std::thread::scope(|s| {
    let c = Arc::clone(&counter);
    s.spawn(move || { *c.lock().unwrap() += 1; });
});`}
      />
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="striv_equivalent.strv"
        code={`struct Counter { hits: int }

fn worker(shared c: Counter) {
    lock c { c.hits += 1 }
}

fn main() -> int {
    shared counter = Counter { hits: 0 }
    concurrent {
        spawn worker(counter)
    }
    lock counter {
        assert(counter.hits == 1)
    }
    return 0
}`}
        odinCode={`Counter :: struct { hits: int }

worker :: proc(shared c: Counter) {
    lock c { c.hits += 1 }
}

main :: proc() -> int {
    shared counter = Counter { hits: 0 }
    concurrent {
        spawn worker(counter)
    }
    lock counter {
        assert(counter.hits == 1)
    }
    return 0
}`}
      />

      <DocH2 id="crate-interop" theme={theme}>
        {tr(lang, 'Crates via striv sync & rust fn', 'Crates via striv sync & rust fn')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Any Rust crate can be listed under [dependencies] in striv.toml and bound into the JIT runtime via striv sync. For custom Rust helpers, use inline rust { ... } and rust fn.',
          'Beliebige Rust-Crates werden in striv.toml unter [dependencies] eingetragen und mit striv sync automatisch in die JIT-Runtime eingebunden. Für eigene Rust-Helper stehen rust { ... } und rust fn bereit.',
        )}
      </DocP>
    </div>
  )
}

export function FromOdinChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Guide: Coming from Odin', 'Guide: Umstieg von Odin')}</DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'With syntax = "odin" in striv.toml (or striv new app --odin), Striv directly uses Odin-style declarations, := bindings, foreign C-DLL blocks, tuple swizzling, and defer.',
          'Mit syntax = "odin" in striv.toml (oder striv new app --odin) nutzt Striv direkt Odin-Deklarationen, :=-Bindungen, foreign-C-DLL-Blöcke, Tupel-Swizzling und defer.',
        )}
      </DocP>

      <DocH2 id="odin-dialect" theme={theme}>
        {tr(lang, 'Enabling the Odin Dialect', 'Odin-Dialekt aktivieren')}
      </DocH2>
      <CodeBlock
        lang="bash"
        theme={theme}
        filename="terminal"
        code={`# New Odin-style project
striv new my_game --odin --kr

# Or convert an existing Rust-style file to Odin-style
striv switch-syntax odin src/main.strv`}
      />

      <DocH2 id="syntax-mapping" theme={theme}>
        {tr(lang, 'Side-by-Side Comparison', 'Syntax-Vergleich')}
      </DocH2>
      <DocTable
        theme={theme}
        headers={[
          tr(lang, 'Construct', 'Konstrukt'),
          tr(lang, 'Odin Dialect in Striv', 'Odin-Dialekt in Striv'),
          tr(lang, 'Rust Dialect in Striv', 'Rust-Dialekt in Striv'),
        ]}
        rows={[
          ['Procedure / Fn', 'main :: proc() -> int { ... }', 'fn main() -> int { ... }'],
          ['Struct', 'Ship :: struct { x: float }', 'struct Ship { x: float }'],
          ['Enum / Open Enum', 'State :: open enum { Idle }', 'open enum State { Idle }'],
          ['Extend Enum', 'State :: extend enum { Warp }', 'extend enum State { Warp }'],
          ['RPC Procedure', 'fire :: rpc proc(mut w: W) { ... }', 'rpc fn fire(mut w: W) { ... }'],
          ['Constant', 'MAX :: 100', 'const MAX = 100'],
          ['Immutable Var', 'x := 10', 'let x = 10'],
          ['Mutable Var', 'hp := 100 (immer mut)', 'let mut hp = 100'],
          ['Typed Var', 'x : float = 10.0', 'let x: float = 10.0'],
          ['Zero-Init / Declare', 'x: Ship / tex: TextureHandle', 'let mut tex: int = null'],
          ['C DLL Import', 'foreign "msvcrt.dll" { ... }', 'extern "msvcrt.dll" fn ...'],
        ]}
      />

      <DocH2 id="odin-zero-init" theme={theme}>
        {tr(lang, 'Zero-Initialization & Null-Handles (null & .is_valid())', 'Zero-Initialisierung & Null-Handles (null & .is_valid())')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'In Odin mode, pure type declarations (x: Type without =) are type-initialized by the JIT: primitives to 0/0.0, structs with their default fields, containers (Vec, Bytes, Ecs, String) via ::new(), and external handles to a safe 0 (null) checkable in 1 CPU cycle via .is_valid().',
          'Im Odin-Modus werden reine Typ-Deklarationen (x: Typ ohne =) typgesteuert vom JIT initialisiert: Primitive auf 0/0.0, Structs inkl. Default-Feldern, Container (Vec, Bytes, Ecs, String) per ::new() und externe Handles auf ein sicheres 0 (null), prüfbar in 1 CPU-Takt über .is_valid().',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="null_handles.strv"
        code={`fn check_handle() -> int {
    let mut tex = null
    if !tex.is_valid() {
        tex = 42
    }
    return tex
}`}
        odinCode={`check_handle :: proc() -> int {
    tex: TextureHandle
    if !tex.is_valid() {
        tex = 42
    }
    return tex
}`}
      />

      <DocH2 id="smart-move-clone" theme={theme}>
        {tr(lang, 'Smart Auto-Move / Auto-Clone & Drop-on-Reassign', 'Smart Auto-Move / Auto-Clone & Drop-on-Reassign')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'You never need to type .clone() for heap containers (Bytes, Vec, String). The compiler performs last-use analysis: if a is read again later, b := a automatically clones; if it is the last use, b := a performs a 0-ns move. Reassigning a = ... frees the old memory immediately, and assigning null to built-in containers is rejected by both the compiler and LSP.',
          'Für Heap-Container (Bytes, Vec, String) musst du niemals .clone() tippen. Der Compiler prüft per Last-Use-Analyse: Wird a weiter unten noch gelesen, klont b := a automatisch; bei der letzten Nutzung erfolgt ein 0-ns-Move. Neuzuweisungen (a = ...) geben alten Speicher sofort frei (Drop-on-Reassign), und null auf eingebauten Containern wird von Compiler & LSP blockiert.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="smart_move_clone.strv"
        code={`fn demo() -> int {
    let a = Bytes::new()
    a.write_u32(111)
    let b = a // a is read below -> Auto-Clone!
    b.write_u32(222)
    assert(a.len() == 4)
    let c = b // last use of b -> 0-ns Auto-Move!
    return c.len()
}`}
        odinCode={`demo :: proc() -> int {
    a := Bytes::new()
    a.write_u32(111)
    b := a // a wird unten gelesen -> Auto-Clone!
    b.write_u32(222)
    assert(a.len() == 4)
    c := b // letzte Nutzung von b -> 0-ns Auto-Move!
    return c.len()
}`}
      />

      <DocH2 id="odin-examples" theme={theme}>
        {tr(lang, 'Odin-Style Examples', 'Odin-Beispiele')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Tuple swizzling (.x/.y/.z/.w and .0/.1/.2), LIFO defer, and direct foreign C-DLL calls work identically to Odin:',
          'Tupel-Swizzling (.x/.y/.z/.w und .0/.1/.2), LIFO-defer und direkte foreign-C-DLL-Aufrufe funktionieren wie aus Odin gewohnt:',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="odin_features.strv"
        code={`extern "msvcrt.dll" fn abs(x: i32) -> i32
extern "msvcrt.dll" fn strlen(s: cstr) -> int

fn demo() -> float {
    let pos = (1.0, 2.0, 3.0)
    defer print("Scope verlassen")
    let a = abs(-42)
    let n = strlen("Striv")
    return pos.x + pos.y + pos.z + a + n
}`}
        odinCode={`foreign "msvcrt.dll" {
    abs :: proc(x: i32) -> i32
    strlen :: proc(s: cstr) -> int
}

demo :: proc() -> float {
    pos := (1.0, 2.0, 3.0)
    defer print("Scope verlassen")
    a := abs(-42)
    n := strlen("Striv")
    return pos.x + pos.y + pos.z + a + n
}`}
      />

      <DocH2 id="striv-extras" theme={theme}>
        {tr(lang, 'Extras: unit, sync, rpc & Rust Crates', 'Mehrwerte: unit, sync, rpc & Rust-Crates')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'On top of the familiar Odin workflow, Striv adds compile-time physical units (unit), bitmask delta networking (sync / rpc), a multi-core sparse-set Ecs (parallel query), and direct access to Rust crates via striv sync.',
          'Zusätzlich zum gewohnten Odin-Workflow bietet Striv Compile-Time-Einheiten (unit), eingebautes Bitmask-Delta-Networking (sync / rpc), ein Multi-Core Sparse-Set Ecs (parallel query) sowie direkten Zugriff auf Rust-Crates via striv sync.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="odin_with_extras.strv"
        code={`unit m
unit s
unit Vel = m / s

struct Ship {
    sync x: m,
    vel: Vel,
}

fn step(mut s: Ship, dt: s) {
    s.x = s.x + s.vel * dt
}`}
        odinCode={`unit m
unit s
unit Vel = m / s

Ship :: struct {
    sync x: m,
    vel: Vel,
}

step :: proc(mut s: Ship, dt: s) {
    s.x = s.x + s.vel * dt
}`}
      />
    </div>
  )
}
