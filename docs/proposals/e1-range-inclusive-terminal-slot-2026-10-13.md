<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../spec/0.1/README.md).

# E1 decision: RangeInclusive at the terminal value

Status: **ADOPTED — maintainer approved decision A on 2026-10-13; implementation qualification in progress**.
Observed source pin: `939d53e0ad7719c7c4a3eb4b915173fedeb1a129`.

The default-parallel workspace run stops at `test_range_and_inclusive_boundaries`:
138 binaries, 877 passed / 1 failed / 1 ignored, exit101. At `254u8..=255u8`,
`RangeInclusive::next` marks the iterator exhausted, then calls
`current.step_forward()` to manufacture a replacement for the owned terminal
value. The ordinary `255u8 + 1u8` correctly traps L005. This is an iterator
algorithm incompatibility with checked arithmetic, not a missing LLVM guard.

`RangeInclusive.current: T`, `end: T`, and `exhausted: bool` are explicitly
public fields. `Step` provides `step_forward(&Self) -> Self` and comparisons,
without a Copy, Clone, Default, or optional-terminal-extraction contract.
Moving `current` out of `&rw self` without a replacement would leave an invalid
owned slot. Advancing solely to replace the terminal value is unnecessary and
fails at MAX. A raw read would leave alias/double-drop hazards; adding Copy/Clone
bounds would lose the existing non-Copy capability.

## Adopted decision A

Change **only `RangeInclusive.current` from `T` to `std::Option<T>`**.
The constructor puts `Some(start)` in the slot. `next` borrows the current item
for comparisons; it creates a successor only when current is less than end.
At the last item, it marks `exhausted`, replaces the slot with `None`, and returns
the old owned `Some(T)`. Repeated `next` returns `None`.

- `Step` signatures and checked integer implementations remain unchanged.
- `Range<T>` is unchanged; non-Copy/non-Clone steps remain supported.
- No new intrinsic, compiler branch, runtime ABI, MVIR encoding, or metadata
  schema is needed for this slot amendment. The later independently approved
  generic enum alignment repair bumps compiler24 → 25; metadata9/MVIR5/format2
  remain unchanged. See the [alignment continuation](../audits/0.1-alpha-completion-2026-10-04/evidence/REPORT-E1-ENUM-ALIGNMENT-2026-10-13.md).
- Public source compatibility changes for direct struct literals and field
  access: `current: value` becomes `current: Some(value)`; direct readers must
  handle Option. The constructor and Iterator call surface stay unchanged.
- The public nominal layout/interface fingerprint changes. Rebuild the sysroot
  and dependents; strict dependency validation must reject stale layouts.

[Isolated ordinary-Luna prototype](../audits/0.1-alpha-completion-2026-10-04/evidence/range-inclusive-option-prototype-2026-10-13.ln)
passes debug/source, debug/artifact dependencies, release/source, and
release/artifact dependencies. Each compile and native execution exits0.
It covers `254u8..=255u8` and a non-Copy Counter at `i32::MAX`, exactly three
observable drops for two yielded values plus the stored endpoint. It does not
invoke step_forward on the last value. This is feasibility evidence on a user
type, not certification of a changed canonical provider; full provider parity,
all widths/edge cases and drop controls follow approval.

An initial borrowed-enum-pattern prototype rejected with E2001/cascades. The
working prototype uses the existing Option.as_ref API, without introducing
unsafe into the iterator algorithm. That failed development observation remains
separate; no borrowed-pattern compiler capability is claimed.

## Alternative decision B

Keep the current public fields/API and define **builtin integer Step.forward at
MAX to wrap via explicit std::wrapping_add**. This preserves the former modular
replacement behavior and keeps generic non-Copy Step implementations, but
introduces a public modular Step boundary policy. It must be explicitly adopted;
the old unchecked implementation alone is not design authority. Custom Step
types can still fail while producing a terminal replacement, so this does not
provide a general terminal-extraction guarantee.

## Authority and next gate

The task requires isolation/review for a new normative conflict or unapproved
unchecked behavior. The stdlib design protocol requires an explicit public
contract and ownership transition before implementation; it does not itself
require permission for every representation edit. Permission is required here
because these fields are public and decision A changes their type, while B
decides previously unspecified boundary behavior.

The maintainer selected **A — Option slot, keep Step checked**. Implement in ordinary Luna, validate all supported integer
limits and non-Copy drops through source/fresh artifacts, rerun the failed range
suite, pin the final source and restart the three full workspace gates. Candidate
remains `6017ba8d8db40cf0bc16498f639e5d830c0e827c`. E-wave is not CLOSED.
