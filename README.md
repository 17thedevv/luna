<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](docs/spec/0.1/README.md).

# Luna 0.1 compiler and toolchain

Luna is a systems language with a Rust compiler, MVIR, an LLVM backend, explicit
ownership/lifetime contracts and a native C runtime. The canonical compiler is
`luna-rs/`; source uses `.ln`, library artifacts use `.llib`, runtime identity uses
`__luna_*`. Mellis/fdlang are historical names.

## Download Luna 0.1.0-alpha.1

The current distribution is an **alpha prerelease**. Stable `0.1.0` has not been released.

**Windows:** [Download lunainstaller.exe](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/lunainstaller.exe)
([SHA-256](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/lunainstaller.exe.sha256)).
The small online setup downloads and verifies Luna, LLVM 18 and GNU/MinGW,
installs for the current user, and optionally adds Luna to PATH. No administrator
account or Rust/Cargo is needed. See the [installer details](distribution/windows/README.md).

| Platform | Download | Compiler in the ZIP |
|---|---|---|
| Windows x86_64 GNU | [Windows ZIP](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/luna-0.1.0-alpha.1-x86_64-pc-windows-gnu.zip) | `bin/luna.exe` |
| Ubuntu 24.04 x86_64 GNU | [Ubuntu ZIP](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/luna-0.1.0-alpha.1-x86_64-unknown-linux-gnu.zip) | `bin/luna` |

Verify the ZIP with [SHA256SUMS](https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/SHA256SUMS).
See [release notes](https://github.com/17thedevv/luna/releases/tag/v0.1.0-alpha.1)
and [installation instructions](https://lunalang.netlify.app/getting-started/).
Keep `bin`, `runtime` and `libs/external` together after extraction. The Windows
ZIP contains the compiler executable. Portable ZIP users install LLVM 18 and
a native GNU toolchain separately; Windows portable users also need the
Microsoft Visual C++ runtime. The Windows installer handles its private
dependencies automatically. Rust/Cargo is not required to use these packages.

## Specification and readiness

Start at [Luna 0.1 specification](docs/spec/0.1/README.md),
[Vietnamese reference](docs/LanguageReference.md), [gap register](docs/spec/0.1/gaps.md),
and [document inventory](docs/documentation-index.md).

The baseline retains defined contracts even where implementation is incomplete.
**Luna 0.1.0-alpha.1 is published.** The [CPU baseline repair](docs/audits/r5-cpu-baseline-2026-10-20/README.md)
at `b5cb77c3` passed all eight Windows/Ubuntu qualification lanes, with all eight
CI ZIPs independently verified. Both clean package smokes passed. The annotated
tag points to exact packaged source `cd9cd188`, whose implementation trees match
the qualified pin. Candidate is `b5cb77c3`; PR #7 and the release closure PR #8
are merged. [Release closure](docs/audits/r5-cpu-baseline-2026-10-20/release-closure.json)
records the immutable SHAs and published asset digests. Protocol versions remain
**25/9/5/2**. Qualification applies to the declared tested scope and limitations.

The [previous qualification report](docs/audits/r5-move-origin-2026-10-18/REPORT-R5-QUALIFICATION-8525067f.md)
and original archives remain historical evidence. [Release authorization and scope](docs/audits/r5-move-origin-2026-10-18/RELEASE-HANDOFF.md)
record the authorization and the completed passing gates.

The [R4 release ledger](docs/audits/0.1-alpha-completion-2026-10-04/r4/README.md)
and later [R5 historical qualification](docs/audits/r5-final-2026-10-16/REPORT-R5-QUALIFICATION-2026-10-16.md)
retain their exact tested scopes. Historical passes cannot certify a newer pin.
See [current limitations](docs/audits/r5-move-origin-2026-10-18/RELEASE-HANDOFF.md)
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

The release targets are Windows x86_64 GNU and Ubuntu 24.04 x86_64 GNU with
LLVM 18 and a Release C runtime. Source development requires Rust/Cargo.
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
