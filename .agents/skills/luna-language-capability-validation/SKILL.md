---
name: luna-language-capability-validation
description: Validate whether a claimed Luna language or compiler capability satisfies its complete contract across relevant pipeline stages, negative cases, codegen, and source/.llib modes. Use for feature work, compiler semantic bug fixes, support audits, and completion or freeze claims; not for trivial edits.
---

<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](../../../docs/spec/0.1/README.md).

# Luna Language Capability Validation

## Purpose

Use this skill to ensure that a Luna capability works for every case within
its declared contract—not merely that syntax parses or one example succeeds.
Keep four things distinct:

- **Contract:** what Luna intends to promise.
- **Implementation:** what the current compiler/runtime actually does.
- **Evidence:** what tests and executions establish.
- **Freeze authority:** who may declare the contract frozen.

This skill is a validation workflow, not authority to invent or freeze
semantics.

## Capability state

Classify the capability being investigated as one of:

- **SUPPORTED** — contract is defined; valid cases work, invalid cases reject
  appropriately, and relevant end-to-end paths are verified.
- **UNSUPPORTED** — Luna intentionally does not provide it; use is rejected
  deterministically at an appropriate layer.
- **DEFERRED** — explicitly outside the current scope/version; documentation
  must not present it as available.
- **PARTIAL / INCOMPLETE** — some stages accept or implement it, but the whole
  declared contract is not met. Do not report this as supported.
- **UNKNOWN / NEEDS AUDIT** — repository evidence is insufficient.

These are capability classifications, not substitutes for the audit verdicts
in `mellis-audit` when that skill is applicable.

## Workflow

### 1. Establish the intended contract

Inspect the relevant canonical/frozen records, accepted design decisions,
language documentation, implementation, and tests. Use the appropriate
domain-specific Luna skills (for example `mellis-grammar`,
`luna-semantic-compliance`, `luna-lang-contracts`, or
`luna-stdlib-compiler-boundary`). Do not infer semantics from Rust/C++/LLVM,
parser acceptance, a roadmap, or one existing implementation.

State separately:

```text
Intended contract:
Observed implementation:
Evidence:
Disagreement / unknowns:
```

If authoritative sources disagree or a language-level decision is missing,
surface the conflict and stop before silently choosing the interpretation
that makes implementation easiest.

### 2. Trace the relevant compiler path

Follow every stage relevant to the capability. Depending on the feature this
may include:

```text
grammar/parser → imports/macros → name resolution → type checking / traits
→ comptime → mono → ownership/borrow/regions → MVIR → backend/linker
→ runtime and observable executable behavior
```

Do not mechanically require irrelevant stages. Record the first stage that
diverges from the contract. Typecheck success is not full support for an
executable feature; a concrete generic instance must not carry unresolved
generic state into MVIR or backend.

### 3. Build a contract-specific test matrix

For supported behavior, add or run positive and negative cases that exercise
the distinct semantic dimensions of the contract. Consider local and
cross-module use, generic/nested and multiple instantiations, receiver and
ownership behavior, and imported definitions where relevant. Test invalid
types, missing bounds, unresolved inference, borrow/move violations, bad
visibility, or invalid ABI shapes as applicable.

Negative cases must reject for the right semantic reason where observable.
Do not suppress diagnostics or weaken safety checks to make positive cases
pass.

For callable identity/safety, compare safe and unsafe functions with identical
parameter/return shapes. Exercise local values, returned/generic callbacks,
known immutable targets, mutable reassignment and genuinely opaque indirect
calls. A value-selected generic instance must retain its substitution at later
calls; parser acceptance or a direct-call success does not prove this path.
Verify portable callable safety and public interface identity as well as AST
rechecking. Keep unresolved reference/loan failures separate from native ABI
success; a callable matrix cannot waive an independent safety regression.

For casts, validate language type identity, reference capability, callable
safety and unsafe admission independently of equal backend representations.
Include same-layout distinct nominal types, reference reinterpretation,
shared-to-mutable promotion and unsafe-callable erasure as negative controls.
A true identity cast must preserve ownership/place accounting: test one drop
and rejection of use after moving through the cast. Keep legitimate numeric,
raw-address and unsafe ABI-adaptation controls separate from these rejections.

For executable language behavior, include a real `.ln` fixture compiled,
linked, and run through the supported toolchain, checking exit code and/or
observable output. Use `luna-testing-strategy` to choose the highest faithful
test boundary; Rust tests are appropriate for compiler-internal invariants
and for orchestrating genuine CLI/artifact E2E tests.

For borrowed headers with raw fields and validity/anchor contracts, test
transport through generic forwarding and enum wrapping/unwrapping. An existing
validity borrow must survive those transfers; an anchor or raw address alone
must not originate a safe loan. Include independent owned-copy and scalar
return controls to catch phantom borrows, plus owner move, backing mutation,
local escape and mutation after the view dies. Exercise mixed return paths
with different direct and carried borrow sources; their effect join and
portable representation must preserve every possible source. Opaque callbacks
need return-loan transport as well as pointee access checks; missing body
evidence is not proof of independence or non-escape. Keep callback scalar/read
controls to detect phantom borrows. Check raw pointee writes through
helpers and FFI as well as direct writes; reading a pointer's storage slot is
not the same access effect as writing its pointee.

### 4. Validate source and artifact paths

When provider/module boundaries or artifacts are involved, compare source
providers with freshly built canonical `.llib` artifacts. Artifact-only
means the relevant `.ln` source cannot be selected as fallback. Compare
compile result/diagnostics and, when executable, codegen and runtime behavior.
Do not reuse stale artifacts or treat accidental source fallback as parity.

For generic/monomorphized features, exercise distinct impl-, trait-, and
method-level substitutions as applicable, imported definitions, and multiple
instances. Distinct semantic instances must retain distinct backend identity;
portable artifacts must not depend on session-local IDs.

When public generic contracts change, verify canonical interface fingerprints
and stale-dependent rejection as well as execution. Portable AST rechecking
cannot compensate for constraints omitted from the public dependency identity.
Include unchanged-contract controls for generic binder renaming, declaration
order and body edits which preserve public effects.

For public parameter defaults, validate definition-site binding identity as
well as expression text. Distinguish impl/trait-owner and method/function
generic roles, preserve field names when a parameter shares their spelling,
and retain explicit generic arguments. Renaming a local namespace alias must
preserve the resolved contract; retargeting it to another declaration must
invalidate affected dependent interfaces. An alias is a lookup view, not an
exported namespace owner: verify original paths, private alias rejection and
native artifact execution, since metadata/check success can conceal a build
failure. Declaration/default metadata checks do not establish omitted-call
evaluation, ownership or effect correctness.

For omitted calls, test source-order explicit evaluation followed by only the
missing defaults in declaration order, ordinary moves/drops and borrowed
earlier parameters. Map lifetime obligations through the actual supplied ABI
slots and initialized logical parameters; helper preconditions remain caller
proof obligations. Derive receiver presence from the checked signature even
when no constraint mentions `self`. Test both legal forwarding and inverted
argument relations under a longer-lived receiver. Embedded cast/layout types
must retain concrete binder identity through provider transport and mono; a
generic appearing only in `sizeof(T)` must not silently receive a fallback
layout. For async defaults, observe creation before polling, as well as
cancellation and suspension. Source success and artifact publication alone
do not establish imported execution or correct default timing.

Nominal ABI identity must include field order and enum discriminants/payloads,
including private types reachable through public contracts. Test stale dependent
execution after replacing a complete dependency bundle, recursive reachability,
and unchanged-interface controls for unreachable private declarations. Stable
names alone do not prove layout compatibility or grant private visibility.

For target/artifact identity, compare the complete configured contract with
both artifact header/manifest and actual embedded/selected object code. A format
label alone does not prove architecture or sidecar identity. Cross-target
metadata/layout probes do not establish native language/runtime conformance.

For artifact integrity, separately test altered native bytes and altered portable
AST/MVIR/metadata, including a generic body whose source text/object stay unchanged.
Keep integrity rejection controls separate from malformed target/container probes:
the latter need coherent checksum envelopes to reach their intended validator.
Exercise publication failure through the CLI in quiet and ordinary modes; success
status must mean the requested artifact was published. Checksums establish payload
consistency, not publisher authentication or semantic equivalence of native code.

For ownership, references, raw pointers, FFI, or mutation, preserve Luna's
separate semantic domains and frozen safety rules. Unsafe does not disable
ownership, moves, borrow checking, region validity, or provenance. Do not
solve a failing case by globally weakening safety.

Compile-time execution must obey the same safe move/loan/escape rules before
the VM runs. Preserve the checked evaluation root and reachable concrete units;
the driver orchestrates ordinary admission over that prepared program, and
execution consumes the same verified IR. Checking only a materialized constant
after evaluation cannot validate the original loans or moves. Keep VM effect,
resource and pointer-escape guards as independent requirements, with direct
invariant tests as well as source/artifact CLI acceptance.

### 5. Fix compiler defects generically

When a user-visible compiler defect is found:

1. Reduce it to a permanent real `.ln` reproducer independent of the original
   stdlib/provider when possible.
2. Identify the first incorrect pipeline representation or decision.
3. Fix the generic mechanism and add positive/negative controls.
4. Preserve backend invariants; do not accept unresolved semantic types merely
   to get codegen through.

Do not special-case a standard-library type, provider, method name, or fixture.
If the fix requires a new semantic rule or conflicts with a frozen contract,
stop for design authority.

### 6. Report status and freeze readiness

Keep the support matrix evidence-based. A feature is not SUPPORTED merely
because its syntax exists. A freeze recommendation normally requires:

- an agreed contract and reconciled documentation;
- valid and invalid acceptance coverage;
- relevant ownership/generic/cross-module stress cases;
- source/fresh-artifact parity where applicable;
- full-codegen behavior for executable features;
- required focused and workspace regressions.

Report remaining gaps explicitly. Say **READY FOR DESIGN/FREEZE REVIEW — NOT
FROZEN** unless the authorized design authority explicitly freezes it. A
passing test suite alone does not grant freeze authority.

## Related skills

- `mellis-research` for compiler source/test/root-cause workflow.
- `mellis-audit` for completion audits and its audit verdict vocabulary.
- `mellis-grammar` and `luna-semantic-compliance` for syntax and semantic
  contracts.
- `luna-testing-strategy` for test boundary selection.
- `luna-stdlib-compiler-boundary` and `luna-stdlib-design` when stdlib work
  exposes a compiler capability issue.

Activate this skill for capability claims and substantive compiler/language
work, not routine repository edits.
