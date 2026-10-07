<!-- luna-doc-role: evidence -->

# A4 report — cast / poison containment

Branch `a4-cast-poison-containment` (from candidate `c3e55f4f`).

## Method

Freeze matrix first (no compiler change): build reducers for invalid static
casts in every position named in the A4 plan and check that the failure is
contained at the semantic layer. No counterexample was found, so A4 closes as a
**coverage certification** rather than an implementation fix.

## Invariant under test

An invalid static cast must:

1. fail in the semantic/type layer with **one** typed diagnostic (E2026);
2. poison the operand (`error` type) so downstream uses do **not** cascade;
3. never reach MVIR or the backend;
4. never publish an executable.

A cast the contract defines as runtime-trapping (integer-to-char with an
out-of-range runtime value) must instead stay well-typed and trap
deterministically.

## Reducers (`luna-rs/crates/luna-cli/tests/cast_poison_cli.rs`)

- Invalid cast in six positions: nested expression, call argument, return,
  condition, comptime, and a value feeding many downstream uses.
- Invalid cast in a provider body (source + artifact).
- Runtime trap: in-range integer-to-char runs (exit 0); out-of-range builds and
  traps at runtime.
- Valid casts (widening, narrowing, identity pointer) across a provider `.llib`
  source/artifact round trip.

## Checks

- `check` and `build` exit 1 with E2026 in source and fresh artifact modes.
- Exactly **one** error line per invalid cast — no cascade.
- No executable produced; `--emit mvir` fails and writes no `.mvir`.
- Provider-body invalid cast rejected where the provider is compiled.
- Runtime-trapping cast builds (well-typed IR) and traps at runtime (non-zero).

## Result

`cast_poison_cli`: **5 passed / 0 failed**. Existing `cast_contract_cli` already
covers E2026 / E2001 / E2025 / E1003 / E3001 and control casts (numeric
widening/narrowing, shared/mutable references, pointer/reference, function
pointers). Full workspace: 208 binaries, 1340 passed / 0 failed / 1 ignored,
exit 0.

## Verdict

**A4: CLOSED / CONFORMANT IN TESTED SCOPE** — semantic-layer rejection + poison
containment + deterministic runtime trap + provider artifact round trip.
