# R5 — Executable qualification handoff

**PENDING / RELEASE BLOCKED. This is a checklist, not an executed qualification.**
Start R5 only under a separately authorized task. Select the full immutable R4
closure/candidate SHA reported after R4 commits. `3f8646c6` is the R4 input and
`e4f9fb53` is the E-wave tested implementation; neither may be silently recorded
as an R5 test of the later documentation commit.

## Gates and evidence still to acquire

| Gate | Existing evidence | Missing exact-R5 evidence and pass condition |
| --- | --- | --- |
| R5-PIN | R4 ledger, exact earlier closures, implementation tree equality | Fresh clone detached at full selected SHA; clean initial worktree/index; remote URL, toolchain and native target identity; no local E1 sysroot dependency |
| R5-BUILD | E1 Windows build/sysroot provenance | CLI debug/release, runtime and official 49-provider rebuild from clean source; exit0; native headers/object hashes and versions 25/9/5/2 unless separately approved new boundary |
| R5-FOCUSED | A/C/D/E named suites in final6 | Ownership/drop/async/casts/calls/provider/comptime/retained-contract focused matrix on the selected pin, source-only and fresh artifact-only roots, negatives retain correct identity; no accidental source fallback |
| R5-DIAG-STRUCTURAL | D1/D2, E1 typed admission and common registry parity | Complete canonical DIAG-1..10 producer/code/span/related-label/phase/test coverage map from diagnostic inventory; actual code assertions and full per-code coverage, no ad-hoc error paths; not supplied by rerunning four D1 tests alone |
| R5-COMPTIME-LIMITS | E1 expression arithmetic and 64-bit GNU layout cases; withdrawn C4 branch probe is not evidence | Characterize advertised comptime branch behavior with a valid permanent reducer; classify the withdrawn probe explicitly. Confirm VM/runtime/static layout parity on the selected GNU target; keep standalone statement execution unsupported unless separately repaired and approved |
| R5-TARGET-WINDOWS | E1 Windows tested scope only | Native Windows GNU runtime ABI, Whole-File I/O, formatting/Unicode, CLI/parity and final workspace; matching exact CI SHA and job URLs |
| R5-TARGET-UBUNTU | NOT VERIFIED | Ubuntu GNU normal native ABI + Whole-File I/O + formatting/Unicode + CLI/parity + final workspace; matching exact CI SHA and job URLs |
| R5-ASAN-UBUNTU | NOT VERIFIED | Sanitized runtime RuntimeAbiTests including partial-read cleanup with no sanitizer/leak report and exit0; record which components are instrumented. Broader language/stdlib sanitizer claims require additional executable evidence |
| R5-DOCS | R4 docs consistency only | Docs generation/validator and 16 CLI example rows meeting gate expectations on the exact R5 target; recorded output directory outside tracked history; public quick start builds and executes |
| R5-FULL | E1 three runs at e4f9fb53 | Triage workspace with --no-fail-fast; final default-parallel workspace exits0 with zero failures, expected ignored cases named, no crash/timeout reclassified as PASS; preserve raw logs/hashes/exits |
| R5-REVIEW | Accepted deferrals and limits | Every gate has exact pin/command/target/mode/result/evidence; zero unresolved critical/high defects in adopted scope; final target matrix and limitations agree; maintainer release decision after qualification |

If code/runtime/tests change during R5, record a new implementation pin,
refresh its relevant artifacts, repeat clean-checkout and affected qualification,
and do not transfer old exact-candidate evidence to it. A fail is retained as a
fail; a later rerun receives a new label. Never weaken diagnostics or arithmetic
trap classification, increase a timeout to hide a hang, or suppress failing
safety tests. Expected arithmetic traps require the Luna L003/L005 signature;
an ICE, missing DLL, signal, timeout or unrelated native crash is not a PASS.

## Windows GNU command sequence

Run from an empty task output directory. Supply `R5_REPOSITORY_URL`, full
40-character `R5_SHA`, and LLVM 18 / MinGW GNU / CMake / Ninja / Python / Rust in
the environment. The following recorder preserves the native command's exit
before doing any logging. Output and logs are outside the clean checkout.

```powershell
$ErrorActionPreference = 'Stop'
if ($env:R5_SHA -notmatch '^[0-9a-f]{40}$') { throw 'Set full R5_SHA' }
if (-not $env:R5_REPOSITORY_URL) { throw 'Set R5_REPOSITORY_URL' }
$r5Out = Join-Path (Get-Location) ('r5-' + $env:R5_SHA)
New-Item -ItemType Directory -Path $r5Out -ErrorAction Stop | Out-Null
function Invoke-R5Gate {
    param([string]$Label, [string]$Program, [string[]]$Arguments)
    $r5Log = Join-Path $r5Out ($Label + '.log')
    & $Program @Arguments *> $r5Log
    $r5Exit = $LASTEXITCODE
    @{ label=$Label; program=$Program; arguments=$Arguments;
       exit=$r5Exit; sha=$env:R5_SHA;
       log_sha256=(Get-FileHash -LiteralPath $r5Log -Algorithm SHA256).Hash } |
        ConvertTo-Json -Depth 6 | Set-Content (Join-Path $r5Out ($Label + '.json'))
    if ($r5Exit -ne 0) { throw "$Label failed: exit $r5Exit; retain log" }
}
Invoke-R5Gate 'clone' 'git' @('clone', $env:R5_REPOSITORY_URL, (Join-Path $r5Out 'checkout'))
Set-Location (Join-Path $r5Out 'checkout')
Invoke-R5Gate 'checkout' 'git' @('checkout', '--detach', $env:R5_SHA)
if ((git rev-parse HEAD) -ne $env:R5_SHA) { throw 'Wrong pin' }
if (git status --porcelain) { throw 'Checkout is not clean' }
Invoke-R5Gate 'rustc' 'rustc' @('-vV')
Invoke-R5Gate 'llvm' 'llvm-config' @('--version')
Invoke-R5Gate 'gnu-cc' 'gcc' @('-dumpmachine')
Invoke-R5Gate 'runtime-config' 'cmake' @('-S','runtime','-B','build/r5-runtime','-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_C_COMPILER=gcc','-DCMAKE_CXX_COMPILER=g++')
Invoke-R5Gate 'runtime-build' 'cmake' @('--build','build/r5-runtime','--parallel')
Invoke-R5Gate 'runtime-abi' 'ctest' @('--test-dir','build/r5-runtime','-R','^RuntimeAbiTests$','--output-on-failure','--no-tests=error')
$env:LUNA_RUNTIME_LIB = (Resolve-Path 'build/r5-runtime/libluna-runtime.a').Path
$env:LUNA_SYSROOT = (Resolve-Path 'luna-rs').Path
Invoke-R5Gate 'cli-debug' 'cargo' @('build','--manifest-path','luna-rs/Cargo.toml','-p','luna-cli')
Invoke-R5Gate 'cli-release' 'cargo' @('build','--manifest-path','luna-rs/Cargo.toml','-p','luna-cli','--release')
$env:LUNA_E1_RELEASE_CLI = (Resolve-Path 'luna-rs/target/release/luna.exe').Path
$env:LUNA_DOCS_CLI = (Resolve-Path 'luna-rs/target/debug/luna.exe').Path
Invoke-R5Gate 'sysroot' $env:LUNA_DOCS_CLI @('build-sysroot','--quiet')
Invoke-R5Gate 'workspace-triage' 'cargo' @('test','--manifest-path','luna-rs/Cargo.toml','--workspace','--no-fail-fast')
# Run the focused/common gates and the structural audit described below.
Invoke-R5Gate 'workspace-final' 'cargo' @('test','--manifest-path','luna-rs/Cargo.toml','--workspace')
```

The runtime path above is for the explicitly selected standalone Ninja/MinGW
configuration. Verify actual object/LLIB GNU target headers and executable
behavior; a successful host build alone is not target evidence. Do not run
Cargo mutations concurrently with CLI sysroot/matrix acquisitions.

## Ubuntu GNU normal and ASan command sequence

Start another fresh output directory and clone on Ubuntu with native GNU tools
and LLVM 18 configured (`LLVM_SYS_180_PREFIX`, `LLVM_CONFIG_PATH`, `LLVM_HOME`
as in existing CI). Set `R5_OUT` to a new absolute directory and set
`R5_REPOSITORY_URL` / `R5_SHA`. The shell recorder stops on a failed command
after saving its real exit code; it does not pipe the tested process to a logger.

```bash
set -euo pipefail
: "${R5_OUT:?new absolute output directory}"
: "${R5_REPOSITORY_URL:?repository URL}"
: "${R5_SHA:?full selected SHA}"
[[ "$R5_SHA" =~ ^[0-9a-f]{40}$ ]]
mkdir "$R5_OUT"
record() {
  local label="$1" status=0
  shift
  "$@" >"$R5_OUT/$label.log" 2>&1 || status=$?
  printf '%s\n' "$status" >"$R5_OUT/$label.exit"
  printf '%s\n' "$R5_SHA" >"$R5_OUT/$label.sha"
  printf '%q ' "$@" >"$R5_OUT/$label.command"
  sha256sum "$R5_OUT/$label.log" >"$R5_OUT/$label.log.sha256"
  return "$status"
}
record clone git clone "$R5_REPOSITORY_URL" "$R5_OUT/checkout"
cd "$R5_OUT/checkout"
record checkout git checkout --detach "$R5_SHA"
[[ "$(git rev-parse HEAD)" == "$R5_SHA" ]]
[[ -z "$(git status --porcelain)" ]]
record rustc rustc -vV
record llvm llvm-config-18 --version
record gnu-cc gcc -dumpmachine
record runtime-config cmake -S runtime -B build/r5-runtime -DCMAKE_BUILD_TYPE=Release
record runtime-build cmake --build build/r5-runtime --parallel
record runtime-abi ctest --test-dir build/r5-runtime -R '^RuntimeAbiTests$' --output-on-failure --no-tests=error
export LUNA_RUNTIME_LIB="$PWD/build/r5-runtime/libluna-runtime.a"
export LUNA_SYSROOT="$PWD/luna-rs"
record cli-debug cargo build --manifest-path luna-rs/Cargo.toml -p luna-cli
record cli-release cargo build --manifest-path luna-rs/Cargo.toml -p luna-cli --release
export LUNA_E1_RELEASE_CLI="$PWD/luna-rs/target/release/luna"
export LUNA_DOCS_CLI="$PWD/luna-rs/target/debug/luna"
record sysroot "$LUNA_DOCS_CLI" build-sysroot --quiet
record workspace-triage cargo test --manifest-path luna-rs/Cargo.toml --workspace --no-fail-fast
# Run focused/common gates and the structural audit before final qualification.
record workspace-final cargo test --manifest-path luna-rs/Cargo.toml --workspace
export ASAN_OPTIONS=detect_leaks=1:halt_on_error=1
record asan-config cmake -S runtime -B build/r5-runtime-asan -DCMAKE_BUILD_TYPE=Debug \
  '-DCMAKE_C_FLAGS=-fsanitize=address -fno-omit-frame-pointer' \
  '-DCMAKE_EXE_LINKER_FLAGS=-fsanitize=address'
record asan-build cmake --build build/r5-runtime-asan --parallel
record asan-abi ctest --test-dir build/r5-runtime-asan -R '^RuntimeAbiTests$' --output-on-failure --no-tests=error
```

ASan commands above execute the current CI's **runtime** scope. Normal Luna
executables still link the normal runtime in this sequence. They must not be
reported as fully instrumented Luna/stdlib/LLVM execution. Record any separately
adopted broader sanitizer gate rather than inferring it from this ABI result.

## Focused/common commands, both targets

Run these from the fresh checkout root through the appropriate recorder above,
giving each command a unique label. These are real current test target names;
each CLI parity harness creates fresh isolated source/artifact roots. Preserve
check/build negatives and native outputs, not just Cargo's final exit.

```text
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test generic_drop_cli --test async_cleanup_cli --test async_provider_export_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test cast_contract_cli --test cast_poison_cli --test raw_slice_view_a5_cli --test raw_slice_provenance_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test named_arguments_cli --test default_arguments_cli --test default_omission_cli --test call_arity_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test impl_header_metadata_cli --test projection_fallback_cli --test comptime_dependency_precision_cli --test comptime_loans_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test diagnostic_conformance_cli --test expression_span_precision_cli --test comptime_diagnostic_cascade_cli --test compiler_trust_diagnostics_cli_parity
cargo test --manifest-path luna-rs/Cargo.toml -p luna-common registry_parity
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test provider_tiers_cli --test retained_contracts_cli --test alpha_modules_cli --test module_const_storage_cli --test fresh_artifact_classification_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test checked_arithmetic_qualification_cli --test wrapping_arithmetic_cli --test v01_grammar_acceptance_tests --test test_temp_workspace_lifecycle_cli
cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test stdlib_format_cli_acceptance --test char_scalar_cli_parity --test char_operator_cli_parity
cargo test --manifest-path luna-rs/Cargo.toml -p luna-driver --test whole_file_io_v1_acceptance_tests
python docs/tools/build_v01_docs.py
python docs/tools/validate_v01_docs.py
```

The [57-fixture module/config record](../../alpha-modules-2026-10-03/README.md)
defines the expected module cases; require the alpha_modules_cli matrix, not a
provider-count proxy. Inventory actual rebuilt bundle pairs against
[sysroot.toml](../../../../luna-rs/libs/external/sysroot.toml), checking all 49
names and compiler/format/MVIR/target headers plus object identity. Use the
[E1 provenance shape](../evidence/e1-final5-sysroot-provenance-2026-10-13.json)
as the recording template; never reuse the old local `.e1-fp-*` roots.

For examples run `python docs/tools/check_v01_examples.py --gate --cli CLI
--runtime-lib RUNTIME --out NEW_OUTPUT_DIRECTORY`, substituting the absolute
paths set above and an unused directory outside tracked history. Require 16/16
contract expectations and keep the legacy foreach row a characterization.
Do not overwrite docs/spec/0.1/evidence or any earlier E-wave label. Build and
execute the README quick start separately, saving its source, compiler/runtime
hashes, stdout/stderr and exit. Regeneration alone is not public CLI acceptance.

Finally complete the [structural diagnostic inventory](diagnostic-inventory.md)
with a per-code and per-producer artifact. This step intentionally has no
single existing command claiming to certify the whole contract. The commands
above are prerequisites, not a substitute for the missing audit. Save every CI
job's exact SHA and result, not only its workflow name.

Record `tested_commit`, `code_tree_identity`, evidence hashes and the later
report commit separately. No self-referential report SHA. Do not automatically
merge main or create a release tag; R5 findings and release readiness need the
explicit closing decision after all required gates actually pass.
