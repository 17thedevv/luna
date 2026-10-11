<!-- luna-doc-role: evidence -->

# E2 report — grammar / spec reconciliation

Branch `e2-grammar-spec-reconciliation` (from candidate `c750faec`). Scope:
documentation, evidence and characterization only — **no grammar expansion and no
compiler semantics change**.

## Reconciled rows (`docs/spec/0.1/gaps.md` 40–43)

| ID | State |
|---|---|
| V01-GRAMMAR-01 | Semicolon field delimiters reject with typed `E0002` + primary span (both modes); comma fields accept and run |
| V01-GRAMMAR-02 | `for (pattern in expression)` is the canonical 0.1 head and runs in both modes. The unparenthesized head is kept only as **LEGACY / CHARACTERIZATION** — not standardized, not used by stdlib or official docs. C-style `for` is unaffected |
| V01-GRAMMAR-03 | `&self`, `&rw self`, bare `self` build and run in both modes; duplicate/non-first receivers reject with a typed code |
| V01-DIAG-01 | Evidence-backed parts closed; the structural invariants remain **R5 preconditions**, not deferred |

## Evidence (regenerated, not hand-edited)

`docs/tools/check_v01_examples.py` was re-run against the candidate:

- `docs/spec/0.1/evidence/cli-examples.json` — 16 attempts, 14 met, 2
  characterization (`foreach_current_parser`).
- `docs/integration/evidence/cli-examples.json` — the same 16 attempts.
- `docs/spec/0.1/evidence/historical-2026-10-04/` and
  `docs/integration/evidence/historical-2026-10-04/` preserve the pre-fix record
  (the same three cases unmet). History is kept as history and never rewritten to
  look like a pass.
- `docs/tools/validate_v01_docs.py` now asserts both the reconciled current
  evidence and that the historical record still shows the gaps.

### Negative parser case

`reject_semicolon_fields` renders `error[E0002]` with a primary span at the
offending field (`...ln:2:22`), in both source and artifact modes. Asserted in
`v01_grammar_acceptance_tests` (span presence, both modes).

## Tests

| Suite | Result |
|---|---|
| `v01_grammar_acceptance_tests` | 7 passed (semicolon code+span both modes; canonical parenthesized foreach both modes; legacy characterization; C-style for unaffected; receiver shorthand) |
| `luna-common registry_parity` | 2 passed |
| `diagnostic_conformance_cli` | 4 passed |
| docs validator | 0 errors |

## Workspace

`cargo test --workspace --no-fail-fast` → **224 binaries, 1384 passed / 0 failed /
1 ignored, exit 0** (parallel mode; temp 0 dirs / 0 GB → 1 dir / 0 GB).

## Versioning

No bump — documentation, evidence and characterization only.

## Verdict

**E2: CLOSED / FIXED IN TESTED SCOPE.**
