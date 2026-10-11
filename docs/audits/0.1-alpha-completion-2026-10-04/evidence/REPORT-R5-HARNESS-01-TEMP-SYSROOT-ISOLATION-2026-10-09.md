<!-- luna-doc-role: evidence -->

# R5-HARNESS-01 report — deterministic test infrastructure

Branch `r5-harness-temp-sysroot-isolation` (from candidate `d7129cd1`). Release blocker.

## Two distinct problems

| Problem | Evidence | Status |
|---|---|---|
| Temp sysroot never cleaned up | 3467 abandoned `%TEMP%\luna*` dirs / 21.3 GB; runs aborted with `Os { code: 112, kind: StorageFull }` at `support/stdlib.rs:34` | Root cause identified + **fixed** |
| Shared mutable output under parallel execution | full run failed `compiler_gap_c_gap_12_safe_loan_effect_tests` with `"Build lock already exists"`; the same binary passed in isolation (4/4), the single test alone (1/1) | **Reproduced** + **fixed** |

The earlier D1-FU2 workspace aborts were this leak, not a compiler regression.

**Second problem root cause**: suite helpers made paths unique from `pid` +
`SystemTime::now()` only. Windows `SystemTime` granularity is coarse (~15 ms), so
two tests running in parallel inside one binary produced the **same** directory —
hence the same `.build.lock` and the same artifacts. `owned_temp_root()` now
returns a unique child per call, so clock resolution cannot alias two tests.

## Ownership model

New crate `luna-test-support` (dev-dependency of `luna-cli` and `luna-driver`):

- **`TempWorkspace`** — RAII owner of a unique temp directory, removed on `Drop`,
  including during panic unwind. Removal is guarded **structurally**, not
  heuristically: the name starts with `luna_test_ws_`, the directory is a direct
  child of the process temp dir, and it contains the marker file this type wrote.
  A real sysroot or a user directory satisfies none of these.
- **`owned_temp_root()`** — one owned root per test process, for the many
  suite-local helpers that hand a `PathBuf` to existing call sites.
- **`sweep_orphan_workspaces()`** — once per process, reclaims workspaces whose
  owning pid is no longer alive. This covers process-lifetime sysroots (which a
  `static` can never drop) and killed runs.

## Changes

- `ProviderModes` now owns a `TempWorkspace`; the hand-rolled `Drop` and its
  path heuristic are gone.
- 108 test files / 123 call sites switched from `std::env::temp_dir()` to the
  owned root; 6 unit-test sites in `luna-driver/src` likewise.
- Not done (deliberately): no `ignore NotFound` in `remove_artifacts()`, no
  compiler semantics / provider protocol / cache-invalidation change.

## Guards

- `luna-test-support` unit tests: drop, panic unwind, no accumulation, retain,
  refuse-to-delete-foreign, sweep reclaims orphans but spares living/markerless.
- `temp_workspace_lifecycle_cli`: sequential `fresh()` drops cleanly; dropped
  workspace leaves no sysroot; panic unwind cleans up; the process root is
  owned and stable; parallel workspaces are distinct; a failing compile still
  leaves the workspace removable.
- D1-FU2 completeness (R5 precondition): an already-diagnosed comptime failure
  is suppressed **only** while a visible error-level root diagnostic exists;
  otherwise the compiler fails closed rather than dropping the failure.

## Acceptance (same parallel mode as the gate)

| Run | Binaries | Passed | Failed | Ignored | Exit | Temp before | Temp after |
|---|---|---|---|---|---|---|---|
| A | 224 | 1383 | 0 | 1 | 0 | 0 dirs / 0 GB | 1 dir / 0 GB |
| B | 224 | 1383 | 0 | 1 | 0 | 1 dir / 0 GB | 1 dir / 0 GB |
| C | 224 | 1383 | 0 | 1 | 0 | 1 dir / 0 GB | 1 dir / 0 GB |

Residual is **bounded residual cleanup**, not full immediate cleanup: one 0-byte
directory owned by the last test process remains and is reclaimed by the next
process's orphan sweep. This is a known infrastructure limitation for
0.1-alpha.1 — it is not a claim that every temp directory is removed the moment a
process exits. The sweep only removes directories named `luna_test_ws_*`, directly
under the process temp dir, carrying our marker file, and owned by a pid that is
no longer alive, so a running process's workspace is never touched.

`--test-threads=1` is **not** used as evidence that the race is fixed; it is at
most a release-gate fallback.

## Versioning

No bump — test infrastructure only (compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**R5-HARNESS-01: CLOSED / FIXED IN TESTED SCOPE.**
