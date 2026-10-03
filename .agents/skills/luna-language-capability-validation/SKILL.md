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

For executable language behavior, include a real `.ln` fixture compiled,
linked, and run through the supported toolchain, checking exit code and/or
observable output. Use `luna-testing-strategy` to choose the highest faithful
test boundary; Rust tests are appropriate for compiler-internal invariants
and for orchestrating genuine CLI/artifact E2E tests.

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

For ownership, references, raw pointers, FFI, or mutation, preserve Luna's
separate semantic domains and frozen safety rules. Unsafe does not disable
ownership, moves, borrow checking, region validity, or provenance. Do not
solve a failing case by globally weakening safety.

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
