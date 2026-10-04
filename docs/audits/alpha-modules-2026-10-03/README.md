# Luna alpha modules: implementation and verification record

Base: `b330ee3`, branch `codex/antigravity-repair-0.1`. The compiler-integrity
repairs were committed and pushed before this feature work. Target: Windows
x86_64 GNU, LLVM 18.1.8, Rust 1.98, development/test debug information disabled,
existing `build/runtime-win/libluna-runtime.a`. This is feature evidence, not
certification of the Luna alpha release or other targets.

Contracts: [NAMESPACE-USING-v1](../../spec/0.1/namespace-using-v1.md) and
[PROVIDER-CONFIG-v1](../../spec/0.1/provider-config-v1.md).
Verification status: **EXPANDED FEATURE MATRIX PASS / WORKSPACE HAS BASELINE FAILURES**.
All 57 permanent fixtures pass the CLI matrix in source-only and fresh
artifact-only modes on the final feature code, including the alias/identity
correction. Broad workspace snapshots failed; isolated baseline comparisons
confirm named failures predate the additions. The snapshot before the final
alias/cache correction completed with 1,231 passed, 44 failed and 1 ignored.
Final targeted regressions are recorded separately. No full-workspace PASS or
release conformance is claimed.

## Implementation

- The parser distinguishes namespace openings from existing aliases. The new
  AST variant is appended, preserving existing serialized variant indices and
  the old alias representation. Old readers cannot silently reinterpret a new
  variant as an old declaration.
- A scope stores namespace candidate scopes, rather than copied declarations.
  Direct bindings win at each lexical level; openings then participate before
  walking outward. Canonical SymbolId deduplication and sorted ambiguity labels
  preserve identity and deterministic error E1008. Targets are resolved without
  using other bare openings. Function/block directives reject.
- Macro-generated names use their definition environment; captures keep their
  call-site syntax context. Imported macro environments retain canonical binding
  IDs for private helpers without exposing those helpers through public namespace
  exports. Source and portable-AST reconstruction remap these IDs for consumers.
  Only bindings accessible at the actual definition scope enter this environment;
  a macro cannot grant access to a private sibling namespace. Imported private
  symbols also retain their canonical logical paths for ABI/codegen, without
  inserting them in public namespace scopes.
- CLI configuration selects a nearest entry-relative TOML, an explicit file or
  disabled mode. One typed binding table enters the driver and all dependencies.
  Semantic analysis and backend never read TOML. Sysroot names/aliases remain
  reserved according to the selected manifest.
- Configured stems reuse the ordinary provider loader and strict validator.
  Discovery is authoritative, with config-key related locations on failures.
  Canonical physical paths deduplicate discovery aliases/local imports; distinct
  physical providers claiming the same logical identity reject instead of merging.
- Source identity follows the provider stem; new artifacts record their genuine
  module identity, independent of configuration aliases and artifact filenames. Discovery aliases occupy a
  separate cache from genuine provider identities. Aliases of one canonical
  provider reuse one injected scope, and stable metadata always uses canonical
  interface names.
  Fingerprints are never patched to make a mapped artifact load.
- `run` now actually executes the compiled program, propagates its exit status
  and cleans only its uniquely owned temporary output directory. Infrastructure
  errors use typed diagnostics; E6008 covers configuration, E6009 process startup.

No library-name mapping, new intrinsic, library-owned type layout or backend
instruction was introduced. All reproducers use user-defined providers/types.

## Acceptance matrix

The 57-fixture permanent CLI harness is
[alpha_modules_cli.rs](../../../luna-rs/crates/luna-cli/tests/alpha_modules_cli.rs),
with real [Luna fixtures](../../../tests/luna/language/alpha_modules/).
It starts with a source-only sysroot copy, invokes the official CLI builder for
all 49 providers, and makes source-only and artifact-only roots. Custom geometry
and dependent artifacts are built in independent CLI processes; their sources
are removed before artifact consumption.

Coverage includes lexical/direct/parameter precedence, namespace aliases and
repetition, child namespace access, nonrecursive lookup, type/function/macro
ambiguity and declaration order, related candidate locations, private/missing/
leaf targets, rejected exported/local directives, macro definition-site names,
private helpers versus direct privacy rejection, macro shadowing by ordinary
functions/types, parameters, generics, block/loop/match/lambda bindings, qualified
macro access and scope exit, configured aliases differing
from filename/namespace, public generic signatures/bodies, no auto-import,
entry/cwd independence, nearest-only/explicit/disabled configurations, one active
transitive table, malformed/schema/key/path/duplicate/reserved-key errors,
unused missing providers, authoritative missing mappings over loaded providers,
alias cycles, aliases crossing other providers' genuine names in either import
order, portable APIs using both providers' types, distinct identity collisions,
source/artifact precedence, corrupt
and stale artifacts, renamed artifacts with genuine dependency fingerprints,
relocation without original paths, and actual `run` exit propagation.

## Regression observations

The pre-shadow-fix full workspace run completed with exit **101**:
**1,243 passed, 32 failed, 1 ignored**, across 179 test targets/doc-test outputs.
It is superseded for final feature-code validation, not relabeled as successful.
The subsequent pre-alias-correction snapshot also exited 101: **1,231 passed,
44 failed, 1 ignored**, again across 179 test targets/doc-test outputs (13 failed
targets). Its failed-case set contains all prior 32 cases plus HT7 storage parity
and all 11 HashSet cases. Each added case rejected an artifact because of the
`__raw_table` dependency interface fingerprint. HT7 writes a newly compiled
artifact into the shared sysroot; the consumer graph then contains a mixture of
artifact generations. Fresh canonical sysroot rebuilding and targeted reruns
then pass **8/8 storage and 11/11 HashSet** on the final files. All 12 added
fingerprint failures are absent in that controlled run. The snapshot itself
remains a failed result, and HT7's mutation of shared artifacts remains a harness
isolation limitation.

The broad snapshots precede the final alias/identity-cache correction and the
nested sysroot layout correction in the trait harness. Their results are not
presented as tests of those later changes. The 57-fixture CLI matrix and final
selected driver/stdlib regressions exercise the final files.

An isolated `git archive b330ee3` baseline was compiled using the same toolchain,
runtime archive and target. Exact comparisons reproduce:

| Baseline case | Observation at b330ee3 and pre-shadow workspace |
|---|---|
| Iterator collect C11 pipeline | Native process exits -1073741819 (Windows access violation); root cause is not yet isolated |
| Option/Result borrowed provider parity | Provider compilation rejects with LocalBorrowEscape |
| Associated-type and mixed artifact graph metadata tests | Two consumer compilation failures |
| Dependency implementation-only freshness | Dependency execution fingerprint mismatch |
| Struct lifetime/storage-anchor negatives | Rejection occurs, but four assertions still inspect legacy code text rather than typed identity |
| Generic trait dispatch harness | Five positives fail before their assertion because copied `-I` sources are not the active canonical sysroot |

These comparisons establish pre-existence for the named cases, not that every
failed test is harmless or that every root cause is a compiler defect. The runtime
crash, borrow and metadata failures need separate minimal reproducers and contract
checks. Remaining pre-shadow failures include hardcoded old provider counts,
combinator/parity harness bootstrap failures and an out-of-range unsuffixed
integer in a numeric fixture. They remain explicit failures until reconciled.

The generic trait dispatch harness's first correction selected a flat temporary
root that the sysroot loader did not recognize. The final fixture uses the
canonical `libs/external` layout and explicitly validates that layout before
selecting it. Tests serialize environment changes and restore the previous
selection. This does not grant trust through search paths or weaken compiler
authorization. The final targeted suite passes **7/7**, including both rejection
cases; the broad workspace run had already compiled the older fixture.


An initial parallel workspace build collided with a running `luna.exe` Windows
file lock. Subsequent compilation/testing runs are sequential; this is separate
from compiler correctness.

The archived baseline comparison reused a Cargo target directory. Later
standalone driver builds read cached common/semantic libraries whose type
metadata lacked fields present in the current source, while the CLI and
workspace check feature variants compiled. A scoped `cargo clean` of the six
changed crates invalidated those cached variants; source was unchanged. Final
CLI and driver results below follow that invalidation. Future revision controls
should use separate Cargo target directories, not rely on shared timestamp-based
build-cache state. Failed cache-attempt logs remain separate from executed-test
failures.

The broad regression run found a stale CLI assertion expecting the legacy text
`E_RAW_STORAGE_ANCHOR_FIELD`, while the compiler correctly rejects the same
invalid pointer-field contract as typed `error[E3011]`. The assertion is updated
to the stable code; rejection, source/artifact diagnostic parity and executable
absence remain required.

A separate counterexample found that opening a caller namespace could capture a
macro's uncaptured helper name. A definition-site context fix and permanent
regressions address that. An imported macro/private-helper probe additionally
exposed loss of the definition lookup environment at interface injection; the
provider environment retention fixes that generic mechanism. Artifact-native
linking of the same probe exposed loss of the private helper's canonical path;
retaining imported canonical paths fixes symbol mangling without publishing
private declarations. A negative private-sibling macro probe confirms that
definition environments do not broaden access.

A late, valid counterexample found that a file-level function did not shadow an
opened macro during expansion because only module/macro names were preregistered.
Ordinary file/module declarations now register identity before expansion; local
binding frames enforce the same precedence for parameters, generics and lexical
bodies. Nine negative and two positive permanent fixtures verify rejection,
qualified macro calls, ambiguity suppression and restoration after scope exit.
These frames do not copy symbols into public namespaces or artifact exports.

A further CLI counterexample crossed a configured alias with another provider's
real name. The loader had checked the discovery-name cache as if it were a
canonical identity table. Duplicate identity checks now inspect actual provider
interfaces; stable metadata names and legacy decoder inputs also use those
interfaces rather than arbitrary alias keys. Permanent forward/reverse import
and two-provider public-type artifact probes cover this distinction in the
expanded final matrix. Genuine duplicate identities still reject.

## Final commands and evidence

Full command lines, literal tested-source SHA-256 hashes, compiler/runtime hashes,
per-run counts and failed-case names are in [verification.json](evidence/verification.json).
Broad snapshots and final targeted runs are explicitly separate; no arithmetic
subtraction is used to relabel a failed workspace command as PASS.

| Check | Actual result | Evidence |
|---|---|---|
| CLI feature matrix, after scoped cache invalidation | PASS; 57 Luna fixture files in source/fresh-artifact modes; one Rust orchestration test, 61.88 s | [CLI log](evidence/alpha-modules.log) |
| Official canonical sysroot rebuild | PASS; 49 providers built | [Sysroot log](evidence/canonical-sysroot.log) |
| Corrected generic trait dispatch fixture | PASS; 7/7 | [Trait log](evidence/trait-dispatch.log) |
| Selected import/provider/artifact/freshness regressions | 48 passed, 3 failed; exit 101 | [Driver log](evidence/driver-regressions.log) |
| Storage plus HashSet after fresh canonical build | PASS; 19/19 | [Storage log](evidence/storage-hashset.log) |
| `cargo check --workspace -j2` | PASS; existing warnings remain | [Check log](evidence/workspace-check.log) |
| Full pre-alias workspace snapshot | 1,231 passed, 44 failed, 1 ignored; exit 101 | [Snapshot log](evidence/workspace-pre-alias.log) |

The three final selected-regression failures are
`test_associated_type_round_trip_canonical_llib`,
`test_mixed_source_artifact_graph_parity` and
`test_dependency_implementation_only_change_preserves_validity`. All three
also fail on the [archived baseline](evidence/baseline-metadata-freshness-lifetimes.log).
They remain open. The earlier [native C11 crash](evidence/baseline-c11.log) and
[borrow parity failure](evidence/baseline-option-parity.log) also remain explicit
correctness concerns; no claim is made that baseline failures are harmless.

Documentation generation and local validation are recorded in
[documentation.log](evidence/documentation.log). Final-tree full-workspace
conformance has not been established; final-tree evidence is the bounded CLI
matrix and selected regressions above.

## Boundaries

No module re-export, selective leaf using, function-local using, package/version
solver, config merging, automatic rebuild, acquisition or new platform is added.
Equal logical provider identities at different files are rejected in alpha;
configuration aliases cannot create separately named package instances.
Existing W8/diagnostics/overall release gaps are not automatically closed by these
features. Historical logs and prior repair claims remain dated evidence.

SKILL IMPACT: **CORRECTION** of grammar guidance/EBNF for the approved namespace
opening, plus removal of an undated claim treating historical parser failures
as current restrictions. Historical grammar results remain in the gap register;
current checks are reported separately. Implementation status remains separate
from contract authority.
