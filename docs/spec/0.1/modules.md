# Luna 0.1 — modules, bootstrap and artifacts

## Providers and namespaces

`import` acquires a provider. `module` creates an inline namespace. `::` resolves
a namespace path. `using path as alias;` creates a local namespace alias.
`using path;` opens accessible direct namespace members for unqualified lookup,
under [NAMESPACE-USING-v1](namespace-using-v1.md).
None of these operations implies another: a filename does not create a module,
an import does not introduce arbitrary unqualified bindings, and an alias does
not load a dependency. Multiple providers may contribute exported declarations
to the same explicit namespace, subject to duplicate/coherence rules.

Canonical stdlib APIs live under `std::`, including the explicitly defined
`std::fmt` and `std::io` subnamespaces. A physical `alloc/` or `core/` directory
does not imply `std::alloc` or `std::core`. Container namespaces such as
`std::collections::Vec` must not be inferred from another language's conventions.

Private declarations follow module ancestry rules. Field visibility is
independent of containing type visibility under Visibility-02. Exported
declarations and all generic/lifetime/ownership contracts must cross provider
boundaries without bypassing privacy or coherence.

## Controlled language bootstrap

The baseline has six compiler-required contract families and one controlled
standard-prelude contribution. The manifest is explicit; no arbitrary directory
scan may make a new file automatically visible.

| Family | Provider | Selected logical surface |
|---|---|---|
| Drop | `__lang_drop` | `std::Drop` and its language hook identity |
| Option | `__lang_option` | `std::Option`, Some/None identities, `std::option_flatten` |
| Iterator | `__lang_iterator` | `std::Iterator` |
| IntoIterator | `__lang_into_iterator` | `std::IntoIterator` |
| Copy | `copy` | `std::Copy` |
| Try / FromResidual / ControlFlow | `try` | Selected `std::` protocol declarations and hooks |
| Controlled standard prelude | `result` | `std::OptionExt`, not general Result auto-visibility |

Auto-visible paths remain qualified under `std`; they do not create unqualified
root Option, Iterator or Drop bindings. Other ordinary APIs require explicit
provider imports. `Result` remains an ordinary library type implementing generic
Try contracts; it is not a compiler-native container.

Exact bootstrap language hook mappings are documented in
[core-language-contract.md](../../core-language-contract.md) and represented by
`luna-driver/src/lang_contracts.rs`. Provider names choose acquisition, while
canonical declaration identities identify semantics.

Explicitly imported canonical memory providers identify their existing unsafe
drop/raw-slice primitives with checked `#[lang]` declarations. These three hook
identities do not add auto-loaded families or namespace visibility. Ordinary
functions with matching API names retain ordinary behavior. Implementation and
remaining raw-slice loan gaps are recorded in the
[memory audit](../../audits/0.1-alpha-completion-2026-10-04/MEMORY-CALLABLES.md).

## Artifact discovery, validity and parity

Discovery finds candidates; validation checks identity. Canonical `.llib` takes
precedence over `.ln`; canonical candidates take precedence over legacy reads.
Directory-package discovery has its specified order in the compatibility
contract; local relative imports have their own supported lookup scope.
An invalid selected artifact must be rejected, not silently replaced by source.

Identity includes source, canonical public interface, dependency interfaces,
compiler, target, feature/configuration and artifact-kind information as defined
by the artifact contract. Mere file existence is not validity. Private changes
and public interface changes must not be confused. The writer emits `LLIB`;
legacy `MLIB` reads do not authorize legacy artifact emission.

An ordinary native call depends on the callee's canonical interface. When a
dependency body or value is materialized into the consumer (generic instances,
comptime results, embedded constants or bundled source definitions), its
execution identity is also required. Changing only an ordinary native callee's
body permits relinking; changing a materialized dependency requires rejection
of a stale dependent artifact. Provider identity comes from validated discovery,
including the canonical sysroot manifest, rather than an incidental file stem.

The current execution identity protocol hashes provider source bytes with a
versioned domain tag. It conservatively includes private bodies and formatting,
but excludes session arena offsets and dependency load order. Compiler header
protocol version 13 rejects compiler versions 1–12. This revision requires
checked memory-hook identities and callable safety in addition to
native integrity envelopes and portable payload checksums, and retains
ordered nominal representations, validated target contracts, public generic
constraints and individual impl contracts, alongside the earlier
method-identity, ownership-cleanup and module-constant-storage repairs. MVIR
version 4 carries portable immutable static data whose initializer and type
shape contain no semantic-session IDs or VM addresses. Rebuild incompatible
artifacts through build tooling; import never rebuilds or falls back from a
selected invalid artifact. File format version remains 2; semantic metadata
version is 6. These are internal compatibility revisions, not a declaration of
language release readiness.

Target identity binds the selected triple, CPU/features, emitted object format,
LLVM default ABI/data-layout identity, pointer width and endianness. All fields
must match the consuming target configuration; this implementation uses strict
CPU/features equality rather than assuming cross-machine compatibility. The
artifact header and manifest must agree on the triple. Embedded objects must
be valid relocatable objects with the expected format, architecture, width and
endianness where represented by the object container. An existing object
sidecar must match the embedded object bytes and target identity; its mere
existence cannot authorize a different implementation. Invalid selected
artifacts/sidecars reject without falling back or silently replacing them.
Every emitted native payload has writer-owned object metadata containing its
format, byte length and SHA-256 digest. Missing/orphaned metadata, empty native
payloads and inconsistent size/hash/format reject before use. Each section-table
checksum is the first eight SHA-256 digest bytes interpreted as a little-endian
`u64`. Readers validate section ranges and checksums before decoding manifest,
AST, semantic metadata or MVIR. These checks establish payload consistency;
they do not authenticate a publisher or prove native/source semantic equivalence.
Build output failures propagate diagnostics and failure status independently of
CLI verbosity. Failed `.llib` serialization/publication must not report success;
publication keeps incomplete bytes in a sibling temporary file and cleans that
temporary on failure.
Descriptor probes for another target do not certify Luna's lowering, runtime
or native-language conformance on that target. Current native lowering rejects
non-64-bit pointers until its remaining word-size assumptions are repaired;
this is an implementation gap, not an adoption of a narrower language scope.

Canonical generic contracts retain declaration-owned binders, trait arguments,
trait bounds and associated-type equalities. Impl contracts retain individual
self patterns, method signatures/constraints and associated-type definitions;
a nominal-head grouping cannot replace an individual checked header. Stable
binder identities use their owner and parameter position, not a source name or
session ID. Changing a public constraint must change interface identity;
renaming generic binders, reordering independent declarations or changing a
body without changing its public effects must preserve that identity. Function
body locals do not belong to its exported symbol children.

Canonical interface identity includes the ordered fields of a struct and the
ordered variants/payloads of an enum. Variant position determines its
discriminant. This representation contract covers owned nominal types reachable
from public signatures and impl contracts, including nested or private types;
it does not make private declarations accessible. Foreign representations are
validated through their owning dependency's interface identity. Recursive
pointer/reference graphs retain nominal identity and a separate representation
table. Reordering fields/variants or changing a reachable field/payload type
must invalidate stale dependent artifacts. Changes to unreachable private types
must not change public interface identity.

Comptime dependency collection remains conservative: a compilation that
evaluates comptime retains execution identities for its loaded dependencies.
More precise dependency selection remains an optimization requirement.

Portable metadata must not serialize session-local IDs as stable semantic
identity. It must preserve generic bodies, canonical instances, trait impls,
lifetimes, anchors, raw-pointer effects and other required public contracts.
Importing an interface must not multiply an identical bound for each dependency
path. Source-only and freshly rebuilt artifact-only consumers must agree on
acceptance, rejection, behavior and relevant diagnostics.

The compiler resolves and validates providers. The package/build tool orchestrates
rebuild, caching, acquisition and invalidation policy. The compiler must not
silently adopt package-manager policy or rebuild a stale library on import.

## Optional project provider discovery

[PROVIDER-CONFIG-v1](provider-config-v1.md) adds optional `luna.toml` file-stem
bindings for `import <name>`. The CLI selects the nearest configuration from the
entry file, or uses `--config FILE` / `--no-config`. A single selected table
applies to the entire invocation, including transitive imports. Relative paths
must be relative file values based on the configuration directory. Absolute,
drive-qualified/rooted values reject with E6008; internal absolute driver inputs
and CLI search paths remain supported. Existing `import "path"` remains
relative to its importing file. Configuration keys select providers, not
namespace names, and cannot override sysroot names or aliases.
