# Luna 0.1 specification

Contract baseline: **0.1**. Release conformance: **BLOCKED / NOT VERIFIED**.
Consolidated 2026-10-03 at compiler revision `3dac3ac`.

Subsequent [all-worktree integration](../../integration/2026-10-03-all-worktrees.md)
preserves newer implementation and prior approved records. The original audit
and CLI evidence keep their revision boundary; they do not certify the merge.

This specification retains previously defined language contracts even where
the implementation is incomplete. Compiler acceptance, a historical freeze
report, and successful execution of one program do not redefine a contract.
Known failures and remaining design questions belong in [gaps.md](gaps.md).
This consolidation does not declare the compiler or the release frozen.

## Authority and document roles

1. The versioned specification and the adopted contract documents listed below
   define the Luna 0.1 contract. MUST, MUST NOT, and SHALL are requirements.
2. Implementation and executable tests establish conformance or a gap; a
   conflicting implementation is not automatically the specification.
3. Reference pages, agent skills, and website pages explain these requirements.
4. Plans, RFC proposals without adoption, old status pages, and audit reports
   record history or evidence. They cannot independently add a 0.1 feature or
   establish current release readiness.

Where retained contracts disagree and no subsequent adopted amendment resolves
the conflict, the decision remains explicit in the gap register. Do not choose
a convenient interpretation silently.

Language version **0.1** is distinct from runtime ABI revision, artifact format
version, and contract identifiers such as REGION-SPEC-01 or RAW-STORAGE-ANCHOR-v1.
Their existing revision numbers and prior adoption decisions remain meaningful;
they are not claims that Luna language version 1.0 has shipped.

## Chapters

| Chapter | Scope |
|---|---|
| [Syntax](syntax.md) | Canonical spelling, declarations, expressions, rejected legacy syntax |
| [Semantics](semantics.md) | Types, ownership, traits, lifetime relations, unsafe, FFI, async, comptime |
| [Macros and compiler boundary](macros.md) | Generic expansion; prohibition of library-name mappings |
| [Modules and artifacts](modules.md) | Providers, namespaces, visibility, bootstrap, identity and parity |
| [Standard library](stdlib.md) | Component inventory, public contracts and invariant requirements |
| [Runtime](runtime.md) | ABI identity, allocation, process entry, I/O and target evidence |
| [Conformance](conformance.md) | Diagnostics, evidence levels and release gates |
| [Gaps](gaps.md) | Confirmed defects, unavailable evidence and unresolved specification debt |
| [Document inventory](../../documentation-index.md) | Role and owning chapter of repository documentation |

## Adopted detailed contracts

These documents retain their detailed rules as part of the baseline. Statements
about past test runs or implementation completion remain dated evidence.

| Contract | Detailed source |
|---|---|
| Namespace openings and aliases | [NAMESPACE-USING-v1](namespace-using-v1.md) |
| Optional provider discovery configuration | [PROVIDER-CONFIG-v1](provider-config-v1.md) |
| Core semantic rules A–K | [Normative rules](../../normative-rules-p0-p1.md) |
| Lifetime relation algebra | [REGION-SPEC-01](../../../luna-rs/docs/spec/lifetime-formalism.md) |
| Borrow joins, carried references, closure escape | [SPEC-HARDENING-01](../../../luna-rs/docs/spec-hardening-borrow-closure.md) |
| FFI reference and aggregate restrictions | [SPEC-HARDENING-02](../../../luna-rs/docs/spec-hardening-ffi-contracts.md) |
| Raw pointer validity and unsafe | [Raw-pointer semantics](../../../luna-rs/docs/memory/raw-pointer-unsafe-semantics.md) |
| Logical owner storage anchors | [RAW-STORAGE-ANCHOR-v1](../../../luna-rs/docs/raw_storage_anchor_v1.md) |
| Lifetime elision | [LLE-v1](../../../.agents/skills/luna-semantic-compliance/references/lle_v1_rfc.md) |
| Independent type/field visibility | [Approved Visibility-02 amendment](../../../.agents/skills/luna-semantic-compliance/references/visibility-02-public-field-default-rfc.md) |
| Logical standard namespace | [STD-NAMESPACE-01](../../../luna-rs/docs/std_namespace_01_canonical_namespace.md) |
| Whole-file APIs | [Whole-file I/O v1](../../../luna-rs/docs/whole_file_io_v1.md) |
| Formatting foundation | [Formatting v1](../../../luna-rs/docs/phase6_core_formatting_v1.md) |
| Canonical identity and legacy reads | [Rename compatibility contract](../../project/rename-luna-v1.md) |
| Diagnostic identity and structure | [Diagnostics contract](../../diagnostics/diagnostics-v1.md) |
| Core runtime ABI | [ABI contract](../../runtime/abi-v1.md) |

New behavior requires an explicit contract amendment, implementation, and
appropriate conformance evidence. A roadmap entry or a new annotation alone is
insufficient.
