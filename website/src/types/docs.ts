export type DialectMode = 'rust' | 'odin'
export type ThemeMode = 'light' | 'dark'
export type LangMode = 'en' | 'de'

export function tr(lang: LangMode, en: string, de: string): string {
  return lang === 'de' ? de : en
}

export type ChapterId =
  | 'overview'
  | 'cli'
  | 'from-rust'
  | 'from-odin'
  | 'basics'
  | 'structs'
  | 'enums-generics'
  | 'units'
  | 'keywords-types'
  | 'reference'
  | 'concurrency'
  | 'ecs'
  | 'networking'
  | 'modding-interop'
  | 'stdlib-testing'

export interface TocItem {
  id: string
  label: { en: string; de: string }
}

export interface ChapterMeta {
  id: ChapterId
  title: { en: string; de: string }
  group: { en: string; de: string }
  description: { en: string; de: string }
  toc: TocItem[]
}

export const CHAPTERS: ChapterMeta[] = [
  {
    id: 'overview',
    title: { en: 'Overview', de: 'Start / Übersicht' },
    group: { en: 'Getting Started', de: 'Einstieg' },
    description: {
      en: 'Architecture (Cranelift JIT + Rust/LLVM release), dual syntax, and measured timings.',
      de: 'Architektur (Cranelift-JIT + Rust/LLVM-Release), Dual-Syntax und gemessene Zeiten.',
    },
    toc: [
      { id: 'what-is-striv', label: { en: 'What is Striv?', de: 'Was ist Striv?' } },
      { id: 'dual-syntax', label: { en: 'Dual Syntax (Rust & Odin)', de: 'Dual-Syntax (Rust & Odin)' } },
      { id: 'benchmarks', label: { en: 'Measured Timings', de: 'Gemessene Zeiten' } },
      { id: 'first-program', label: { en: 'First Program', de: 'Erstes Programm' } },
    ],
  },
  {
    id: 'cli',
    title: { en: 'Installation & CLI', de: 'Installation & CLI' },
    group: { en: 'Getting Started', de: 'Einstieg' },
    description: {
      en: 'All real CLI commands from striv-cli: new, run, dev, test, build, sync, switch-syntax, bindgen-cs.',
      de: 'Alle echten CLI-Befehle aus striv-cli: new, run, dev, test, build, sync, switch-syntax, bindgen-cs.',
    },
    toc: [
      { id: 'project-setup', label: { en: 'striv new & striv.toml', de: 'striv new & striv.toml' } },
      { id: 'jit-commands', label: { en: 'run, dev (:4040) & test', de: 'run, dev (:4040) & test' } },
      { id: 'build-sync', label: { en: 'build, sync & bindgen-cs', de: 'build, sync & bindgen-cs' } },
      { id: 'fmt-lsp', label: { en: 'fmt, switch-syntax & lsp', de: 'fmt, switch-syntax & lsp' } },
    ],
  },
  {
    id: 'from-rust',
    title: { en: 'Guide: Coming from Rust', de: 'Guide: Umstieg von Rust' },
    group: { en: 'Migration Guides', de: 'Umsteiger-Guides' },
    description: {
      en: 'Scope RAII without lifetimes, shared/lock instead of Arc<Mutex<T>>, and Rust crate integration.',
      de: 'Scope-RAII ohne Lifetimes, shared/lock statt Arc<Mutex<T>> und Rust-Crate-Integration.',
    },
    toc: [
      { id: 'values-raii', label: { en: 'Values & Scope RAII', de: 'Werte & Scope-RAII' } },
      { id: 'concurrency-diff', label: { en: 'concurrent & shared vs. Arc<Mutex>', de: 'concurrent & shared vs. Arc<Mutex>' } },
      { id: 'crate-interop', label: { en: 'Crates via striv sync & rust fn', de: 'Crates via striv sync & rust fn' } },
    ],
  },
  {
    id: 'from-odin',
    title: { en: 'Guide: Coming from Odin', de: 'Guide: Umstieg von Odin' },
    group: { en: 'Migration Guides', de: 'Umsteiger-Guides' },
    description: {
      en: 'Odin dialect in striv.toml, :: proc, :: struct, :=, tuple swizzling, defer, and Striv extras.',
      de: 'Odin-Dialekt in striv.toml, :: proc, :: struct, :=, Tupel-Swizzling, defer und Striv-Extras.',
    },
    toc: [
      { id: 'odin-dialect', label: { en: 'Enabling Odin Dialect', de: 'Odin-Dialekt aktivieren' } },
      { id: 'syntax-mapping', label: { en: 'Side-by-Side Comparison', de: 'Syntax-Vergleich' } },
      { id: 'odin-examples', label: { en: 'Odin-Style Examples', de: 'Odin-Beispiele' } },
      { id: 'striv-extras', label: { en: 'Extras: unit, sync, rpc & Crates', de: 'Mehrwerte: unit, sync, rpc & Crates' } },
    ],
  },
  {
    id: 'basics',
    title: { en: 'Variables, Functions & Control Flow', de: 'Variablen, Funktionen & Kontrollfluss' },
    group: { en: 'Fundamentals', de: 'Grundlagen' },
    description: {
      en: 'let/mut (:=), const, tweak, bitwise ops, Result/Option, if/else, while, for, defer & Trailing Blocks.',
      de: 'let/mut (:=), const, tweak, Bit-Ops, Result/Option, if/else, while, for, defer & Trailing Blocks.',
    },
    toc: [
      { id: 'variables', label: { en: 'Variables, const & tweak', de: 'Variablen, const & tweak' } },
      { id: 'bitwise-math', label: { en: 'Bitwise Ops & Math Intrinsics', de: 'Bit-Operatoren & Mathe' } },
      { id: 'control-flow', label: { en: 'Control Flow & Result/Option', de: 'Kontrollfluss & Result/Option' } },
      { id: 'defer-blocks', label: { en: 'defer (LIFO) & Trailing Blocks', de: 'defer (LIFO) & Trailing Blocks' } },
    ],
  },
  {
    id: 'structs',
    title: { en: 'Structs, Tuples & Operators', de: 'Structs, Tupel & Operatoren' },
    group: { en: 'Fundamentals', de: 'Grundlagen' },
    description: {
      en: 'C-layout types, default fields, impl methods, operator overloading, tuples, and JSON.',
      de: 'C-Layout-Typen, Default-Felder, impl-Methoden, Operator-Overloading, Tupel und JSON.',
    },
    toc: [
      { id: 'c-layout', label: { en: 'C-Layout Types & Defaults', de: 'C-Layout-Typen & Defaults' } },
      { id: 'impl-ops', label: { en: 'impl & Operator Overloading', de: 'impl & Operator-Overloading' } },
      { id: 'tuples', label: { en: 'Tuples & Swizzling', de: 'Tupel & Swizzling' } },
      { id: 'json-io', label: { en: 'to_json() & from_json()', de: 'to_json() & from_json()' } },
    ],
  },
  {
    id: 'enums-generics',
    title: { en: 'Enums, Pattern Matching & Generics', de: 'Enums, Pattern Matching & Generics' },
    group: { en: 'Type System', de: 'Typsystem' },
    description: {
      en: 'Standard enums, payload enums (ADTs), exhaustive match, and monomorphization.',
      de: 'Normale Enums, Payload-Enums (ADTs), exhaustive match und Monomorphisierung.',
    },
    toc: [
      { id: 'enums-adt', label: { en: 'Enums & Payload ADTs', de: 'Enums & Payload-ADTs' } },
      { id: 'match', label: { en: 'Pattern Matching (match)', de: 'Pattern Matching (match)' } },
      { id: 'generics', label: { en: 'Generics & Monomorphization', de: 'Generics & Monomorphisierung' } },
    ],
  },
  {
    id: 'units',
    title: { en: 'Physical Units (unit)', de: 'Physikalische Einheiten (unit)' },
    group: { en: 'Type System', de: 'Typsystem' },
    description: {
      en: 'Base units, derived formulas, unit literals, and 0-overhead type erasure.',
      de: 'Basis-Einheiten, abgeleitete Formeln, Unit-Literale und 0-Overhead Typ-Erasure.',
    },
    toc: [
      { id: 'base-derived', label: { en: 'Base & Derived Units', de: 'Basis- & abgeleitete Einheiten' } },
      { id: 'literals-checks', label: { en: 'Literals & Dimensional Analysis', de: 'Literale & Dimensionsprüfung' } },
      { id: 'zero-overhead', label: { en: '0-ns Type Erasure', de: '0-ns Typ-Erasure' } },
    ],
  },
  {
    id: 'keywords-types',
    title: { en: 'Keywords, Types & Operators', de: 'Keywords, Typen & Operatoren' },
    group: { en: 'Type System', de: 'Typsystem' },
    description: {
      en: 'Complete reference of every Striv keyword, primitive/runtime type, and operator.',
      de: 'Vollständige Referenz aller Striv-Keywords, primitiven/Runtime-Typen und Operatoren.',
    },
    toc: [
      { id: 'all-keywords', label: { en: 'All Keywords (47)', de: 'Alle Keywords (47)' } },
      { id: 'all-types', label: { en: 'All Types & Layouts', de: 'Alle Typen & Layouts' } },
      { id: 'all-operators', label: { en: 'Operators & Precedence', de: 'Operatoren & Präzedenz' } },
    ],
  },
  {
    id: 'reference',
    title: { en: 'Reference Docs (Rustdoc)', de: 'Referenz-Docs (Rustdoc)' },
    group: { en: 'Type System', de: 'Typsystem' },
    description: {
      en: 'Interactive Rustdoc-style reference for keywords, types, and built-in modules.',
      de: 'Interaktive Referenz im Rustdoc-Stil für Keywords, Typen und eingebaute Module.',
    },
    toc: [],
  },
  {
    id: 'concurrency',
    title: { en: 'Concurrency (concurrent, shared, select)', de: 'Concurrency (concurrent, shared, select)' },
    group: { en: 'Concurrency & ECS', de: 'Concurrency & ECS' },
    description: {
      en: 'Structured threads, shared/lock, parallel for, Channel, Atomic, and select.',
      de: 'Strukturierte Threads, shared/lock, parallel for, Channel, Atomic und select.',
    },
    toc: [
      { id: 'concurrent-spawn', label: { en: 'concurrent & spawn', de: 'concurrent & spawn' } },
      { id: 'shared-lock', label: { en: 'shared & lock', de: 'shared & lock' } },
      { id: 'parallel-for', label: { en: 'parallel for (Rayon)', de: 'parallel for (Rayon)' } },
      { id: 'channels-select', label: { en: 'Channel, Atomic & select', de: 'Channel, Atomic & select' } },
    ],
  },
  {
    id: 'ecs',
    title: { en: 'Native Multi-Core ECS', de: 'Natives Multi-Core ECS' },
    group: { en: 'Concurrency & ECS', de: 'Concurrency & ECS' },
    description: {
      en: 'Ecs::new(), spawn(), set(), remove_entity(), query, and parallel query.',
      de: 'Ecs::new(), spawn(), set(), remove_entity(), query und parallel query.',
    },
    toc: [
      { id: 'ecs-model', label: { en: 'Sparse-Set Architecture', de: 'Sparse-Set Architektur' } },
      { id: 'ecs-api', label: { en: 'spawn, set & remove_entity', de: 'spawn, set & remove_entity' } },
      { id: 'ecs-queries', label: { en: 'query & parallel query', de: 'query & parallel query' } },
    ],
  },
  {
    id: 'networking',
    title: { en: 'Networking (sync & rpc)', de: 'Networking (sync & rpc)' },
    group: { en: 'Networking & Mods', de: 'Networking & Mods' },
    description: {
      en: 'sync fields with dirty bitmask, delta packing, and rpc fn / rpc_dispatch.',
      de: 'sync-Felder mit Dirty-Bitmask, Delta-Packing und rpc fn / rpc_dispatch.',
    },
    toc: [
      { id: 'sync-fields', label: { en: 'sync Fields & Delta Packing', de: 'sync-Felder & Delta-Packing' } },
      { id: 'rpc-fn', label: { en: 'rpc fn & rpc_dispatch', de: 'rpc fn & rpc_dispatch' } },
    ],
  },
  {
    id: 'modding-interop',
    title: { en: 'Modding, Sandbox & Rust/C# Interop', de: 'Modding, Sandbox & Rust/C#-Interop' },
    group: { en: 'Networking & Mods', de: 'Networking & Mods' },
    description: {
      en: 'open/extend enum, JitCompiler::new_sandboxed(), bindgen-cs, striv sync, rust fn & C-FFI.',
      de: 'open/extend enum, JitCompiler::new_sandboxed(), bindgen-cs, striv sync, rust fn & C-FFI.',
    },
    toc: [
      { id: 'open-enums', label: { en: 'open enum & extend enum', de: 'open enum & extend enum' } },
      { id: 'sandbox', label: { en: 'Sandbox (Gas & I/O Lock)', de: 'Sandbox (Gas-Limit & I/O-Sperre)' } },
      { id: 'csharp-bindgen', label: { en: 'C# Bindings (striv bindgen-cs)', de: 'C#-Bindings (striv bindgen-cs)' } },
      { id: 'rust-c-ffi', label: { en: 'Rust Interop & Direct C-FFI', de: 'Rust-Interop & direktes C-FFI' } },
    ],
  },
  {
    id: 'stdlib-testing',
    title: { en: 'Standard Library & Testing', de: 'Standardbibliothek & Testing' },
    group: { en: 'Stdlib & CLI', de: 'Stdlib & CLI' },
    description: {
      en: 'All types from collections.rs (Vec, Deque, Map, Grid, Bytes, etc.), String, fs, and test.',
      de: 'Alle Typen aus collections.rs (Vec, Deque, Map, Grid, Bytes, etc.), String, fs und test.',
    },
    toc: [
      { id: 'collections', label: { en: 'Collections & Data Structures', de: 'Collections & Datenstrukturen' } },
      { id: 'bytes-math', label: { en: 'Bytes, Rng, BitSet, Vec3 & Mat4', de: 'Bytes, Rng, BitSet, Vec3 & Mat4' } },
      { id: 'string-fs', label: { en: 'String, fs & Math Intrinsics', de: 'String, fs & Mathe-Intrinsics' } },
      { id: 'testing', label: { en: 'test "..." & assert(...)', de: 'test "..." & assert(...)' } },
    ],
  },
]

export const CHAPTER_GROUPS: { en: string; de: string }[] = [
  { en: 'Getting Started', de: 'Einstieg' },
  { en: 'Migration Guides', de: 'Umsteiger-Guides' },
  { en: 'Fundamentals', de: 'Grundlagen' },
  { en: 'Type System', de: 'Typsystem' },
  { en: 'Concurrency & ECS', de: 'Concurrency & ECS' },
  { en: 'Networking & Mods', de: 'Networking & Mods' },
  { en: 'Stdlib & CLI', de: 'Stdlib & CLI' },
]
