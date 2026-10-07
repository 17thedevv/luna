<!-- luna-doc-role: evidence -->

# A5 report — raw slice / view contract

Branch `a5-raw-slice-view-contract` (from candidate `7aee1528`).

## Method

Freeze matrix first (no compiler change). The invariant under test is that a
**raw pointer/slice projection is not an owned aggregate subplace** — the exact
seam A3 touched when it changed `PtrOffset`/`Load` tracking. No counterexample
was found, so A5 closes as a **coverage certification**.

## Reducers (`luna-rs/crates/luna-cli/tests/raw_slice_view_a5_cli.rs`)

Negatives (source + fresh artifact, `check` and `build` must fail):

| reducer | situation | expected |
|---|---|---|
| `a5_shared_view_rw_conflict` | shared view alive, backing written | E3003 |
| `a5_mut_view_competing_read` | mutable view alive, backing read | E3003 |
| `a5_projected_view_conflict` | view derived by pointer arithmetic, backing written | E3003 |
| `a5_view_escape_backing_lifetime` | view returned past backing lifetime | E3005 |

Positives (build and run, exit 0): shared-view read, mutable-view mutate,
multiple shared views, slice copy that does not transfer backing ownership,
projected (sub-slice via pointer arithmetic) read, zero-length slice.

Trap: `a5_slice_index_oob_trap` builds (well-typed IR) and traps at runtime.

## Guard

`raw_slice_provenance_cli` is the mandatory guard (A3 proved that changing
`PtrOffset` tracking can silently drop `slice_middle_conflict`'s E3003). It stays
green.

## Result

- `raw_slice_view_a5_cli`: 1 passed / 0 failed.
- `raw_slice_provenance_cli`: 1 passed / 0 failed.
- `cargo test -p luna-borrowck`: all binaries pass / 0 failed.
- Full workspace: 209 binaries, 1341 passed / 0 failed / 1 ignored, exit 0.

## Verdict

**A5: CLOSED / CONFORMANT IN TESTED SCOPE** — conflict rejection, lifetime
escape, projected/sub/zero-length views, slice copy, and OOB trap all hold in
source and fresh-artifact modes.
