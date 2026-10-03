# Luna 0.1 — implementation gaps and specification debt

Baseline date: 2026-10-02; implementation revision: `3dac3ac`.
Audit IDs below are local to [stdlib-2026-10-02](../../audits/stdlib-2026-10-02/README.md),
not substitutes for earlier compiler-gap IDs from unrelated phases. Existing
contracts are retained. No row here grants permission to weaken them.

## Confirmed audit defects

| Audit ID | Priority | Requirement violated / observed failure |
|---|---|---|
| C-GAP-01 | P1 | Deterministic method resolution: forwarding trait/inherent collisions can compile into recursion and stack overflow |
| C-GAP-02 | P1 | Source-aware integer widening: dynamic 255u8 widens by sign extension; Convert/Display inherit incorrect values |
| C-GAP-03 | P2 | Entry argument bridge: main receives an empty slice despite argv input |
| C-GAP-04 | P2 | Import metadata identity: one bound becomes 2,048 copies over a 12-level provider chain; storage import can timeout |
| S-01 | P1 | Vec allocation/reserve arithmetic can wrap while preserving invalid usable capacity metadata |
| S-02 | P2 | RawTable capacity round-up can wrap to zero and loop indefinitely |
| S-03 | P2 | Shared/mutable slice iterators return no elements for a nonempty ZST slice |
| S-04 | P1 | Safe String char encoding can violate the UTF-8 invariant |
| S-05 | P2 | Filter/find consume non-Copy Item through the predicate and subsequently use it; borrow checker correctly rejects |
| S-06 | P1 | Generic Vec.dedup_i32 reinterprets arbitrary T as i32 and produces wrong results |
| S-07 | P2 | Range<T: Step> moves a non-Copy current value before using it to advance |
| A-01 | P2 | Semantic compiler code recognizes SliceIter/SliceIterMut names and fabricates stdlib layout |

Exact triggers, code locations, logs and limits remain in the audit report.
Potential heap corruption inferred from invalid storage is distinct from a
deliberately executed invalid write. Timeout attribution is limited to the
instrumented pipeline evidence, not a claimed full profile of every old suite.

## Grammar and document inconsistencies

| ID | Contract / correction | Current implementation or documentation |
|---|---|---|
| V01-GRAMMAR-01 | Comma-separated struct fields; reject semicolon delimiters | CLI confirms both modes accept and execute semicolon-delimited fields despite the rejection requirement |
| V01-GRAMMAR-02 | Retained parenthesized foreach head from normative Rule K.4 | CLI confirms both modes reject the contract head and accept the unparenthesized implementation form; reconciliation must retain an explicit decision |
| V01-GRAMMAR-03 | Receiver shorthand equivalence from Rule K.5 | CLI confirms both modes reject &self shorthand; the complete equivalence contract remains a requirement |
| V01-DIAG-01 | DIAG-1 requires typed diagnostic codes for every compiler error | The foreach/receiver parser failures render `error:` without a code in both modes; the private-field rejection correctly renders E1003. Code identity/propagation still needs a focused diagnostic audit |
| V01-DOC-01 | Visibility-02: fields default public independently of type accessibility | Old grammar skill/reference says private by default; corrected to the approved amendment |
| V01-DOC-02 | `requires life(...)` is the current relation spelling | Older docs/skills use removed `where outlives`; current guidance is corrected without changing the relation model |
| V01-DOC-03 | Luna .ln/.llib, 32 providers, six language contract families plus OptionExt contribution | Old overview/bootstrap pages claim Mellis core.ms, 25 providers or four families; current entry points are replaced |

V01-GRAMMAR rows are conformance gaps or unverified contracts, not new parser
fixes made by this documentation task. Historical plans retain their original
examples under a historical-role notice.

## Decisions still required

| ID | Question / boundary |
|---|---|
| V01-DESIGN-01 | Exact inherent/trait method collision precedence or ambiguity policy; hash iteration cannot decide it |
| V01-DESIGN-02 | Domain of char and failure policy for invalid literal/dynamic integer-to-char conversion; UTF-8 text sinks must remain valid meanwhile |
| V01-DESIGN-03 | Runtime integer overflow policy outside defined checked/compile-time operations; avoid declaring current LLVM wrapping normative |
| V01-DESIGN-04 | Complete formal grammar beyond the consolidated productions; parser acceptance of legacy/uncontracted syntax is not adoption |

No new format-macro hook, interpolation syntax, runtime opcode or container
language item is adopted. These require their own complete generic contract.

## Evidence not completed

- All compiler integration tests, exhaustive input/generic domains, sanitizers
  and fuzzing have not been verified by the stdlib audit.
- Storage HT5/HT6 internal suites still timeout; public growth and 34-drop
  controls pass in both modes. Import metadata inflation is reproduced.
- Async cancellation, all comptime/resource boundaries, dynamic dispatch and
  every adopted lifetime contract need their own current acceptance matrix;
  old freeze decisions remain contract authority, not new test evidence.
- Extreme 128-bit construction/formatting, POSIX whole-file behavior and the
  full hosted/freestanding target matrix remain limited/unverified.
- HashMap has measured collision scaling; allocation counts, optimizer pipeline
  effects and broad stdlib performance are not fully characterized.
- Existing public_surface and reject_std_vec fixtures contain stale expectations;
  they must be corrected as tests, not treated as reasons to alter valid APIs.


## Documentation CLI characterization — 2026-10-03

[evidence/cli-examples.json](evidence/cli-examples.json) records eight standalone
fixtures in source-only and artifact-only sysroots, 16 attempts total. Eight
attempts meet their positive/negative requirements; six expose the three grammar
gaps above; two characterize the accepted unparenthesized implementation form.
The custom outln macro returns a value and emits no output, demonstrating that
this name currently uses ordinary expansion. These checks are not a certification
of every macro, field visibility or language capability.
