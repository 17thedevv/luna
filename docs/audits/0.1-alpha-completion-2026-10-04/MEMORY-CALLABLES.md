# Memory primitives and callable identity — 2026-10-05

Status: **PARTIAL; release blocked**. This is an implementation/evidence record,
not a language freeze. The candidate builds on 9e45ca96. Focused results do not
replace an exact-revision workspace gate.

## Semantic contract and root cause

Ordinary user-defined functions retain ordinary behavior regardless of their
names. Unsafe call requirements survive local values, returns, generics and
imports. Independent impl/method binders remain the adopted policy; put Vec
Eq/Clone bounds on the impl rather than joining same-spelled binders.

At9e45ca9, MVIR call lowering and mono Drop discovery selected operations using
substring checks for library function names. Ordinary `drop_in_place(5)` did
not return its declared value; zero-argument `slice_from_raw_parts()` crashed
the compiler with an out-of-bounds argument index. Local function values also
lost unsafe requirements, while a global set of function-type IDs conflated
safe and unsafe functions with the same signature.

The permanent [fixtures](../../../tests/luna/language/intrinsic_identity/)
exercise exact/containing names, generic and nongeneric values, aliasing,
reassignment, branch selection, same-signature safe/unsafe callables, returned
and generic callbacks, custom Drop-bearing payloads, and real native execution.

## Implementation and architectural boundary

Three typed LangItems describe **existing** memory primitives:
`drop_in_place`, `slice_from_raw_parts`, `slice_from_raw_parts_mut`. These are
language-hook identifiers, not callee-name matching. The declaration can be
renamed while preserving its explicit `#[lang("...")]` identity. No operation,
provider autoload family, prelude visibility or container-specific IR was added.

Source and portable AST resolve attributes under the existing canonical sysroot
provenance authorization. User-provider `#[lang]` rejects with E0005. Reconstructed
interfaces remap their locally owned hooks to current SymbolIds; duplicate
identities diagnose. A typechecker validator requires one unbounded type binder,
unsafe/bodyless intrinsic declarations, exact pointer/slice mutability, u64
length, correct return shape and no async/comptime/variadic/receiver form.
Malformed declarations fail before argument indexing or native lowering.

Mono substitutes bodyless intrinsic signatures and discovers generic pointee
Drop glue by hook identity. MVIR direct calls use that identity. Callable values
get generated native thunks so real indirect ABI calls can execute. Ordinary
nongeneric functions referenced only as values are retained as mono roots.
Known immutable local function values keep their canonical GlobalId, including
its substitution; lowering must not remangle them using an empty substitution.
Mutable bindings and opaque callbacks keep their indirect dispatch.

`SemanticType::Function` and `CanonicalType::Function` carry `is_unsafe`.
Substitution, normalization, cloning, unification, trait proof, reflection,
mangling, metadata encoding/decoding and public type-shape fingerprints preserve
it. Compiler protocol13 and semantic metadata6 reject incompatible artifacts;
file format2 and MVIR4 are unchanged. This does not claim complete callable
coercion/effect inference or support for unresolved generic function values.

The current writer's authoritative interface fingerprint is in
`Manifest.provenance`; it is computed from `CanonicalInterface`. The duplicated
`SemanticMetadata.interface_fingerprint` still contains the builder's zero
placeholder and is not the fingerprint used by canonical imports. The new test
checks the manifest against a recomputed interface hash. Populating or removing
that redundant metadata field is separate representation cleanup; it must not
be mistaken for evidence that public safety changes have identical identities.

### Generated primitive thunk validation

Raw-slice thunks implement the existing unsafe primitive boundary. Borrowck
accepts this boundary only for the registered typed hook and exact generated
recipe: two parameter-origin allocas, their loads, one MakeSlice, and a return
of that result, with matching pointer/element/length/reference types. Ordinary
source bodies do not qualify. Malformed recipes fail closed with the backend
invariant diagnostic. Move and return analyses still run; caller loan/escape
checks remain required. The mutation unit test changes recipe, types, origins
and terminators to verify rejection. This narrow primitive body boundary must
not become an exemption for arbitrary unsafe or library functions.

## Evidence and remaining safety blocker

The [CLI checkpoint](evidence/memory-intrinsic-cli-canonical.txt.gz) has 2 passed
harness cases, 1 failed and 0 ignored. Its passing cases cover the original and
renamed canonical declarations in independent source-only/artifact-only roots.
All ordinary/native positive fixtures run successfully; invalid annotations,
arity, unsafe calls, local escapes and callable safety erasure reject.

The independent raw-slice reference regression is deliberately nonignored.
Both direct construction and immutable callback construction allow a conflicting
write to the original local array before the shared slice's last use. Check and
build accept in both provider modes: eight expected E3003 observations fail.
This is a correctness blocker, not an expected-failure waiver.

The [strict prototype log](evidence/memory-intrinsic-cli-loans.txt.gz),
[complete changed-source snapshot](evidence/raw-slice-strict-candidate-sources.json.gz)
and [pin](evidence/raw-slice-strict-candidate-pin.json) preserve a rejected
candidate. Applying the normal Borrow promotion gate to reference-valued
MakeSlice exposed unanchored CStr raw storage and primitive-str raw-cast origin
problems while building string. The prototype was not accepted or relabeled
PASS. Further work must establish legitimate provenance/loans and repair the
library contracts; inventing an owner or weakening unknown-origin checks is
not a valid solution.

Code inspection also shows safe CStr raw-pointer constructors and borrowed
views represented by raw headers without an explicit backing reference. These
are additional audit candidates, not yet independent native counterexamples.
Do not call the entire memory/string capability supported from this matrix.

## Workspace baseline and next gates

The lossless [workspace log](evidence/workspace-integrity.txt.gz),
[start pin](evidence/workspace-integrity-start.json),
[exit pin](evidence/workspace-integrity-exit.json) and
[summary](evidence/workspace-integrity-summary.json) establish the uninterrupted
9e45ca9 run: 1,297 pass, 2 fail, 1 ignored, exit 101. The failures are moved-closure
cleanup and an input-copy sharing violation in the safe-loan test. The copy
helper now selects only canonical provider inputs and reports both paths on
failure. The original log did not identify the locked path, so its precise file
cause is not retrospectively asserted.

[Workspace no-run](evidence/memory-callable-workspace-no-run.txt.gz) succeeds for
the initial repaired candidate. Later focused results must be appended with
their binary/source pins. Required next gates include memory loan/provenance
repair, closure destruction, adopted named/default calls, complete contract and
provider API coverage, native target scope and clean exact-revision CI/workspace
validation. No merge/tag/release readiness is asserted.

## Compiler Change report

- Capability: checked memory primitive identity, callable safety and canonical generic function values.
- Why stdlib exposed it: public ptr/mem APIs exercised name dispatch, generic Drop and source/artifact imports.
- Why generic: arbitrary declaration names, user payloads and callback signatures use SymbolId/binder identity.
- User-defined beneficiaries: ordinary::my_slice_from_raw_parts<T>, ordinary safe/dangerous functions and custom Tracker Drop.
- Tests: memory_intrinsic_identity_cli, memory_intrinsic_contract_tests, generic_function_value_parser_tests and generated-thunk mutation tests.
- New intrinsic/lang_item: three explicit tags for existing primitives; no new operation or autoload family.
- Stdlib-specific branch: none added; incomplete raw-slice loans remain documented.


## Final focused verification

The final [CLI log](evidence/memory-callable-oracle-cli.txt.gz) reports2 pass,
1 fail,0 ignored, with12 native successes and88 typed reject observations.
The public callback metadata controls now compare the authoritative manifest
fingerprint to a recomputed canonical interface hash. The earlier
[incorrect-oracle run](evidence/memory-callable-final-cli.txt.gz) remains0 pass,
3 fail and is not relabeled.

Semantic194/194, borrowck61/61, parser2/2 and driver18/18 pass; the official
sysroot builds49 providers. See [execution details](EXECUTION.md) and the
[candidate pin](evidence/memory-callable-candidate-pin.json). Raw-slice loans
and the broader release gates remain incomplete.
