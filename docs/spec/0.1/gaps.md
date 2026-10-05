# Luna 0.1 — implementation gaps and specification debt

Baseline date: 2026-10-02; implementation revision: `3dac3ac`.
Audit IDs below are local to [stdlib-2026-10-02](../../audits/stdlib-2026-10-02/README.md),
not substitutes for earlier compiler-gap IDs from unrelated phases. Existing
contracts are retained. No row here grants permission to weaken them.

**Integration update — 2026-10-03:** later worktree changes have been merged.
The rows and CLI evidence below describe `3dac3ac` unless explicitly stated
otherwise. Some fixes may now be present; each original finding requires a
fresh check before being marked closed. The merged manifest has 49 providers.
V01-ARCH-02 is newly observed in the imported source, not the old audit.

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
| V01-ARCH-02 | Compiler must not map library macro names, streams, suffixes or callee spellings | Imported D:/fdlang macro_engine.rs contains the eight-name/format_macro interception and print_val/newline lookup. It violates the retained boundary; preservation in Git is not semantic approval |
| V01-DOC-01 | Visibility-02: fields default public independently of type accessibility | Old grammar skill/reference says private by default; corrected to the approved amendment |
| V01-DOC-02 | `requires life(...)` is the current relation spelling | Older docs/skills use removed `where outlives`; current guidance is corrected without changing the relation model |
| V01-DOC-03 | Luna .ln/.llib, 32 providers, six language contract families plus OptionExt contribution | Old overview/bootstrap pages claim Mellis core.ms, 25 providers or four families; current entry points are replaced |

V01-GRAMMAR rows are conformance gaps or unverified contracts, not new parser
fixes made by this documentation task. Historical plans retain their original
examples under a historical-role notice.

## Decisions still required

| ID | Question / boundary |
|---|---|
| V01-DESIGN-03 | Runtime integer overflow policy outside defined checked/compile-time operations; avoid declaring current LLVM wrapping normative |
| V01-DESIGN-04 | Complete formal grammar beyond the consolidated productions; parser acceptance of legacy/uncontracted syntax is not adoption |

No new format-macro hook, interpolation syntax, runtime opcode or container
language item is adopted. These require their own complete generic contract.

V01-DESIGN-01 was resolved by maintainer adoption of Option A on 2026-10-04:
[METHOD-RESOLUTION-v1](method-resolution-v1.md). The resolver's complete
conformance remains an implementation/acceptance task; deterministic selection
of a first candidate is not proof of ambiguity rejection.

V01-DESIGN-02 is resolved by the previously approved 2026-10-01 formatting
contract, now recovered from the other branch: char is a Unicode scalar;
invalid static casts diagnose, invalid dynamic casts trap, and arithmetic/
bitwise/shift/negation operators on char reject. The approval and acceptance
apply to their recorded baseline, not every newly merged addition.

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

The [integration CLI run](../../integration/evidence/cli-examples.json), after
rebuilding the merged CLI/runtime and all 49 providers, repeats the same 16
attempts and outcomes. All three grammar gaps remain reproduced. The custom
macro case uses an integer argument; it does not exercise or excuse the newly
imported format-string interception recorded as V01-ARCH-02.


## Alpha module additions — implementation update 2026-10-04

NAMESPACE-USING-v1 and PROVIDER-CONFIG-v1 now have implementation plus a
permanent CLI matrix passing on Windows x86_64 GNU, in source-only and fresh
artifact-only modes. The [verification record](../../audits/alpha-modules-2026-10-03/README.md)
tracks the tested baseline and final workspace regression separately. This
update adds bounded module capabilities; it does not change the dated defects
or automatically close W8, the full diagnostic contract, release or target gaps.

## Module constant storage — implemented baseline, release verification pending

[MODULE-CONST-STORAGE-v1](module-const-storage-v1.md) adopts program-lifetime
immutable storage for references to module-level constants; local constants
retain lexical storage lifetime. `StaticAddress` now represents actual immutable
target data with canonical provider/declaration naming and a portable initializer.
Private evaluated data and module storage scope survive provider reconstruction;
local escape, mutable access and VM pointer materialization remain rejected.
The borrowed Option/Result artifact regression now passes. Focused CLI tests
exercise scalar/aggregate data, field references, native and portable generic
bodies, repeated references and relocated source/fresh-artifact provider graphs.
See the [execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md).
Clean-checkout and advertised-target release gates remain open, as do the
independent ownership, method-selection and named/default-call gaps.


## Method/binder checkpoint — 2026-10-05, still PARTIAL

The maintainer's independent impl/method binder decision is implemented in
selected method headers and monomorphization. Vec element bounds now reside on
constrained impls. Applicable inherent/trait candidates are checked separately;
ambiguity reports E1008, with explicit qualification selecting a trait contract.
Provider-local checked impl headers retain individual declaration identity.
Blanket-impl proof now checks recursive trait/associated-type premises; caller
binders are rigid within that proof. Array header matching and mutable-reference
receiver checks have dedicated positive/negative controls. Compiler artifact
version 6 rejects earlier bodies; metadata schema 3 and MVIR 4 are unchanged.

This does **not** close R2 or release conformance. The full run at `fbf4e72`
records 1,270 PASS, 18 FAIL and 1 ignored (cargo exit101, 14 failing targets).
Subsequent focused fixes do not turn that run into PASS. See the
[execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md) for
revision-specific evidence and follow-ups.

At the `c268372` checkpoint, two additional standalone counterexamples were
retained as required regressions, not passing certificates:

- `tests/luna/language/generic_typing/rigid_return_required_reject.ln`: a generic
  function returning an unrelated `i32` is accepted for `T = bool`, builds and
  exits0. General TypeChecker unification treated rigid generic parameters
  as wildcards, independently of the stricter bound prover.
- `tests/luna/language/generic_typing/nested_struct_literal_required_accept.ln`:
  nested struct literals in field initializers were rejected by the parser.

The subsequent rigid-body/parser follow-up rejects the first with E2001 and
executes the second successfully. The standalone `generic_typing_cli` matrix
checks four native executions and 24 typed check/build rejections across
source-only and freshly published, relocated artifact-only provider graphs.
Generic declaration binders remain rigid in ordinary unification; trait `Self`
and parameter substitution precedes receiver checking. Empty struct literals no
longer depend on a whitelist of following tokens; condition/subject positions
retain their existing disambiguation. Artifact compiler identity is now 7;
format2, metadata3 and MVIR4 are unchanged. This closes these two reduced
counterexamples, not all generic typing or parser coverage. See the execution
ledger for preserved failed runs, corrected reruns and remaining obligations.

Canonical public metadata still groups some impl information by nominal head;
its per-header identity/fingerprint contract needs further audit. Later fallback
lookup paths and broader associated-projection domains remain unverified.
Moved closure capture destruction, named/default argument implementation,
remaining provider/diagnostic/target coverage and exact-candidate R5 stay open.

The subsequent full workspace run at `de9977d` records **1,290 PASS, 1 FAIL,
1 ignored**, cargo exit101; only the owned-closure cleanup target fails.
Additional reducers outside that matrix exposed concrete inherent method
symbol collisions and incomplete public-bound fingerprints. A later focused
repair uses each selected checked self header for method ABI naming and removes
name-based mangler substitution; the two concrete dispatch/provider reducers
now execute/build correctly across source and relocated artifacts. Compiler
artifact identity is 8, with other schema versions unchanged. This does not
rewrite the full run as PASS. The changed-public-bound fingerprint reducer and
Windows manifest/object-format mismatch remain open in the execution ledger.


## Public generic metadata checkpoint — 2026-10-05, still PARTIAL

The subsequent compiler9 / metadata4 checkpoint retains individual checked
impl headers and declaration-owned constraints for exported functions,
nominals, traits and methods. Associated equalities/definitions and trait
arguments survive decoding. Binder spelling, declaration order and body-local
variables no longer affect the tested public interface identities. Changes to
public constraints reject stale dependent artifacts. Forward impl lookup and
inferred struct-constructor bounds have source/relocated-artifact controls.
See the [execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md)
for failed controls, corrected reruns and bounded final evidence.

This closes the changed-bound fingerprint reducer above, not all canonical
public ABI/layout or generic well-formedness domains. Nominal constraints in
all type positions, trait argument premises, associated-projection fallback,
recursive canonical types and closure metadata need further audit. The
Windows manifest/object-format mismatch remains open. The last full workspace
run remains FAIL at de9977d; no full compiler9 candidate has been certified.
Closure capture cleanup, named/default calls and R3–R5 remain open.


## Target/object identity checkpoint — 2026-10-05, still PARTIAL

Compiler10 replaces the fabricated ELF/64/empty target fields with the selected
LLVM target contract, validates CPU/features, ABI/layout, pointer width and byte
order, and checks header/manifest agreement. Embedded relocatable architecture
and selected sidecar identity have standalone CLI rejection controls. This closes
the Windows manifest/object reducer recorded above. Source and freshly built,
relocated artifact providers run successfully; stale sidecars reject explicitly.

Accurate cross-target descriptors do not establish native conformance. Current
lowering rejects non-64-bit pointers while its remaining word/layout assumptions
are repaired; this is a retained implementation gap, not a new language exclusion.
Native target scope, runtime/toolchain coverage, broader ABI/layout domains and
R5 clean-checkout/full-candidate gates remain open. The last full workspace run
remains FAIL at de9977d; focused compiler10 passes do not replace it. See the
[execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md).

### Ordered nominal ABI identity — bounded follow-up, 2026-10-05

Artifact-only reducers proved that reordered struct fields and enum variants
shared fingerprints while stale code ran with the changed ABI (exit6 instead
of exit0). Compiler11/metadata5 now include ordered, reachable nominal
representations, including hidden types in public contracts, while keeping
unreachable private changes outside public identity. Source/artifact/rebuilt
native controls and typed stale-dependent rejection pass in the dedicated CLI
suite; recursive identity, binder/body invariance and corruption checks are
also recorded. This closes those reducers, not all ABI domains or the full
release gate. The completed full workspace run at39a2a9b remains FAIL:
1,262 pass, 34 fail, 1 ignored. Test isolation/oracle repairs subsequently pass
96 focused cases. Closure cleanup and named/default calls remain open. See the
[execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md).

### Artifact payload consistency/publication — bounded follow-up, 2026-10-05

At20f682f, standalone artifact-only reducers accepted a valid replacement object
and executed exit6 rather than exit0; a changed generic AST literal, with unchanged
source text/object, executed exit2. A failed `.llib` destination rename also
reported CLI success. Compiler12 now populates native metadata, validates native
integrity and all section checksums before decoding, and propagates publication
errors independently of `--quiet`. Dedicated CLI controls cover these reducers,
section ranges, coherent target/container mismatches and valid native providers.

The workspace invocation at20f682f exits101. Completed suites report1,143 passes,
116 failures and1 ignored test; those counts exclude an aborted native test target.
StorageFull and a hung out-of-bounds child invalidate this run as a complete
environment gate. Closure capture destruction still fails independently. The raw
log and intervention are preserved; focused repair passes do not turn this into
a release PASS. Named/default calls, broader contract validation and R3–R5 remain
open. See the [execution ledger](../../audits/0.1-alpha-completion-2026-10-04/EXECUTION.md).


### Memory-hook and callable-safety checkpoint — 2026-10-05, PARTIAL

The completed workspace run at 9e45ca9 exits101: 1,297 pass, 2 fail, 1 ignored.
The failures are moved-closure cleanup and a Windows sharing violation while
copying a safe-loan test's inputs. These results remain failures at that revision.

Independent ordinary user functions whose names contain `drop_in_place` or
`slice_from_raw_parts` exposed incorrect intrinsic substitution, including a
compiler panic for a zero-argument ordinary function. The current repair uses
checked language-hook identity for these existing unsafe memory primitives,
keeps unsafe callable safety in semantic/portable types, and preserves canonical
generic function-value identity through lowering. Protocol13/metadata6 require
fresh artifact rebuilds. The CLI checkpoint reports2 pass and1 fail: ordinary
names, renamed canonical hooks and native callbacks work in both modes, but
raw-slice construction still fails to establish required reference loans.

The raw-slice mutation rejection remains an active, nonignored regression.
Reusing the normal raw-to-safe promotion gate exposed additional existing CStr
and primitive-str provenance gaps during the official sysroot build. That strict
prototype and its failure evidence are preserved; no broad safety exemption was
applied to make source library bodies pass. This capability is incomplete.
Closure cleanup, named/default calls, broader contracts and R3–R5 remain open.
See the [memory audit](../../audits/0.1-alpha-completion-2026-10-04/MEMORY-CALLABLES.md).


### Raw slice/view checkpoint — 2026-10-05, scoped checks pass; release incomplete

The active repair applies raw-to-safe promotion/loan checking to reference-valued
MakeSlice, preserves borrowed-header and callback validity facts, and keeps
mixed direct/carried return sources. Body-derived call facts remain canonical
across .llib import. Safe CStr constructors retain input validity; raw pointer
constructors require unsafe caller proof. Protocol14/metadata7 require rebuilding
previous artifacts; import does not rebuild them.

The [scoped audit](../../audits/0.1-alpha-completion-2026-10-04/RAW-SLICE-VIEWS.md)
reports 273 internal and 20 selected driver passes, a 3/3 memory CLI matrix,
and 12 native plus 68 typed rejection view observations in independent source/
artifact modes. Earlier counterexamples and failed candidate logs are retained.
The completed expanded workspace exits101: 1,305 pass, seven fail, one ignored.
Five targets expose an abstract-callee monomorphization regression; closure
cleanup and a stale metadata-version oracle account for the other two. The
[evidence](../../audits/0.1-alpha-completion-2026-10-04/evidence/workspace-raw-slice-exit.json)
is a failing release gate, not provider certification. Closure cleanup, the adopted named/default call
contract and broader R3–R5 obligations remain open.


The [subsequent mono repair](../../audits/0.1-alpha-completion-2026-10-04/MONO-CALLEE-ROLE.md)
passes five CLI, 43 driver and 211 semantic/metadata tests, with an expanded
raw-view matrix of 14 native and 72 typed rejections. It removes spurious abstract
trait callee instances without weakening the generic barrier. These focused
results require a new full-workspace verdict; they do not subtract failures
from the recorded full run or establish release readiness.


### Cast identity/capability counterexamples — 2026-10-05

Independent native probes on 9558701 expose safe shared-to-mutable reference
casting, same-layout nominal/private-field bypass, unsafe-callable erasure,
referent reinterpretation producing an invalid char, and raw-to-safe reference
creation without unsafe admission. All five pass check/build and execute.
They are correctness blockers under retained contracts, not optional features.
See the [pinned cast audit](../../audits/0.1-alpha-completion-2026-10-04/CAST-GATE.md).
The same audit records wrong/missing expression spans and nominal-to-integer
check/build disagreement. Existing scoped and workspace results do not certify
these new paths. A protocol-15 repair candidate passes focused admission/span
source/artifact acceptance; an additional identity-cast move/drop defect is
repaired and expanded focused acceptance passes. Whole-workspace, imported
user-provider and broader diagnostic containment gates remain open.

### Call arguments prerequisite — 2026-10-05, completion still open

[CALL-ARGUMENTS-GATE](../../audits/0.1-alpha-completion-2026-10-04/CALL-ARGUMENTS-GATE.md)
retains native counterexamples where historical named labels are ignored,
Python-style label/default syntax is missing, and ordinary arity passed check
before build rejected it. The isolated positional-arity repair passes focused
source/fresh-artifact checks and 195 semantic tests. A compatibility amendment
restores unsized-value cast diagnostics; the unchanged raw/dyn driver suite
passes 14/14. Neither repair implements named/default binding or its portable
definition-scope/ownership contract.

The next Windows workspace result is preserved as exit101, 1311/3/1 across four
failed targets; eight pinned inputs changed during the run, and one failed
harness did not emit a test-result summary. It cannot certify an immutable
candidate. Independent Ubuntu e3b4402 CI still rejects release readiness through
owned closure capture native exit3 in both provider modes. All broader R3–R5
and retained contract gates remain required.

### Named binding update — 2026-10-05; CALL-ARGUMENTS-v1 still PARTIAL

The [named checkpoint](../../audits/0.1-alpha-completion-2026-10-04/CALL-ARGUMENTS-GATE.md#named-binding-implementation-checkpoint--2026-10-05)
implements declaration label binding, source-order value capture and portable
public signature identity (compiler16/metadata8). Its final fresh source/
artifact CLI matrix passes, including generic/method/trait calls, ownership,
early return, non-generic comptime, relocated source removal and stale-dependent
rejection. These results supersede the named-label baseline for their stated
scope; older findings retain their dated evidence.

Defaults remain unimplemented. Generic comptime calls fail E4005 for both named
and positional arguments because an early concrete callee is unavailable; this
is an implementation gap, not a deferred contract. The broad CLI regression
still fails closure_capture native exit3 in both modes. No new full-workspace,
cross-platform or release PASS is established; all R3–R5 obligations remain.

The [subsequent comptime root repair](../../audits/0.1-alpha-completion-2026-10-04/COMPTIME-PREP-GATE.md)
fixes the missing generic callee reproducers: named/positional, nested and
multiple instances plus local generic drop pass fresh source/artifact check,
build and native execution. Pinned regressions pass 324 internal and 45 separate
CLI/driver tests. A new simultaneous mutable-loan probe rejects E3003 in native
check but passes inside comptime, including build/native exit0. Borrow admission
must precede VM execution; it is a correctness gap, not a comptime exemption.
Defaults and all remaining release gates retain their full scope.

The [early admission follow-up](../../audits/0.1-alpha-completion-2026-10-04/COMPTIME-PREP-GATE.md#follow-up-ordinary-admission-before-execution--2026-10-05)
supersedes that loan counterexample within its scoped tests: ordinary lifetime/
borrow/drop validation precedes VM execution and the VM consumes the verified
prepared program. Mutable alias, shared/mutable and return-loan conflicts reject
with original typed diagnostics/spans in fresh source/artifact modes; valid
controls pass. Final immutable verification passes workspace/all-target
compilation, 324 internal and 43 CLI/driver tests, exit0. Defaults remain
unimplemented; the known closure failure and broader release gates are not
closed by this checkpoint.
