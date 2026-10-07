<!-- luna-doc-role: evidence -->

# A2-FU2 report — CLI async cleanup matrix

Branch `a2-async-suspended-cleanup`.

## Purpose

Complete the CLI source/fresh-artifact matrix for async cancellation cleanup
(A2-FU2), strengthening release evidence for the A2 correctness fix.

## Harness and fixtures

- `luna-rs/crates/luna-cli/tests/async_cleanup_cli.rs` runs the fixtures in a
  source-only sysroot and a freshly built artifact-only sysroot.
- `tests/luna/language/async_cleanup/`:
  - `value_across_suspend` — an owned value captured by a future and cancelled
    before polling is destroyed exactly once.
  - `borrow_only_across_suspend` — cancelling a borrowed capture does not destroy
    the pointee.
  - `generic_owned_across_suspend` — two generic owned captures are each dropped
    exactly once.

## Result

3/3 positive fixtures build and exit 0 in **source and fresh artifact-only**
modes.

Focused suites re-run: `a2_suspension_cleanup_tests` 3 passed, `p1c_tests` 15
passed.

## Observation (separate finding)

A provider exporting a **parameterless** `async fn` produced
`Invalid semantic metadata: CorruptedData` when its `.llib` was imported. The
provider now uses parameterized async fns (`future_consume` / `future_pair`),
mirroring the already-artifact-safe `resource.ln` shape. This is recorded as a
separate item to investigate; it does not block A2-FU2, which passes with the
supported pattern.

## Scope note

CLI fixtures cancel before polling (state 0). Suspend-state cleanup
(conditional/partial/projected, cancel at state #1/#2) is covered by the
driver-level `a2_suspension_cleanup_tests`.

## Verdict

**A2 remains CLOSED / CONFORMANT IN TESTED SCOPE; coverage strengthened.**
