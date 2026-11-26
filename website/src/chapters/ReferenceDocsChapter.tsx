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
    id: 'kw-declare',
    name: 'x: Type (Zero-Init)',
    category: 'keyword',
    signature: 'x: Type  (Odin)  |  let mut x: int = null  (Rust)',
    summary: {
      en: 'Pure type declarations without = are initialized by the JIT: primitives to 0/0.0, structs with default fields, containers via ::new(), and external handles to 0 (null).',
      de: 'Reine Typ-Deklarationen ohne = werden vom JIT typgesteuert initialisiert: Primitive auf 0/0.0, Structs inkl. Default-Feldern, Container per ::new() und externe Handles auf 0 (null).',
    },
    rustCode: `let mut tex = null
if !tex.is_valid() { tex = 42 }`,
    odinCode: `ship: Ship
items: Vec
tex: TextureHandle`,
  },
  {
    id: 'kw-struct',
    name: 'struct',
    category: 'keyword',
    signature: 'struct Name { field: T = default }  |  Name :: struct { ... }',
    summary: {
      en: 'Value-type data structure stored on the stack with optional default field values and sync field modifiers.',
      de: 'Werttyp-Datenstruktur auf dem Stack mit optionalen Default-Feldwerten und sync-Feldmodifikatoren.',
    },
    rustCode: `struct Ship {
    x: float = 0.0,
    hp: int = 100,
}`,
    odinCode: `Ship :: struct {
    x: float = 0.0,
    hp: int = 100,
}`,
  },
  {
    id: 'kw-enum',
    name: 'enum / open enum / extend enum',
    category: 'keyword',
    signature: 'open enum Name { A, B }  |  extend enum Name { C }',
    summary: {
      en: 'Supports algebraic payload enums and modding-safe open enums where every variant receives a stable 64-bit FNV-1a hash.',
      de: 'Unterstützt algebraische Payload-Enums sowie modding-sichere open enums, bei denen jede Variante einen stabilen 64-Bit FNV-1a-Hash erhält.',
    },
    rustCode: `open enum Resource { LiquidFuel, Oxidizer }
extend enum Resource { Uranium }`,
    odinCode: `Resource :: open enum { LiquidFuel, Oxidizer }
Resource :: extend enum { Uranium }`,
  },
  {
    id: 'kw-defer',
    name: 'defer',
    category: 'keyword',
    signature: 'defer <stmt>',
    summary: {
      en: 'Schedules a statement to execute in LIFO order when the current function or scope exits.',
      de: 'Führt ein Statement beim Verlassen der aktuellen Funktion bzw. des Scopes in LIFO-Reihenfolge aus.',
    },
    rustCode: `fn work() {
    defer print("Cleanup!")
    print("Working...")
}`,
    odinCode: `work :: proc() {
    defer print("Cleanup!")
    print("Working...")
}`,
  },
  {
    id: 'kw-unit',
    name: 'unit',
    category: 'keyword',
    signature: 'unit m  |  unit Velocity = m / s',
    summary: {
      en: 'Compile-time physical unit & dimensional analysis. Erased to native f64 registers with 0 ns runtime overhead.',
      de: 'Compile-Time-Dimensionsprüfung für physikalische Einheiten. Wird nach der Prüfung komplett zu nativen f64-Registern gelöscht (0 ns Overhead).',
    },
    rustCode: `unit m
unit s
unit Velocity = m / s
let v: Velocity = 100.0_m / 2.0_s`,
    odinCode: `unit m
unit s
unit Velocity = m / s
v : Velocity = 100.0_m / 2.0_s`,
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
    id: 'kw-concurrent-shared',
    name: 'concurrent / spawn / shared / lock',
    category: 'keyword',
    signature: 'concurrent { spawn f(x) }  |  shared x = ...  |  lock x { ... }',
    summary: {
      en: 'Structured concurrency: concurrent joins all spawned threads at block end; shared allocates a mutex-protected box accessed safely inside lock blocks.',
      de: 'Structured Concurrency: concurrent wartet am Blockende auf alle spawn-Threads; shared legt eine mutex-geschützte Box an, die über lock sicher bearbeitet wird.',
    },
    rustCode: `shared c = Counter { hits: 0 }
concurrent { spawn worker(c) }
lock c { print("{c.hits}") }`,
    odinCode: `shared c = Counter { hits: 0 }
concurrent { spawn worker(c) }
lock c { print("{c.hits}") }`,
  },
  {
    id: 'kw-query',
    name: 'query / parallel query',
    category: 'keyword',
    signature: '[parallel] query (id: entity, [mut] c: Comp, !Tag) in ecs { ... }',
    summary: {
      en: 'Iterates over all entities matching the component pools in StrivEcs 4.0 (Hybrid Archetype + Sparse-Set), optionally binding entity IDs (id: entity) and excluding components (!Tag).',
      de: 'Iteriert über alle Entities in StrivEcs 4.0 (Hybrid Archetype + Sparse-Set), die die Komponenten besitzen – optional mit Entity-ID (id: entity) und Ausschlussfilter (!Tag).',
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
    name: 'Bytes / Vec<T> / String',
    category: 'type',
    signature: 'Bytes::new()  |  Vec<T>  |  String',
    summary: {
      en: 'Heap-allocated containers with Smart Auto-Move/Auto-Clone (no .clone() needed), Drop-on-Reassign, and RAII cleanup.',
      de: 'Heap-allozierte Container mit Smart Auto-Move/Auto-Clone (kein .clone() nötig), Drop-on-Reassign und RAII-Cleanup.',
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
    signature: 'Ecs::new(), spawn(), set(), mark_sparse(Comp), add_rel(e, Rel, target), remove_rel(), has_rel(), rel_target(), was_added(), was_removed(), clear_events(), archetype_count(), entity_count(), set_parent(), detach(), parent_of(), has(), remove_comp(), set_group(), group_of(), in_same_group(), is_changed(), clear_changed(), remove_entity()',
    summary: {
      en: 'Hybrid Archetype + Sparse-Set ECS (4.0) with query caching, typed entity relationships (add_rel / rel_target), lifecycle tracking (was_added / was_removed), parent-child hierarchies, entity groups, and per-component change detection.',
      de: 'Hybrides Archetype + Sparse-Set-ECS (4.0) mit Query-Caching, typisierten Entity-Relationships (add_rel / rel_target), Lifecycle-Events (was_added / was_removed), Eltern-Kind-Hierarchien, Entity-Gruppen und Komponenten-Change-Detection.',
    },
    rustCode: `let mut ecs = Ecs::new()
ecs.mark_sparse(EngineActive)
let ship = ecs.spawn()
let planet = ecs.spawn()
ecs.set(ship, Orbit { alt: 100.0 })
ecs.add_rel(ship, Orbiting, planet)`,
    odinCode: `ecs := Ecs::new()
ecs.mark_sparse(EngineActive)
ship := ecs.spawn()
planet := ecs.spawn()
ecs.set(ship, Orbit { alt: 100.0 })
ecs.add_rel(ship, Orbiting, planet)`,
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
  {
    id: 'mod-window',
    name: 'Window',
    category: 'module',
    signature: 'Window::new(title, w, h), is_open(), clear(rgb), draw_rect(x, y, w, h, rgb), draw_circle(cx, cy, r, rgb), draw_line(x0, y0, x1, y1, rgb), draw_text(x, y, text, rgb, scale), is_key_down(code), mouse_x(), mouse_y(), mouse_down(btn), mouse_scroll(), width(), height(), present()',
    summary: {
      en: 'Built-in 2D framebuffer window (powered by minifb) for instant prototyping, orbit maps, and debug views. For full 3D GPU games, use foreign "raylib.dll" / "SDL3.dll" or striv add wgpu.',
      de: 'Eingebautes 2D-Framebuffer-Fenster (via minifb) für sofortiges Prototyping, Orbit-Karten und Debug-Ansichten. Für aufwendige 3D-GPU-Spiele nutze foreign "raylib.dll" / "SDL3.dll" oder striv add wgpu.',
    },
    rustCode: `let win = Window::new("Space Engine", 1280, 720)
while win.is_open() {
    win.clear(0x0B0E14)
    win.draw_rect(100, 100, 40, 40, 0xFFAA00)
    win.present()
}`,
    odinCode: `win := Window::new("Space Engine", 1280, 720)
while win.is_open() {
    win.clear(0x0B0E14)
    win.draw_rect(100, 100, 40, 40, 0xFFAA00)
    win.present()
}`,
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
        className={`grid grid-cols-1 md:grid-cols-[270px_1fr] min-h-[540px] rounded border overflow-hidden ${
          isDark ? 'border-zinc-800 bg-[#12151a]' : 'border-zinc-200 bg-white'
        }`}
      >
        <aside
          className={`border-b md:border-b-0 md:border-r p-4 space-y-5 ${
            isDark ? 'border-zinc-800 bg-zinc-900/40' : 'border-zinc-200 bg-zinc-50/70'
          }`}
        >
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={tr(lang, 'Filter reference...', 'Referenz filtern...')}
            className={`w-full px-3 py-2 text-xs rounded border focus:outline-none ${
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
                <div className="px-2 text-[11px] font-mono uppercase tracking-wider text-zinc-500">
                  {g.label[lang]}
                </div>
                {items.map((item) => {
                  const active = item.id === selected.id
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setSelectedId(item.id)}
                      className={`w-full text-left px-3 py-2 rounded text-xs font-mono transition-colors cursor-pointer ${
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

        <div className="p-6 space-y-5 min-w-0">
          <div className="flex flex-wrap items-center gap-2.5">
            <span
              className={`text-[11px] font-mono px-2 py-0.5 rounded border ${
                isDark
                  ? 'border-zinc-700 bg-zinc-800 text-amber-300'
                  : 'border-zinc-300 bg-zinc-100 text-amber-800'
              }`}
            >
              {selected.category}
            </span>
            <h2 className="text-xl font-mono font-semibold">{selected.name}</h2>
          </div>

          <div
            className={`px-3.5 py-2.5 rounded border text-xs font-mono overflow-x-auto ${
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
