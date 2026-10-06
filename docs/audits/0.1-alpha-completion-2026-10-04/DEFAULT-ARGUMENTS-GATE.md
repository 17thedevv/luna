# Default declarations and namespace identity checkpoint — 2026-10-05

The [2026-10-06 omission follow-up](DEFAULT-OMISSION-GATE.md) supersedes the
absent synchronous execution path below. This record preserves its original
declaration-only revision and evidence boundary; the complete contract remains
PARTIAL.

**CALL-ARGUMENTS-v1 remains PARTIAL.** This checkpoint implements parameter
default declarations, definition-site checking and portable interface identity.
It does not implement calls which omit a defaulted argument. Those calls remain
required positives under the [adopted contract](../../spec/0.1/call-arguments-v1.md),
and currently reject E2001. The complete R0–R5 objective remains active; this
record grants no merge, tag, freeze or release authority.

## Implemented declaration path

- AST parameters retain the default expression and complete source span;
  relocation adjusts both. Parser tests exercise a complete nested expression.
- Resolution and macro expansion use the defining scope with only earlier
  parameters visible. Parameter identities are reused, not copied or joined
  by spelling. Function, impl and method generic binders remain independent.
- Declaration defaults undergo ordinary expected-type checking under rigid
  generic binders and the declaration's unsafe context. Receiver defaults,
  required parameters after defaults, later/self parameter references, invalid
  types, unsafe operations and changed/introduced trait impl defaults reject.
- Trait signature defaults are the call contract. An implementation may omit
  their redeclaration; it cannot substitute a different contract.
- Callable metadata carries canonical default token contracts, while executable
  expressions remain in the portable AST. Compiler protocol is **17** and
  semantic metadata schema is **9**; file format and MVIR epochs are unchanged.
  Readers reject malformed length, empty defaults, receiver defaults and
  required-after-default metadata. The isolated sysroot was rebuilt officially.

Canonicalization ignores whitespace/comments and replaces references to
parameters and generic binders using their resolved identities and ordinal roles.
Impl/trait owner roles and method/function roles are separate. Field spelling
is preserved: `self.base` must not treat `base` as a parameter merely because
the method also has a parameter with that name. Resolved declaration references
retain the original logical declaration path rather than the local namespace
alias. Explicit generic argument tokens remain part of the default contract.

The original generic fingerprint regression exposed missing resolution of
types inside casts and other expressions. The resolver now visits cast target
types, call/method/struct generic arguments and sizeof/alignof types. Renaming
function, method and owner binders preserves the verified public identity.
Changing a default expression or retargeting its namespace alias changes it.
Changing a private helper's body alone retains interface identity; source/body
and dependency identities remain separate obligations.

## Namespace alias defect and repair

A standalone provider declares original namespaces `first` and `second`, then
`using second as selected`. Before the repair, source check/build/native pass;
artifact check passes but build rejects `second::make` with E1001. The export
walker can visit the alias's shared target scope before its original owner and
then suppress the owner as already visited. Public metadata can even contain
the private alias instead of the original namespace.

Extraction now excludes namespace aliases before visiting their shared scopes.
Logical owner-path discovery also excludes aliases. The permanent
[namespace CLI regression](../../../luna-rs/crates/luna-cli/tests/namespace_alias_export_cli.rs)
checks original namespace identity, native behavior after relocation/source
removal, and rejection of external lookup of the private alias in both modes.
This preserves NAMESPACE-USING-v1 and does not add re-export semantics.

## Evidence boundaries

The lossless [broad record](evidence/default-declarations/broad/exit.json) is an
immutable **788-input** candidate preceding the owner-role and alias amendments.
Workspace/all-target compilation exits0; 331 internal and 46 CLI/driver tests
pass. Do not attach that result to later edits. Its logs preserve development
failures, the incorrect unsafe diagnostic oracle, the genuine generic identity
defect, owner/alias probes and the required omission-positive failure.

The [amended record](evidence/default-declarations/amended/exit.json)
pins **792 inputs** and compiler SHA256
`fbe2e7d1bbac4d1dab35a416a4dbdfc0ecb6a2fe45873eb8f96500b90bcd69c6`.
Inputs, compiler and runtime match their start hashes. Workspace/all-target
compilation, **331 internal and 48 CLI/driver tests** pass, with every command
exit0. This is a Windows GNU native language check using LLVM18 and the recorded
Windows runtime; it is not new POSIX or full-workspace evidence. Its scope
includes default declarations, original namespace export, default-binding
identity/freshness and regressions for named calls, `using`/`luna.toml`, comptime
admission, module const, method selection and portable constraints. Each public
CLI matrix creates independent fresh source and artifact provider roots;
artifact consumers have no provider source fallback. Checks, builds and native
executions are separate observations, not inferred from metadata acceptance.

The default CLI matrix includes ten declaration-negative fixtures in both
check/build and provider modes (40 typed rejections), full-arity native controls,
identity controls, and stale-dependent rejection after replacing complete
provider bundles. Full-arity controls exercise direct/generic/reference/inherent
and qualified trait calls. They establish preservation of existing calls, not
execution of a default. A separate final-candidate omission probe still exits1
with E2001 for source/artifact check/build and produces no executable. Its logs
are retained in the amended record as a required positive gap.

### Follow-up: namespace aliases selecting module constants

A subsequent standalone identity probe found another defect: changing the
target of `selected::VALUE` between two different module constants retained
the same public interface fingerprint. The canonicalizer recorded resolved
function/type references but missed module-level constant/value references.
This is a real interface-identity gap; it does not justify dropping defaults
from the retained contract.

Canonicalization now includes resolved module/global constant/value declaration
paths. It does not reinterpret arbitrary local variables as global declarations.
Permanent controls separately rename and retarget the constant alias, retaining
the former identity and changing the latter, and reject stale dependents after
coherent provider-bundle replacement. Raw before/after metadata probes remain
separate from the earlier immutable amended candidate. The last candidate's
focused verification is recorded separately; the 331/48 result above precedes
this amendment and is not attributed to it.

The [last constant-identity candidate](evidence/default-declarations/constant/exit.json)
pins 792 inputs and compiler SHA256
`fccc2d009ad54980180636a25b405bb88a78aec8192e3734d693eadaf4cd7f6f`.
All input/compiler/runtime hashes match at completion. Workspace/all-target
check, **213 semantic/metadata tests and three CLI suites** pass, all exit0.
Those suites exercise expanded default declarations/identity/freshness, original
namespace export and public constraint identity. The amendment only changes
default reference canonicalization and its fixtures; it does not change call,
ownership or native ABI lowering. The earlier broader regression remains dated
evidence, while these focused checks cover the new identity change. Raw failed
development build and before/after constant probes are retained. Final omission
check/build probes still reject E2001 in both modes; no default runtime or
release conformance is inferred.

## Remaining required implementation

1. Record missing parameter ordinals and the authoritative default declaration
   in the checked call plan. Structural callable values retain full arity.
2. Lazily materialize the actually requested omission pattern and its concrete
   substitutions. Preserve the full-arity ABI and distinct entry identity.
3. Capture explicit arguments once in source order. Initialize supplied
   parameter storage, then evaluate omitted defaults in declaration order and
   execute the original body in that same logical callee frame. A forwarding
   owner move must not invalidate a default borrowing the earlier parameter.
4. Carry definition-site helper identity and trait-to-impl parameter roles by
   ordinal through mono and portable artifacts. Visit only omitted defaults;
   discover their concrete calls and drop obligations normally.
5. Derive omission-entry ownership/lifetime/access effects from its actual
   inputs/body. Do not copy full-arity parameter indices or imported effect
   summaries onto an entry with fewer inputs.
6. Complete omission/override/order, fresh non-Copy ownership, earlier-parameter
   borrow, move/escape, unsafe/async/comptime and relocated artifact matrices;
   run relevant regressions and a new immutable full workspace candidate.

No zero/null placeholder, eager exponential specialization, stdlib-name branch,
new intrinsic or backend opcode is authorized by this feature.

## Compiler boundary and skill impact

Compiler Change
    Capability: Definition-site parameter defaults and stable namespace ownership.
    Why stdlib exposed it: Maintainer-requested language capability; no stdlib shortcut.
    Why it is generic: Uses ordinary declarations, binder identities, scopes and metadata.
    User-defined type benefiting: Counter, Cell<T>, Processor<U>, arbitrary providers/functions.
    Tests: default_arguments_cli, namespace_alias_export_cli, parser/AST/metadata invariants.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

SKILL IMPACT: **REFINEMENT** of luna-language-capability-validation's canonical
identity checks, based on the adopted contracts and permanent regressions.
The guidance distinguishes binder roles/field names, alias rename/retarget and
native namespace export checks from omitted-call execution. Re-read and
cross-check with grammar and semantic guidance; no semantic rule or maturity
claim is introduced. Existing semantic guidance already requires independent
binders. Test counts and this partial status remain in the audit record.
