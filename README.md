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
stdlib defects at `3dac3ac`. Later compiler, runtime, stdlib and website work
has been integrated; prior audit results do not certify the merged implementation.

The merged sysroot contains **49 component providers**: 32 in the original audit
and 17 later additions requiring their own acceptance evidence. Six language-contract
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

A full workspace test remains a required implementation check. The integration
record distinguishes current checks from historical acceptance. The documentation validator performs local
consistency checks; CLI example checks are recorded separately.

## Optional module conveniences

Namespace openings are additive: `using geometry;` makes accessible direct
members available for unqualified lookup, while `using geometry as geo;`
keeps the alias form. Both apply at file/module scope. Distinct candidates
produce E1008; qualification resolves the collision.

An optional `luna.toml` shortens external provider discovery:

```toml
schema = 1
[providers]
geo = "../shared/geometry"
```

```luna
import <geo>;
using geometry;
fn main() -> i32 { return answer() - 42; }
```

Here the selected provider must declare `geometry::answer`. The `geo` key
does not rename its namespace. File values must be relative, extensionless
stems resolved from this configuration; use `/` on every host. Absolute,
drive-qualified/rooted values and home/environment markers reject with E6008.
Absolute CLI search paths and internal driver paths remain available. `check`, `build` and `run` select the nearest config
from the entry file; `--config FILE` overrides it and `--no-config` disables
it. Existing relative imports remain available. See the
[using contract](docs/spec/0.1/namespace-using-v1.md) and
[provider configuration contract](docs/spec/0.1/provider-config-v1.md).

## Repository

- `luna-rs/crates/`: 12 canonical compiler workspace crates.
- `luna-rs/libs/external/`: component source, manifest and canonical artifacts.
- `runtime/`: C runtime and ABI conformance tests.
- `docs/spec/0.1/`: versioned contract, conformance requirements and gaps.
- `docs/site/`: static bilingual entry pages and generated spec mirrors.
- `tests/luna/` and `luna-rs/tests/luna/`: public-language acceptance fixtures.

No optimization, platform support or release completion is inferred solely from
successful provider builds. See [status](Status.md) and [roadmap](ROADMAP.md).
