<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](docs/spec/0.1/README.md).

# Luna 0.1 compiler and toolchain

Luna is a systems language with a Rust compiler, MVIR, an LLVM backend, explicit
ownership/lifetime contracts and a native C runtime. The canonical compiler is
`luna-rs/`; source uses `.ln`, library artifacts use `.llib`, runtime identity uses
`__luna_*`. Mellis/fdlang are historical names.

## Specification and readiness

Start at [Luna 0.1 specification](docs/spec/0.1/README.md),
[Vietnamese reference](docs/LanguageReference.md), [gap register](docs/spec/0.1/gaps.md),
and [document inventory](docs/documentation-index.md).

The baseline retains defined contracts even where implementation is incomplete.
**Release conformance is blocked**, not certified by historical FROZEN labels.
The [2026-10-02 audit](docs/audits/stdlib-2026-10-02/README.md) found compiler and
stdlib defects. This documentation refresh does not repair those defects.

The sysroot currently contains **32 component providers**. Six language-contract
families and controlled OptionExt visibility are bootstrapped; ordinary library
APIs require their specified imports. Stdlib is a language consumer: compiler
macro names, stream prefixes and newline suffixes must not select stdlib callees.

## Build and verify

The audited target is Windows x86_64 GNU with LLVM 18 and a Release C runtime.
Set `LLVM_SYS_180_PREFIX` to the LLVM 18 installation and configure the target C
compiler/linker in PATH. Other targets require their own evidence.

```powershell
cargo build --manifest-path luna-rs/Cargo.toml -p luna-cli
cmake -S . -B build/host
cmake --build build/host --config Release
```

CMake output paths and runtime library suffixes depend on generator/target.
Point `LUNA_RUNTIME_LIB` to the matching runtime artifact and `LUNA_SYSROOT` to
`luna-rs`. With the built CLI on PATH:

```powershell
luna build-sysroot
luna build program.ln -o program.exe
cargo test --manifest-path luna-rs/Cargo.toml --workspace
python docs/tools/validate_v01_docs.py
```

A full workspace test is a required implementation check, not a command verified
by this documentation-only update. The documentation validator performs local
consistency checks; CLI example checks are recorded separately.

## Repository

- `luna-rs/crates/`: 12 canonical compiler workspace crates.
- `luna-rs/libs/external/`: component source, manifest and canonical artifacts.
- `runtime/`: C runtime and ABI conformance tests.
- `docs/spec/0.1/`: versioned contract, conformance requirements and gaps.
- `docs/site/`: static bilingual entry pages and generated spec mirrors.
- `tests/luna/` and `luna-rs/tests/luna/`: public-language acceptance fixtures.

No optimization, platform support or release completion is inferred solely from
successful provider builds. See [status](Status.md) and [roadmap](ROADMAP.md).
