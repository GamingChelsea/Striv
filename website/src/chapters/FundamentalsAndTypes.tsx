import { CodeBlock } from '../components/CodeBlock'
import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocH2, DocP, DocTable, InlineCode } from './DocPrimitives'

export function BasicsChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Variables, Functions & Control Flow', 'Variablen, Funktionen & Kontrollfluss')}</DocH1>

      <DocH2 id="variables" theme={theme}>
        {tr(lang, 'Variables, const & tweak', 'Variablen, const & tweak')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Local bindings are immutable by default. Add mut to allow reassignment. const is evaluated at compile time (eval_comptime_consts), while tweak values can be edited live on port :4040 in dev mode.',
          'Lokale Variablen sind standardmäßig unveränderlich. Mit mut werden Zuweisungen erlaubt. const wird zur Kompilierzeit ausgewertet (eval_comptime_consts), während tweak-Werte im dev-Modus über Port :4040 live änderbar sind.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="vars.strv"
        code={`const MAX_SHIPS = 1024
tweak SPEED = 12

fn demo() -> int {
    let a = 0x10
    let mut b: int = 20
    b += a
    print("b = {b}")
    return b
}`}
        odinCode={`MAX_SHIPS :: 1024
tweak SPEED = 12

demo :: proc() -> int {
    a := 0x10
    b : int = 20
    b += a
    print("b = {b}")
    return b
}`}
      />

      <DocH2 id="bitwise-math" theme={theme}>
        {tr(lang, 'Bitwise Operators & Math Intrinsics', 'Bit-Operatoren & Mathe-Intrinsics')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="bits_math.strv"
        code={`fn bit_and_math() -> float {
    let flags = (1 << 4) | (1 << 2)
    let masked = flags & 4
    let toggled = flags ^ 20
    return sqrt(16.0) + cos(0.0) + masked + toggled
}`}
        odinCode={`bit_and_math :: proc() -> float {
    flags := (1 << 4) | (1 << 2)
    masked := flags & 4
    toggled := flags ^ 20
    return sqrt(16.0) + cos(0.0) + masked + toggled
}`}
      />

      <DocH2 id="control-flow" theme={theme}>
        {tr(lang, 'Control Flow & Result / Option', 'Kontrollfluss & Result / Option')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="loops_and_result.strv"
        code={`fn sum_filtered() -> int {
    let mut items = [10, 20, 30]
    let mut sum = 0
    for i in 0..10 {
        if i == 2 { continue }
        if i == 7 { break }
        sum += i
    }
    for mut x in items {
        sum += x
    }
    return sum
}`}
        odinCode={`sum_filtered :: proc() -> int {
    items := [10, 20, 30]
    sum := 0
    for i in 0..10 {
        if i == 2 { continue }
        if i == 7 { break }
        sum += i
    }
    for mut x in items {
        sum += x
    }
    return sum
}`}
      />

      <DocH2 id="defer-blocks" theme={theme}>
        {tr(lang, 'defer (LIFO) & Trailing Blocks', 'defer (LIFO) & Trailing Blocks')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'defer runs statements or blocks at function exit in reverse order (LIFO). Functions taking a block parameter can be called with trailing block syntax and yield.',
          'defer führt Anweisungen oder Blöcke am Funktionsende in umgekehrter Reihenfolge (LIFO) aus. Funktionen mit einem block-Parameter lassen sich mit yield als Trailing Block aufrufen.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="defer_and_block.strv"
        code={`fn repeat_times(n: int, body: block) {
    for _i in 0..n {
        yield
    }
}

fn run() -> int {
    defer print("2. LIFO-Ende")
    defer print("1. LIFO-Start")
    let mut acc = 0
    repeat_times(5) {
        acc += 3
    }
    return acc
}`}
        odinCode={`repeat_times :: proc(n: int, body: block) {
    for _i in 0..n {
        yield
    }
}

run :: proc() -> int {
    defer print("2. LIFO-Ende")
    defer print("1. LIFO-Start")
    acc := 0
    repeat_times(5) {
        acc += 3
    }
    return acc
}`}
      />
    </div>
  )
}

export function StructsChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Structs, Tuples & Operators', 'Structs, Tupel & Operatoren')}</DocH1>

      <DocH2 id="c-layout" theme={theme}>
        {tr(lang, 'C-Layout Types & Default Fields', 'C-Layout-Typen & Default-Felder')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Striv lays out struct fields using natural C alignment. Supported field types are int (8B), float (8B), f32 (4B), u32/i32 (4B), u8/i8/bool (1B), and String (8B pointer). Fields may declare default values.',
          'Striv legt Struct-Felder nach natürlichem C-Alignment im Speicher ab. Unterstützte Feldtypen sind int (8B), float (8B), f32 (4B), u32/i32 (4B), u8/i8/bool (1B) sowie String (8B Pointer). Felder können Default-Werte besitzen.',
        )}
      </DocP>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Type', 'Typ'), tr(lang, 'Size', 'Größe'), 'Alignment', tr(lang, 'Use Case', 'Verwendung')]}
        rows={[
          ['int / float', '8 Bytes', '8 Bytes', tr(lang, 'Standard 64-bit integer & f64', 'Standard-64-Bit Integer & f64')],
          ['f32 / u32 / i32', '4 Bytes', '4 Bytes', tr(lang, 'Compact GPU / C structs', 'Kompakte GPU-/C-Structs')],
          ['u8 / i8 / bool', '1 Byte', '1 Byte', tr(lang, 'Flags, tags & bytes', 'Flags, Tags & Bytes')],
          ['String', '8 Bytes', '8 Bytes', tr(lang, 'Heap string pointer', 'Heap-String-Zeiger')],
        ]}
      />

      <DocH2 id="impl-ops" theme={theme}>
        {tr(lang, 'impl & Operator Overloading', 'impl & Operator-Overloading')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Methods inside impl blocks support self and mut self. Operators can be overloaded via op_add, op_sub, op_mul, op_div, and op_eq.',
          'Methoden in impl-Blöcken unterstützen self und mut self. Über op_add, op_sub, op_mul, op_div und op_eq lassen sich Operatoren überladen.',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="vec2.strv"
        code={`struct Vec2 { x: float, y: float }

impl Vec2 {
    fn op_add(self, o: Vec2) -> Vec2 {
        return Vec2 { x: self.x + o.x, y: self.y + o.y }
    }
    fn op_sub(self, o: Vec2) -> Vec2 {
        return Vec2 { x: self.x - o.x, y: self.y - o.y }
    }
    fn op_mul(self, s: float) -> Vec2 {
        return Vec2 { x: self.x * s, y: self.y * s }
    }
}`}
        odinCode={`Vec2 :: struct { x: float, y: float }

impl Vec2 {
    fn op_add(self, o: Vec2) -> Vec2 {
        return Vec2 { x: self.x + o.x, y: self.y + o.y }
    }
    fn op_sub(self, o: Vec2) -> Vec2 {
        return Vec2 { x: self.x - o.x, y: self.y - o.y }
    }
    fn op_mul(self, s: float) -> Vec2 {
        return Vec2 { x: self.x * s, y: self.y * s }
    }
}`}
      />

      <DocH2 id="tuples" theme={theme}>
        {tr(lang, 'Tuples & Swizzling', 'Tupel & Swizzling')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="tuples.strv"
        code={`fn tuple_demo() -> float {
    let p = (10.0, 20.0, 30.0)
    let (mut x, y, _z) = p
    x += p.z
    return x + y
}`}
        odinCode={`tuple_demo :: proc() -> float {
    p := (10.0, 20.0, 30.0)
    let (mut x, y, _z) = p
    x += p.0
    return x + y
}`}
      />

      <DocH2 id="json-io" theme={theme}>
        to_json() &amp; from_json()
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="json_roundtrip.strv"
        code={`struct RocketPart {
    name: String,
    thrust: float = 100.0,
}

fn load_part() -> float {
    let p = RocketPart { name: "Mainsail", thrust: 1500.0 }
    let txt = p.to_json()
    let loaded = RocketPart::from_json(txt)
    return loaded.thrust
}`}
        odinCode={`RocketPart :: struct {
    name: String,
    thrust: float = 100.0,
}

load_part :: proc() -> float {
    p := RocketPart { name: "Mainsail", thrust: 1500.0 }
    txt := p.to_json()
    loaded := RocketPart::from_json(txt)
    return loaded.thrust
}`}
      />
    </div>
  )
}

export function EnumsGenericsChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Enums, Pattern Matching & Generics', 'Enums, Pattern Matching & Generics')}</DocH1>

      <DocH2 id="enums-adt" theme={theme}>
        {tr(lang, 'Enums & Payload ADTs', 'Enums & Payload-ADTs')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'Standard enums use a 64-bit integer tag. Payload enums (ADTs) like Shape occupy 24 bytes on the stack (8B tag + two 8B payload slots) and are passed by pointer.',
          'Normale Enums belegen einen 64-Bit-Integer-Tag. Payload-Enums (ADTs) wie Shape belegen 24 Bytes auf dem Stack (8B Tag + zwei 8B Payload-Slots) und werden per Zeiger übergeben.',
        )}
      </DocP>

      <DocH2 id="match" theme={theme}>
        Pattern Matching (match)
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="shape_match.strv"
        code={`enum Shape {
    Empty,
    Circle(float),
    Rect(float, float),
}

fn shape_area(sh: Shape) -> float {
    match sh {
        Shape::Empty => { return 0.0 }
        Shape::Circle(r) => { return 3.0 * r * r }
        Shape::Rect(w, h) => { return w * h }
    }
    return 0.0
}`}
        odinCode={`Shape :: enum {
    Empty,
    Circle(float),
    Rect(float, float),
}

shape_area :: proc(sh: Shape) -> float {
    match sh {
        Shape::Empty => { return 0.0 }
        Shape::Circle(r) => { return 3.0 * r * r }
        Shape::Rect(w, h) => { return w * h }
    }
    return 0.0
}`}
      />

      <DocH2 id="generics" theme={theme}>
        {tr(lang, 'Generics & Monomorphization', 'Generics & Monomorphisierung')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'monomorphize.rs specializes generic functions (clamp<T>) and structs (Pair<A, B>) into concrete symbols (clamp__G_int, clamp__G_float) before JIT codegen.',
          'monomorphize.rs spezialisiert generische Funktionen (clamp<T>) und Structs (Pair<A, B>) vor der Codegenerierung vollständig zu konkreten Symbolen (clamp__G_int, clamp__G_float).',
        )}
      </DocP>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="generics.strv"
        code={`struct Pair<A, B> {
    first: A,
    second: B,
}

fn clamp<T>(val: T, lo: T, hi: T) -> T {
    if val < lo { return lo }
    if val > hi { return hi }
    return val
}`}
        odinCode={`Pair :: struct<A, B> {
    first: A,
    second: B,
}

clamp :: proc<T>(val: T, lo: T, hi: T) -> T {
    if val < lo { return lo }
    if val > hi { return hi }
    return val
}`}
      />
    </div>
  )
}

export function UnitsChapter({ dialect, theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>{tr(lang, 'Physical Units (unit)', 'Physikalische Einheiten (unit)')}</DocH1>

      <DocH2 id="base-derived" theme={theme}>
        {tr(lang, 'Base & Derived Units', 'Basis- & abgeleitete Einheiten')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'unit declares base dimensions and derived formulas (* and /). The checker in units.rs tracks exponents at compile time.',
          'Mit unit deklarierst du Basisdimensionen und abgeleitete Formeln (* und /). Der Checker in units.rs berechnet Exponenten exakt.',
        )}
      </DocP>

      <DocH2 id="literals-checks" theme={theme}>
        {tr(lang, 'Literals & Dimensional Analysis', 'Literale & Dimensionsprüfung')}
      </DocH2>
      <CodeBlock
        dialect={dialect}
        theme={theme}
        filename="ksp_units.strv"
        code={`unit m
unit s
unit kg
unit Velocity = m / s
unit Accel = m / (s * s)
unit Force = kg * Accel

struct RocketStage {
    dry_mass: kg,
    fuel_mass: kg,
    thrust: Force,
}

fn calc_delta_v(stage: RocketStage, dt: s) -> Velocity {
    let total_mass = stage.dry_mass + stage.fuel_mass
    let acc = stage.thrust / total_mass
    return acc * dt
}`}
        odinCode={`unit m
unit s
unit kg
unit Velocity = m / s
unit Accel = m / (s * s)
unit Force = kg * Accel

RocketStage :: struct {
    dry_mass: kg,
    fuel_mass: kg,
    thrust: Force,
}

calc_delta_v :: proc(stage: RocketStage, dt: s) -> Velocity {
    total_mass := stage.dry_mass + stage.fuel_mass
    acc := stage.thrust / total_mass
    return acc * dt
}`}
      />

      <DocH2 id="zero-overhead" theme={theme}>
        {tr(lang, '0-ns Type Erasure', '0-ns Typ-Erasure')}
      </DocH2>
      <DocP theme={theme}>
        {tr(
          lang,
          'After dimensional analysis passes, all unit types are erased and compiled directly to native f64 registers in Cranelift (0 ns runtime overhead).',
          'Nach erfolgreicher Dimensionsprüfung werden alle unit-Typen verworfen und im Cranelift-JIT direkt als native f64-Register kompiliert (0 ns Laufzeit-Overhead).',
        )}
      </DocP>
      <p className="text-xs font-mono text-zinc-500">
        <InlineCode theme={theme}>units.rs</InlineCode>
      </p>
    </div>
  )
}
