# Raw slices and borrowed headers — 2026-10-05

This is scoped implementation evidence for the active R0–R5 completion plan.
It does not freeze Luna 0.1 or certify the whole standard library. Base revision:
`f832db24459b5a2675dec1c6836f3bc1502340a9`.

## Counterexamples and first incorrect decisions

The base compiler accepted a safe call to `cstr_from_ptr(0 as *u8)`. Its native
program exited `0xC0000005`. A CStr constructed from a byte array also permitted
mutation of that array before a subsequent view read; the executable observed
the mutated byte. The separate before probes pin the base CLI and artifact-only
sysroot; later results do not relabel those failures.

Raw slice `MakeSlice` lowered the fat value without applying the thin-reference
raw-to-safe promotion gate. Missing raw-origin facts could also survive as empty
sets. Pointer casts merged a loaded pointer value's origin with the address of
its storage slot. Raw writes checked pointer-slot aliases rather than the live
UnsafeRawRoot loan of the pointee. Effect inference lost helper writes by
tracking the read of the pointer slot without tracking its pointee access.
A later opaque `unsafe fn(*rw i32)` callback also bypassed pointee checking;
its independent CLI check incorrectly accepted a write followed by a live
slice read. Opaque callback returns also lost incoming view loans. The
permanent matrix now includes direct/forwarded callback writes, an Unknown
access union, callback view returns, and read-only/view-return native controls.
Opaque results retain all potential incoming validity loans while raw results
retain Unknown origin/anchor facts. Unknown parameter escape effects are not
interpreted as proof of independence or non-escape. The partial workspace was stopped before changing
its candidate; no completed-workspace result is inferred from that run.

Borrowed raw-backed headers were filtered out of safe-loan effect transport
because their physical fields contained no safe references. Result extraction
and generic forwarding could consequently lose an already established validity
borrow. A mixed return summary selected only the direct source set and lost
the carried set from another branch. The independent `views::pick` reducer
was accepted before that repair, including a mutation of the carried backing
array; the after check rejects E3003. That intermediate before log has no
retained binary hash and is not exact-revision release evidence. Region return checking ignored such actual loans when the return type
had no reference fields. Move transfers did not request a loan invalidation
check for consumed owner storage.

Artifacts preserved raw return/anchor effects but rebuilt the other call
effects as opaque. A temporary raw-derived slice therefore failed to call a
non-escaping copying helper through `.llib`, despite a checked provider body.

## Implementation and language boundary

- Reference-valued MakeSlice uses the existing raw-to-safe promotion and loan
  machinery. Empty origin evidence becomes Unknown; ambiguous evidence rejects.
- Raw VALUE origin remains separate from slot address and safe-loan liveness.
  Direct, helper and FFI pointee accesses check the corresponding loan domain.
- A semantic potential-carrier query recognizes existing anchor contracts and
  recursive aggregates. It transports incoming validity facts and never turns
  a raw pointer or an anchor into a safe loan by itself. Owned-copy and scalar
  controls must remain independent of input borrows.
- Region checking observes actual carried loans for raw-backed return values.
  Move analysis exports consumed-storage events; loan analysis checks them
  without inventing a second ownership/Copy policy.
- CStr uses the existing raw-storage anchor contract. Its raw constructors are
  unsafe, safe byte constructors declare `life_from(bytes)`, and CString's
  view declares `life_from(self)`. Qualified associated lookup handles namespace
  prefixes for arbitrary nominal types rather than naming CStr.
- Metadata7 records typed parameter access/ownership/escape and safe return
  effects, separately from raw origin/anchor effects. Arity and canonical return
  indices validate before import. Mixed direct/carried return paths preserve
  both sets in a typed canonical variant; union, partial ordering and source
  mapping keep both groups. Public interface fingerprints include these
  facts. Protocol14 requires rebuilding earlier checked provider bodies; file2
  and MVIR4 remain unchanged. Generic bodies still reanalyze in the consumer.

No new syntax, intrinsic, lang item, stdlib-specific branch or owner relation
is introduced. The contracts remain RAW-STORAGE-ANCHOR-v1, raw-pointer unsafe
semantics, existing lifetime relations and ordinary move/borrow rules.

## Acceptance and limits

The public `raw_slice_provenance_cli` harness uses a fresh canonical sysroot,
source-only providers and `.llib/.obj`-only providers. Ordinary `views::View`
is the independent compiler reproducer. Native positives cover generic
forwarding, namespace-qualified associated construction, view death followed
by backing mutation, a temporary raw slice passed to an artifact copying/reading
helper, and CStr/CString/String APIs. Typed negatives cover live backing
mutation, local escape, unsafe constructor authorization, owner move, slice
extent, mixed-source return paths, and raw direct/helper/FFI writes. Native
controls invoke both branches and mutate backing storage after the views die.
All expected rejections stay active.

Earlier failed candidate matrices remain evidence. The
[focused candidate pin](evidence/raw-slice-candidate-pin.json) and
[command exits](evidence/raw-slice-verified-exits.json) record a fresh 49-provider
sysroot, 273 internal passes, 20 selected driver passes, the 3/3 memory CLI
harness, and 12 native plus 68 typed-rejection view observations. These are
scoped source/artifact results on Windows GNU.

The [completed workspace](evidence/workspace-raw-slice-exit.json) fails with
1,305 pass, seven fail, one ignored. Five targets expose a source trait-callee
monomorphization regression; owned closure capture cleanup and a stale metadata
version oracle account for the remaining two. The
[input/binary audit](evidence/workspace-raw-slice-audit.json) distinguishes
unchanged pinned inputs from a generated test dump and Cargo's rebuilt CLI.
Closure cleanup, named/default calls and broader R3–R5 requirements remain
separate gates. Subsequent repairs must have their own candidate/results;
they do not change this workspace verdict.

## Compiler Change

Capability: raw-to-safe slice validity, borrowed-header transport, pointee
effects and portable body-derived call facts. Stdlib exposed it through CStr,
slice APIs and file buffer copying. The fix is generic: arbitrary anchored
headers, parameter-relative effects and raw lineages use typed identities.
User-defined beneficiary: `views::View` and ordinary raw write helpers.
Tests: `raw_slice_provenance_cli`, memory-hook regression, raw FFI and C-GAP-12
effect controls, metadata validation and semantic carrier-shape tests.
New intrinsic/lang item: NO. Stdlib-specific branch: NO.

SKILL IMPACT: REFINEMENT of capability-validation acceptance guidance for
borrowed raw-backed headers and pointee effects. CORRECTION of semantic skill
guidance that overstated unsafe exemptions, generic/container guarantees and
implementation authority; align examples with component providers and adopted
contracts. Changed skills are reread and related semantic/testing/boundary
guidance checked; no new semantic or freeze authority is added.


## Later mono repair and enum acceptance

The [later candidate](MONO-CALLEE-ROLE.md) repairs direct-callee instantiation
and updates the metadata-version oracle. Its expanded view matrix includes
an ordinary generic Wrapped<T> enum, verifying native extraction/read and
mutation after the view dies, plus live backing mutation rejection. It passes
14 native and 72 typed-rejection observations. The earlier pin/logs retain their
original scope and counts; this extension does not overwrite them.
