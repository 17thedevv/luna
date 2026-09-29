# STD-NAMESPACE-01 — Canonical Standard Library Namespace

Status: **RESOLVED & FROZEN — 2026-09-27**

## Frozen direction

Every public Luna Standard Library API is named under the logical namespace
`std::`. Provider identity and physical source/artifact layout remain
independent implementation and build concepts; they do not create public
namespaces.

Examples:

```luna
std::Vec<T>
std::String
std::Box<T>
std::HashMap<K, V>
std::path::join(...)
std::io::println(...)
```

`import <vec>;`, `import <path>;`, and similar declarations continue to name
providers. They do not make `Vec`, `path`, or `io` canonical root namespaces.
There will be no permanently co-equal legacy root API. If migration requires a
temporary compatibility spelling, it must be explicitly marked transitional
and removed before STD-NAMESPACE-01 is frozen. A one-time breaking migration is
preferred where no clean alias/re-export mechanism exists.

Path lexical semantics are unchanged by this migration. Only the public name
changes from `path::...` to `std::path::...`.

## Provider/public-surface inventory

Physical providers currently remain independently declared in
`libs/external/sysroot.toml`. Public declarations needing canonical namespace
placement include:

| Provider family | Canonical logical surface | Current migration concern |
| --- | --- | --- |
| `alloc/vec` | `std::Vec`, vector APIs and iterator types | Owned/container surface slice complete; root exports removed. |
| `alloc/box` | `std::Box`, box constructors | Owned/container surface slice complete; root alias removed. |
| `alloc/string` | `std::String` and string APIs | Owned/container surface slice complete; root wrappers removed. `utf8` remains a provider-private implementation module. |
| `alloc/hashmap`, `alloc/hashset` | `std::HashMap`, `std::HashSet`, public collection iterators and APIs | Collection surface slice complete; root type/function aliases removed. |
| `alloc/iter_collect` | `std::iter_collect_vec`, `std::iter_collect_hashmap`, `std::iter_collect_hashset` | Collection surface slice moves the public helpers under `std`. |
| `core/result` | `std::Result`, `std::OptionExt`, `std::result_flatten`, `std::option_transpose`, `std::result_transpose` | Ordinary-core surface slice complete; public declarations now live under `std`. |
| `core/cmp` | `std::Eq`, `std::Ord`, `std::{min,max,clamp}`, primitive comparison helpers | Ordinary-core surface slice complete; public declarations now live under `std`. |
| `core/hash`, `core/clone`, `core/copy`, `core/default` | `std::{Hash,Clone,Copy,Default}` | Ordinary-core surface slice complete; scalar `Copy` impls and public traits are under `std`. |
| `core/convert` | `std::{Convert,TryConvert,TryConvertError}` | Ordinary-core surface slice complete; both conversion protocols are under `std`. |
| `core/float` | `std::{ToBits,FromBits,FloatOps}`, bit helpers and NaN/Infinity factories | Ordinary-core surface slice complete; methods remain ordinary trait dispatch. |
| `core/num` | Checked, saturating and absolute-value helpers under `std::` | Ordinary-core surface slice complete; provider name `num` is not a logical namespace. |
| `core/try` | `std::{Try,FromResidual,ControlFlow,Infallible}` | Language-contract slice is already canonical; Result references use `std::Result`. |
| `core/iter_adapters`, `iter_consumers`, `slice`, `mem`, `ptr` | `std::iter`, `std::slice`, `std::mem`, `std::ptr` | Utility/module surface slice complete; two independent iterator providers aggregate under `std::iter`. |
| `core/panic` | `std::panic` | Ordinary-core surface migrated; provider identity remains `core/panic`. |
| `io/io` | `std::io::{print, println, eprintln}` | Canonical namespace slice complete; provider identity remains `io`. |
| `path/path` | `std::path::{join, parent, file_name, extension}` | Canonical namespace slice complete; lexical behavior remains frozen. |
| language-contract providers | `std::Option`, `std::Iterator`, `std::Drop`, etc. | Language-contract slice migrated. The global closure gate below verifies canonical paths and rejects bare legacy trait impl paths. |

Internal providers such as `__alloc_global` and `__raw_table` are not public
surface. `__raw_table` implementation declarations remain provider-internal;
RawTable, RawTable iterator engines, ProbeResult, and raw-table constructors do
not appear as `std::` API. Internal ABI names are not reclassified as
standard-library API.

## Migration constraints and gates

1. Preserve provider IDs, physical source paths, dependency edges and artifact
   boundaries unless a separate design explicitly changes them.
2. Preserve behavior and semantic contracts. Do not special-case provider or
   type names in the compiler to implement namespace migration.
3. Language-contract auto-visibility must become path-aware and selective:
   compiler-required items may be made visible according to their canonical
   logical path without making all of `std` implicit and without exporting
   legacy root aliases.
4. Migrate stdlib-internal consumers, repository Luna fixtures, compiler test
   snippets and documentation before removing old spellings.
5. Rebuild canonical `.llib`/`.obj` artifacts from current sources and prove
   fresh-session source/artifact parity. No source fallback may mask missing
   artifact exports.
6. Add positive tests for representative `std::` paths and negative tests
   proving that removed root/provider-derived paths no longer resolve.
7. Update namespace freeze/audit documents only after tests and provider
   invariants pass. Until then the status remains migration-in-progress.

## Language-contract audit — canonical targets

Compiler searches were audited in `luna-semantic/src/typechecker.rs`,
`luna-semantic/src/mono.rs`, `luna-semantic/src/lib.rs`, and
`luna-driver/src/registry.rs`. The exact compiler-facing items are:

| Language item | Provider | Existing attribute/declaration | Canonical path | Compiler use |
| --- | --- | --- | --- | --- |
| `Drop` | `__lang_drop` | `#[lang("drop")] trait Drop` | `std::Drop` | `SemanticContext::needs_drop`; move/drop glue in `mono.rs`; move/type validation in `typechecker.rs`. |
| `DropFn` | `__lang_drop` | `#[lang("drop_fn")] Drop::drop` | `std::Drop::drop` | Explicit drop-call recognition in `typechecker.rs`. |
| `Option` | `__lang_option` | `#[lang("option")] enum Option<T>` | `std::Option` | `for` lowering/typing and iteration result handling in `typechecker.rs`. |
| `OptionSome` / `OptionNone` | `__lang_option` | `#[lang("some")]` / `#[lang("none")]` variants | `std::Option::Some` / `std::Option::None` | Enum variant identity used with the Option language item. |
| `Iterator` | `__lang_iterator` | `#[lang("iterator")] trait Iterator<Item>` | `std::Iterator` | `for` protocol selection in `typechecker.rs`. |
| `IntoIterator` | `__lang_into_iterator` | `#[lang("into_iterator")] trait IntoIterator<...>` | `std::IntoIterator` | `for` protocol selection in `typechecker.rs`. |
| `Copy` | `copy` | `#[lang("copy")] trait Copy` | `std::Copy` | Copy-bound/ownership checks in `typechecker.rs`; Drop-vs-Copy invariant. |
| `Try` | `try` | `#[lang("try")] trait Try` | `std::Try` | `?` desugaring/typechecking in `typechecker.rs`. |
| `TryFromOutput` / `TryBranch` | `try` | `Try::from_output` / `Try::branch` lang methods | `std::Try::from_output` / `std::Try::branch` | `?` method selection in `typechecker.rs`. |
| `FromResidual` | `try` | `#[lang("from_residual")] trait FromResidual<R>` | `std::FromResidual` | `?` residual conversion in `typechecker.rs`. |
| `FromResidualFn` | `try` | `FromResidual::from_residual` lang method | `std::FromResidual::from_residual` | `?` conversion method selection in `typechecker.rs`. |
| `ControlFlow` / `ControlFlowContinue` / `ControlFlowBreak` | `try` | `#[lang("control_flow")]` enum and `continue` / `break` variants | `std::ControlFlow`, `std::ControlFlow::Continue`, `std::ControlFlow::Break` | Required by the `Try::branch` signature and `?` result protocol. |

`TestTrait` and `TestStruct` in `LangItem` are test-only placeholders, not
stdlib contracts. `option_flatten` and `OptionExt` are library/prelude APIs,
not compiler language items; they may have a separately controlled canonical
visibility path, but are not registered in `LangItemRegistry`.

Historical milestone note (language-contract slice; superseded by the later
global audit below): the manifest uses exact logical paths (`auto_visible_paths` and
`language_items`); provider IDs only acquire provider scopes. Compiler-required
identity is registered by resolving canonical paths, and a narrow synthetic
global `std` module exposes only explicitly listed prelude paths. It does not
create root aliases or make the complete `std` namespace implicit. Generic
trait-bound resolution walks every path segment, so `T: std::Iterator<_>` does
not depend on a hidden bare `Iterator` alias. Drop recognition uses registered
language-item identity rather than root-name checks.

The contract-provider sources for Drop, Option, Iterator, IntoIterator, Copy,
and Try are nested under `std`; internal provider imports still use their
physical provider IDs. A fresh full sysroot artifact rebuild and the new Luna
contract namespace tests pass, including valid/invalid canonical-path checks
against fresh-artifact and source-only sysroots. At that historical milestone,
the ordinary-core surface had migrated in its own slice while other provider
families remained in progress. Later slices are recorded below; this milestone
text is not a current global-closure claim.

## `?` codegen blocker isolation and closure

The permanent `tests/luna/language/try_codegen_canonical_std_try.ln` fixture
uses the canonical `?` protocol and exercises both `Ok` and `Err` propagation.
Before the generic lowering repair, parsing, resolution, typechecking,
monomorphization, MVIR generation, borrow checking, and optimization passed;
backend type mapping rejected an `Extract` whose output was still
`GenericParam(T)`. After substituting the selected impl signature, the next
failure was an undefined link symbol because the selected `Try::branch` and
`FromResidual::from_residual` instances had not entered the monomorphization
worklist.

History locates the `Expr::Try` method selection/lowering before
STD-NAMESPACE-01 (the path was introduced in the September 8, 2026 language
contract implementation; the direct generic-signature use is present in the
September 23, 2026 compiler baseline). The namespace migration did not change
that lowering path. This is classified as a **pre-existing generic
instantiation defect exposed by the new end-to-end acceptance**, not a
namespace regression. No existing canonical gap entry matched this exact
defect, so no C-GAP ID was created.

Semantic tables now retain the selected `Try`/`FromResidual` impl
substitutions and instantiated `Try::branch` return type. `MonoCollector`
roots both selected impl methods; MVIR uses the instantiated signature and
emits the specialized call identities. The backend continues rejecting any
unresolved generic type. The permanent E2E test rebuilds canonical artifacts,
then compiles and executes with either a source-only sysroot or a fresh
artifact-only sysroot (no `.ln` files), asserting exit code `0` in both modes.
The root `Try` negative case now checks that `value.branch()` is unavailable
under `T: Try`; `std::Try` remains exercised by the positive `?` fixture.

This closes only the `?` full-codegen blocker. It does not complete or freeze
STD-NAMESPACE-01; ordinary public stdlib surfaces are being migrated in
separate slices.

## Initial verified slice

The Path provider now exports the nested modules `std` and `std::path`; callers
use `std::path::...` while continuing to load the physical provider with
`import <path>;`. The focused path provider source/`.llib` tests pass after
rebuilding isolated artifacts. This proves module aggregation can expose the
canonical path independently of provider identity; it does not prove the full
stdlib namespace migration.

## Ordinary core surface migration slice

The current implementation slice moves the public core/result, comparison,
hash, clone, copy, default, conversion, float, numeric-helper and panic
declarations under the logical `std` module. Physical provider IDs and source
paths remain unchanged; users still load the relevant provider with
`import <provider>;`, but the provider name does not form an API namespace.
Scalar primitive `Copy` implementations are part of the canonical `std::Copy`
protocol and let source providers such as Vec express scalar-copy behavior
through the ordinary trait.

The permanent `ordinary_core_surface_is_canonical_and_matches_fresh_artifacts`
test exercises generic Clone/Hash/Eq/Ord/Convert dispatch, Result, Default,
numeric helpers and FloatOps in a fresh source-only sysroot and a fresh
artifact-only sysroot. Removed bare and provider-derived paths are negative
cases in both modes. This paragraph records the ordinary-core milestone only;
subsequent slices below migrated the remaining listed provider families. It is
not a current inventory or closure result.

## Owned container surface migration slice

The `alloc/vec`, `alloc/string` and `alloc/box` providers retain their existing
provider identities and physical files. Their public exports now contribute to
`std` only:

| Provider | Canonical public surface | Consumers / compatibility decision |
| --- | --- | --- |
| `vec` | `std::Vec<T>`, `std::vec_new`, `std::vec_with_capacity`, `std::VecIter<T>`, `std::VecIterMut<T>`, `std::VecIntoIter<T>`, and the existing inherent/trait APIs | String, path, iterator/collection clients and Luna acceptance snippets use `std::` paths. Root `Vec`/constructor aliases are removed. |
| `string` | `std::String`, `std::string_new`, `std::string_with_capacity`, `std::string_from_str`, `std::string_from_bytes`, and existing String methods | It stores `std::Vec<u8>` and keeps its UTF-8 implementation module private; no `std::utf8` public API was added. Root aliases/wrappers are removed. |
| `box` | `std::Box<T>`, `std::box_new`, `std::box_into_inner`, and existing Box methods | Existing consumers use `std::Box`; the root type alias is removed. |

The Vec and Box declarations retain their original raw-storage anchor field
names and contracts. String still owns its Vec and preserves UTF-8 validation;
no ownership, lifetime, Drop, allocator, or runtime contract was changed.
`owned_container_surface_is_canonical_and_matches_fresh_artifacts` compiles
and executes `owned_container_surface.ln` with current freshly built
artifacts, then compares source-only and artifact-only behavior. The fixture
exercises Vec mutation/iteration/Clone/Default, String UTF-8 and byte APIs,
Box access and consuming extraction, and exactly-once destruction through
`std::Vec<std::Box<std::Box<DropProbe>>>`. Bare and provider-derived Vec,
String and Box paths are rejected in both sysroot modes.

The owned/container slice was completed before collection migration began;
STD-NAMESPACE-01 remained in progress at that milestone, and Phase 5 remained
blocked.

## Collection surface migration slice

`alloc/hashmap` and `alloc/hashset` remain physical providers loaded explicitly
with `import <hashmap>;` and `import <hashset>;`. Their public declarations are
available under `std` only. The canonical types are `std::HashMap<K, V>` and
`std::HashSet<T>`; canonical constructors are `std::hashmap_new`,
`std::hashmap_with_capacity`, `std::hashset_new`, and
`std::hashset_with_capacity`. Existing operations, trait bounds, default
semantics, borrowing, ownership, Drop, and iterator lifetime contracts are
unchanged.

The existing public companion types keep their already-declared flat `std`
paths: `std::MapIter`, `std::MapIntoIter`, `std::Keys`, `std::Values`,
`std::SetIter`, and `std::SetIntoIter`. No `std::hashmap` or `std::hashset`
submodules were created. `alloc/iter_collect` now contributes
`std::iter_collect_vec`, `std::iter_collect_hashmap`, and
`std::iter_collect_hashset`; the free HashSet owned-set helpers remain under
`std` alongside the existing methods.

The internal `__raw_table` provider remains outside public `std`: RawTable,
RawTableIter, RawTableIntoIter, ProbeResult, and raw table constructors/helpers
are not public collection APIs. Raw storage representation, field names,
`anchor(field) = self` contracts, null-sentinel behavior, and unsafe
establishment boundaries are unchanged. The current source/artifact regression
builds fresh providers, strips `.llib`/`.obj` for source-only mode and `.ln`
for artifact-only mode, then executes the same canonical collection fixture;
the removed root/provider spellings and raw-table names are rejected in both.

Historical collection-slice status: this closed the collection surface at that
milestone; the utility slice followed below. STD-NAMESPACE-01 remained in
progress and Phase 5 remained blocked.

## Utility/module surface migration slice

The final feature migration slice places utility APIs under semantic logical
modules while keeping physical providers independent:

| Physical provider(s) | Canonical public paths | Surface treatment |
| --- | --- | --- |
| `ptr` | `std::ptr::{read, write, drop_in_place, add, add_mut, offset, offset_mut, diff, copy, swap}` | Raw-pointer API remains unsafe with its existing provenance, mutability, arithmetic, and ownership contracts. |
| `mem` | `std::mem::{slice_from_raw_parts, slice_from_raw_parts_mut, copy, set, zero, size_of, align_of, swap, replace}` | Raw memory and fat-slice intrinsic boundaries retain their existing unsafe requirements. Duplicate root wrappers were removed. |
| `slice` | `std::slice::{SliceIter, SliceIterMut, slice_iter, slice_iter_mut, slice_len, slice_get, slice_get_mut, split_at, split_at_mut, slice_reverse, slice_contains_i32, slice_binary_search_i32, slice_sort_by, slice_sort_i32, slice_sort, slice_copy_from_slice, slice_fill, slice_copy_i32, slice_fill_i32}` | Existing borrow, iterator lifetime, raw provenance, and sort behavior are unchanged. |
| `iter_adapters` | `std::iter::{Map, Filter, Enumerate, Take, Skip, Zip, Chain, Peekable, FilterMap, TakeWhile, Step, Range, RangeInclusive, iter_map, iter_filter, iter_enumerate, iter_take, iter_skip, iter_zip, iter_chain, iter_peekable, iter_filter_map, iter_take_while, range, range_inclusive}` | Adapter types and functions are contributed by this provider. |
| `iter_consumers` | `std::iter::{iter_fold, iter_count, iter_for_each, iter_any, iter_all, iter_find, iter_position, iter_nth, iter_last, iter_sum_*}` | Consumer functions are contributed independently to the same logical `std::iter` module. |
| `panic` | `std::panic()` | Remains one function; no nested `std::panic` module was introduced. `__luna_panic` remains runtime/internal ABI. |
| `string` | no public UTF-8 module | The provider-private UTF-8 validation/encoding helpers remain private; String validity and ownership semantics are unchanged. |

The physical `iter_adapters` and `iter_consumers` providers aggregate into
`std::iter` without provider-derived public namespaces. The permanent
`utility_surface_and_cross_provider_iter_module_match_fresh_artifacts` test
builds fresh artifacts, removes artifacts for source-only execution and
removes all `.ln` files for artifact-only execution. A single executable uses
slice iteration, adapter functions, and terminal consumers contributed by
different providers and compares exit status plus stdout/stderr in both modes.
Legacy root/provider paths are rejected in both modes.

No provider manifest entries or dependency edges were added or removed; the
verified sysroot baseline remains 30 providers and 81 direct DAG edges. This
completes the utility/module surface slice only. The later global closure audit
and its former blocker are recorded below.

## Global closure audit — source/.llib parity and Step I complete

Audit status: **STD-NAMESPACE-01 RESOLVED & FROZEN — 2026-09-27**.
This audit introduced no public APIs, namespace rules, or language semantics.
It closed the previously observed bare `Drop`/`Copy` impl-path leak, completed
the repository-wide consumer sweep, and completed global source/.llib parity.

### Export inventory

The current source inventory counts 381 explicit `export` declarations across
the 30 providers in `libs/external/sysroot.toml`. Classification by owning
provider/scope is:

| Classification | Count | Canonical path / treatment |
| --- | ---: | --- |
| Public provider declarations | 332 | Under `std::...`; provider and source path do not form namespaces. |
| Language-contract declarations | 17 | Internal compiler/language infrastructure under canonical `std::...` paths; only the registered contract paths are auto-visible. |
| RawTable implementation exports | 25 | `__raw_table` internal provider; not public `std` API. |
| Allocator runtime ABI exports | 3 | Internal `__alloc_global` ABI. |
| Provider-private UTF-8 declarations | 3 | Nested in String's non-exported `utf8` implementation module. |
| Panic runtime ABI | 1 | `__luna_panic`; `std::panic` is the user-facing wrapper. |

The inventory sweep found no unexplained explicit public root export. Runtime
ABI, internal language-contract, `__raw_table`, and provider-private entries
are classified by role rather than by whether their existing name is
convenient. Public APIs in the provider families are enumerated in the
provider/public-surface inventory above; provider sources remain the
declaration-level source of truth.

### Fresh source/artifact closure

The permanent test
`global_std_namespace_closure_matches_fresh_source_and_artifact_only_sysroots`
copies the current canonical external tree to an isolated temporary sysroot,
then uses `SysrootBuilder::build_all(true)` to rebuild all artifacts from those
sources. Before the build, the manifest yielded 30 provider IDs. For every
manifest `path` stem, the corresponding `.ln`, `.llib`, and `.obj` were
present. The independent `test_canonical_dag_integrity` test parses imports
from the current source files, derives the graph, and passes with 30 nodes and
exactly 81 direct edges.

The physical manifest inventory (source and artifact paths are each this stem
with `.ln` and `.llib`; each also has its `.obj` sidecar) is:

| Provider ID | Path stem |
| --- | --- |
| `result` | `core/result` |
| `ptr` | `core/ptr` |
| `slice` | `core/slice` |
| `mem` | `core/mem` |
| `cmp` | `core/cmp` |
| `hash` | `core/hash` |
| `clone` | `core/clone` |
| `copy` | `core/copy` |
| `iter_adapters` | `core/iter_adapters` |
| `iter_consumers` | `core/iter_consumers` |
| `try` | `core/try` |
| `io` | `io/io` |
| `path` | `path/path` |
| `num` | `core/num` |
| `default` | `core/default` |
| `convert` | `core/convert` |
| `float` | `core/float` |
| `box` | `alloc/box` |
| `vec` | `alloc/vec` |
| `string` | `alloc/string` |
| `hashmap` | `alloc/hashmap` |
| `hashset` | `alloc/hashset` |
| `iter_collect` | `alloc/iter_collect` |
| `panic` | `core/panic` |
| `__alloc_global` | `alloc/global` |
| `__raw_table` | `alloc/raw_table` |
| `__lang_drop` | `lang/drop` |
| `__lang_option` | `lang/option` |
| `__lang_iterator` | `lang/iterator` |
| `__lang_into_iterator` | `lang/into_iterator` |

For source-only mode, the test copies the fresh-built tree and recursively
removes every `.llib` and `.obj`; the compiler search path is only that
isolated root. For artifact-only mode it copies the same fresh-built tree and
recursively removes every provider `.ln`, then asserts the `.ln` count is
exactly zero before compiling. No repository provider source path is added as
a consumer search root, so source fallback cannot mask artifact loading.

Ten canonical Luna consumers collectively exercise `std::Option`,
`std::Result`, `std::Try`/`?`, `std::Iterator`, `std::IntoIterator`,
`std::Drop`, `std::Copy`, `std::Clone`, `std::Hash`, `std::Default`,
`std::Convert`, `std::TryConvert`, `std::FloatOps`, `std::Vec`,
`std::String`, `std::Box`, `std::HashMap`, `std::HashSet`, iterator
collection, `std::path`, `std::io`, `std::ptr`, `std::mem`, `std::slice`,
`std::iter`, and `std::panic`. Each compiles, links, and executes in both
modes: 20 paired executions. The harness compares compile result, process
exit code, stdout, and stderr exactly; all match and exit 0.

The new `global_provider_load.ln` is one consumer that imports and uses 14
physical providers together: `result`, `ptr`, `slice`, `mem`,
`iter_adapters`, `iter_consumers`, `io`, `default`, `box`, `vec`, `string`,
`hashmap`, `hashset`, and `iter_collect`. It exercises the combined logical
`std::...` surface in one semantic/codegen session. It passes source-only and
artifact-only with identical observable results.

The same test reverses the two physical providers contributing to `std::iter`;
both source-only and artifact-only runs still compile and execute. This checks
logical namespace aggregation independent of provider load order. The manifest
and sysroot invariant test measure 30 providers and exactly 81 direct DAG
edges; `test_sysroot_build_invariants` passes 5/5.

### Artifact metadata audit

The serialized raw-pointer effect channel is parameter-relative and stable:
return origins use parameter indexes, owner anchors use a parameter index and
canonical field name, and exported declaration identities use stable provider
name plus symbol path. The metadata contains no raw `DeclId`, `SymbolId`,
`ValueId`, `PlaceId`, or `RegionId`; the decoder allocates/reconstructs
session-local symbols after loading. `SEMANTIC_METADATA_VERSION` is 3, and
metadata parity tests pass 9/9, including generic identity, lifetime, impl,
and mixed source/artifact graph cases.

### Negative namespace and language-contract evidence

The global suite exercises 22 permanent negative fixtures in both fresh modes:
44/44 reject old root/provider-derived or private spellings. The five language
contract implementation-path probes additionally check bare `Option`,
`Iterator`, `IntoIterator`, `Try`, and an unknown trait in both modes (10/10);
all fail with a trait-path resolution diagnostic. In particular, bare
`impl Drop for Probe` and `impl Copy for Probe` now fail specifically with
`cannot resolve trait path`, while `impl std::Drop` and `impl std::Copy`
compile and execute through the canonical positive fixture. Compiler language
item registration and lookup remain path-based; no ordinary stdlib API was
added to compiler knowledge.

The active Luna consumers that had unqualified standard `Iterator` bounds or
impls were migrated to `std::Iterator`/`std::IntoIterator`, including direct
user iterator examples and Vec ownership/drop examples. The remaining matches
from broad lexical searches are classified as: intentional negative fixtures;
provider implementation references resolved inside lexical `module std`;
local synthetic `Option`/`Result`/`Vec`/`Box` types or traits in isolated UI
compiler tests; or prose/comments. Those local test types do not name the
stdlib surface. Historical reports remain historical and have not been
rewritten as though their original examples used canonical paths.

### Regression evidence and remaining gates

`lang_namespace_contract_tests` passes 12/12 after adding the generic
cross-module `?` full-codegen regression. It includes the 22×2 legacy/private negative matrix, canonical
language-contract checks, `?` full-codegen source/artifact parity, provider
aggregation, and all global consumers. After the global fresh parity gate,
23 focused parity-sensitive target invocations also completed with exit code
0, covering generic trait dispatch, C-GAP-12, RAW-STORAGE-ANCHOR-v1,
Box/Vec/String/HashMap/HashSet/iter_collect, ptr/mem, slice borrow/index/
iterator, iterator adapters/consumers, Path, IO, Phase 4B/4C parity, artifact
metadata, and sysroot invariants. The individual invariant suites passed 5/5
(sysroot) and 9/9 (artifact metadata); C-GAP-12 passed 4/4 and the anchor
contract suite passed 24/24.

### Step I — full workspace regression

The first exact workspace run for Step I started at `2026-09-27T13:58:14Z`
and stopped in the UI harness at `try_cross_module_generic.ln` with process
exit `-1073741571` (`STATUS_STACK_OVERFLOW`, `0xc00000fd`). The failure was
reproduced in the focused UI target, so it was isolated rather than dismissed
as flaky. Typechecking and monomorphization completed; recursion occurred
while MVIR built the specialized `Try::branch` call identity. The generic
collector enqueued the substituted trait call but did not retain that
per-enclosing-instance substitution for MVIR, which then read the original
generic `Try` call metadata. The unresolved `GenericParam` reached symbol
mangling and recursively substituted itself.

The generic fix records each instantiated `?` branch/`FromResidual` call and
its specialized branch return type on the enclosing mono instance; MVIR now
uses those per-instance records. No `std::Try` or `?` special case was added.
A permanent executable fixture,
`tests/luna/language/try_codegen_generic_cross_module_std_try.ln`, covers
generic `?` codegen and both `Ok`/`Err` behavior. Its source-only and freshly
built artifact-only sysroot executions agree and exit 0. The existing UI
harness (including `try_cross_module_generic.ln`) and the full namespace
contract suite also pass after the fix.

The final exact workspace command,
`cargo test --workspace -- --test-threads=1`, started at
`2026-09-27T13:58:14.992Z`, completed at `2026-09-27T15:40:52.008Z`, and
returned process exit code **0**. It completed all workspace unit,
integration, UI, and doc-test binaries; `STATUS_STACK_OVERFLOW` did not
recur. No runtime code was changed. The post-workspace namespace closure
passed `lang_namespace_contract_tests` **12/12**, including **44/44**
legacy/private-path rejections, and `test_sysroot_build_invariants` **5/5**,
confirming **30 providers / 81 direct DAG edges**. The final
`git diff --check` returned **0**; Git emitted only LF/CRLF working-copy
conversion warnings and no whitespace errors.

**STEP I — FULL WORKSPACE REGRESSION COMPLETE.** The design authority approved
the freeze on 2026-09-27. **STD-NAMESPACE-01 — Canonical Standard Library
Namespace — RESOLVED & FROZEN.** This is now a frozen regression dependency
for subsequent stdlib phases. Phase 5 Whole-File I/O v1 is unblocked; its
public surface is `std::read_file`, `std::write_file`, `std::copy_file`, and
`std::FileError` only, with no `std::fs` module or root aliases.
