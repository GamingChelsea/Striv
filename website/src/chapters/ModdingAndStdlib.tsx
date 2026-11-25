import { CodeBlock } from '../components/CodeBlock'
import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocH2, DocP, DocTable, InlineCode } from './DocPrimitives'

export function ModdingInteropChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Modding, Sandbox & Rust/C# Interop', 'Modding, Sandbox & Rust/C#-Interop')}</DocH1>

      <DocH2 id="open-enums" theme={theme}>
        open enum &amp; extend enum
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Variants of an open enum receive stable 64-bit FNV-1a hashes (fnv1a_64("Resource::Uranium")). Mods can add variants with extend enum without shifting existing IDs.',
          'Varianten eines open enum erhalten stabile 64-Bit FNV-1a Hashes (fnv1a_64("Resource::Uranium")). Mods können per extend enum neue Varianten hinzufügen, ohne bestehende IDs zu verschieben.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="modding_enums.strv"
        code={`open enum Resource { LiquidFuel, Oxidizer }
extend enum Resource { Uranium }

fn check_res() -> int {
    return Resource::Uranium
}`}
        odinCode={`Resource :: open enum { LiquidFuel, Oxidizer }
Resource :: extend enum { Uranium }

check_res :: proc() -> int {
    return Resource::Uranium
}`}
      />

      <DocH2 id="sandbox" theme={theme}>
        {tr(lang, 'Sandbox (Gas Limit & I/O Lock)', 'Sandbox (Gas-Limit & I/O-Sperre)')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'With JitCompiler::new_sandboxed(), Striv instruments loops with gas guards (set_sandbox_gas, take_sandbox_aborted) and blocks file I/O and external C libraries at compile time.',
          'Über JitCompiler::new_sandboxed() kompiliert Striv Mod-Code mit Schleifen-Gas-Guards (set_sandbox_gas, take_sandbox_aborted) und blockiert Datei-I/O sowie externe C-Bibliotheken bereits zur Compile-Zeit.',
        )}
      </DocP>

      <DocH2 id="csharp-bindgen" theme={theme}>
        {tr(lang, 'C# Bindings (striv bindgen-cs)', 'C#-Bindings (striv bindgen-cs)')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'striv bindgen-cs api.strv StrivModApi.cs generates [StructLayout(LayoutKind.Sequential)] structs (including __dirty_mask), matching open enum constants, RpcId_*, and [LibraryImport] signatures.',
          'striv bindgen-cs api.strv StrivModApi.cs generiert [StructLayout(LayoutKind.Sequential)]-Structs (inklusive __dirty_mask), identische open enum-Konstanten, RpcId_* und [LibraryImport]-Signaturen.',
        )}
      </DocP>

      <DocH2 id="rust-c-ffi" theme={theme}>
        {tr(lang, 'Rust Interop & Direct C-FFI', 'Rust-Interop & direktes C-FFI')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="interop.strv"
        code={`extern "msvcrt.dll" fn abs(x: i32) -> i32
extern "msvcrt.dll" fn strlen(s: cstr) -> int

rust { pub struct RustRocket { pub fuel: i64 } }
rust fn make_rocket(f: int) -> int {
    crate::box_any_handle(RustRocket { fuel: f })
}`}
        odinCode={`foreign "msvcrt.dll" {
    abs :: proc(x: i32) -> i32
    strlen :: proc(s: cstr) -> int
}

rust { pub struct RustRocket { pub fuel: i64 } }
rust fn make_rocket(f: int) -> int {
    crate::box_any_handle(RustRocket { fuel: f })
}`}
      />
    </div>
  )
}

export function StdlibTestingChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Standard Library & Testing', 'Standardbibliothek & Testing')}</DocH1>

      <DocH2 id="collections" theme={theme}>
        {tr(lang, 'Collections & Data Structures (collections.rs)', 'Collections & Datenstrukturen (collections.rs)')}
      </DocH2>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Type', 'Typ'), tr(lang, 'Constructor', 'Konstruktor'), tr(lang, 'Methods', 'Methoden')]}
        rows={[
          ['Vec / Vec<Struct>', 'let mut v: Vec<T> = []', 'push(x), len(), byte_len(), as_bytes_ptr(), clear(), v[i]'],
          ['Deque', 'Deque::new()', 'push_back(v), push_front(v), pop_front(), pop_back(), len()'],
          ['Map', 'Map::new()', 'insert(k, v), get(k), contains(k), remove(k), len()'],
          ['StrMap', 'StrMap::new()', 'insert(k, v), get(k), contains(k)'],
          ['BTree', 'BTree::new()', 'insert(k, v), get(k), first_key(), last_key()'],
          ['Set', 'Set::new()', 'insert(v), contains(v), remove(v), len()'],
          ['Heap', 'Heap::new()', 'push(v), pop(), peek(), len()'],
          ['Grid', 'Grid::new(w, h, def)', 'get(x, y), set(x, y, val), in_bounds(x, y)'],
        ]}
      />

      <DocH2 id="bytes-math" theme={theme}>
        Bytes, Rng, BitSet, Vec3 &amp; Mat4
      </DocH2>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Type', 'Typ'), tr(lang, 'Constructor', 'Konstruktor'), tr(lang, 'Methods', 'Methoden')]}
        rows={[
          ['Bytes', 'Bytes::new() / Bytes::read_file(p)', 'push(b), get(i), len(), write_u32/i64/f64, read_u32/i64/f64, write_file(p)'],
          ['Rng', 'Rng::new(seed)', 'range(min, max)'],
          ['BitSet', 'BitSet::new(bits)', 'set(i), get(i), clear(i)'],
          ['Vec3', 'Vec3::new(x, y, z)', 'dot(other), len()'],
          ['Mat4', 'Mat4::identity()', 'get(r, c), set(r, c, val)'],
        ]}
      />

      <DocH2 id="string-fs" theme={theme}>
        {tr(lang, 'String, fs & Math Intrinsics', 'String, fs & Mathe-Intrinsics')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Strings support .trim(), .contains(s), .starts_with(s), .replace(a, b), .to_int(), and .to_float(). File I/O uses fs::read_text(path) and fs::write_text(path, text). Built-in math intrinsics: sin, cos, tan, atan2, sqrt.',
          'Strings bieten .trim(), .contains(s), .starts_with(s), .replace(a, b), .to_int() und .to_float(). Dateizugriffe erfolgen über fs::read_text(path) und fs::write_text(path, text). Eingebaute Mathe-Intrinsics sind sin, cos, tan, atan2 und sqrt.',
        )}
      </DocP>

      <DocH2 id="testing" theme={theme}>
        test "..." &amp; assert(...)
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="suite.strv"
        code={`test "String und Mathe" {
    let raw = "  42  "
    let t = raw.trim()
    assert(t.to_int() == 42)
    assert(sqrt(81.0) == 9.0)
}`}
        odinCode={`test "String und Mathe" {
    raw := "  42  "
    t := raw.trim()
    assert(t.to_int() == 42)
    assert(sqrt(81.0) == 9.0)
}`}
      />
      <p className="text-xs font-mono text-zinc-500">
        <InlineCode theme={theme}>striv test suite.strv</InlineCode>
      </p>
    </div>
  )
}
