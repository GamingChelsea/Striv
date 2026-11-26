# Striv 1.2

> **Language / Sprache:** [Deutsch](README.md) | English
> **Transparency Note:** This entire project (compiler, Cranelift JIT, runtime, bindgen, LSP, CLI, and documentation website) was completely **vibe-coded** — developed iteratively with AI assistance and verified directly against real test suites.

**Striv** is a statically compiled systems programming language featuring a **2-ms Cranelift JIT** for rapid iteration and a **Rust/LLVM release transpiler** for optimized executables. It supports both **Rust syntax** (`syntax = "rust"`) and **Odin syntax** (`syntax = "odin"`) via `striv.toml`.

- **Documentation (GitHub Pages):** https://gamingchelsea.github.io/Striv/
- **Windows Release (`striv.exe` + LSP v1.2.0):** https://github.com/GamingChelsea/Striv/releases/tag/v1.2.0

---

## Core Features

- **Dual Syntax (`rust` | `odin`) & Brace Styles (`--kr` | `--allman`)**:
  - `--rust` / `--odin`: Switch seamlessly between Rust-style (`fn`, `struct`, `let mut`) and Odin-style (`:: proc`, `:: struct`, `:=` mutable by default, `::` constants, `x: Type` zero-init).
  - `--kr` (K&R): Opening `{` on the same line (`fn main() {`).
  - `--allman` (Allman): Opening `{` on a new line.
- **2-ms Cranelift JIT & LLVM Release**: `striv run`, `striv dev` (hot-reload + live tweak server on port `:4040`), and `striv test` compile to native x86_64 in milliseconds. `striv build` transpiles to Rust and builds a standalone release executable.
- **Smart Auto-Move / Auto-Clone & Null-Handles**: Last-use analysis automatically moves containers (`0 ns`) or clones them when read later — no `.clone()` needed. External handles default to `0` (`null`) with `.is_valid()`.
- **Space-ECS 2.0**: Native multi-core Sparse-Set ECS (`Ecs::new()`, `spawn()`, `set()`, `set_parent()`, `detach()`, `parent_of()`, `has()`, `remove_comp()`, cascade `remove_entity()`, `query (id: entity, mut c: Comp, !Tag)`).
- **Sandboxed Mod Loader (`Mods::load("mods")`)**: Loads `mods/*/mod.strv` (or `mod.dll` / `libmod.so`), merges `api.strv`, jails `fs::read_text` / `fs::write_text` to each mod folder, and isolates panics/infinite loops via 500k gas limits.
- **Package Manager (`striv add` / `striv remove`)**: Install crates from crates.io (with auto `striv sync`) or pure Striv packages from GitHub (`git:user/repo` into `.striv/pkgs/`).
- **Direct C-FFI & Rust Interop**: `foreign "msvcrt.dll" { ... }` / `extern "msvcrt.dll" fn ...` with automatic `cstr` null-termination, plus `striv sync` and `rust fn` / `rust proc`.

---

## Quick Start

```bash
# Create a new project (Rust + K&R braces, or Odin + Allman braces)
striv new space_game --rust --kr
striv new space_game --odin --allman

# Run with 2-ms Cranelift JIT
striv run src/main.strv

# Add a Rust crate or pure Striv package
striv add glam
striv add git:user/orbital-math

# Format or switch dialect
striv fmt src/main.strv
striv switch-syntax odin src/main.strv --kr
```

---

## Editor Support (VS Code & Zed LSP)

1. Download `striv.exe` from [GitHub Releases](https://github.com/GamingChelsea/Striv/releases/tag/v1.2.0) and place it on your system `PATH`.
2. **VS Code**: Download `striv-vscode-ext.zip`, extract it, or run `code --install-extension ...`.
3. **Zed**: Download `striv-zed-ext.zip`, extract it, and load it via **Extensions → Install Dev Extension** in Zed.
