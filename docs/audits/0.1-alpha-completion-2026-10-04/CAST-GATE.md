# Cast identity, capability and source diagnostics — 2026-10-05

Before-repair status: **BROKEN / RELEASE BLOCKER** on commit
`9558701462a36e1699a70bd25defb3e53d20f68c`. These are newly reproduced
requirements in the active R0–R5 plan, not adopted language extensions.
Compiler code is unchanged while the pinned workspace run finishes.

## Public counterexamples

The [identity observations](evidence/cast-identity-before-observations.json.gz)
and [boundary observations](evidence/cast-boundary-before-observations.json.gz)
pin the CLI, runtime and check/build/native commands on Windows GNU.
All five cases below are accepted by check and build, then execute with exit0:

| Case | Observed violation | Contract |
|---|---|---|
| shared_to_mutable | `&i32 as &rw i32` writes an immutable local | Reference capability and immutable storage |
| nominal_privacy | Secret casts to an unrelated same-layout Public struct and reads its private payload | Nominal type identity and VIS-STRUCT |
| unsafe_callable_erasure | Unsafe function casts to safe fn and invokes without unsafe | Callable safety must survive value transport |
| reference_reinterpret | `&i32 as &char` reads surrogate D800 as a safe char | Referent type identity and Unicode scalar validity |
| raw_reference_without_unsafe | Raw pointer casts to a safe reference without unsafe authorization | Raw-to-safe operation admission |

The paired private_direct_control rejects E1003 in both check/build, showing
ordinary field lookup protects the same private member. Nominal and generic
nominal-to-integer casts pass check but reach E6001 in build. The backend fails
closed for those different LLVM shapes; that does not fix the source admission
or same-layout cases.

Permanent standalone fixtures live in tests/luna/language/cast_contract.
They are not yet part of the running workspace's committed target set. A
passing existing suite cannot certify these newly exposed paths.

## First incorrect decisions

Expr::Cast performs limited char/range/fat-metadata checks and then returns
the target type without a complete source/target capability judgment. The
backend treats equal LLVM types as identity, even when semantic nominal,
referent or callable identities/capabilities differ. LLVM representation is
not language authority.

Repair must validate casts in language terms before lowering, preserve generic
binder identities, keep unsafe admission separate from proof of memory validity,
and retain a backend invariant guard for malformed typed casts. Preserve legal
numeric/address/reference-to-raw controls and existing fat-metadata rejections.
Exercise generic and relocated source/artifact paths, callable erasure,
mutability, ownership/drop and lifetime controls. Do not fix by naming library
types or globally weakening borrow checking. Existing thread spawn_with uses
an unsafe fn-pointer ABI adaptation and needs its own validation/control rather
than a blanket signature-shape assumption.

## Diagnostic evidence

The [corrected span matrix](evidence/r3-expression-spans-before-observations.json.gz)
uses valid i32 entrypoints and UTF-8/tab source. E2002 dereference, E2012
negation and E2026 known-invalid char casts point to 1:1 instead of the offending
expression; tuple projection E2003 has no primary location. Ordinary scalar
index E2003 is a control with the correct location. These are DIAG-2 gaps;
wrong codes/entrypoint errors are not used as acceptance evidence.

Relevant code: TypeChecker::get_expr_span_for_diag omits several expression
shapes; TupleIndex error constructors omit spans and return inference variables
rather than poison Error. The repair must keep code identity, actual source
locations, and poison containment. An AST fallback span at file start is not
successful source mapping.

## Release status

No fix or full acceptance is claimed here. Closure cleanup, named/default calls,
these cast/diagnostic findings and broader R3–R5 gates remain open. The running
workspace on 9558701 must retain its own verdict, even if it passes every existing
cast-related test. These independent CLI probes extend the acceptance obligations.

## Repair checkpoint (protocol 15; release still blocked)

Language-owned cast admission now precedes lowering. It preserves nominal and
referent identity, forbids shared-to-mutable promotion and callable safety
changes, requires unsafe for raw-to-safe promotion and function signature ABI
adaptation, and rejects unsupported generic value casts. Fat metadata is not
stripped, fabricated or reinterpreted. Numeric and raw-address controls remain.
Backend typed casts use the same type-shape judgment before LLVM equality.
Protocol 15 rejects artifacts whose bodies were admitted under the old rules;
metadata remains version 7.

The isolated admission/span candidate built the canonical sysroot, passed 215
semantic/metadata/backend tests and two CLI suites with fresh source/artifact
roots. Their [pin](evidence/cast-candidate-pin.json) and exact compressed sources
retain that narrower revision. The first CLI attempt had a fixture-root setup
error; the failed log is retained and excluded from semantic acceptance.

A subsequent owned identity-cast probe exposed a separate MVIR defect:
`first as Owned` lost the source place, permitted use-after-move and dropped
an owner twice. The [before observations](evidence/cast-ownership-before.json)
pin that executable. Semantic identity casts now retain the original operand,
using ordinary move/drop accounting. Main-checkout native probes now report one
drop (exit0) and E3001 for use-after-move. Expanded final source/artifact tests
pass: two CLI suites, 32 typed reject commands, six native controls and 20
source-span checks, plus 215 semantic/metadata/backend tests. The final
[pin](evidence/cast-final-pin.json) and compressed sources include the identity
lowering repair. No full final-tree PASS is claimed. Private-member access after a
poisoned cast still produces a secondary diagnostic and remains broader DIAG-5
containment debt. Imported user-provider cast bodies and cross-target execution
need additional coverage before declaring the entire capability certified.

## Completed baseline workspace and CI

The immutable 9558701 Windows workspace finished with **1311 passed, 1 failed,
1 ignored**, exit101. All 1227 pinned compiler/library/fixture inputs and the
runtime remained unchanged. Cargo rebuilt the CLI, so its binary hash differs
from the starting executable; both hashes remain in the raw records.
The only failed target is generic_drop_cli, closure_capture native exit3.
The ignored test is an old empty placeholder, not safety evidence.
[Workspace log](evidence/workspace-mono-role.txt.gz),
[start](evidence/workspace-mono-role-start.json.gz),
[end](evidence/workspace-mono-role-exit.json.gz).

Both Ubuntu workspace jobs on 9558701 stop on the same closure_capture failure
in source/artifact modes. Runtime ABI, Ubuntu ASan, whole-file IO and formatting
jobs succeeded. These CI results are for the baseline, not the protocol-15
candidate. Raw fetched job responses are retained:
[platform workspace](evidence/ci-9558701-job-111687344189.json.gz),
[formatting workspace](evidence/ci-9558701-job-111687343797.json.gz).


### Broader regression follow-up

Memory intrinsic/unsafe/borrow source-artifact CLI regressions pass **3/3**.
The generic-drop matrix preserves its normal source/artifact drop/move cases,
but closure_capture still exits3 in both modes. The first broader char run
rejects correctly with E2026 but fails an older reason-string oracle; the
canonical `source must be an integer or char` reason is restored without
weakening type admission. That failed run is [retained](evidence/cast-regression-cli-before-char-message.txt.gz).
Char and the focused suites are rerunning on the amended candidate. None of
these targeted results changes the immutable baseline workspace verdict.

The amended candidate passes all three CLI suites (cast, source spans, char)
and 286 semantic/metadata/backend/MVIR/borrow tests, zero failures.
The [amended source/binary pin](evidence/cast-amended-pin.json) and logs preserve
this precise scope. Memory 3/3 and the generic-drop known closure failure belong
to the preceding diagnostic-wording candidate; their raw failure is not erased.
