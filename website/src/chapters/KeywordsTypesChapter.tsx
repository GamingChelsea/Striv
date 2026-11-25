import { tr } from '../types/docs'
import { ChapterProps, DocH1, DocH2, DocP, DocTable, InlineCode } from './DocPrimitives'

export function KeywordsTypesChapter({ theme, lang }: ChapterProps) {
  return (
    <div className="space-y-6">
      <DocH1>
        {tr(
          lang,
          'Keywords, Types & Operators Reference',
          'Keywords, Typen & Operatoren (Vollreferenz)',
        )}
      </DocH1>
      <DocP theme={theme}>
        {tr(
          lang,
          'Every keyword, primitive/runtime type, and operator below is extracted directly from crates/striv-syntax/src/lib.rs and crates/lang-runtime.',
          'Jedes Keyword, jeder primitive/Runtime-Typ und jeder Operator unten stammt direkt aus crates/striv-syntax/src/lib.rs und crates/lang-runtime.',
        )}
      </DocP>

      <DocH2 id="all-keywords" theme={theme}>
        {tr(lang, 'All Keywords (47)', 'Alle Keywords (47)')}
      </DocH2>
      <DocTable
        theme={theme}
        headers={[
          tr(lang, 'Category', 'Kategorie'),
          tr(lang, 'Keywords', 'Keywords'),
          tr(lang, 'Purpose', 'Bedeutung'),
        ]}
        rows={[
          [
            tr(lang, 'Declarations', 'Deklarationen'),
            'fn, proc, struct, impl, enum, open, extend, const, tweak, unit, use, test',
            tr(
              lang,
              'Functions/procs, C-structs, methods, enums, comptime const, live tweaks (:4040), units, imports, tests.',
              'Funktionen/Procs, C-Structs, Methoden, Enums, Comptime-const, Live-Tweaks (:4040), Units, Imports, Tests.',
            ),
          ],
          [
            tr(lang, 'Variables', 'Variablen'),
            'let, mut, self, _',
            tr(
              lang,
              'Immutable/mutable bindings (:= in Odin), method receiver, wildcard pattern.',
              'Unveränderliche/veränderliche Bindungen (:= in Odin), Methoden-Receiver, Wildcard.',
            ),
          ],
          [
            tr(lang, 'Control Flow', 'Kontrollfluss'),
            'if, else, while, for, in, match, return, break, continue, defer, block, yield',
            tr(
              lang,
              'Conditionals, loops, exhaustive match, LIFO defer, trailing blocks.',
              'Verzweigungen, Schleifen, exhaustive match, LIFO-defer, Trailing Blocks.',
            ),
          ],
          [
            tr(lang, 'Values & Errors', 'Werte & Fehler'),
            'true, false, Ok, Err, Some, None, print, assert',
            tr(
              lang,
              'Booleans, Result/Option constructors, stdout print, test assertions.',
              'Booleans, Result/Option-Konstruktoren, Ausgabe, Test-Assertions.',
            ),
          ],
          [
            tr(lang, 'Concurrency & ECS', 'Concurrency & ECS'),
            'concurrent, spawn, shared, lock, parallel, query, select, timeout',
            tr(
              lang,
              'Structured threads, mutex boxes, Rayon parallel loops/ECS queries, channel select.',
              'Strukturierte Threads, Mutex-Boxen, Rayon-Parallelschleifen/ECS-Queries, Channel-Select.',
            ),
          ],
          [
            tr(lang, 'Net, Mods & FFI', 'Net, Mods & FFI'),
            'sync, rpc, rust, extern, foreign',
            tr(
              lang,
              'Dirty-tracked struct fields, RPC expansion, inline Rust, direct C DLL/.so FFI.',
              'Dirty-getrackte Struct-Felder, RPC-Expansion, Inline-Rust, direktes C-DLL/.so-FFI.',
            ),
          ],
        ]}
      />

      <DocH2 id="all-types" theme={theme}>
        {tr(lang, 'All Types & Memory Layouts', 'Alle Typen & Speicherlayouts')}
      </DocH2>
      <DocTable
        theme={theme}
        headers={[
          tr(lang, 'Type', 'Typ'),
          tr(lang, 'Width / Align', 'Größe / Align'),
          tr(lang, 'Auto-Drop', 'Auto-Drop'),
          tr(lang, 'Notes', 'Hinweise'),
        ]}
        rows={[
          ['int', '8 B / 8 B', 'No', tr(lang, '64-bit signed integer (i64)', '64-Bit Ganzzahl (i64)')],
          ['float', '8 B / 8 B', 'No', tr(lang, '64-bit IEEE-754 float (f64)', '64-Bit Gleitkommazahl (f64)')],
          ['f32', '4 B / 4 B', 'No', tr(lang, '32-bit float in C-structs & FFI', '32-Bit Float in C-Structs & FFI')],
          ['u32 / i32', '4 B / 4 B', 'No', tr(lang, '32-bit unsigned/signed in structs & FFI', '32-Bit in Structs & FFI')],
          ['u8 / i8 / bool', '1 B / 1 B', 'No', tr(lang, '1-byte integer / boolean', '1-Byte Ganzzahl / Boolean')],
          ['String', '8 B ptr', 'Yes', tr(lang, 'Heap UTF-8 string', 'Heap UTF-8 String')],
          ['cstr', '8 B ptr', 'No', tr(lang, 'Null-terminated *const u8 for extern/foreign', 'Null-terminierter *const u8 für FFI')],
          ['Vec / Vec<T>', '8 B ptr', 'Yes', tr(lang, 'Dynamic array / flat RawStructVec', 'Dynamisches Array / flacher Struct-Puffer')],
          ['Deque, Map, StrMap, BTree, Set, Heap, Grid', '8 B ptr', 'Yes', tr(lang, 'Standard collections (collections.rs)', 'Standard-Collections (collections.rs)')],
          ['Bytes, Rng, BitSet, Channel, Atomic, Ecs, Vec3, Mat4', '8 B ptr', 'Yes', tr(lang, 'Binary buffers, PRNG, sync, ECS, math', 'Binärpuffer, PRNG, Sync, ECS, Mathe')],
          ['Result / Option', '8 B + TLS tag', 'No', tr(lang, 'Propagated via postfix ?', 'Propagiert per Postfix-?')],
        ]}
      />

      <DocH2 id="all-operators" theme={theme}>
        {tr(lang, 'Operators & Precedence', 'Operatoren & Präzedenz')}
      </DocH2>
      <DocTable
        theme={theme}
        headers={[tr(lang, 'Level', 'Stufe'), tr(lang, 'Operators', 'Operatoren'), tr(lang, 'Description', 'Beschreibung')]}
        rows={[
          ['1 (Postfix)', '(), [], ., ::, ?', tr(lang, 'Call, index, field/swizzle, path, error propagation', 'Aufruf, Index, Feld/Swizzle, Pfad, Fehler-Propagation')],
          ['2 (Unary)', '-, !', tr(lang, 'Negation, logical NOT', 'Negation, logisches NICHT')],
          ['3 (Mul)', '*, /, %', tr(lang, 'Multiplication, division, modulo', 'Multiplikation, Division, Modulo')],
          ['4 (Add)', '+, -', tr(lang, 'Addition, subtraction', 'Addition, Subtraktion')],
          ['5 (Shift)', '<<, >>', tr(lang, 'Bitwise left/right shift', 'Bit-Shift links/rechts')],
          ['6 (Cmp)', '<, >, <=, >=, ==, !=', tr(lang, 'Comparisons', 'Vergleiche')],
          ['7 (Bitwise)', '&, ^, |', tr(lang, 'Bitwise AND, XOR, OR', 'Bitweises UND, XOR, ODER')],
          ['8 (Logic)', '&&, ||', tr(lang, 'Logical AND, OR', 'Logisches UND, ODER')],
          ['9 (Assign)', '=, +=, -=, :=', tr(lang, 'Assignment, compound assign, Odin bind', 'Zuweisung, Verbundzuweisung, Odin-Bindung')],
        ]}
      />
      <DocP theme={theme}>
        {tr(
          lang,
          'Note: Arithmetic operators (+, -, *, /, ==) can be overloaded on structs via op_add, op_sub, op_mul, op_div, and op_eq.',
          'Hinweis: Arithmetische Operatoren (+, -, *, /, ==) können auf Structs über op_add, op_sub, op_mul, op_div und op_eq überladen werden.',
        )}
      </DocP>
      <p className="text-xs font-mono text-zinc-500">
        {tr(lang, 'End of keyword & type reference.', 'Ende der Keyword- & Typreferenz.')} <InlineCode theme={theme}>striv-syntax</InlineCode>
      </p>
    </div>
  )
}
