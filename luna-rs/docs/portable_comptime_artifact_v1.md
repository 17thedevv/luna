<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../../docs/spec/0.1/README.md).

# Portable ordinary comptime execution in canonical .llib artifacts

Status: DESIGN APPROVED; IMPLEMENTATION PENDING; NOT FROZEN.

Maintainer approval: 2026-10-02. The approved design requires a
session-independent, provider-owned execution representation, not merely
retaining bodies in the producer's global AST arena. This record does not
declare source/artifact parity complete or authorize changing comptime effects.

## Scope and authority

This task closes the ordinary imported-function body-availability gap exposed
by LITERAL-TYPING-01. It does not add metaprogramming syntax, make arbitrary
runtime operations evaluable at comptime, or change ownership/borrow rules.

On 2026-10-02 the maintainer explicitly stopped the concurrent task and handed
the compiler work to this agent ("toi ngung luon roi ban tu lam lai luon di").
Implementation can proceed without waiting for the previous actor. This is
not permission to discard that actor's legitimate edits or reuse historical
test output as evidence for the current compiler.

## Confirmed current implementation, not a proposed contract

- `luna-llib/src/writer.rs` clones the supplied arena and removes bodies of
  non-generic functions, except methods in generic impls.
- `luna-driver/src/lib.rs` supplies an arena that has already participated in
  import resolution. Dumping this arena is not provider-owned normalization.
- `luna-driver/src/external.rs` relocates the AST into the loading session and
  resolves it in a provider context. Its dependency freshness validation uses
  interface fingerprints, not execution dependency fingerprints.
- `luna-ast/src/relocator.rs` currently leaves literal token spans unchanged.
  This is a confirmed relocation omission; an execution failure caused by it
  has not been demonstrated by this audit.
- `luna-mvir/src/interp.rs` reports missing ordinary direct callees, but its
  explicit-callee Drop path can skip a destructor not found in the module.
- The existing serialized MVIR is not established as a complete typed,
  portable comptime execution representation. This design uses portable AST
  plus provider-context re-elaboration, not interpretation of that section.

## Approved semantic contract

Canonical .llib artifacts store a session-independent, provider-owned
execution representation sufficient for ordinary comptime execution. Internal
references are canonicalized to artifact-local identities. Dependency-owned
entities remain external references resolved through dependency identity.
Loading reconstructs fresh session identities and provider lexical scope.
Interface identity and execution identity are independently fingerprinted and
validated. Missing required execution data always fails closed.

### Interface and execution are separate logical domains

Interface contains public types, signatures, traits/impl surface, and the
canonical public interface fingerprint. Execution contains provider-owned
bodies, their required private declarations/constants, external execution
references, diagnostic source context, and an execution fingerprint.

A private body change may change execution identity while leaving interface
identity unchanged. Execution availability must not export private symbols,
create a root alias, or change provider-versus-namespace rules.

### Provider-owned execution closure

The serialization unit is the provider-owned execution closure required to
analyze and execute artifact bodies, not all declarations in a global arena.

All ordinary function bodies owned by the provider are execution roots for
the proposed first implementation slice. Include the provider-owned private
helpers, constants, type declarations, impl context, and lexical context
required by those roots. Keeping a body does not make it comptime-evaluable;
the existing effect and operation restrictions still govern execution.

References to dependency-owned declarations must identify the dependency and
the canonical declaration within it. Do not copy a dependency body into the
current provider's execution ownership domain. A consumer must not resolve a
private helper merely because its definition occurs in an execution payload.

Use existing provider ownership information, declaration context, and source
unit ownership to derive this partition. Do not infer ownership from terminal
names, stdlib provider names, arena position alone, or import order.

### Artifact-local identity and fresh-session reconstruction

Producer DeclId, ExprId, StmtId, TypeId, PatId, FileId, SymbolId, ValueId, and
PlaceId are not portable identities. Internal execution references use local
indices in the canonical artifact representation; decode allocates/reconstructs
fresh session-local identities.

Source units must have their own artifact-local mapping. Diagnostic spans are
checked against the corresponding source unit and relocated, including literal
token spans. They must not refer to the importing consumer's source by accident.
Embedded diagnostic source context is not permission to reparse a missing .ln
provider or bypass the canonical execution payload.

External references use dependency identity plus canonical symbol identity.
Separate providers with the same private helper name remain distinct. Changing
import order or pre-populating the consumer arena must not change execution.

### Execution identity and dependency validation

ExecutionFingerprint includes compiler semantic version, target-relevant
configuration, enabled semantic features, canonical provider body content,
private constants/context, and referenced execution dependency identities and
fingerprints.

It must not be a hash of the unnormalized global AST serialization. Absolute
paths, producer FileIds, relocated session IDs, diagnostic-only spans, and
incidental arena/import ordering must not become semantic identity.

Interface dependencies and execution dependencies are distinct. If B's body
changes while its public interface does not, an A artifact whose comptime
execution depends on B must not retain a stale result merely because B's
interface fingerprint still matches. Validation must cover transitive
execution dependencies through the loaded provider graph.

The loader validates and loads or rejects. It does not rebuild, search for
source as a recovery path, or treat an existing artifact as proof of freshness.
Build/cache orchestration remains outside the compiler loader.

### Missing data and unsupported operations fail closed

Required ordinary body or destructor unavailable -> diagnostic -> comptime
evaluation fails. Do not skip Drop, guess a return value, assume a no-op, or
fallback to source. A moved/uninitialized value for which no drop is required
remains governed by the existing ownership state; it is not the missing-callee
case.

FFI and forbidden runtime effects remain forbidden at comptime. Unsupported
operations must reject equivalently in source and artifact mode, not become
allowed because a body was retained.

### Version/capability gate

The new execution payload must be explicitly versioned and capability-gated.
Reject unsupported/corrupt payloads and artifacts missing required execution
capability deterministically. Old versions require rebuilding through the
official toolchain, not loader-side source compilation.

Choose concrete schema version numbers during implementation after auditing
all reader/writer/manifest consumers. Do not repurpose an unrelated semantic
metadata version or silently deserialize a changed schema as the old one.

## Implementation gates

1. Re-read current source and tests after the concurrent compiler handoff.
   Preserve unrelated edits; rebuild a coherent compiler baseline separately
   from interpreting historical test output.
2. Implement provider-owned extraction, canonical local-reference/source-unit
   normalization, and stable external references. Unit tests must exercise
   producer arena offsets, declaration ownership, private declarations, and
   diagnostics independently from the end-to-end fixtures.
3. Implement versioned execution encode/decode and canonical fingerprinting.
   Separate payload/domain identity from interface fingerprint construction.
   Test round trips, malformed/out-of-bounds references, unsupported versions,
   and invariance under diagnostic-only/session-layout changes.
4. Integrate execution availability and execution dependency validation into
   artifact construction, provider registry, loader, and comptime preparation.
   Reconstruct provider lexical scope and fresh session identity. No
   stdlib/type/provider-name branch is allowed.
5. Make required ordinary/Drop callee availability fail closed. Add compiler
   invariant tests alongside real source/artifact CLI negative acceptance.
6. Run the acceptance matrix below with fresh artifacts and physically
   unavailable provider/dependency source in artifact mode. Then rerun the
   literal contract suites, affected generic/comptime/ownership/artifact
   regressions, sysroot invariants, and full workspace with explicit exit codes.
7. Record evidence and remaining limitations; request design/freeze review.
   Do not freeze this capability or merge red-gate mixed worktree changes.

## Mandatory acceptance matrix

Every row remains UNVERIFIED until current implementation and execution
evidence establish it. This is a requirements matrix, not a passing-results
table.

| Case | Source | Fresh artifact-only |
| --- | --- | --- |
| Ordinary exported body called at comptime | Correct result | Same result |
| Provider-private helper | Correct result | Same result; private remains inaccessible |
| Provider-private constant | Correct result | Same result |
| Recursion within existing comptime limits | Correct result | Same result |
| Generic call / multiple instantiations | Correct result | Same result |
| Supported destructor execution | Correct drop behavior | Same drop behavior |
| Two providers with same private helper name | Provider-specific results | Same results |
| Reverse import order | Unchanged results | Unchanged results |
| Literal/error diagnostic in provider body | Correct diagnostic/context | Equivalent diagnostic; correct embedded provider context; no panic |
| Forbidden FFI | Reject | Reject |
| Forbidden runtime effect | Reject | Reject |
| Required ordinary callee body unavailable | Reject | Reject; no fallback |
| Required destructor body unavailable | Reject | Reject; never skip |
| Execution dependency stale, interface unchanged | Fresh source semantics / invalidate stale result | Reject/invalidate stale execution dependency |
| Interface dependency stale | Respect current source contract | Reject stale interface |
| Old/unsupported execution payload | Not applicable | Deterministic rejection |
| Source physically unavailable | Not applicable | All supported positive rows still work |
| Producer/consumer processes independent | Not applicable | Fresh process B consumes artifacts from terminated process A |

Additionally verify that diagnostic-only span/absolute-path changes and
session arena offsets do not change canonical execution fingerprints, while
body/private-constant/execution-dependency changes do. Interface fingerprint
must remain unchanged for private implementation-only edits.

Artifact-only tests must remove or omit provider and dependency .ln candidates
from the actual search roots. A flag claiming fallback is disabled is not
sufficient evidence. Producer and consumer compile in separate CLI processes;
same-session loading alone cannot prove portable identity.

## Current integration map and coverage boundaries

This map was checked against current source on 2026-10-02. It names integration
points, not completed implementation.

| Responsibility | Current component | Required integration |
| --- | --- | --- |
| Provider/source declaration ownership | driver `lib.rs`, `registry.rs`, semantic symbol ownership | Derive provider roots and walk their full typed AST reference closure |
| Arena identity/source context | AST `relocator.rs`; driver `external.rs` | Canonical local indices, checked source-unit mapping, fresh-session relocation |
| Execution encode/decode | llib `writer.rs`, `reader.rs`, `format.rs` | Separate versioned execution domain; reject missing/unsupported/corrupt data |
| Public interface identity | driver `metadata_builder.rs`; llib `metadata.rs` | Preserve interface/body distinction and private visibility |
| Execution dependency identity | driver registry/build/load; llib validation | Independent execution fingerprint graph and freshness checks |
| Ordinary body preparation | driver comptime preparation / MVIR generation | Resolve required bodies in the owning provider scope |
| Missing callee behavior | MVIR `interp.rs` | Required direct/Drop calls fail closed |
| Public acceptance | CLI fixtures and parity harness | Official build/load, separate processes, physically source-free artifact roots |

`ArenaRanges` currently tracks expr/decl/pat ranges only. It does not describe
statement/type/source-unit ownership, so taking those three ranges is not a
complete execution-closure extraction algorithm. Provider ownership and graph
reachability must supply the missing information; filtering by global indices
alone is not sufficient.

The existing `literal_typing_cli_parity.rs` prepares fresh bootstrap artifacts,
removes their source candidates, builds a user provider in a separate process,
and compares executable outcomes. Its six positive fixtures do not include
`literal_comptime_provider_gap.ln`. Thus that green matrix does not establish
the ordinary imported-comptime capability being added here. Extend/add an
explicit execution parity matrix; do not relabel those historical positives
as evidence for the missing feature.

The existing llib test file covers serialization round-trip, format/MVIR
version mismatches, invalid magic, corruption and unknown sections. These are
useful foundations, not tests of execution payload ownership, execution
fingerprint determinism, execution dependency invalidation, private helper
scope, literal source-unit relocation, or fresh-process comptime parity.

## Status and merge policy

The design is approved; no implementation gate is certified by this document.
LITERAL-TYPING-01 remains PARTIAL. Historical green focused results do not
certify the current concurrently edited compiler source. The latest previously
recorded full workspace run exited 101 during fresh JSON provider preparation.

The requested branch integration into main remains pending artifact/comptime
closure, coherent compiler/sysroot/full regression evidence, and audited
handling of legacy recovery contents. No merge, commit, push, reset, or branch
deletion is implied by this design record.
