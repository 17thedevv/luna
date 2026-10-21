<!-- luna-doc-role: guidance -->

# Lunar: local projects and builds

`luna` compiles individual Luna source files. `lunar` manages projects, local
dependency graphs and build commands, invoking `luna` as an external process.
Both use **`luna.toml`**. Lunar is new development tooling; it is not included in
the published `v0.1.0-alpha.1` installer or compiler ZIPs.

Build Lunar from source with Rust 1.89 or newer (LLVM is not needed to build
Lunar itself):

```text
cargo build --manifest-path luna-rs/Cargo.toml --locked -p lunar --release
```

The result is `luna-rs/target/release/lunar.exe` on Windows, or
`luna-rs/target/release/lunar` on Linux. Put it beside the installed `luna`
executable, set `LUNA` to the compiler path, or use `lunar --compiler PATH`.
Native builds still require the compiler's LLVM and C runtime toolchain; the
Windows Luna installer already configures these for its compiler launcher.

## Start a project

```text
lunar new hello
cd hello
lunar check
lunar build
lunar run
```

`lunar init` initializes an existing directory without overwriting its manifest
or source. `lunar new geometry --lib` creates a library project.

Lunar searches the current directory and its ancestors for `luna.toml`.
`--manifest-path path/to/luna.toml` selects a project explicitly, independently
of the current working directory. Compiler selection order is `--compiler`,
the `LUNA` environment variable, a sibling `luna` executable, then `luna` on PATH.

## One configuration file

```toml
schema = 1

[package]
name = "hello"
version = "0.1.0"

[[bin]]
name = "hello"
path = "src/main.ln"

[[test]]
name = "smoke"
path = "tests/smoke.ln"

[dependencies]
geo = { path = "../geometry" }

[providers]
extra = "vendor/extra/extra"
```

All file values use relative paths with `/` separators. No environment or home
expansion occurs. Target sources must be `.ln` files inside their package.
Dependency paths may use `..` and select another directory containing
`luna.toml` and a `[lib]` target. Vendored libraries use exactly the same format:
`geo = { path = "vendor/geometry" }`.

Each package may declare one library:

```toml
[lib]
path = "src/geometry.ln"
```

Its provider identity remains the source filename stem (`geometry` here).
Dependency keys are discovery aliases: `import <geo>;` selects that library,
while a declaration such as `export module geometry { ... }` determines the
namespace `geometry::`. Lunar does not rename namespaces or import anything
automatically. Library stems and dependency/provider aliases must be portable
ASCII identifiers. Package and executable names also allow `-`.

`[[bin]]` and `[[test]]` each require `name` and `path`. If no library or binary
is declared, an existing `src/main.ln` becomes the binary named by
`package.name`. `version` is optional descriptive metadata, with no resolution
or compatibility policy. Unknown manifest fields reject, including registry,
URL, Git, or version-based dependency declarations.

The compiler only reads `[providers]`; current source recognizes the project
tables without interpreting their contents. Lunar validates the project tables
and generates a provider-only configuration for every compiler invocation.
This also supports the published alpha compiler, whose original strict parser
rejects project tables when reading the shared file directly. With that older
compiler, use Lunar, or `luna ... --no-config` with explicit search paths when
invoking the compiler directly. The installed alpha binary is not changed by
building Lunar.

## Commands and outputs

| Command | Behavior |
|---|---|
| `lunar build` | Build local libraries in dependency order, then all project binaries |
| `lunar build --bin NAME` | Build one binary and its local library graph |
| `lunar check` | Check library, binary and test targets using dependency source stems |
| `lunar run --bin NAME -- ARG...` | Build one binary, run it with native arguments and return its exit code |
| `lunar test` | Build and run all declared test programs; any nonzero exit fails the command |
| `lunar clean` | Remove the owned `target/lunar` build directory |

Successful builds publish to `target/lunar/build/`: executables are in `bin/`,
test executables in `test/`, and `.llib` bundles in `providers/`. A generated
`luna.toml` records the effective provider bindings. Failed builds never execute
a previous binary. The previous successful output can remain until a new build
succeeds or `clean` removes it; incomplete output stays in the disposable
`stage/` directory and is removed on the next invocation.

Lunar rebuilds libraries from source each time. It does not treat an existing
artifact as evidence of validity or modify compiler fingerprints. The compiler
retains responsibility for `.llib` validation, stdlib-name collisions and all
language semantics. Explicit `[providers]` entries retain compiler candidate
precedence, including rejection of an invalid selected `.llib`.

The graph has one provider-binding context. Canonical library stems and aliases
must identify the same provider wherever reused; conflicts and dependency
cycles reject. A diamond dependency on the same canonical manifest builds once.
Project and external provider paths must be on the same filesystem volume,
because generated compiler bindings must remain relative.

Build and clean operations use an OS file lock per root project. `clean`
requires Lunar's ownership marker and rejects symlinks and Windows reparse
points inside the disposable output tree. It preserves package sources,
dependency source directories and other files under `target`.

There is no package registry, download command, automatic Git checkout,
version solver, incremental cache, workspace command, cross-target option or
arbitrary build script in this first version. Copy or vendor dependencies into
local directories yourself. No package hosting service is required.

## Validation

```text
cargo test --manifest-path luna-rs/Cargo.toml --locked -p lunar
python docs/tools/smoke_lunar.py --lunar PATH/TO/lunar --luna PATH/TO/luna --out NEW_DIRECTORY
```

The smoke test invokes the real public compiler, links and executes programs,
and exercises local transitive/diamond libraries, changed source rebuilds,
test success/failure, compiler diagnostics and cleaning under Unicode paths.
It writes per-command logs, digests and a summary to the output directory.
