<!-- luna-doc-role: evidence -->

# D4 report — retained contracts

Branch `d4-retained-contracts` (from candidate `70002d84`).

## Method

Ledger, not a mega-test: re-certify the nine retained contract groups on the
current candidate by re-running the existing acceptance suites and adding
reducers only where coverage was missing (FFI, dyn, entry, UTF-8, lifetime).
Each group has a positive semantic case and a negative/fail-closed case, in
source and artifact modes where applicable.

## Nine groups

| Group | Positive | Negative / fail-closed | Primary suites |
|---|---|---|---|
| lifetime | `lifetime_positive` (move/borrow/second move/rw write) | E3001 `use_after_move` | `generic_drop_cli`, `raw_slice_provenance_cli`, borrowck |
| FFI | `ffi_positive` (`extern fn` + `#[link]`, called) | E2030 `ffi_non_ffi_safe` | `struct_contract_cli_parity` |
| dyn | `dyn_positive` (`&dyn` dispatch) | E2010 `dyn_not_object_safe` | `adv_*_dyn_tests`, `generic_trait_dispatch_acceptance_tests` |
| closure | — | — | `generic_drop_cli` (`closure_env_*`), A1 regressions |
| async | — | — | `a2_suspension_cleanup_tests`, `async_cleanup_cli` |
| comptime | — | — | `comptime_dependency_precision_cli`, `artifact_execution_dependencies_cli` |
| UTF-8 | `utf8_positive` (scalar round-trip) | E2026 `char_invalid` | `stdlib_string_acceptance_tests` |
| entry | `entry_argv` (`fn main(args: [str])`) | E2024 `entry_bad_main` | `diagnostic_conformance_cli` |
| backend | — | — | `fresh_artifact_classification_cli`, `artifact_target_contract_cli` |

`entry` records the allowed signatures from the E2024 message: `fn main() -> void`,
`fn main() -> i32`, `fn main(args: [str]) -> i32`.

## Guard batch (re-certification)

`generic_drop_cli` 1, `async_cleanup_cli` 1, `a2_suspension_cleanup_tests` 3,
`comptime_dependency_precision_cli` 1, `artifact_execution_dependencies_cli` 1,
`provider_tiers_cli` 4, `fresh_artifact_classification_cli` 1,
`retained_contracts_cli` 1 — all passed.

## Findings carried (not hidden)

- **FIND-ASYNC-PROVIDER-01 — promoted to its own task immediately after D4**
  (decision B). The async *runtime* contract passes, but the zero-parameter
  exported `async fn` `.llib` corruption is real artifact corruption reproduced
  twice, so it is not left as an ambiguous observation.
- **A2-FU1** remains **OPEN** (anti-drift debt, must complete before R5). The D4
  async matrix is green, so the A2 contract stays conformant.
- **D4-FU1 (observation):** a source file whose *first* item is `#[link(...)]`
  fails to parse with E0003; the FFI fixture places another declaration first.

## Versioning

No bump: test-only (compiler22 / metadata9 / MVIR5 / format2). Full workspace:
218 binaries, 1358 passed / 0 failed / 1 ignored, exit 0.

## Next sequence

`FIND-ASYNC-PROVIDER-01` -> `A2-FU1` -> decide `C4-FU1` / `D1-FU1` / `D1-FU2`
-> E-wave / R4 -> R5 exact release gate.

## Verdict

**D4 — Retained Contracts: CLOSED / CONFORMANT IN TESTED SCOPE** — all nine
groups covered with no newly regressed behaviour, with
`FIND-ASYNC-PROVIDER-01` explicitly excluded and promoted.
