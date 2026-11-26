import { CodeBlock } from '../components/CodeBlock'
import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocH2, DocP, DocTable, InlineCode } from './DocPrimitives'

export function ConcurrencyChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>Concurrency (concurrent, shared, select)</DocH1>

      <DocH2 id="concurrent-spawn" theme={theme}>
        concurrent &amp; spawn
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'concurrent { ... } launches all spawn calls in parallel and joins every thread before leaving the block (structured concurrency).',
          'concurrent { ... } startet alle darin enthaltenen spawn-Aufrufe parallel und wartet automatisch am Blockende, bis alle Threads beendet sind (Structured Concurrency).',
        )}
      </DocP>

      <DocH2 id="shared-lock" theme={theme}>
        shared &amp; lock
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'shared allocates a thread-safe mutex-protected box (SharedBox) on the heap. Any thread can receive a shared parameter, and lock var { ... } acquires the mutex for exclusive read/write access.',
          'shared legt eine threadsichere, mutex-geschützte Box (SharedBox) auf dem Heap an. Jeder Thread kann einen shared-Parameter entgegennehmen, und lock var { ... } sperrt den Mutex für exklusiven Lese- und Schreibzugriff.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="shared_lock.strv"
        code={`struct SharedCounter { hits: int }

fn inc_shared(shared c: SharedCounter, n: int) {
    for _i in 0..n {
        lock c { c.hits = c.hits + 1 }
    }
}

fn run_workers() -> int {
    shared counter = SharedCounter { hits: 0 }
    concurrent {
        spawn inc_shared(counter, 1000)
        spawn inc_shared(counter, 1000)
    }
    let mut out = 0
    lock counter { out = counter.hits }
    return out
}`}
        odinCode={`SharedCounter :: struct { hits: int }

inc_shared :: proc(shared c: SharedCounter, n: int) {
    for _i in 0..n {
        lock c { c.hits = c.hits + 1 }
    }
}

run_workers :: proc() -> int {
    shared counter = SharedCounter { hits: 0 }
    concurrent {
        spawn inc_shared(counter, 1000)
        spawn inc_shared(counter, 1000)
    }
    out := 0
    lock counter { out = counter.hits }
    return out
}`}
      />

      <DocH2 id="parallel-for" theme={theme}>
        parallel for (Rayon)
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="parallel_for.strv"
        code={`struct Particle { x: float, vx: float }

fn step_particles(dt: float) {
    let mut parts: Vec<Particle> = []
    parts.push(Particle { x: 0.0, vx: 2.0 })
    parallel for mut p in parts {
        p.x = p.x + p.vx * dt
    }
}`}
        odinCode={`Particle :: struct { x: float, vx: float }

step_particles :: proc(dt: float) {
    parts : Vec<Particle> = []
    parts.push(Particle { x: 0.0, vx: 2.0 })
    parallel for mut p in parts {
        p.x = p.x + p.vx * dt
    }
}`}
      />

      <DocH2 id="channels-select" theme={theme}>
        Channel, Atomic &amp; select
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="select_timeout.strv"
        code={`fn wait_msg() -> int {
    let ch1 = Channel::new()
    let ch2 = Channel::new()
    select {
        msg = ch1.recv() => { return msg }
        msg = ch2.recv() => { return msg }
        timeout(50) => { return -1 }
    }
    return 0
}`}
        odinCode={`wait_msg :: proc() -> int {
    ch1 := Channel::new()
    ch2 := Channel::new()
    select {
        msg = ch1.recv() => { return msg }
        msg = ch2.recv() => { return msg }
        timeout(50) => { return -1 }
    }
    return 0
}`}
      />
    </div>
  )
}

export function EcsChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Native Multi-Core ECS', 'Natives Multi-Core ECS')}</DocH1>

      <DocH2 id="ecs-model" theme={theme}>
        {tr(lang, 'Sparse-Set Architecture', 'Sparse-Set Architektur')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'StrivEcs (ecs.rs) stores each component in a dense byte pool (ComponentPool) with an index_of map. Queries automatically pick the smallest pool as driver and use a single flat pointer buffer with zero per-entity heap allocations.',
          'StrivEcs (ecs.rs) speichert jede Komponente in einem eigenen dichten Byte-Pool (ComponentPool) mit index_of-Mapping. Bei einer Query wählt die Runtime automatisch den kleinsten Pool als Treiber und nutzt einen einzigen flachen Pointer-Puffer ohne Pro-Entity-Allokationen.',
        )}
      </DocP>

      <DocH2 id="ecs-api" theme={theme}>
        spawn, set &amp; remove_entity
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'world.spawn() allocates a new entity ID, world.set(e, Comp { ... }) upserts the component bytes, and world.remove_entity(e) removes all components in O(1) via swap-remove.',
          'world.spawn() vergibt eine neue Entity-ID, world.set(e, Comp { ... }) schreibt die Komponente per Typ-Hash in den Pool und world.remove_entity(e) entfernt alle Komponenten per O(1) Swap-Remove.',
        )}
      </DocP>

      <DocH2 id="ecs-queries" theme={theme}>
        query &amp; parallel query
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="ecs_world.strv"
        code={`struct Position { x: float, y: float }
struct Velocity { vx: float, vy: float }

fn step(dt: float) {
    let mut world = Ecs::new()
    let e = world.spawn()
    world.set(e, Position { x: 0.0, y: 0.0 })
    world.set(e, Velocity { vx: 10.0, vy: 0.0 })

    parallel query (mut pos: Position, vel: Velocity) in world {
        pos.x = pos.x + vel.vx * dt
    }

    query (p: Position) in world {
        print("x = {p.x}")
    }
}`}
        odinCode={`Position :: struct { x: float, y: float }
Velocity :: struct { vx: float, vy: float }

step :: proc(dt: float) {
    world := Ecs::new()
    e := world.spawn()
    world.set(e, Position { x: 0.0, y: 0.0 })
    world.set(e, Velocity { vx: 10.0, vy: 0.0 })

    parallel query (mut pos: Position, vel: Velocity) in world {
        pos.x = pos.x + vel.vx * dt
    }

    query (p: Position) in world {
        print("x = {p.x}")
    }
}`}
      />

      <DocH2 id="ecs-2-0" theme={theme}>
        {tr(lang, 'Space-ECS 2.0: Entity IDs, Hierarchies & !Without', 'Space-ECS 2.0: Entity-IDs, Hierarchien & !Without')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Bind entity IDs directly in queries via id: entity, exclude components with !Tag, inspect or strip single components with ecs.has(id, Comp) / ecs.remove_comp(id, Comp), and build parent-child hierarchies with set_parent, detach, parent_of, and automatic cascade delete in remove_entity.',
          'Binde Entity-IDs direkt in Queries per id: entity, schließe Komponenten per !Tag aus, prüfe oder entferne Einzel-Komponenten mit ecs.has(id, Comp) / ecs.remove_comp(id, Comp) und baue Hierarchien mit set_parent, detach, parent_of sowie automatischem Kaskaden-Delete in remove_entity.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="ecs_staging.strv"
        code={`struct Orbit { alt: float = 0.0 }
struct Fuel { kg: float = 100.0 }
struct Docked { bay: int = 1 }

fn stage_step() {
    let mut ecs = Ecs::new()
    let pod = ecs.spawn()
    let booster = ecs.spawn()
    ecs.set_parent(booster, pod)
    ecs.set(booster, Orbit { alt: 100.0 })
    ecs.set(booster, Fuel { kg: 0.0 })

    query (id: entity, mut orb: Orbit, !Docked) in ecs {
        orb.alt += 50.0
        if ecs.has(id, Fuel) == 1 {
            ecs.detach(id)
        }
    }
    ecs.remove_entity(booster) // cascades to children
}`}
        odinCode={`Orbit  :: struct { alt: float = 0.0 }
Fuel   :: struct { kg: float = 100.0 }
Docked :: struct { bay: int = 1 }

stage_step :: proc() {
    ecs := Ecs::new()
    pod := ecs.spawn()
    booster := ecs.spawn()
    ecs.set_parent(booster, pod)
    ecs.set(booster, Orbit { alt: 100.0 })
    ecs.set(booster, Fuel { kg: 0.0 })

    query (id: entity, mut orb: Orbit, !Docked) in ecs {
        orb.alt += 50.0
        if ecs.has(id, Fuel) == 1 {
            ecs.detach(id)
        }
    }
    ecs.remove_entity(booster) // löscht Kinder per Kaskade mit
}`}
      />
    </div>
  )
}

export function NetworkingChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>Networking (sync &amp; rpc)</DocH1>

      <DocH2 id="sync-fields" theme={theme}>
        {tr(lang, 'sync Fields & Delta Packing', 'sync-Felder & Delta-Packing')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Structs with sync fields receive an 8-byte __dirty_mask header at offset 0. Only actual value changes set the dirty bit. Use .is_dirty(), .clear_dirty(), .pack_delta(), .pack_full(), and .apply_delta(pkt).',
          'Structs mit sync-Feldern erhalten einen 8-Byte-__dirty_mask-Header an Offset 0. Nur echte Wertänderungen setzen das jeweilige Bit. Über .is_dirty(), .clear_dirty(), .pack_delta(), .pack_full() und .apply_delta(pkt) werden Zustände kompakt synchronisiert.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="delta_sync.strv"
        code={`struct SyncShip {
    sync x: float,
    sync shield: f32,
    local_bob: f32,
}

fn sync_step(mut c: SyncShip, mut s: SyncShip) {
    c.clear_dirty()
    c.x = 555.25
    let delta = c.pack_delta()
    s.apply_delta(delta)
}`}
        odinCode={`SyncShip :: struct {
    sync x: float,
    sync shield: f32,
    local_bob: f32,
}

sync_step :: proc(mut c: SyncShip, mut s: SyncShip) {
    c.clear_dirty()
    c.x = 555.25
    delta := c.pack_delta()
    s.apply_delta(delta)
}`}
      />

      <DocH2 id="sync-transport" theme={theme}>
        {tr(lang, 'Language vs. Your Engine (Transport-Agnostic)', 'Sprache vs. Deine Engine (kein Server-Zwang)')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Striv has no built-in host or server: sync only packs and unpacks changed bytes, while your game code decides whether to send them over UDP, QUIC, Steamworks P2P, or write pack_full() to disk as a savegame.',
          'Striv erzwingt keinen eingebauten Host oder Server: sync übernimmt nur das automatische Ein- und Auspacken geänderter Bytes, während dein Spiel-Code entscheidet, ob die Pakete über UDP, QUIC, Steamworks-P2P verschickt oder mit pack_full() als Savegame gespeichert werden.',
        )}
      </DocP>
      <DocTable
        theme={theme}
        headers={[
          tr(lang, 'Task', 'Aufgabe'),
          tr(lang, 'Striv (sync)', 'Macht Striv automatisch'),
          tr(lang, 'Your Game Code', 'Dein Spiel-Code (Host / Client)'),
        ]}
        rows={[
          [
            tr(lang, 'Track changes', 'Änderungen merken'),
            tr(lang, 'Sets dirty bit on ship.x += 10.0', 'Setzt unsichtbar das Dirty-Bit bei Wertänderung'),
            tr(lang, 'Nothing — normal assignment', 'Nichts — normale Zuweisung'),
          ],
          [
            tr(lang, 'Pack bytes', 'Paket schnüren'),
            tr(lang, 'ship.pack_delta() -> Bytes', 'ship.pack_delta() liefert fertiges Bytes-Paket'),
            tr(lang, 'Nothing — no manual byte counting', 'Nichts — kein manuelles Byte-Zählen'),
          ],
          [
            tr(lang, 'Send & receive', 'Senden & Empfangen'),
            tr(lang, 'Transport-agnostic', 'Mischt sich nicht ein (kein Engine-Zwang)'),
            tr(lang, 'Host sends via UDP/QUIC/P2P to clients', 'Host sendet Bytes per UDP/QUIC/P2P an Clients'),
          ],
          [
            tr(lang, 'Apply state', 'Werte anwenden'),
            tr(lang, 'ship.apply_delta(pkt)', 'ship.apply_delta(pkt) trägt neue Werte ein'),
            tr(lang, 'Client calls apply_delta on receive', 'Client ruft apply_delta bei Paketempfang auf'),
          ],
        ]}
      />

      <DocH2 id="rpc-fn" theme={theme}>
        rpc fn &amp; rpc_dispatch
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'For every rpc fn (or rpc proc), the compiler generates <name>_pack(...) -> Bytes (4-byte FNV-1a ID + 8-byte args) and a shared rpc_dispatch(mut ctx, pkt) dispatcher with length and ID validation.',
          'Für jede rpc fn (bzw. rpc proc) erzeugt der Compiler automatisch <name>_pack(...) (4-Byte FNV-1a Hash + 8-Byte Argumente) sowie einen gemeinsamen rpc_dispatch(mut ctx, pkt)-Verteiler mit Längen- und ID-Prüfung.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="rpc_example.strv"
        code={`struct RpcWorld { sync hull: int }

rpc fn repair_hull(mut w: RpcWorld, amount: int) {
    w.hull = w.hull + amount
}

fn net_demo(mut w: RpcWorld) -> int {
    let pkt = repair_hull_pack(25)
    return rpc_dispatch(w, pkt)
}`}
        odinCode={`RpcWorld :: struct { sync hull: int }

repair_hull :: rpc proc(mut w: RpcWorld, amount: int) {
    w.hull = w.hull + amount
}

net_demo :: proc(mut w: RpcWorld) -> int {
    pkt := repair_hull_pack(25)
    return rpc_dispatch(w, pkt)
}`}
      />
      <p className="text-xs font-mono text-zinc-500">
        <InlineCode theme={theme}>expand_rpc_functions</InlineCode>
      </p>
    </div>
  )
}
