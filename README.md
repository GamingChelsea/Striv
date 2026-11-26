# Striv 1.0

> **Transparenz-Hinweis:** Dieses gesamte Projekt (Compiler, Cranelift-JIT, Runtime, Bindgen, LSP, CLI und Dokumentations-Website) wurde vollständig **vibe-coded** – iterativ mit KI-Unterstützung entwickelt und direkt gegen echte Testsuiten verifiziert.

**Striv** ist eine statisch kompilierte Systemsprache mit **2-ms Cranelift-JIT** für die Entwicklung und einem **Rust/LLVM-Release-Transpiler** für optimierte Executables. Sie unterstützt wahlweise **Rust-Syntax** (`syntax = "rust"`) oder **Odin-Syntax** (`syntax = "odin"`) über `striv.toml`.

- **Dokumentation (GitHub Pages):** https://gamingchelsea.github.io/Striv/
- **Windows Release (`striv.exe` + LSP v1.0.0):** https://github.com/GamingChelsea/Striv/releases/tag/v1.0.0

---

## Kern-Features

- **Dual-Syntax (`rust` | `odin`)**: Parser, Formatter (`striv fmt`) und Language Server (`striv lsp`) schalten per `striv.toml` nahtlos zwischen Rust-Stil (`fn`, `struct`, `let mut`) und Odin-Stil (`:: proc`, `:: struct`, `:=`, `mut x :=`) um.
- **2-ms Cranelift-JIT & LLVM-Release**: `striv run`, `striv dev` (Hot-Reload + Live-Tweak-Server auf Port `:4040`) und `striv test` kompilieren in wenigen Millisekunden in nativen x86_64-Code. `striv build` transpiliert nach Rust und baut ein eigenständiges Release-Binary.
- **Natives Multi-Core Sparse-Set ECS**: `Ecs::new()`, `spawn()`, `set()`, `remove_entity()`, `query` und `parallel query` über Rayon mit flachem Pointer-Puffer.
- **Physikalische Einheiten (`unit`)**: Compile-Time-Dimensionsprüfung (`unit m`, `unit Velocity = m / s`, `1000.0_kg`) mit 0 ns Laufzeit-Overhead.
- **Networking (`sync` & `rpc`)**: `sync`-Felder mit 64-Bit Dirty-Bitmask (`.pack_delta()`, `.pack_full()`, `.apply_delta()`) sowie `rpc fn` mit automatischem `<name>_pack()` und `rpc_dispatch()`.
- **Direktes C-FFI & Rust-Interop**:
  - `.dll` / `.so` direkt laden per `extern "msvcrt.dll" fn ...` (Odin: `foreign "msvcrt.dll" { ... }`) inklusive null-terminiertem `cstr` über `libloading`.
  - Rust-Crates aus `striv.toml` automatisch einbinden per `striv sync` (`lang-bindgen 3.0`) oder Inline-Rust (`rust { ... }` / `rust fn`).
- **Modding & Sandbox**: `open enum` / `extend enum` (64-Bit FNV-1a), `JitCompiler::new_sandboxed()` (Gas-Limit & File-I/O-Sperre) und C#-Bindings via `striv bindgen-cs`.

---

## Schnellstart

```bash
# Neues Projekt anlegen (Rust- oder Odin-Dialekt)
striv new space_game --rust --kr
cd space_game

# Per Cranelift-JIT ausführen
striv run src/main.strv

# Hot-Reload + Tweak-Webserver (:4040)
striv dev src/main.strv

# Unit-Tests ausführen
striv test src/main.strv

# Optimiertes Release-Binary bauen
striv build src/main.strv
```

---

## VS Code Extension & LSP installieren

Der Language Server (`lang-lsp`) ist direkt in `striv.exe` (`striv lsp`) integriert. Für Syntax-Highlighting, Diagnostics, Hover, Autocomplete und Formatierung in VS Code:

1. Lade im [GitHub Release v1.0.0](https://github.com/GamingChelsea/Striv/releases/tag/v1.0.0) sowohl **`striv.exe`** als auch **`striv-vscode-ext.zip`** herunter.
2. Lege `striv.exe` in einen Ordner, der im System-`PATH` liegt (damit VS Code `striv lsp` starten kann).
3. Entpacke `striv-vscode-ext.zip` in deinen VS-Code-Extensions-Ordner:
   - Windows: `%USERPROFILE%\.vscode\extensions\striv-lang\`
   - Führe dort einmalig `npm install --omit=dev` aus und starte VS Code neu.

### Zed Editor

Der gleiche Language Server (`striv lsp`) funktioniert auch in **Zed**:

1. Stelle sicher, dass `striv.exe` im System-`PATH` liegt.
2. Entpacke `striv-zed-ext.zip` aus dem Release (oder nutze `zed-ext/` im Repo) und wähle in Zed **Extensions → Install Dev Extension** auf den Ordner `zed-ext`.
3. Alternativ kannst du in deiner Zed-`settings.json` direkt auf `striv lsp` verweisen:
   ```json
   {
     "lsp": {
       "striv": {
         "binary": { "path": "striv", "arguments": ["lsp"] }
       }
     }
   }
   ```

---

## Workspace-Struktur

- `crates/striv-syntax` — Lexer (`logos`), Dual-Dialect-Parser, Formatter, Units-Checker, Monomorphizer, Release- & C#-Codegen
- `crates/striv-jit` — Cranelift-JIT-Compiler & `libloading` C-FFI-Loader
- `crates/lang-runtime` — Sparse-Set ECS, Collections (`Vec`, `Deque`, `Map`, `StrMap`, `BTree`, `Set`, `Heap`, `Grid`, `Bytes`, `Rng`, `BitSet`, `Channel`, `Atomic`), JSON-I/O, Tweak-Server
- `crates/lang-bindgen` — Automatischer Rust-Crate-Scanner (`striv sync`)
- `crates/lang-lsp` — `tower-lsp` Language Server
- `crates/striv-cli` — CLI (`striv.exe`)
- `website/` & `docs/` — Zweisprachige (EN/DE) Dokumentations-Website
