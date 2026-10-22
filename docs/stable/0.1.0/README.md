<!-- luna-doc-role: guidance -->

# Luna 0.1.0 stable execution

Started 2026-10-21 at the maintainer's request to execute the
[stable plan](../../plans/0.1.0-stable.md). **IN PROGRESS / NOT QUALIFIED**.

The integration baseline is `c13a9852279666a56526968d4b61622bf7310bb8`,
merging the plan and Lunar head `483b03d3` without changing main or the alpha tag.
It contains compiler25/metadata9/MVIR5/format2 and 49 manifest providers.
[Baseline provenance](baseline.json) records immutable inputs and source trees.
It is not a stable qualification result.

## Current work

| Milestone | State |
| --- | --- |
| S0 baseline/backlog | Integrated baseline recorded; publication-state contradictions repaired; grammar/compatibility and maintainer-requested stdlib redesign in progress |
| S1 correctness | DEBUG removal and borrow maturity passed locally; I/O source/artifact, quiet CLI and diagnostic conformance passed at `0b09ed8b`; broader S1 audit remains open |
| S2 Lunar | 20 Windows candidate CLI smoke steps PASS with LLVM/GNU wrapper; raw-GCC Unicode-path failure retained; Ubuntu/package consumers pending |
| S3 packaging/license/docs | GPL-3.0-only applied to first-party source; notices/package source inventory and the wider website example audit remain open |
| S4 exact-pin qualification | Not started; no prior-pin result transferred |
| S5 RC/use/release review | Not started; no stable tag or publication |

[Work backlog](backlog.json) gives each `STABLE-*` item an owner, source paths,
gate, state and evidence. [Scope matrix](scope-matrix.json) preserves all 25 R4
wave/deferral records and 24 legacy repair records without replacing their
historical identities or treating a historical PASS as a current execution.

The [first local preparation record](evidence/windows-preparation-0387/summary.json)
includes the successful runtime ABI, CLI/sysroot, quiet CLI, borrow and Lunar
unit checks, and the failed candidate smoke. It is **not** an eight-lane result.
The reported `println("hello world")` example conflicts with the existing
`&[u8]` API: literal `str` uses `println_str`. STABLE-015's EN/VI I/O repair and
source/artifact regression passed locally at `0b09ed8b`:

- Both complete examples check/build/run with fresh source-only and
  artifact-only providers; stdout is `hello world` plus newline.
- The original literal-to-byte call rejects with E2001 and a source span;
  its type message is now `expected &[u8], got str`, without interner IDs.
- Three internal type-display tests cover source spelling, nominal generic
  names, inference resolution and bounded recursive/large type rendering.
- Quiet CLI and all four diagnostic-conformance tests pass; importer DEBUG
  noise is absent while original typed failures remain observable.
- The two EN/VI code blocks exactly match the executable fixtures. Local
  website build passes (33 pages, existing warnings); 243 maintained docs /
  1,956 links validate with no errors.
- Lunar's 20-step candidate smoke passes with freshly built compiler,
  runtime and 49-provider sysroot using the LLVM18/GNU wrapper.

[Command/log evidence](evidence/windows-io-0b09/summary.json),
[Lunar steps](evidence/windows-io-0b09/lunar/summary.json) and
[documentation checks](evidence/windows-io-0b09/documentation.json) record the
tested pin. These focused Windows results do not qualify an RC or stable
release. Remaining incorrect website examples and live deployment stay open
under STABLE-009. The user's installed alpha compiler and project are unchanged.

Console-macro clarification at `5b470fa7`: `io` already exports `println!`
and `std::println!` (plus print/stderr counterparts). The guide now includes
`std::println!("hello world")` and `std::println!("value: ", 42)`, alongside
the ordinary functions. The expanded three-example CLI matrix passes with
fresh source-only and artifact-only providers, and the EN/VI pages build with
exact fixture matches. [CLI evidence](evidence/windows-macros-5b47/cli/summary.json)
and [website evidence](evidence/windows-macros-5b47/site/summary.json) retain
this separate tested pin. No compiler or stdlib implementation was added.

Independent [installed-compiler probes](evidence/windows-macros-5b47/installed/summary.json)
and [installed Lunar project smoke](evidence/windows-macros-5b47/project/summary.json)
also pass: these macros are available in the user's alpha installation now.
Current console macros concatenate Display values; braces remain literal,
so `std::println!("value: {}", 42)` prints `value: {}42`. Format-string parsing
remains outside the retained formatting scope.

SKILL IMPACT: none. Existing Luna semantic, compiler/stdlib boundary and public
CLI testing guidance already cover this correction; no language rule changed.

The alpha [release closure](../../audits/r5-cpu-baseline-2026-10-20/release-closure.json)
supersedes the old `8525067f` CPU-package blocker for alpha only. Its eight
lanes and two clean package smokes are complete in that recorded scope.
Historical raw logs, archives, package SHAs and typed failure records remain
unchanged. Metadata-only Git history mappings are separate provenance; stable
must build and test a new immutable final pin.

## Stdlib API/layout redesign

At the maintainer's request, [joint API/layout design](../../proposals/stdlib-redesign-0.1.md)
now precedes stable qualification. The confirmed direction is compact std names,
type-associated constructors, usize memory sizes/indices and the existing
luna-rs/libs/external source root. STD-D06 supplements aborting allocation with
fallible try_* APIs; the [allocator design](../../proposals/stdlib-redesign-0.1/allocator.md)
records runtime primitives, ownership on failure and acceptance. The maintainer
then requested implementation: [STD-D06 execution](../../audits/stdlib-fallible-2026-10-21/README.md)
now has ABI 2 primitives and the first Vec/Box/String try_* APIs under validation.
This adds alloc_error (50 current providers); full redesign remains in progress.
Internal grouping, per-domain contracts and migration/compatibility remain open.
The [parser inventory](../../audits/stdlib-redesign-2026-10-21/README.md) maps 49
providers / 2,358 declaration records / 213 explicit imports at 264390da; it is
not resolved API or conformance evidence. STABLE-016 tracks design and migration.
Existing alpha and focused I/O results remain historical results for their pins.

The next [API foundation slice](../../audits/stdlib-usize-2026-10-22/README.md)
implements STD-D05 through containers and consumers, constructor expansion,
ordered-set algebra and u64 bit utilities. It requires compiler protocol 26 and
new bundles. The [C++23 functional map](../../proposals/stdlib-redesign-0.1/cpp-library-gap-map.md)
adds a prioritized backlog, with graph/basic CPU matrix/tensor as maintainer-
authorized extensions. Focused validation and final qualification are separate;
the foundation pin 28802f12 passes eight Windows CLI suites (29 tests) and 40
existing acceptance tests, with 50 fresh canonical artifact/object pairs.
The refreshed parser snapshot maps 2,411 declaration records / 219 explicit
imports. Ubuntu/workspace and final package qualification remain pending;
STABLE-016 remains open.

On 2026-10-23 the maintainer prioritized the [I/O API design](../../proposals/stdlib-redesign-0.1/io-api.md)
before further numerical/graph expansion and selected `$name` interpolation.
The proposal records byte/text/formatting boundaries, errors/progress, buffer
recovery and a separate generic macro prerequisite. No new I/O implementation,
runtime ABI or dollar interpolation support is claimed by that design work.

## Release boundaries

Supported native release scope remains Windows x86_64 GNU and Ubuntu 24.04
x86_64 GNU. Lunar's Rust host identity is separate from the Luna target.
ASan claims remain limited to instrumented C runtime/tests. Provider counts,
workspace green totals and ancillary macOS checks do not certify every public
API/domain or expand the advertised target matrix.

Current accepted non-blocking deferrals are C4-FU1 dependency precision and
D1-FU1 presentation, under their original reopening conditions. Standalone
comptime statements remain unsupported execution with spanned E4005 rejection.
No language requirement is removed by migrating a record into this ledger.

Stable requires no open P0/P1 and no correctness/contract violation within the
advertised scope, plus complete final-pin qualification, package/installer
consumer tests, truthful version labels and a maintainer release review.
