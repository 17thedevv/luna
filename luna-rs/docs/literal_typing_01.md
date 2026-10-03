<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../../docs/spec/0.1/README.md).

# LITERAL-TYPING-01 — Numeric and Byte Literal Closure

Status: PARTIAL — NOT FROZEN (updated 2026-10-02).
Authority: maintainer requested implementation of the preceding closure plan.

## Contract

Integer suffixes select the integer type; unsuffixed literals receive a concrete
integer expected type when available, otherwise default to i32. Validation uses
exact magnitude, signedness and target width before lowering. Existing typed
variables are never implicitly converted. Explicit suffixes are never overridden.
Unary-negative literals permit the signed minimum, but negative unsigned literals
are rejected. Malformed and out-of-range input receives a source diagnostic.

The maintainer chose migration of oversized legacy `value as T` literals to
suffixes, not changing `as` into contextual literal typing. Cast expressions do
not provide their result type as the operand's expected type. Existing conversion
semantics and expression precedence remain unchanged.

Byte literals have type u8. Byte strings are owned value arrays `[u8; N]`, where N
counts decoded bytes, with no implicit trailing NUL, UTF-8 conversion or immortal
borrow. Direct characters are ASCII; escapes support `\n`, `\r`, `\t`, `\0`,
`\\`, `\'`, `\"` and `\xNN`. Unicode escapes/non-ASCII direct characters are
rejected. Ordinary string and char semantics are unchanged.

## Preflight

Generic capability: numeric literal elaboration and byte-array literals.
Enforcing phases: lexical decoding, semantic expected types/range checks,
typed MVIR constants, comptime and LLVM constant emission.
Runtime/ABI changes: none intended. No stdlib-name recognition or new intrinsic.
Source/artifact: original literal AST is portable; consumers re-elaborate using
fresh semantic types. If artifact schema changes it must be version-gated.

## Closure gates

1. Decimal/radix/underscore/suffix decoding with precise diagnostics.
2. All existing integer widths, signed extrema and target-sized boundaries.
3. Declaration/assignment/return/call/method/aggregate/operator/generic context.
4. Strict-typing, ambiguity, malformed-input and overflow negative fixtures.
5. Byte literal/array length/escape/const/borrow lifetime tests.
6. Runtime and comptime equivalence; fully typed full-codegen constants.
7. Fresh source-only versus artifact-only user-provider parity.
8. Numeric, char, generic, frozen stdlib and sysroot regression gates.
9. Full workspace exit 0, scoped formatting and diff checks.

The current worktree also contains uncommitted Antigravity work. Its previously
demonstrated independent defects must not be hidden or included in this feature's
completion claims. The maintainer subsequently authorized merging all branches
after completion; a focused pass does not satisfy that prerequisite.

## Implementation and generic compiler repairs

`luna-lexer/src/literal.rs` decodes exact u128 magnitudes and byte escapes.
`luna-parser/src/expr.rs` accepts the existing byte literal tokens. No new
keyword or AST/artifact schema was introduced.

`luna-semantic/src/typechecker.rs` selects suffix/context/default types and
checks bounds before lowering. Expected types flow through declarations,
assignments, returns/tails, ordinary and method calls, generic result context,
struct/tuple/array/enum payloads, typed binary peers, and integer patterns.
Context is isolated from cast operands, match subjects, indexes and unrelated
block declarations. Index literals receive usize context. Pointer-width
selection is explicit in `SemanticContext::target_pointer_bits`; the native
driver defaults to host width. The 32-bit tests establish semantic validation,
not native 32-bit execution or cross-compilation support.

`luna-mvir/src/generator.rs` emits typed canonical numeric constants and owned
byte arrays. `luna-mvir/src/interp.rs` preserves integer width, unsigned
ordering, checked comptime arithmetic and byte-array memory initialization /
indexing. Constant arrays used as lvalues are materialized rather than treated
as undefined globals. `luna-optimizer/src/passes/const_fold.rs` receives immutable
integer range facts from the driver; it leaves overflow operations for runtime
lowering instead of folding an out-of-range value as a wider integer.

Generic infrastructure repairs were needed:

- Early comptime evaluates only reachable lowered bodies, including concrete
  trait-object implementations and drop methods. Semantic analysis elaborates
  source bodies before lowering, preserving callee scope and caller state.
  Imported bodies are already elaborated in their owning provider and their
  tables are injected. They must not be rechecked using reconstructed export
  scopes in a different provider. Preparation now requires the same provider
  identity as well as a local body scope. Virtual dispatch selects the concrete impl's
  method identity, not a matching suffix of a function name.
- Contextual generic enum instantiation must not change declared payload
  arity. A single tuple-valued field is one argument. Multi-field enum codegen
  now stores and extracts each field using the instantiated tuple layout;
  previously backend lowering stored only the first argument.
- Typed numeric literals introduce an immutable `Assign(Number(...))` value
  before casts. The null-sentinel proof now follows `Assign` as well as `Cast`;
  this restores existing empty-owner behavior without granting provenance to
  nonzero integers. Dedicated negative fixtures still reject nonzero safe
  anchor establishment and null-to-safe-reference promotion.
- Monomorphization now records rejected `MonoInstance` values separately from
  successfully instantiated units. A recursive call could previously enqueue
  the same instance after its concretization barrier failed, growing the
  worklist and diagnostics indefinitely. Semantic inference precedes this
  barrier; retrying the identical substitution cannot resolve it. The fix
  retains the original error and never publishes an unresolved unit. A real
  recursive negative fixture must terminate with `E_UNCONSTRAINED_INFERENCE`;
  a concrete recursive positive still executes correctly in both provider modes.

These repairs apply to ordinary user-defined types (`ByteSource` / `Parcel` / `EmptyOwner` in
the fixtures). No stdlib type/provider branch, new intrinsic or lang item was
added. Backend unresolved-generic checks and borrow/ownership rules remain in
force. Existing unmonomorphized generic comptime rejection is unchanged.

## Evidence and limitations

Focused literal CLI matrix: six positive fixtures in source and fresh
artifact modes, plus 25 negative cases in both modes. Positive programs execute
and compare exit code/stdout/stderr; artifact roots contain no provider `.ln`
fallback. The user provider is built in a fresh process against the same fresh
bootstrap artifact context. No artifact identity validation is bypassed.

Internal lexer/parser/semantic/MVIR/optimizer regression: 223 tests passed,
exit 0 after the latest changes. Borrowck: 60 tests passed, exit 0.
Comptime/dyn regression: 16/16; generic dispatch: 7/7; generic method
monomorphization: 5/5; non-nominal bounds: 1/1, exit 0.
After the monomorphization convergence repair, the full semantic crate ran
184 tests, exit 0; the updated CLI matrix also exited 0. Negative compiler
invocations now have a 30-second termination gate so a recursive retry defect
cannot leave this regression running indefinitely.

Full workspace was actually run and ended exit 101, not exit 0. The latest
completed workspace run stopped in `char_operator_cli_parity` while its fresh
whole sysroot failed building `string` (`cannot find type std::Option`). The
follow-up trace proved that this error was caused by comptime preparation
rechecking an imported `slice` body in the wrong provider scope. The subsequent
anchor failure was also a literal integration regression: the null recognizer
did not follow the new typed `Assign`. Both were repaired generically; they
must not be blamed on stdlib naming or hidden by a stdlib workaround.

The next fresh whole-sysroot run built through `string`, encoding, crypto and
file (46 artifacts), but emitted no further artifact for over five minutes.
Only the verified owned harness/child processes were stopped. That run exited
abnormally and is INCOMPLETE, not PASS. A fresh verbose trace located the stall
at `json`, and a full-source diagnostic probe located it in monomorphization,
after typechecking. The same rejected instance was re-enqueued repeatedly;
queue and diagnostic counts grew while the completed-instance count stayed
unchanged. The generic failed-instance fix above makes the same JSON probe
terminate with exit 1 and unresolved-inference diagnostics in approximately
two seconds. This proves convergence, not JSON correctness or a green sysroot.
The current manifest changed from 48 to 49 provider entries during the audit.
The concurrent stdlib changes are therefore not a stable freeze baseline.
No full-workspace exit 0 after the latest fixes is claimed.
The latest post-convergence workspace rerun (`literal-workspace-convergence.log`)
completed with exit 101. Both `char_operator_cli_parity` cases fail while
preparing the fresh whole sysroot: `Failed to build json` with
`E_UNCONSTRAINED_INFERENCE`. They do not reach their char acceptance fixtures.
This is a red current-baseline gate, not evidence that char semantics regressed
or that the underlying JSON inference errors have been classified as pre-existing.

Concurrent-source warning (2026-10-02): a subsequent read found that the
`failed_instances` change had disappeared from `mono.rs`, whose tracked diff
had become empty. No commit or merge explains this change. This task did not
restore or overwrite the concurrent edit. The passing convergence tests above
belong to the earlier binary/source state, not the current source tree. Compiler
edit ownership must be coordinated and the chosen source rebuilt/retested
before those results can certify the worktree. No actor or intent is inferred.

Additional isolated probes were retained: `recursive_nominal_container_probe.ln`
uses only ordinary `Bucket<T>`, `Node` and `Entry` declarations and reproduces
unresolved inference without JSON, Vec or String. `forward_nominal_type_probe.ln`
and `forward_nominal_unused_probe.ln` further reduce the symptom to a struct
field naming a later-declared struct; member lookup fails. In `lower_type`, a
resolved nominal symbol lacking a populated type is replaced by a new inference
variable. `populate_types` populates declarations sequentially. Both mechanisms
also exist in the source of `main`; this is history evidence, not execution of
an older compiler. No new gap ID or fix is asserted for these probes.
`forward_nominal_ordered_control.ln` differs only in declaring `Later` before
`First`; it compiled and executed with exit 0. The diagnostic binary SHA-256
was unchanged across that run. This isolates declaration-order sensitivity.
The maintainer then directed this task to wait for the concurrent compiler/JSON
task to finish and audit afterward, and to review the artifact design in detail
before implementation. No compiler fix is authorized while waiting.
Full stdlib/char/formatting regression and canonical artifact sidecar closure
therefore remain pending. A run against the older source-only audit snapshot
is not evidence for current sysroot counts or persistent artifact invariants.

A separate boundary probe found source/artifact divergence for **comptime
calling an imported non-generic function**:

`literal_comptime_provider_gap.ln` calls `literals::bytes()` in a comptime
initializer. Source provider compilation accepts it; fresh artifact compilation
rejects it because the function body is not available in the comptime module
(`symbol ... not found in comptime context`). Non-generic function bodies are
not retained by the existing artifact contract. This is not specific to byte
decoding and is not repaired by guessing a return value or reading unavailable
source. The investigation fixture is retained separately; it is not counted
as passing parity evidence. A deeper artifact audit found that a MVIR section
does exist, but it is not a complete typed executable IR: `MlibValue` has no
semantic type, the writer emits empty type/string tables, and the loader
discards the decoded module. Its presence cannot justify interpreting it as
portable comptime code. Extending portable comptime body availability needs
a separately scoped artifact/compiler investigation and contract review.

## Requested main merge

Remote `origin/main` was fetched successfully and matches local `main`
(`3dac3ac`) at this audit. No merge or push has been performed.
The unmerged branches are `codex/phase6-formatting`,
`codex/phase6-formatting-closure`, and `recovery/pattern-matching`.
The recovery branch's historical diff contains 4,825 files, including old
compiler trees and `.diff-tmp` failure artifacts; its inclusion versus archival
needs an explicit maintainer choice. It must not be indiscriminately copied
into the canonical source tree. No branches or user changes were deleted.

Scoped formatting applies only to the new Rust files. Scoped tracked compiler
diff check passed. Whole-worktree diff check remains nonzero: the latest check
reports unrelated trailing blank lines in `luna-driver/src/importer.rs`,
`luna-driver/tests/test_sysroot_build_invariants.rs`, and
`runtime/src/platform/windows/sync.c`.
Runtime code was not changed by this feature; no runtime ABI pass is claimed.

The approved numeric migration changes only oversized legacy literal casts
needed to express the same intended value with a suffix. Ordinary casts and
strict typing remain unchanged. Deliberate malformed/out-of-range fixtures
are not silently converted into positives.

Skill impact: no skill files changed by literal work. A minimal documentation
clarification about literal expected-type elaboration can be proposed after
closure; this report does not create a frozen language rule.

## Remaining closure gates

1. Review the imported non-generic comptime body-availability boundary; do not
   claim source/artifact support for it before a generic, portable solution.
   On 2026-10-02 the maintainer approved opening a separate design-first task
   for this boundary. That task is audit-only until a portable artifact contract
   is reviewed; it does not authorize silently extending the schema.
   Design task: `01a0faac-fd24-7e70-bad8-29dd2f269de0`, titled
   "Thiết kế artifact/comptime portable cho Luna". No artifact implementation
   or main merge is authorized by its audit-only scope.
   Its read-only design audit is complete. On 2026-10-02 the maintainer approved
   the refined design: separate interface/execution payloads, provider-owned
   execution closure with artifact-local identities and stable external refs,
   provider-context re-elaboration, independent execution fingerprint/dependency
   validation, diagnostic source relocation, version gates, and fail-closed
   ordinary/Drop callee availability. Body availability remains distinct from
   evaluability; this does not expand permitted comptime effects.
   See `portable_comptime_artifact_v1.md` for the approved contract and mandatory
   positive/negative, import-order and fresh-process artifact-only matrix.
   Implementation/evidence are pending. On 2026-10-02 the maintainer stopped
   the concurrent task and explicitly handed the compiler work to this agent.
   Shared-file work can resume; unrelated legitimate edits must be preserved.
2. Stabilize the separate stdlib/provider changes and obtain fresh sysroot,
   frozen numeric/char/formatting and full workspace regression evidence.
3. Only then request design/freeze review. This record is not authorization
   to merge the mixed worktree or declare LITERAL-TYPING-01 frozen.
