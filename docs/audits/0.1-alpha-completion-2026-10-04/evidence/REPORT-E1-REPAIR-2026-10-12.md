# E1 checked-arithmetic and diagnostic repair — 2026-10-12

Task base/E2 candidate: `6017ba8d8db40cf0bc16498f639e5d830c0e827c`.
Implementation base: `2fcce5a4ea1973100a3d8a99781e1188aeed5d78`; historical
blocker checkpoint: `0fff28d83ba317d1ab6bb6edef6ce1b383a9fd3b`.
This report records implemented repairs and development evidence. Final
qualification remains pending; candidate advancement is not justified yet.

## Adopted amendments and actual ownership

The maintainer adopted E1-DIAG-OWNERSHIP-01 and its supplemental mappings:

| Condition | Code | Owning admission |
|---|---|---|
| Malformed/unknown annotation grammar | E0005 | Parser |
| Reserved compiler symbols/renames; language binding trust/identity | E1009 InvalidCompilerBinding | Resolver |
| Invalid contextual annotation; link on main; resolved intrinsic signature | E2032 InvalidAttributeApplication | TypeChecker |
| Invalid integer/byte literal grammar | E0003 | Parser, also re-admits materialized AST |
| Macro used as a runtime value | E2001 | TypeChecker; Resolver only binds identity |
| Macro rule mismatch/expansion recursion | E1010 InvalidMacroExpansion | Resolver admission of structured MacroEngine failures |
| Unsupported standalone comptime statement | E4005 | Comptime admission; support gap retained |
| Invalid/unexpanded internal AST/token | E6001 | Infrastructure invariant reporting |

AttributeProcessor performs transformations and transports structured failures;
it does not assign semantic diagnostic codes. TypeChecker owns derive
preparation admission needed before symbol binding. Parser-created reparse
diagnostics retain their origin. Resolver admission handles binding guards
after expansion, including nested/user-provider declarations. Source import
propagation preserves the provider's primary span and adds import context as
a related label. Registry parity now checks every row's declared owner.
Entry constraints use the resolved root function's scope/provider identity;
a helper named `main` in a namespace is an ordinary callable, including with
`#[link]` and parameters. It does not inherit the program entry's constraints.
Mono uses the same identity when preserving the runtime entry name, preventing
namespace helpers from colliding with the Model B entry wrapper.

The historical [blocker report](REPORT-E1-DIAGNOSTIC-BLOCKER-2026-10-12.md)
and acquisition JSON remain immutable. The amendment supersedes the prior
E0005 mapping; it does not relax DIAG-8 or alter existing code meanings.

## Additional checked-arithmetic defect and compiler24

MVIR DCE treated ordinary integer operations as removable when unused. It
erased `2147483647i32 + 1i32;` and `7u32 / 0u32;` before LLVM could emit guards;
both programs returned zero. DCE now preserves potentially trapping integer
Add/Sub/Mul/Div/Rem/Shl/Shr/Neg and their operands. With explicit type facts,
dead floating-point/pointer operations remain removable. Constant folding may
remove a proven-safe integer operation; it cannot discharge an overflowing one.

A genuine compiler23 provider published by the pre-repair release binary
embedded this unchecked execution behavior. A compiler23 consumer accepted
it and returned zero; a fresh repaired provider trapped with L005. The
maintainer explicitly approved **compiler24**, retaining **metadata9/MVIR5/
format2**. Both genuine compiler22 and compiler23 archives are rejected before
payload decoding. No header bytes were edited to manufacture old evidence.
See [the old23 execution probe](e1-compiler23-pre-dce-artifact-probe-2026-10-12.json)
and [producer provenance](compat23/provenance.json). The prior compiler23
Stage 4 DONE record is historical and is superseded by this boundary.

## Provider parity defects found during qualification

Source admission used the requested `core/panic` alias as stable identity,
while publication used canonical `panic`. Every stable symbol and impl identity
therefore hashed differently. Admission now selects authenticated manifest
identity and keeps invocation aliases as discovery keys. Fingerprint checks
are unchanged; no comparison or trust check is bypassed.

Provider macro qualification also exposed missing relocation of top-level
statement IDs. Source and portable AST now relocate statements as well as
declarations. Source providers run registration/expansion before resolution,
and publication serializes the expanded, admitted item list. User-provider
macro negative controls must reject without publishing an artifact; valid
macro-generated arithmetic is tested through source and relocated artifacts.
Generated private parameters also exposed an access-scope error: hygienic
expansion-local bindings incorrectly inherited the macro definition scope for
privacy checks. Same-expansion bindings now use their lexical scope and existing
provider/visibility checks; uncaptured definition-environment names retain
definition-site authority. The generic hygiene regression passes with the
existing anti-collision and definition-site controls.
The valid provider explicitly captures its public namespace/function names;
uncaptured template names retain hygiene. Artifact publication now extracts
metadata from the union of original root nodes and expansion nodes, excluding
intervening imported AST. Previously, callable signatures for generated
functions fell outside the original range, and the unchanged strict reader
correctly rejected the artifact as corrupt. The repair includes generated
callable/impl/const/type facts without relaxing metadata validation.

The Windows debug CLI previously exhausted its platform main stack before the
128-expansion recursion guard. Compilation now runs on a worker with an explicit
16 MiB stack, preserving the existing limit. A valid 100-expansion chain and
recursive invalid controls exercise the real CLI; stack overflow is not counted
as a diagnostic or arithmetic trap.

## Development evidence and remaining scope

Focused internal evidence: registry parity 3 PASS; annotation ownership 4 PASS;
macro expansion/fragment/hygiene/type suites 21 PASS; prior annotation system
20 PASS and memory intrinsic contracts 3 PASS. DCE/effect tests 5 PASS.
Compiler24 reader compatibility rejects real22 and real23: 1 PASS.
These are development executions on the working tree, not the final SHA gate.

The fixture set now contains **59 rows**: expected **34 TRAP / 9 PASS /
12 WRAP / 4 COMPILE_ERROR**. The original 46-row subset expects **22 TRAP /
8 PASS / 12 WRAP / 4 COMPILE_ERROR**. TRAP acquisition requires exact Luna
L003/L005 arithmetic panic text; crashes, timeouts, signal exits, missing DLLs
and compiler ICEs never count as traps. Comptime errors require one E4005.
Final four-way source/fresh-artifact and debug/release acquisition, relocated
bundle acceptance and three consecutive default-parallel workspace passes
remain pending at this checkpoint.

The related E0003 scan covered Resolver, MacroEngine and TypeChecker. Resolver
scope-grammar checks now delegate to Parser; MacroEngine's direct E0003 match/
recursion emitters are replaced by structured failures. The remaining fallback
scan reproduced a standalone comptime statement rejected without a span. Its
approved replacement is E4005 from Comptime admission, with body/source-file
span (including empty blocks). Four internal AST/token fallback emitters now
use Infrastructure-owned E6001. Structural tests forbid direct E0003/E0005
construction in the audited semantic modules. This scoped repair is not proof
of repository-wide DIAG-1..10 conformance; broader diagnostic coverage remains
a release-audit obligation. Standalone comptime statement execution remains an
explicit support gap, while comptime expressions retain their MVIR evaluation.

Windows native GNU execution is being qualified on this host. Ubuntu GNU
normal/ASan is **NOT VERIFIED** here and remains an R5 obligation. Runtime ABI,
MVIR operations/wire version, metadata layout, format, promotion/signedness,
and explicit wrapping contracts are unchanged. No R4/R5 tagging or main merge
is performed.
