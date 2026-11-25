import { ChapterId } from '../types/docs'

export interface SearchEntry {
  id: string
  title: { en: string; de: string }
  chapter: ChapterId
  chapterLabel: { en: string; de: string }
  sectionId: string
  summary: { en: string; de: string }
  keywords: string[]
}

export const SEARCH_INDEX: SearchEntry[] = [
  {
    id: 'overview',
    title: { en: 'Overview: Cranelift JIT & Dual Syntax', de: 'Start / Übersicht: Cranelift-JIT & Dual-Syntax' },
    chapter: 'overview',
    chapterLabel: { en: 'Overview', de: 'Übersicht' },
    sectionId: 'what-is-striv',
    summary: {
      en: '2-ms Cranelift JIT for dev, Rust/LLVM transpiler for release, measured timings.',
      de: '2-ms Cranelift-JIT für Dev, Rust/LLVM-Transpiler für Release, gemessene Zeiten.',
    },
    keywords: ['striv', 'jit', 'cranelift', 'llvm', 'odin', 'rust', 'benchmark'],
  },
  {
    id: 'cli',
    title: { en: 'Installation & CLI (new, run, dev, test, build, sync)', de: 'Installation & CLI (new, run, dev, test, build, sync)' },
    chapter: 'cli',
    chapterLabel: { en: 'CLI', de: 'CLI' },
    sectionId: 'project-setup',
    summary: {
      en: 'All real CLI commands from striv-cli including dev server on port :4040.',
      de: 'Alle echten CLI-Befehle aus striv-cli inklusive dev-Server auf Port :4040.',
    },
    keywords: ['new', 'run', 'dev', 'live', 'test', 'build', 'sync', 'switch-syntax', 'bindgen-cs', 'fmt', 'lsp', '4040'],
  },
  {
    id: 'from-rust',
    title: { en: 'Guide: Coming from Rust', de: 'Guide: Umstieg von Rust' },
    chapter: 'from-rust',
    chapterLabel: { en: 'From Rust', de: 'Von Rust' },
    sectionId: 'values-raii',
    summary: {
      en: 'Scope RAII without lifetimes, shared/lock instead of Arc<Mutex<T>>, and striv sync.',
      de: 'Scope-RAII ohne Lifetimes, shared/lock statt Arc<Mutex<T>> und striv sync.',
    },
    keywords: ['rust', 'lifetimes', 'arc', 'mutex', 'shared', 'lock', 'sync', 'rust fn'],
  },
  {
    id: 'from-odin',
    title: { en: 'Guide: Coming from Odin', de: 'Guide: Umstieg von Odin' },
    chapter: 'from-odin',
    chapterLabel: { en: 'From Odin', de: 'Von Odin' },
    sectionId: 'odin-dialect',
    summary: {
      en: 'syntax = "odin", :: proc, :: struct, :=, swizzling .x/.y/.z/.w, defer, unit, sync/rpc.',
      de: 'syntax = "odin", :: proc, :: struct, :=, Swizzling .x/.y/.z/.w, defer, unit, sync/rpc.',
    },
    keywords: ['odin', 'proc', 'struct', ':=', 'swizzle', 'defer', 'foreign'],
  },
  {
    id: 'basics',
    title: { en: 'Variables, Functions & Control Flow', de: 'Variablen, Funktionen & Kontrollfluss' },
    chapter: 'basics',
    chapterLabel: { en: 'Basics', de: 'Grundlagen' },
    sectionId: 'variables',
    summary: {
      en: 'let/mut, const, tweak, bitwise ops, Result/Option, if/else, while, for, defer, block/yield.',
      de: 'let/mut, const, tweak, Bit-Ops, Result/Option, if/else, while, for, defer, block/yield.',
    },
    keywords: ['let', 'mut', 'const', 'tweak', 'if', 'while', 'for', 'defer', 'block', 'yield', 'Result', 'Option'],
  },
  {
    id: 'structs',
    title: { en: 'Structs, Tuples & Operators', de: 'Structs, Tupel & Operatoren' },
    chapter: 'structs',
    chapterLabel: { en: 'Structs', de: 'Structs' },
    sectionId: 'c-layout',
    summary: {
      en: 'C-layout types, default fields, impl, op_add..op_eq, tuple swizzling, to_json/from_json.',
      de: 'C-Layout-Typen, Default-Felder, impl, op_add..op_eq, Tupel-Swizzling, to_json/from_json.',
    },
    keywords: ['struct', 'impl', 'op_add', 'op_sub', 'op_mul', 'op_div', 'op_eq', 'tuple', 'to_json', 'from_json'],
  },
  {
    id: 'enums-generics',
    title: { en: 'Enums, Pattern Matching & Generics', de: 'Enums, Pattern Matching & Generics' },
    chapter: 'enums-generics',
    chapterLabel: { en: 'Enums & Generics', de: 'Enums & Generics' },
    sectionId: 'enums-adt',
    summary: {
      en: 'Standard enums, payload ADTs, exhaustive match, and monomorphization.',
      de: 'Normale Enums, Payload-ADTs, exhaustive match und Monomorphisierung.',
    },
    keywords: ['enum', 'match', 'generics', 'monomorphize', 'adt', 'payload'],
  },
  {
    id: 'units',
    title: { en: 'Physical Units (unit)', de: 'Physikalische Einheiten (unit)' },
    chapter: 'units',
    chapterLabel: { en: 'Units', de: 'Einheiten' },
    sectionId: 'base-derived',
    summary: {
      en: 'unit m, unit Velocity = m / s, literals 500.0_m, and 0-ns type erasure.',
      de: 'unit m, unit Velocity = m / s, Literale 500.0_m und 0-ns Typ-Erasure.',
    },
    keywords: ['unit', 'dimensions', 'kg', 'm', 's', 'velocity', 'force', 'energy'],
  },
  {
    id: 'keywords-types',
    title: { en: 'Complete Keywords, Types & Operators Reference', de: 'Alle Keywords, Typen & Operatoren' },
    chapter: 'keywords-types',
    chapterLabel: { en: 'Keywords & Types', de: 'Keywords & Typen' },
    sectionId: 'all-keywords',
    summary: {
      en: 'Every Striv keyword, primitive/runtime type, byte layout, and operator precedence.',
      de: 'Jedes Striv-Keyword, jeder primitive/Runtime-Typ, Byte-Layout und Operator-Präzedenz.',
    },
    keywords: ['keywords', 'types', 'operators', 'precedence', 'tokens', 'int', 'float', 'f32', 'u32', 'cstr'],
  },
  {
    id: 'concurrency',
    title: { en: 'Concurrency (concurrent, shared, select)', de: 'Concurrency (concurrent, shared, select)' },
    chapter: 'concurrency',
    chapterLabel: { en: 'Concurrency', de: 'Concurrency' },
    sectionId: 'concurrent-spawn',
    summary: {
      en: 'concurrent { spawn ... }, shared/lock, parallel for, Channel, Atomic, select timeout.',
      de: 'concurrent { spawn ... }, shared/lock, parallel for, Channel, Atomic, select timeout.',
    },
    keywords: ['concurrent', 'spawn', 'shared', 'lock', 'parallel', 'channel', 'atomic', 'select', 'timeout'],
  },
  {
    id: 'ecs',
    title: { en: 'Native Multi-Core ECS', de: 'Natives Multi-Core ECS' },
    chapter: 'ecs',
    chapterLabel: { en: 'ECS', de: 'ECS' },
    sectionId: 'ecs-model',
    summary: {
      en: 'Ecs::new(), spawn(), set(), remove_entity(), query, and parallel query.',
      de: 'Ecs::new(), spawn(), set(), remove_entity(), query und parallel query.',
    },
    keywords: ['ecs', 'spawn', 'set', 'remove_entity', 'query', 'parallel query', 'sparse-set'],
  },
  {
    id: 'networking',
    title: { en: 'Networking (sync & rpc)', de: 'Networking (sync & rpc)' },
    chapter: 'networking',
    chapterLabel: { en: 'Networking', de: 'Networking' },
    sectionId: 'sync-fields',
    summary: {
      en: 'sync fields (pack_delta, pack_full, apply_delta) and rpc fn (pack + rpc_dispatch).',
      de: 'sync-Felder (pack_delta, pack_full, apply_delta) und rpc fn (pack + rpc_dispatch).',
    },
    keywords: ['sync', 'is_dirty', 'clear_dirty', 'pack_delta', 'pack_full', 'apply_delta', 'rpc', 'rpc_dispatch'],
  },
  {
    id: 'modding-interop',
    title: { en: 'Modding, Sandbox & Rust/C# Interop', de: 'Modding, Sandbox & Rust/C#-Interop' },
    chapter: 'modding-interop',
    chapterLabel: { en: 'Modding & Interop', de: 'Modding & Interop' },
    sectionId: 'open-enums',
    summary: {
      en: 'open/extend enum, new_sandboxed() gas limit, bindgen-cs, striv sync, rust fn, extern/foreign.',
      de: 'open/extend enum, new_sandboxed() Gas-Limit, bindgen-cs, striv sync, rust fn, extern/foreign.',
    },
    keywords: ['open enum', 'extend enum', 'sandbox', 'gas', 'bindgen-cs', 'striv sync', 'rust fn', 'extern', 'foreign', 'cstr'],
  },
  {
    id: 'stdlib-testing',
    title: { en: 'Standard Library & Testing', de: 'Standardbibliothek & Testing' },
    chapter: 'stdlib-testing',
    chapterLabel: { en: 'Stdlib & Testing', de: 'Stdlib & Testing' },
    sectionId: 'collections',
    summary: {
      en: 'Vec, Deque, Map, StrMap, BTree, Set, Heap, Grid, Bytes, Rng, BitSet, Vec3, Mat4, String, fs, test.',
      de: 'Vec, Deque, Map, StrMap, BTree, Set, Heap, Grid, Bytes, Rng, BitSet, Vec3, Mat4, String, fs, test.',
    },
    keywords: ['vec', 'deque', 'map', 'strmap', 'btree', 'set', 'heap', 'grid', 'bytes', 'rng', 'bitset', 'vec3', 'mat4', 'fs', 'test', 'assert'],
  },
]
