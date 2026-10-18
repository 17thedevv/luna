# R5 generic Linux native math linkage

Status: implementation under validation; no release qualification claim.

## Contract and observed gap

The advertised Ubuntu GNU target must link executable code that calls ordinary
C math functions through `#[link(name = "...")] extern fn` declarations.
Both source and freshly emitted `.llib` providers must preserve that behavior.
The native linker owns platform system dependencies; library names, namespaces
and concrete Luna types must not influence linker selection.

At implementation pin `52ef3711813ac0e2ded272f5ebe71897752766bc`, the complete
Ubuntu workspace triage reported **1408 passed, 5 failed, 1 ignored**. All five
failures are unresolved C math symbols at native executable link. The final
workspace job was skipped. The unchanged tests and their raw log are preserved
in the [previous pin's evidence](../r5-missing-return-2026-10-17/evidence/ubuntu-workspace-triage/failure-analysis.json).

## Implementation plan and boundary

1. Share platform system-library suffix selection between the two native
   executable linker entrypoints. Add Linux `-lm` after input objects/runtime.
2. Exercise ordinary user FFI through the public CLI with a user-defined
   `Sample`, f64 `sqrt`/`sin`, and f32 `ceilf`, in source and fresh artifact modes.
3. Run the five unchanged failing tests through their three original suites,
   then all exact-pin gates on Windows and Ubuntu at a new immutable pin.

Compiler Change

    Capability: Native Linux executable linkage of ordinary C math FFI.
    Why stdlib exposed it: core float providers call host C math symbols.
    Why it is generic: System-library selection depends only on host platform;
        no provider, symbol, namespace or concrete type matching is added.
    User-defined type benefiting: measurement::Sample in native_math_link.
    Tests: native_math_link_cli; unchanged lang_namespace_contract_tests,
        stdlib_phase4c_llib_parity_tests and stdlib_phase4c_numerics;
        complete exact-pin qualification remains required.
    New intrinsic/lang_item?: NO. No serialized format or ABI changes.
    Stdlib-specific branch?: NO.

The first local fixture attempt had a missing struct terminator and an f32/f64
comparison mismatch. Both fixture errors were corrected to canonical syntax;
the direct CLI source build and native execution then exited 0 on Windows.
The [fresh source/artifact CLI preflight](evidence/windows-cli-preflight.json)
then passed on Windows (1 test, 0 failures; both modes check/build/native exit0).
The runtime was reused locally, so this is preflight evidence only. Linux
execution and every exact-pin release gate remain separately required.

## Qualification policy

The previous pin is not qualified. Historical passes remain scoped to that SHA.
All required lanes must be reacquired at the new implementation pin, including
Windows and Ubuntu full workspace triage and final. Hosted Windows workspace
passes use separate fresh jobs; final depends on successful triage. No test
oracle or failure policy is weakened.

The candidate stays at `c085449b` until the selected pin satisfies all gates.
Tagging, main merge, PR creation and publication remain maintainer decisions.

Skill impact: no reusable semantic or architecture rule changed; existing
generic compiler-boundary and public CLI testing guidance covers this fix.

## Active acquisition

Implementation pin: `3a72f9464fa921410b63551f7eac03414cf51e2b`.
Harness commit: `14a608660cdec0c3bb8884ab893f74acda200dcd`.

- [Ubuntu normal / ASan / comptime](https://github.com/17thedevv/luna/actions/runs/39056178504)
- [Ubuntu full workspace](https://github.com/17thedevv/luna/actions/runs/39056178775)
- [Windows normal / full workspace](https://github.com/17thedevv/luna/actions/runs/39056178610)

All three workflows were observed running through the authenticated GitHub
connector. Final workspace jobs depend on triage success. These are acquisition
locators, not PASS evidence; verify terminal results and artifact hashes before
changing the verdict. The [state file](qualification-state.json) records jobs.

## Verified selected-pin results

Ubuntu ASan passed its C runtime/RuntimeAbiTests scope. The [verification](evidence/ubuntu-asan-verification.json) checks the GitHub ZIP digest, all 42 manifest files and compressed/raw hashes of all 19 gate logs. Raw logs show libasan linkage, the partial-read-capacity test executing, 100% CTest pass, and no sanitizer report. This does not certify compiler/generated-code/stdlib instrumentation.

The [requirement-by-requirement matrix](REQUIREMENTS.md) keeps all remaining gates pending until their own evidence is verified.

Ubuntu comptime-vm also passed: [verified artifact](evidence/ubuntu-comptime-vm-verification.json), 981 manifest files and 28 compressed/raw gate logs checked. Nested verdicts are **88 PASS / 0 FAIL / 8 CHARACTERIZED**. The eight historical final-if statement cases are not value-producing if-expression conformance. All 49 providers have canonical Linux GNU target and native ELF integrity; the protected source-tree diff is empty. This verifies the named 64-bit GNU boundary scope.

Ubuntu normal passed **48/48 gates**. The [verified archive](evidence/ubuntu-normal-verification.json) matches GitHub digest and all 122 manifest files / 48 compressed-and-raw gate logs. The user FFI test passes in source and fresh artifact modes. All five previously failing tests now report `ok` in the unchanged three driver suites (19 tests total, zero failures). E3004 negative/control CLI, diagnostic policy/inventory, canonical target, native quickstart and docs example gates also pass. The 16 docs rows contain 14 contract passes and two legacy foreach characterizations.

The normal checkout was initially clean. Its final status lists only generated `docs/documentation-index.json` and `.md`, changed by the docs generation gate. The protected compiler/runtime/test diff is empty. Normal lane success is therefore **not** an assertion of a fully clean final worktree. The independently acquired workspace lanes and hosted Windows normal still require their final checkout to be clean; those results remain pending.

Windows native normal also passed. The [verified original archive](evidence/windows-normal-verification.json) matches GitHub digest; all 35 recorded gate-log hashes were checked. All 12 focused batches, diagnostics, native runtime ABI, both CLI profiles, 49 canonical Windows GNU/COFF providers, docs examples and native quickstart pass. Comptime reports **88 PASS / 0 FAIL / 8 CHARACTERIZED**. The hosted wrapper enforces an exact clean checkout after the final replay before writing its successful summary.

The strict A5 replay passes all four profile/provider-mode rows with actual `luna: PANIC [L002]` and `index 3, length 2` in preserved native stderr. It is not inferred from a nonzero exit alone. The original archive preserves all acquired data; selected metadata and raw logs are extracted alongside it. Complete Windows and Ubuntu workspace triage/final remain required and pending.

## Terminal workspace results and final audit finding

All four full workspace passes completed: Windows and Ubuntu each report **1414 passed / 0 failed / 1 ignored**, in both no-fail-fast triage and default-parallel final. The sole ignored test is `mutable_references::sem_mutref_06_invalid_fixture`. All four ZIP digests and raw/compressed gate logs were verified. Ubuntu initial/final status and protected-tree diff are empty; Windows wrapper clean-final checks completed successfully.

**This pin is not final release-qualified.** Final source review exposed a real nested-branch E3001 reducer that loses its structural antecedent move span. [The preserved finding](../r5-move-origin-2026-10-18/evidence/finding.json) shows the invalid program still rejects, but DIAG-3/9 remains incomplete. Candidate stays c085449b. A compiler repair requires a new pin and qualification; these eight green lanes remain historical evidence for 3a72f946.
