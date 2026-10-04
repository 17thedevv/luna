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

Exact language hook mappings are documented in
[core-language-contract.md](../../core-language-contract.md) and represented by
`luna-driver/src/lang_contracts.rs`. Provider names choose acquisition, while
canonical declaration identities identify semantics.

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
protocol version 3 rejects compiler versions 1 and 2, including previously
compiled native/portable bodies with incorrect ownership cleanup. Rebuild them
through the build tooling; import never rebuilds or falls back from a selected
invalid artifact. This is an internal compiler compatibility revision, not a
declaration of language release readiness. Format and MVIR versions are unchanged.
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
