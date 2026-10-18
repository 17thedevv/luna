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
Current qualification is tracked in the [move-origin repair report](docs/audits/r5-move-origin-2026-10-18/README.md)
and its [replacement requirements](docs/audits/r5-move-origin-2026-10-18/REQUIREMENTS.md).
Implementation `3a72f946` completed all eight release lanes, including full
Windows/Ubuntu workspace triage/final (each 1414 passed, 0 failed, 1 ignored).
Final review then found missing or obsolete antecedent spans in E3001 diagnostics
across joins and reinitialization. Follow-up review also found message-based
dedup and unstable multi-function diagnostic order. The generic repairs need a
new pin and fresh qualification. Candidate remains `c085449b`.
Protocol remains compiler/metadata/MVIR/format **25/9/5/2**.

The [R4 release ledger](docs/audits/0.1-alpha-completion-2026-10-04/r4/README.md)
and later [R5 historical qualification](docs/audits/r5-final-2026-10-16/REPORT-R5-QUALIFICATION-2026-10-16.md)
retain their exact tested scopes. Historical passes cannot certify a newer pin.
See [current limitations](docs/audits/r5-missing-return-2026-10-17/RELEASE-HANDOFF.md)
and the [executable R5 checklist](docs/audits/0.1-alpha-completion-2026-10-04/r4/r5-checklist.md).

The historical [2026-10-02 audit](docs/audits/stdlib-2026-10-02/README.md) found compiler and
stdlib defects at `3dac3ac`. Later compiler, runtime, stdlib and website work
has been integrated; prior audit results do not certify the merged implementation.

The merged sysroot contains **49 component providers**: 32 in the original audit
and 17 later additions. E1 records fresh bundle/header/object provenance for all
49; D3's provider graph evidence covers a bounded Tier-2/3 matrix. This does not
certify every API/domain. Six language-contract
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
