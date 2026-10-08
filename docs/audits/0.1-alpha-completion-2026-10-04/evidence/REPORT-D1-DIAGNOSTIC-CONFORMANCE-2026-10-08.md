<!-- luna-doc-role: evidence -->

# D1 report — DIAG-1..10 diagnostic conformance

Branch `d1-diagnostic-conformance` (from candidate `7b3c5185`).
Contract: `docs/diagnostics/diagnostics-v1.md` (adopted-contract).
Identity = code + phase + span/origin; rendered wording is presentation only.

## Freeze result — one counterexample

**D1-1.** DIAG-10 requires automated parity between the specification and
`DiagnosticCode::ALL`, but **`DiagnosticCode::ALL` did not exist**, so the
documented parity mechanism was unimplementable. **Layer:** diagnostic registry.
**Fix:** added `DiagnosticCode::ALL` (all 68 codes) and `registry_parity` unit
tests asserting that every code is documented, uniquely numbered and named, and
that every exact code row in the registry exists in the enum (range rows such as
`E1000`-`E1999` are excluded).

## DIAG matrix

| Invariant | Result |
|---|---|
| DIAG-1 typed stable code | PASS — parser/resolver/borrowck fixtures coded; no bare `error:` |
| DIAG-2 primary source span | PASS — `file:line:column` + caret on the offending token |
| DIAG-3 structural related spans | PASS — E1008 ambiguity keeps related locations |
| DIAG-4 poison containment | PASS — poisoned cast emits exactly one root diagnostic |
| DIAG-5 deterministic ordering | PASS — repeated `check` is byte-identical |
| DIAG-6 deduplication | PASS — `check` and `build` agree |
| DIAG-7 phase ownership | PASS — E0001/E1001/E3001 in the correct phase ranges |
| DIAG-8 origin traceability | PASS — stale diagnostic names the provider and boundary |
| DIAG-9 source/artifact parity | PASS — identical identity from source and relocated `.llib` |
| DIAG-10 registry parity | **FIXED** — `ALL` + parity tests |

## Harness

`luna-rs/crates/luna-cli/tests/diagnostic_conformance_cli.rs` plus the
`luna-common::registry_parity` unit tests. Existing guards are referenced for
spans, projection, borrow, poison, artifact classification and move diagnostics.

## Non-blocking observations

- **D1-FU1:** the use-after-move (E3001) message renders an internal value name
  (`%v5.0`) — rendering/presentation, not diagnostic identity.
- **D1-FU2:** a comptime failure reports both the root (E1001) and the comptime
  wrapper (E4005); recorded as a possible cross-phase duplicate for review.

## Result

- `diagnostic_conformance_cli`: 4 passed / 0 failed.
- `luna-common`: 10 passed / 0 failed.
- Full workspace: 215 binaries, 1352 passed / 0 failed / 1 ignored, exit 0.

## Versioning

No bump: a registry const and tests only — no serialized diagnostic/origin
metadata change (compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**D1: CONFORMANT IN TESTED SCOPE** — the DIAG-1..10 matrix passes and registry
parity is now enforced by an automated test.
