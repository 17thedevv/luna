# R5 qualification report — integrated implementation pin

**Pin (tested implementation)**: `8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5` (branch `r5-diag-repair`)
**Harness / gate commit**: `aa3ed8abc2f9001e35530ca3ed473191c40a80ac`
**Docs reconciliation commit**: `eb2b37a9`
**Candidate**: `c085449b207b5860b1d7a79bee221c2ff3d357c3` (not advanced yet) · **main**: `401a95a8` (untouched)
**Versions**: compiler 25 · semantic metadata 9 · MVIR 5 · format 2 (unchanged across the wave)
**Status**: WIN 8/8 lanes green? — see the lane table; two Windows workspace lanes and CI artifact digests remain open.

## Integration

Two divergent repair lines were integrated onto one implementation SHA before
qualification (no PASS was carried from any earlier pin):

| Line | Content |
| --- | --- |
| `r5-diag-repair` base `5d20e571` | DIAG-1..10 wave, canonical target selection, typed comptime `E4002`/`E4003`, `run` keeps compiler diagnostics (no `E6009` reclassification), per-code coverage suite |
| `r5-missing-return` `ce886d09` | duplicate-definition/candidate handling, language-item and coherence ownership, symbol/registry identity, semantic-table and metadata anchors, declaration location suite + fixtures |

Merge commit `8525067f` (clean, no conflicts). Verified on the merged tree:
`cargo check` clean; `diagnostic_declaration_locations_cli` 3/3; per-code
coverage suite; `luna-common` 15 (registry parity + per-code identity/span
policy); `luna-semantic` 27; docs validator 0 errors; inventory drift check 0.

**Retraction**: the earlier claim that `E3004` had no producer was an artifact of
the divergent base. At the integrated tree a probe reports `E3004` for both
`fn f() -> i32 { }` and a declaration-only body, `missing_return_cli.rs` covers
it, and the inventory records it as produced with an anchored producer plus
automated coverage.

**Producer inventory**: 72 codes — produced 64 / reserved-without-producer 8
(`E0004`, `E1006`, `E1007`, `E2014`, `E2015`, `E2020`, `E4004`, `E5002`) /
declared-unreachable-in-test 4 (`E3007`, `E6002`, `E6003`, `E6009`) / coverage
debt 0. Each entry carries its reason; nothing is silently dropped.

## Lanes at the pin

| Lane | Where | Verdict | Evidence |
| --- | --- | --- | --- |
| Windows native normal | local runner `D:/r5-8525067f/windows` | PASS | build PASS; focused 12/12 exit 0; diagnostics PASS; quickstart PASS; comptime exit 0; strict L002 replay 4/4 (`evidence/pin-8525067f/strict-bounds/`) |
| Windows workspace triage | local runner | re-running | attempt 1 interrupted by the host session (preserved); attempt 2 hit `LNK1207` on stale PDBs (preserved); attempt 3 clean so far |
| Windows workspace final | local runner | scheduled after triage | — |
| Ubuntu normal | CI run 23 job 117679870941 | PASS | success 12:49:55Z → 14:38:08Z |
| Ubuntu runtime ASan | CI run 23 job 117679870857 | PASS | success 12:49:54Z → 12:50:41Z |
| Ubuntu comptime/VM | CI run 23 job 117679870976 | PASS | success 12:49:55Z → 12:54:39Z |
| Ubuntu workspace triage | CI run 23 job 117679870999 | PASS | success |
| Ubuntu workspace final | CI run 23 job 117720597415 | PASS | success 15:57:58Z → 19:05:54Z |

Additional repo workflows at the same pin: Full workspace regression (Ubuntu),
Whole-File I/O parity (macos-15-intel), RuntimeAbiTests (ubuntu-24.04 and
macos-15-intel) and Runtime partial-read cleanup (Ubuntu + ASan) all SUCCESS.

## Evidence verification

- Local lane digests: `docs/tools/verify_lane_evidence.py` recomputes every
  recorded log/archive digest — 19 digests, 0 mismatched, 0 missing.
- CI artifacts: **pending** — ZIP digest verification needs an authenticated
  download; lane verdicts were read from the check-run API, which is not a
  substitute for verifying the archives.

## Known limitations carried into the release

- Comptime/layout: eight probes stay explicit characterizations (historical
  final-if statement spelling), not conformance claims.
- ASan certifies the sanitized C runtime and RuntimeAbiTests only; generated
  Luna/compiler/stdlib code is not claimed sanitized.
- Four registered codes have no reproducible test trigger yet (`E3007`,
  `E6002`, `E6003`, `E6009`) and eight have no producer in 0.1; both sets are
  declared with reasons in the inventory.
- Ubuntu CI artifact archives still need digest verification before closure.

## Remaining before closure

1. Windows workspace triage and final at this pin.
2. CI artifact archive digest verification (needs an authenticated download).
3. Then: fast-forward the candidate to this pin, sync public docs/status, and
   hand off; tag, main merge, PR and publication stay with the maintainer.