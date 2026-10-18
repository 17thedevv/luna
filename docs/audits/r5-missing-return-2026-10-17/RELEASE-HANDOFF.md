# Luna 0.1-alpha.1 — draft maintainer handoff

**DRAFT — qualification is still running. Do not publish this as a completed release.**

Selected implementation: `67146195fb025951ca51d54365fd3a47ef9544e6`.
Previous `3a72f946` completed all eight lanes (workspace 1414/0/1 on both targets) but exposed a nested/stale move-origin gap at final review.
Previous implementation `52ef3711` is not qualified (five Ubuntu native math link failures).
Current remote candidate: `c085449b207b5860b1d7a79bee221c2ff3d357c3`.
Current remote main: `401a95a872f0fdfb9840640f41cc4ac629cb17cd`.
Refs were checked on 2026-10-18; recheck before any maintainer action.
The [current qualification report](../r5-move-origin-2026-10-18/README.md) and
[live acquisition locators](../r5-move-origin-2026-10-18/qualification-state.json) own the current status.
The [previous pin report](QUALIFICATION.md) preserves its terminal failures.
Evidence/report commits identify the harness or recorded results; they do not
automatically become the tested implementation pin.

## Draft release notes

Luna 0.1-alpha.1 brings together the Rust compiler, MVIR and LLVM backend,
the native C runtime and the 49-provider component sysroot. The versioned
[0.1 specification](../../spec/0.1/README.md) and its adopted contracts govern
the language; the release evidence remains bounded to its named cases.

The A/C/D/E repair work covers closure environment destruction, async
cancellation cleanup, partial aggregate cleanup, cast and raw-view safety,
named/default calls, provider identity and checked arithmetic. Exact historical
closures and their scopes remain in the
[R4 ledger](../0.1-alpha-completion-2026-10-04/r4/README.md).

The final diagnostic follow-up preserves typed identities for comptime resource
and escape errors (`E4002`/`E4003`), keeps compilation diagnostics under `run`,
and repairs reachable missing returns (`E3004`). A value-returning function
cannot silently acquire a fabricated zero result when its body falls through.
Void functions, valid explicit/tail returns and proven endless loops remain
accepted. The permanent public CLI matrix checks both source and fresh
artifact-only provider routes, failure before executable publication and
native execution of valid controls.

The selected follow-up preserves move antecedents across nested/equal-state joins,
reinitialization and compacted aggregate fields. Public CLI reducers verify E3001
primary and related source lines in source/fresh artifact modes, including
portable generic bodies. The provenance change does not alter ownership-state
transitions or introduce library-specific compiler branches.

Protocol identities remain **compiler25 / metadata9 / MVIR5 / format2**.
This does not waive artifact freshness or dependency-interface validation.
Newly selected code must qualify at its own immutable pin.

Advertised native targets are x86_64 Windows GNU and x86_64 Linux GNU on
Ubuntu. The selected toolchain uses LLVM 18. Rust host triples and LLVM host
spellings are provenance; they do not replace the canonical Luna artifact
target or certify another Luna ABI. Debug/release in the reports names Cargo
CLI profiles and does not promise an independently selected native optimization
level. macOS, other architectures and freestanding targets are not advertised.

Known limits remain explicit:

- Standalone statement-position comptime execution is unsupported and rejects
  with spanned E4005. Named expression/VM/layout evidence is limited to 64-bit
  GNU. Historical final-if statement characterizations do not establish
  value-producing if-expression conformance.
- Comptime dependency selection conservatively retains statically reachable
  providers, including an untaken branch. Branch-sensitive precision is an
  accepted 0.2 deferral; missing an executed dependency would reopen correctness.
- E3001 presentation may include an internal value name under its accepted
  presentation deferral. Code, span and ownership requirements still apply.
- The diagnostic inventory declares eight registered codes without a 0.1
  producer (E0004, E1006, E1007, E2014, E2015, E2020, E4004, E5002) and four
  conditions without portable behavioural fixtures (E3007, E6002, E6003,
  E6009). The [inventory](../../diagnostics/diag-producer-inventory.json)
  records the reasons. No complete behavioural coverage claim includes these
  declared limitations.
- ASan evidence covers the C runtime and RuntimeAbiTests, including partial-read
  cleanup. It does not establish full compiler, generated-code or stdlib
  sanitizer coverage. Provider builds and the named acceptance suites do not
  certify every API, input domain, performance property or fuzzing surface.

## Qualification to finish before approval

The final handoff must cite actual completed results for all of the following:

| Requirement | Evidence to inspect |
| --- | --- |
| Exact pin, clean initial and final identity | Lane provenance, detached HEAD, protected-tree diff, final status |
| Native runtime, both CLI profiles, 49 fresh provider bundles | Build logs, hashes, canonical headers and actual native object checks |
| Focused A/C/D/E and DIAG-1..10 | Full named suite logs, producer inventory, per-code/span/related-label/phase gates |
| Windows native CI | Completed job URL and artifact at the selected pin, independent of local lane status |
| Windows complete workspace | Triage with `--no-fail-fast`, then default-parallel final; both complete with zero failures |
| Ubuntu native normal | Completed job and artifacts for ABI, target contract, Whole-File I/O, formatting/Unicode, CLI/provider parity |
| Ubuntu complete workspace | Dedicated triage job followed by successful default-parallel final, with counts and ignored cases recorded |
| Ubuntu ASan and comptime-vm | Verified exact-pin artifacts, scope limits, nested verdicts and raw hashes |
| Documentation and user quickstart | Generation/validator, all example gate rows, successful native quickstart |
| Final review | Every requirement accounted for; no unresolved critical/high issue in adopted scope; final docs and target/limit matrix agree |

After those requirements pass, reconcile the public readiness documents and
closure ledger without rewriting historical evidence, and fast-forward the
candidate to the exact qualified implementation. A new compiler/runtime/test
change requires selecting and qualifying a new implementation pin.

## Maintainer decision after qualification

Tag creation, the PR/main merge and release publication remain reserved to the
maintainer. Before requesting that decision, fill in the completed gate results,
final candidate SHA, links and limitations here and in the closure report.
The maintainer chooses the tag identity and publication/distribution details.
This draft neither creates a PR nor changes main, tags or release state.
