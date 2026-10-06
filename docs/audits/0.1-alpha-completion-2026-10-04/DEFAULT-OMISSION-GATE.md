# Default omission and logical parameter frame — 2026-10-06

**CALL-ARGUMENTS-v1 remains PARTIAL.** Synchronous omission is implemented;
the adopted [full contract](../../spec/0.1/call-arguments-v1.md) still includes
async, ownership, effects and source/artifact requirements. This checkpoint
does not narrow the contract or close the complete R0–R5 objective. No merge,
tag, freeze or release authority is implied.

## Implementation and semantic enforcement

The checked binding plan separates explicit source order from logical
declaration slots. It records omitted ordinals and the authoritative function
or trait signature. Function pointers still require their full positional
arity. Statically selected trait calls take labels/defaults from the trait;
the selected implementation's parameter spellings are private.

Monomorphization produces an omission entry keyed by declaration, concrete
substitution and omitted slots. Provided values occupy its compact ABI slots;
omitted parameters use ordinary storage in the same logical callee frame.
Only omitted expressions execute, in declaration order, before the original
body. Earlier parameters are available under normal move/borrow rules. The
complete function ABI remains intact. A bodyless extern entry initializes
defaults and forwards to a separately emitted full foreign declaration.

Impl, trait owner and method/function generic binders remain independent.
Qualified paths now retain owner argument groups separately from method
arguments. Checked embedded cast/layout types pass through the provider cache
and concrete instance; the mono barrier also checks types appearing only in
`sizeof`/`alignof`. MVIR no longer joins these generic binders by spelling.

Default entries have ordinary inferred effects and explicit lifetime proof
metadata. Provided-only declaration preconditions are checked at callers;
relations involving initialized defaults are checked inside the entry.
Requirements of helpers invoked by defaults propagate to actual provided
safe-reference inputs and are verified at callers. Owned parameter storage
and raw addresses cannot establish external safe-reference requirements.
Missing provenance for a generated obligation rejects E2016. Only a generated
entry may assume its inferred input requirements while checking its body;
ordinary callers must prove them.

Receiver presence comes from the checked callable signature. A constraint
which mentions only ordinary parameters still needs the receiver offset.
Caller assumptions, direct call checks and default helper requirement
inference use this mapping. Historical manually constructed internal contexts
without callable metadata retain their explicit `SelfVal` fallback.

Compiler protocol is **19**, semantic metadata **9**, MVIR **5**, file format
**2**. Portable MVIR carries canonical subjects and ABI/value slot references,
not semantic session IDs or borrow checker objects. Readers bound decoding
and reject nonexistent argument/value slots. No new intrinsic, lang item,
stdlib-specific branch or executable default opcode is introduced.

## Defects exposed by genuine programs

- A generic identity cast created an artificial temporary because it compared
  symbolic and concrete types. Ordinary full-arity and omitted calls both
  dropped an owned value prematurely. Lowering now compares instantiated
  types; controls assert exactly one drop and E3001 after moving.
- Qualified owner generic arguments were flattened into method arguments.
  Independent trait owner `i32` and method `bool` instances now retain their
  separate identities, with wrong owner/type-argument rejection controls.
- Imported generic `sizeof((T, T))` used a fallback layout because checked AST
  types were lost at the provider interface. Differently sized i32/i64/bool
  defaults now test actual native and comptime results in both modes.
- A generated default entry used full-arity lifetime ordinals and lost helper
  preconditions. Positive relations and inverted relations now exercise the
  compact ABI mapping, inferred obligations and local escape rejection.
- Ordinary methods whose constraints omit `self` incorrectly treated ordinary
  parameter zero as ABI argument zero. With a longer-lived receiver, invalid
  ordering between the real arguments passed both ordinary and default-helper
  calls. The permanent transitive counterexamples and valid forwarding method
  exercise signature-based receiver mapping without weakening the constraint.
- A bodyless default entry did not emit its foreign declaration when no
  full-arity source call existed. The omitted variadic `printf` control tests
  native linking and result; explicit unsafe and comptime-effect negatives
  retain their separate admission requirements.

## Evidence boundary

The final candidate is based on `ce417f199ed3e00294e54ee2bb6a778f8469e61a`,
branch `codex/call-arguments-0.1`, with **821 pinned compiler/library/fixture
inputs**. Rust host is Windows MSVC; generated native programs use Windows GNU
with LLVM18. Runtime SHA256 is
`d8aea737aa7929e79ab70f24db5705d7370570295ddab9f569b3085ee230f5de`.

Final scoped regression completed with **all command exits 0**: workspace
and all-target compilation, **337 internal tests**, **89 driver tests** and
**9 CLI suites**, with no failures or ignored tests. The official sysroot
build completed all **49 providers**. The omission CLI suite separately ran
**18 native programs** and **60 typed negative check/build processes** across
source and source-removed, relocated artifact modes. These process counts are
observations within one Rust harness test, not extra Rust tests.

All 821 input hashes match the start pin; compiler and runtime hashes also
match. Compiler SHA256 is
`0067c991607d5008256ad3cdb5d9612defd2bc654a6add485beac18796d38379`.
The [terminal record](evidence/default-omission/exit.json) contains exact
commands, counts and raw-log hashes. Lossless compressed input bytes and raw
logs are stored alongside it, including start/end pins and development failures.
The earlier 18-native/52-rejection development matrix precedes the receiver
amendment and remains separate evidence. No full-workspace test or POSIX native
compiler/stdlib certification was performed in this checkpoint.

The permanent CLI harness builds a fresh 49-provider sysroot; source roots
have no `.llib`/object files and artifact roots have no `.ln`. It publishes
the user provider, removes its source and relocates the project before
separate check/build/native processes. It covers side-effect order, override,
fresh owned defaults, earlier-parameter borrowing, generic identity casts,
trait defaults, owner/method arguments, layout/comptime, extern and lifetime
negatives. Internal tests cover parser relocation, omitted-slot plan integrity
and bounded portable lifetime annotation validation.

## Failed development oracles and unresolved async requirements

Retain the raw development failures separately from final regression. The
`sizeof`/`alignof` fixture originally omitted the numeric cast from their Luna
i32 result to usize; correcting that fixture did not change the language rule.
The first extern unsafe negative declared `extern fn`, rather than
`extern unsafe fn`. The corrected negative checks the declared unsafe
boundary. Wrong owner count/rejection expectations, missing Rust constructor
fields and setup failures remain identifiable in their logs.

The independent async probe is a **required positive which still fails**.
`async_omission_creation.ln` observes a counter immediately after creating an
unpolled future: explicit arguments leave it unchanged; an omitted default
must run at the call. Source check/build succeed but native execution exits
**2**, showing delayed initialization. Its newly published source-removed
artifact rejects E6001 / `Invalid semantic metadata: CorruptedData` before
execution. These are two separate gaps; a typecheck pass or fresh artifact
publication does not close them.

A separate metadata inspector accepts each of the four semantic validators.
The portable instruction codec has an observed collision: its writer emits
`HeapFree` with tag **17**, while the reader decodes 17 as `Eq` with two
operands and expects `HeapFree` at **15**. A minimal internal HeapFree module
round-trip rejects `CorruptedData`, independently of defaults or async source.
Repair and verify this generic codec defect separately from call-time async
initialization; preserve the old failed artifact as evidence and rebuild fresh
artifacts for the corrected protocol.

Complete call-time async initialization in the owning future frame, preserve
ordinary admission and the prohibition on self-referential future state, and
verify cancellation/drop, suspension and source/artifact execution. Do not
move an owned parameter after initializing a default reference to its stack
slot, and do not permanently reject async omission to make the matrix green.
The known moved-closure cleanup failure, pending maintainer policies and
remaining provider/diagnostic/target/full-candidate R3–R5 gates also stay open.

**SKILL IMPACT: REFINEMENT.** The owning capability-validation skill now
requires receiver mapping from checked signatures, concrete embedded types,
caller proof of helper obligations and async creation/cancellation observations.
This is validation guidance; it adds no language semantics or freeze authority.
