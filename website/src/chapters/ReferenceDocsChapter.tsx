import { useState } from 'react'
import { CodeBlock } from '../components/CodeBlock'
import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocP } from './DocPrimitives'

interface RefItem {
  id: string
  name: string
  category: 'keyword' | 'type' | 'module'
  signature: string
  summary: { en: string; de: string }
  rustCode: string
  odinCode: string
}

const REF_ITEMS: RefItem[] = [
  {
    id: 'kw-fn-proc',
    name: 'fn / proc',
    category: 'keyword',
    signature: 'fn name(args) -> Ret  |  name :: proc(args) -> Ret',
    summary: {
      en: 'Defines a function (Rust dialect) or procedure (Odin dialect). Compiled directly to native x86_64 machine code via Cranelift JIT.',
      de: 'Definiert eine Funktion (Rust-Dialekt) oder Prozedur (Odin-Dialekt). Wird direkt per Cranelift JIT in nativen x86_64-Maschinencode übersetzt.',
    },
    rustCode: `fn add(a: int, b: int) -> int {
    return a + b
}`,
    odinCode: `add :: proc(a: int, b: int) -> int {
    return a + b
}`,
  },
  {
    id: 'kw-let-assign',
    name: 'let / := / ::',
    category: 'keyword',
    signature: 'let [mut] x = val  |  x := val  |  X :: const',
    summary: {
      en: 'In Rust mode, let is immutable by default unless mut is specified. In Odin mode, := is always mutable and :: defines a compile-time constant.',
      de: 'Im Rust-Modus ist let standardmäßig unveränderbar (außer mit mut). Im Odin-Modus ist := immer veränderbar (mut), während :: eine Compile-Time-Konstante definiert.',
    },
    rustCode: `let mut hp = 100
const MAX_HP = 100`,
    odinCode: `hp := 100
MAX_HP :: 100`,
  },
  {
    id: 'kw-foreign-extern',
    name: 'foreign / extern',
    category: 'keyword',
    signature: 'foreign "lib.dll" { ... }  |  extern "lib.dll" fn ...',
    summary: {
      en: 'Loads any C .dll or .so at runtime via libloading with zero Rust wrapper or striv sync needed. Use cstr for null-terminated C strings.',
      de: 'Lädt beliebige C-.dlls oder .so-Bibliotheken zur Laufzeit per libloading – ganz ohne Rust-Wrapper oder striv sync. Nutze cstr für null-terminierte C-Strings.',
    },
    rustCode: `extern "msvcrt.dll" fn abs(x: i32) -> i32
let v = abs(-42)`,
    odinCode: `foreign "msvcrt.dll" {
    abs :: proc(x: i32) -> i32
}
v := abs(-42)`,
  },
  {
    id: 'kw-null',
    name: 'null',
    category: 'keyword',
    signature: 'x = null  |  if x.is_valid()',
    summary: {
      en: 'Represents an uninitialized or cleared external handle (0). Forbidden on built-in heap containers (Vec, Bytes, String, Ecs) at compile time & in LSP.',
      de: 'Repräsentiert ein uninitialisiertes oder geleertes externes Handle (0). Auf eingebauten Heap-Containern (Vec, Bytes, String, Ecs) zur Compile-Zeit & im LSP verboten.',
    },
    rustCode: `let mut tex = null
if !tex.is_valid() {
    tex = 42
}`,
    odinCode: `tex: TextureHandle
if !tex.is_valid() {
    tex = 42
}`,
  },
  {
    id: 'kw-sync-rpc',
    name: 'sync / rpc',
    category: 'keyword',
    signature: 'sync field: T  |  rpc fn / rpc proc',
    summary: {
      en: 'Transport-agnostic delta networking. sync tracks field mutations in an 8-byte __dirty_mask for pack_delta()/apply_delta(); rpc generates <name>_pack() and rpc_dispatch().',
      de: 'Transportunabhängiges Delta-Networking. sync verfolgt Feldänderungen in einer 8-Byte-__dirty_mask für pack_delta()/apply_delta(); rpc generiert <name>_pack() und rpc_dispatch().',
    },
    rustCode: `struct Ship { sync x: float }
rpc fn fire(power: float) {}`,
    odinCode: `Ship :: struct { sync x: float }
fire :: rpc proc(power: float) {}`,
  },
  {
    id: 'kw-query',
    name: 'query / parallel query',
    category: 'keyword',
    signature: '[parallel] query (id: entity, [mut] c: Comp, !Tag) in ecs { ... }',
    summary: {
      en: 'Iterates over all entities matching the component pools in StrivEcs, optionally binding entity IDs (id: entity) and excluding components (!Tag).',
      de: 'Iteriert über alle Entities in StrivEcs, die die Komponenten besitzen – optional mit Entity-ID (id: entity) und Ausschlussfilter (!Tag).',
    },
    rustCode: `query (id: entity, mut orb: Orbit, !Docked) in ecs {
    orb.alt += 50.0
}`,
    odinCode: `query (id: entity, mut orb: Orbit, !Docked) in ecs {
    orb.alt += 50.0
}`,
  },
  {
    id: 'kw-use-import',
    name: 'use / import',
    category: 'keyword',
    signature: 'use physics::orbit  |  import "physics/orbit"',
    summary: {
      en: 'Imports local modules from the same folder, src/, or installed Striv packages in .striv/pkgs/<pkg>/.',
      de: 'Importiert lokale Module aus demselben Ordner, src/ oder installierte Striv-Pakete aus .striv/pkgs/<pkg>/.',
    },
    rustCode: `use physics::orbit
use kepler`,
    odinCode: `import "physics/orbit"
import "kepler"`,
  },
  {
    id: 'type-int-float',
    name: 'int / float / f32 / i32',
    category: 'type',
    signature: 'int (i64), float (f64), i32, u32, f32, u8, bool',
    summary: {
      en: 'Native machine types mapped directly to Cranelift I64/F64/I32/F32 registers with zero boxing overhead.',
      de: 'Native Maschinentypen, die ohne Boxing-Overhead direkt auf Cranelift I64/F64/I32/F32-Register abgebildet werden.',
    },
    rustCode: `let count: int = 42
let dt: float = 0.016`,
    odinCode: `count: int = 42
dt: float = 0.016`,
  },
  {
    id: 'type-cstr',
    name: 'cstr',
    category: 'type',
    signature: 'cstr (*const u8, null-terminated)',
    summary: {
      en: 'C-compatible null-terminated string pointer. Passing a Striv String to a cstr parameter automatically calls striv_str_to_cstr.',
      de: 'C-kompatibler, null-terminierter String-Pointer. Beim Übergeben eines Striv-Strings an cstr ruft der JIT automatisch striv_str_to_cstr auf.',
    },
    rustCode: `extern "msvcrt.dll" fn strlen(s: cstr) -> int
let n = strlen("Striv")`,
    odinCode: `foreign "msvcrt.dll" {
    strlen :: proc(s: cstr) -> int
}
n := strlen("Striv")`,
  },
  {
    id: 'type-bytes-vec',
    name: 'Bytes / Vec<T>',
    category: 'type',
    signature: 'Bytes::new()  |  Vec<T>',
    summary: {
      en: 'Heap-allocated binary buffer and typed dynamic array with Smart Auto-Move/Auto-Clone (no .clone() needed) and RAII cleanup.',
      de: 'Heap-allozierter Binärpuffer und typisiertes dynamisches Array mit Smart Auto-Move/Auto-Clone (kein .clone() nötig) und RAII-Cleanup.',
    },
    rustCode: `let mut buf = Bytes::new()
buf.write_u32(123)
let copy = buf // Auto-Clone or 0-ns Move`,
    odinCode: `buf := Bytes::new()
buf.write_u32(123)
copy := buf // Auto-Clone oder 0-ns-Move`,
  },
  {
    id: 'mod-ecs',
    name: 'Ecs',
    category: 'module',
    signature: 'Ecs::new(), spawn(), set(), set_parent(), detach(), parent_of(), has(), remove_comp(), remove_entity()',
    summary: {
      en: 'Cache-friendly Sparse-Set ECS with parent-child hierarchies, cascade delete on remove_entity, and single-component checks.',
      de: 'Cache-freundliches Sparse-Set-ECS mit Eltern-Kind-Hierarchien, Kaskaden-Delete in remove_entity und Einzel-Komponenten-Checks.',
    },
    rustCode: `let mut ecs = Ecs::new()
let e = ecs.spawn()
ecs.set(e, Orbit { alt: 100.0 })`,
    odinCode: `ecs := Ecs::new()
e := ecs.spawn()
ecs.set(e, Orbit { alt: 100.0 })`,
  },
  {
    id: 'mod-mods',
    name: 'Mods',
    category: 'module',
    signature: 'Mods::load(dir), count(), init(ecs), tick(ecs, dt)',
    summary: {
      en: 'Sandboxed mod loader with automatic api.strv sharing, ACTIVE_MOD_DIR path jail, 500k gas limit, and panic fault isolation.',
      de: 'Sandboxed Mod-Loader mit automatischem api.strv-Sharing, ACTIVE_MOD_DIR-Pfadschutz, 500k Gas-Limit und Panic-Fault-Isolation.',
    },
    rustCode: `let mut mods = Mods::load("mods")
mods.init(ecs)
mods.tick(ecs, 0.016)`,
    odinCode: `mods := Mods::load("mods")
mods.init(ecs)
mods.tick(ecs, 0.016)`,
  },
]

export function ReferenceDocsChapter({ dialect, theme, lang }: ChapterProps) {
  const [selectedId, setSelectedId] = useState<string>(REF_ITEMS[0].id)
  const [filter, setFilter] = useState('')
  const isDark = theme === 'dark'

  const filtered = REF_ITEMS.filter(
    (item) =>
      item.name.toLowerCase().includes(filter.toLowerCase()) ||
      item.summary[lang].toLowerCase().includes(filter.toLowerCase()),
  )

  const selected = REF_ITEMS.find((i) => i.id === selectedId) ?? REF_ITEMS[0]

  const groups: { key: RefItem['category']; label: { en: string; de: string } }[] = [
    { key: 'keyword', label: { en: 'Keywords', de: 'Keywords' } },
    { key: 'type', label: { en: 'Types', de: 'Typen' } },
    { key: 'module', label: { en: 'Built-in Modules', de: 'Eingebaute Module' } },
  ]

  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Reference Docs (Rustdoc-Style)', 'Referenz-Dokumentation (Rustdoc-Stil)')}</DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'Select any keyword, type, or built-in module on the left to inspect its signature, behavior, and Rust/Odin examples.',
          'Wähle links ein Keyword, einen Typ oder ein eingebautes Modul aus, um Signatur, Verhalten und Rust-/Odin-Beispiele anzuzeigen.',
        )}
      </DocP>

      <div
        className={`grid grid-cols-1 md:grid-cols-[240px_1fr] rounded border overflow-hidden ${
          isDark ? 'border-zinc-800 bg-[#12151a]' : 'border-zinc-200 bg-white'
        }`}
      >
        <aside
          className={`border-b md:border-b-0 md:border-r p-3 space-y-4 ${
            isDark ? 'border-zinc-800 bg-zinc-900/40' : 'border-zinc-200 bg-zinc-50/70'
          }`}
        >
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={tr(lang, 'Filter reference...', 'Referenz filtern...')}
            className={`w-full px-2.5 py-1.5 text-xs rounded border focus:outline-none ${
              isDark
                ? 'border-zinc-700 bg-zinc-900 text-zinc-100 placeholder:text-zinc-500'
                : 'border-zinc-300 bg-white text-zinc-900 placeholder:text-zinc-400'
            }`}
          />
          {groups.map((g) => {
            const items = filtered.filter((i) => i.category === g.key)
            if (items.length === 0) return null
            return (
              <div key={g.key} className="space-y-1">
                <div className="px-2 text-[10px] font-mono uppercase tracking-wider text-zinc-500">
                  {g.label[lang]}
                </div>
                {items.map((item) => {
                  const active = item.id === selected.id
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setSelectedId(item.id)}
                      className={`w-full text-left px-2.5 py-1.5 rounded text-xs font-mono transition-colors cursor-pointer ${
                        active
                          ? isDark
                            ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                            : 'bg-amber-50 text-amber-800 border border-amber-300'
                          : isDark
                            ? 'text-zinc-300 hover:bg-zinc-800'
                            : 'text-zinc-700 hover:bg-zinc-200/70'
                      }`}
                    >
                      {item.name}
                    </button>
                  )
                })}
              </div>
            )
          })}
        </aside>

        <div className="p-5 space-y-4 min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={`text-[11px] font-mono px-2 py-0.5 rounded border ${
                isDark
                  ? 'border-zinc-700 bg-zinc-800 text-amber-300'
                  : 'border-zinc-300 bg-zinc-100 text-amber-800'
              }`}
            >
              {selected.category}
            </span>
            <h2 className="text-lg font-mono font-semibold">{selected.name}</h2>
          </div>

          <div
            className={`px-3 py-2 rounded border text-xs font-mono overflow-x-auto ${
              isDark
                ? 'border-zinc-800 bg-zinc-900/70 text-zinc-200'
                : 'border-zinc-200 bg-zinc-50 text-zinc-800'
            }`}
          >
            {selected.signature}
          </div>

          <p className={`text-sm leading-relaxed ${isDark ? 'text-zinc-300' : 'text-zinc-700'}`}>
            {selected.summary[lang]}
          </p>

          <CodeBlock
            dialect={dialect}
            theme={theme}
            filename={`${selected.id}.strv`}
            code={selected.rustCode}
            odinCode={selected.odinCode}
          />
        </div>
      </div>
    </div>
  )
}
