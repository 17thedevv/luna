<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../../docs/spec/0.1/README.md).

# C-GAP-12: Safe-Loan / Raw Provenance Conflation in Effect Inference

**Status:** RESOLVED & FROZEN — 2026-09-26
**Category:** Generic compiler semantic defect
**Registry note:** C-GAP-11 is already allocated to the frozen process-argument contract. C-GAP-12 is the next unused canonical ID found in the repository and its compiler skills/history.

## Problem

Interprocedural effect inference conflated generic/data/raw-pointer provenance with safe-loan carried provenance. This allowed data derived from a borrowed input to be summarized as a safe borrow from that input when the returned owned type contained raw-pointer representation fields. The resulting summary caused false-positive borrow conflicts in source-loaded provider graphs.

The defect was not a general loss of borrow contracts in `.llib`: an explicit safe-reference contract continued to reject conflicting mutation in both fresh source and fresh artifact consumer sessions.

## Frozen Semantic Invariant

```text
Data provenance != raw-pointer provenance != safe-loan provenance
```

- A scalar value may remain data-dependent on a borrowed source, but that dependency does not constitute raw-pointer provenance or safe-loan provenance.
- Storing a scalar does not make its destination a safe-loan carrier.
- Raw pointers may retain pointer provenance for raw/unsafe reasoning, but do not implicitly carry a Luna safe loan.
- Borrow analysis represents raw origin as `RawPointerOrigin::FromPlace { place, is_rw }`, `Null`, or `Unknown`, in a separate state map from safe `Loan` sets. A control-flow join preserves unknown alternatives. Proven raw-to-safe construction still requires a known compatible origin or a declared owner anchor plus lifetime/mutability contract, without rewriting raw origin into the owner loan. `Null` remains non-promotable. An `Unknown` raw value may additionally create a temporary safe loan rooted at a function-local `UnsafeRawRoot` only at a raw-to-safe operation that has passed Luna's unsafe-context checks; this root is not place provenance, an owner anchor, or an exportable lifetime source.
- Safe references and by-value aggregates containing safe references preserve the safe loans they actually carry.
- A raw-pointer-backed abstraction that semantically borrows from an argument must declare that relationship explicitly with `life_from(...)`; representation alone never creates a safe loan. HashMap/HashSet/RawTable borrowed iterator constructors and generic iterator adapters now state/forward this contract.
- `Iterator::next` declares `life_from(self)` for borrowed results. Impl conformance is required only when the instantiated return type can carry a safe reference; scalar/owned results do not acquire a safe loan from the contract.
- A return summary reports `BorrowsFrom` / `BorrowsCarried` only from the safe-loan provenance reaching the returned value. A return type merely being capable of holding a safe reference is not proof that every generic taint reaching it is a borrow.
- If the semantic return type cannot carry a safe reference, a non-empty safe-loan set at return is an inconsistent analysis state.

## Type Queries

`contains_pointer_or_reference(T)` is the representation/capability query and continues to include raw pointers, safe references, and nested aggregate fields. The legacy `contains_reference(T)` API retains that historical meaning for compatibility.

`contains_safe_reference(T)` answers the distinct lifetime question. It recognizes safe references and recursively follows by-value semantic contents, but does not treat a raw pointer or a raw-pointer-backed container as a safe-loan carrier merely because its representation contains a pointer.

FFI admissibility remains governed by its existing explicit checks; this gap does not weaken E2030 policy for aggregates containing either raw pointers or safe references.

## Acceptance Matrix

| Case | Required effect / behavior |
| --- | --- |
| `load_byte(&[u8]) -> u8` | `Independent` |
| `copy_byte(&[u8]) -> OwnedByte { value: u8 }` | `Independent` |
| `raw_passthrough(*u8) -> *u8` | No safe `BorrowsCarried` effect |
| `raw_passthrough(*u8) -> *u8` | Separate raw return summary `RawFrom([0])` |
| Raw-pointer field load without stored-value facts | Raw origin is `Unknown`; field-address origin is not the pointer-value origin |
| Raw field returned through a helper without stored-value facts | `RawPointerReturnEffect::Unknown` |
| `load_byte(&[u8]) -> u8` / `copy_byte(&[u8]) -> OwnedByte` | No raw-pointer return provenance |
| `borrow_value(&T) -> &T life_from(value)` | `BorrowsCarried([0])` |
| `Pair { r: &T, n: u64 }`, where `n` is read from another borrowed input | Summary carries only the parameter that supplied `r` |
| `string_from_bytes(&[u8]) -> Option<String>` source body | `Independent` |
| Contracted `owner.ptr -> &T life_from(owner)` | Anchor + explicit lifetime creates a loan on logical owner; raw origin remains distinct |
| Contracted null sentinel -> `&T` | Reject; `RawPointerOrigin::Null` is not promotable |
| `Unknown` raw -> local safe reference inside unsafe | Allowed; local borrow rules apply, loan is rooted in function-local `UnsafeRawRoot` |
| Two shared loans from the same `UnsafeRawRoot` | Allowed while both live |
| Mutable + shared / mutable + mutable from one `UnsafeRawRoot` | `BorrowConflict` while live ranges overlap |
| `UnsafeRawRoot` returned directly or in an aggregate | Reject with `LocalBorrowEscape` |
| `UnsafeRawRoot` stored in non-local state | Reject with `LocalBorrowEscape` |
| `UnsafeRawRoot` passed to a call | Allowed with a body-derived non-escaping (`NoEscape`/`CallOnly`) summary, or when the canonical callee signature takes a safe reference and its result cannot carry a safe loan; opaque raw-pointer/indirect/MayEscape calls reject |
| `String.as_bytes()` live borrow | Conflicting mutation rejected from source and fresh `.llib` |
| Minimal path/source-closure regression | Source and artifact modes both compile/run with identical observable behavior |

## Implementation Scope

The fix is type-shape- and provenance-domain-based. It does not special-case standard-library providers or named types. Effect inference has distinct safe-loan and raw-pointer return domains (`ReturnEffect` versus `RawPointerReturnEffect`) and records raw pointer VALUE facts in `TaintState::raw_pointer_storage`, keyed by function-local `PlaceDesc`; untracked loads become `RawPointerSource::Unknown`. Borrow analysis separately stores `RawPointerOrigin` from safe `Loan` state. Function-local `unsafe_raw_roots` preserve unknown raw-value lineage through pointer copies, address casts/arithmetic, local storage, and summarized pointer-return calls; roots are analysis IDs only. A raw-to-safe conversion from a singleton unknown lineage creates an ordinary local `Loan` tagged in `BorrowStateData::unsafe_raw_root_loans`, not a `FromPlace` origin or safe-return effect. NLL overlap permits shared/shared and rejects overlapping pairs involving a mutable loan. Return, aggregate return, non-local store, and unproven/opaque call escape are rejected. For a direct call with no body summary, the fallback consults the canonical semantic function type (not the lowered pointer-shaped argument): a safe-reference parameter is a non-escaping call boundary only when the result type is provably unable to carry a safe loan. Generic/projection/reference-bearing results remain conservative. The built-in fat-slice `data` field is the language-defined exception to ordinary pointer-field loading: extracting it retains the actual safe-reference origin because the slice representation defines it as the referent's element pointer. This does not infer origin from the field-slot address and does not apply to user-defined pointer fields. No raw `ValueId`/`PlaceId` or session-local identity is serialized as portable artifact metadata; analysis state is rebuilt per current function/session.

## Evidence

Compiler-internal acceptance tests live in `crates/luna-driver/tests/compiler_gap_c_gap_12_safe_loan_effect_tests.rs`. Source/artifact parity for the original minimal consumer is maintained in `crates/luna-driver/tests/stdlib_path_acceptance_tests.rs` with the fixture `tests/luna/stdlib/path/source_closure_borrow_conflict.ln`.

The C-GAP-12 focused suite has permanent adversarial cases for unknown raw pointers, unknown raw-pointer fields, unknown fields returned through both safe-reference and raw-pointer helpers, `From(B)` stored in A's field then claimed as `life_from(A)`, `From(A)` claimed as `life_from(B)`, `From(A) | Unknown`, and `From(A) | From(B)`. The field cases are compiled against a fresh source-only sysroot and a fresh artifact-only sysroot with an imported string provider. It also checks matching `From(A) + life_from(A)` through raw copy/move and an interprocedural identity helper. Anchor-specific fixtures cover safe same-owner stores, unsafe establishment, pointer offset via `ptr::add_mut`, shared and mutable promotion, move rebasing, stale pointer invalidation, and rejection of CFG safe stores whose alternatives are `Unknown` or anchored to another owner. Generic summary probes cover scalar load/store, raw passthrough and helper summaries, real safe references, Pair-like aggregates, and owned `string_from_bytes`.

The focused C-GAP-12 `.ln` fixtures now exercise local transient conversion, a body-summary non-escaping helper, a trait-dispatched safe-reference-only call, direct/aggregate/unrelated-lifetime escape rejection, a reference-bearing trait-return escape, shared/shared coexistence, mutable conflict, NLL loan death, and lineage through a raw-pointer copy plus `ptr::add_mut`. Effect-level tests assert that unknown raw-to-safe conversion does not become `ReturnEffect::BorrowsFrom`/`BorrowsCarried`. The compiler-internal borrowck suite and focused driver suite pass. Runtime ABI is not needed unless runtime code changes.

## Current Closure Status

The final soundness check confirmed that a field-slot address cannot stand in for the raw pointer value stored there. The new SEM-GAP-21 type-level contract supplies a distinct logical owner anchor; `life_from(...)` remains the RegionEngine lifetime relation and does not fabricate the raw address-origin fact. Unsafe construction/store is represented by one-shot `StoreAnchored` MVIR, while safe stores must prove the same anchor. Raw pointer field facts also propagate through pointer locals and aggregate value moves without session identity entering artifacts. A contracted field loaded at a function/artifact boundary may reconstruct its logical owner anchor from the public type invariant; its raw origin remains independently `Unknown`. Null sentinels preserve a distinct non-promotable raw-origin fact.

The generic trait-dispatch boundary is handled without provider-name branches. A direct-call fallback reads the canonical function signature so a formal safe reference is not misclassified as a raw pointer merely because MVIR represents both as pointer-shaped values. It permits the temporary loan only when the return type cannot carry safe-loan provenance; a reference-bearing or unresolved generic result remains rejected. Permanent tests cover both `std::Ord::cmp`-style local use and a trait call whose return can carry a reference. Generic `Vec<T>` and `Box<T>` now declare direct-field `requires anchor(ptr) = self` contracts for their owned allocations; their constructors/reallocation/extraction establish or clear those owner-relative anchors only at explicit unsafe boundaries. The full Phase 4B slice suite verifies safe-slice data-pointer promotion for shared and mutable helpers, sorting, and reverse without weakening ordinary unknown-field rejection.

## Aggregate raw-field continuation closure

Direct struct-field raw-pointer summaries now use canonical field names and parameter-relative raw origin/anchor effects. Local source inference proves `pack(*rw T) -> Pair { ptr: *rw T, count: u64 }` and a relay helper both return `ptr: RawFrom([0])` without treating `count` as pointer provenance. Imported source structs retain direct field names in-session. For non-generic exported functions whose bodies are stripped from AstInterface, semantic metadata v3 carries the raw return/field effect as stable function path + parameter index + canonical field name; the consumer reconstructs a fresh call summary and keeps other call-effect channels conservative. The focused C-GAP-12 suite passes 4/4 with no ignored parity gate.

`aggregate_raw_field_origin_survives_call_and_fresh_provider_artifact` is now a permanent parity regression and verifies its canonical artifact payload before exercising the real artifact-only resolver path. The `.llib` writer still strips non-generic function bodies from AstInterface; the current versioned SemanticMetadata section therefore exports the distinct raw-pointer effect channel in a stable form. The interface fingerprint includes this payload, old semantic metadata is rejected by the semantic metadata version gate, and raw session IDs are never serialized.

The former trait-dispatched sorting gate is resolved by the canonical safe-reference-only call-signature rule described above. The workspace exposed and closed missing owner contracts on `Vec<T>` and `Box<T>`, plus missing raw-origin propagation when reading the built-in fat-slice element pointer. The focused C-GAP suite passes 4/4; `luna-borrowck`, the RawTable and core/alloc link regressions, path source/artifact parity, iterator adapter/deep-chain and Vec lifetime suites, Phase 2E/FFI, Phase 4B collection/slice/sorting suites, Phase 4C numeric and artifact-parity suites, and sysroot invariants all pass. Canonical provider `.llib`/`.obj` artifacts were rebuilt from the current sources before the final workspace run. Final workspace evidence: `cargo test --workspace -- --test-threads=1` — **exit code 0** (all integration and doc-test binaries completed). Runtime ABI was not rerun because no runtime source changed. C-GAP-12 is RESOLVED & FROZEN by maintainer decision.
