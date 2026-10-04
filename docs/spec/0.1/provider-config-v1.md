# Luna 0.1-alpha.1 — optional provider configuration

Contract ID: **PROVIDER-CONFIG-v1**. Maintainer-requested alpha scope, 2026-10-03.
Status: **IMPLEMENTED / CLI SOURCE-ARTIFACT MATRIX PASS** on Windows x86_64 GNU.
Workspace snapshots and final targeted regressions are tracked in the [verification record](../../audits/alpha-modules-2026-10-03/README.md).
This status does not certify other targets or the overall alpha release.
Maintainer amendment, **2026-10-04**: file values are relative-only. This
supersedes the absolute-path permission in the initial `4a5adf4` implementation;
see the [relative-only verification record](../../audits/provider-config-relative-2026-10-04/README.md).
`luna.toml` supplies optional provider discovery bindings. Existing local
imports, external provider lookup, source/artifact loading and namespace
semantics remain available. This is not a package version/dependency solver.

## File format and example

```toml
schema = 1

[providers]
geometry = "../shared/geometry"
math = "../../libraries/numeric/math"
```

Each key is a single Luna provider-name identifier; each value is a nonempty
path string to a provider **stem**, omitting `.ln`/`.llib` as existing imports do.
The first mapping locates `../shared/geometry.ln` or
`../shared/geometry.llib`.

**All provider paths in `luna.toml` MUST be relative paths resolved against the
directory containing the selected `luna.toml`. Absolute paths, drive-qualified
paths and filesystem-root paths MUST be rejected with E6008.** This applies to
unused entries too, and foreign path syntax must reject independently of the
host OS. `..` is allowed: `libs/math`, `../shared/geometry` and
`../../common/foo` are valid stems. Use `/` as the canonical separator, including
on Windows, to avoid TOML backslash escaping and host-dependent interpretation.

Reject POSIX roots (`/home/user/foo`), Windows roots (`\foo`), drive-qualified
forms (`C:/dev/foo`, `C:foo`, `C:`), UNC roots and device/extended prefixes.
Home prefixes (`~`, `~user`), environment marker characters (`$`, `%`) and
control characters are configuration errors; no expansion is performed.
Globbing and implicit directory scanning are not performed.
Alpha introduces file-provider mappings only; it does not add a new directory
package format. Omitted `schema` means 1. Unknown schema, unknown fields,
duplicate keys, invalid names, empty paths and paths with provider extensions
are configuration errors with file/key locations.

For a provider that declares `export module geometry { ... }`:

```luna
import <geometry>;
using geometry;

fn main() -> i32 {
    dec p = Point { x: 10, y: 32 };
    return p.x + p.y - 42;
}
```

The original `import "../shared/geometry";` remains valid. Configuration does
not require changing existing files. Merely listing a provider does not import
it, execute it or expose any of its declarations.

## Selecting a configuration

1. For `luna check`, `build` and `run`, start from the input source file's
   absolute parent directory and search ancestors for the nearest `luna.toml`.
   Stop at the first match or filesystem root. Do not base discovery on cwd.
2. There is **one active configuration per compilation invocation**, including
   its transitive dependency resolution. Do not merge ancestor configurations
   or switch automatically to configs beside imported providers. Every directory
   may have its own file; compiling an entry file there selects the corresponding
   nearest configuration. This bound prevents source and artifact imports from
   acquiring different hidden configuration contexts.
3. Support explicit CLI `--config <file>` and `--no-config` for these three
   commands. They are mutually exclusive; an explicit missing/invalid file is
   an error. `--no-config` uses existing discovery only.
4. Validate each mapping as a relative file value, then resolve it against the
   selected TOML file's absolute parent, regardless of the caller's cwd,
   entry-file location or transitive importer. The resulting internal `PathBuf`
   may be absolute. This restriction applies to values inside TOML, not the
   `--config FILE` argument, `-I`/`--search-path`, or typed driver inputs.
   Those may still contain absolute machine-local paths. No `luna.local.toml`
   override format is introduced in 0.1.
5. No config is required. With none, behavior is the existing import/search-path
   behavior. Official `build-sysroot` continues to use `sysroot.toml` and does not
   discover project `luna.toml` automatically.

## Discovery and validity

- `import "path";` retains its existing importing-file-relative resolution;
  the provider table does not rewrite it.
- `import <name>;` first uses an explicit configured binding when present.
  Otherwise it uses the existing external-provider/search-path mechanism.
  Configured names must not collide with canonical sysroot provider names or
  aliases, including bootstrap contracts. Check this against the manifest and
  existing generic resolution inputs, never a hardcoded list of stdlib names.
- A configured binding is authoritative for that name. A missing or invalid
  mapped provider rejects; do not silently select an unrelated fallback provider.
  An unused mapping need not exist on disk, but its syntax must still be valid.
- Apply existing candidate order to the mapped stem: canonical `.llib`,
  canonical `.ln`, then supported legacy reads. Invalid selected `.llib`
  rejects instead of falling back to source. All compiler/target/features,
  interface, execution and dependency validation obligations still apply.
- A binding is a discovery alias, not an artifact/provider identity rewrite.
  The filename and key may differ. Namespace paths still come exclusively from
  declarations. Manifest identity and dependency references must remain genuine;
  do not patch fingerprints to make a mapped artifact load.
- Multiple discovery bindings selecting the same canonical provider must not
  instantiate it twice. Conversely, equal display names cannot merge distinct
  providers. Existing import-cycle, duplicate/coherence and privacy checks apply.
- Capture the selected configuration and effective bindings as compilation
  inputs. Diagnostics identify the import, configured key and resolved candidate.
  Portable artifacts must use the existing stable identity/dependency contracts,
  not absolute configuration paths or session-local IDs as semantic identity.

## Architecture and compatibility

The CLI/application discovery layer owns reading TOML and normalizing paths.
It passes typed provider bindings/resolution inputs to the driver. Parser,
semantic analysis, borrow checking and backend do not read TOML or infer module
names from its keys. Driver APIs retain operation without a project file and
may receive explicit equivalent bindings from other callers. No auto-build,
artifact refresh, network fetch, version solving or caching policy is added.

Both additions preserve [provider versus namespace separation](modules.md).
`using` affects lexical lookup; configuration affects provider discovery;
`import` is still required to acquire a provider.

## Required acceptance

| Case | Required outcome |
|---|---|
| No config; old relative imports and external imports | Existing behavior preserved |
| Mapped file outside the entry directory; key differs from filename | Correct provider loaded, declared namespace preserved |
| Change cwd; entry file several directories below config | Same config, path interpretation and result |
| Nested config, explicit config, disabled config | Nearest-only/explicit/disabled policy; no implicit merge |
| Config beside a transitive provider | Does not replace active invocation configuration |
| Malformed TOML/schema/name/path; duplicate key; sysroot collision | Typed configuration error with key/file location |
| Absolute/rooted/drive-relative/UNC/device value, even unused | E6008 before provider discovery, on every host |
| Home/environment marker or control character in path | E6008; never expand |
| `../` and `../../` values with an absolute `--config FILE`; different cwd | Resolve from config directory; check/build/run preserved |
| Unused missing mapping; imported missing mapping | First does not load; second rejects without fallback |
| Artifact and source both exist; artifact is stale or malformed | Existing artifact precedence; invalid selection rejects |
| Two names select the same provider; dependency cycle | Stable single provider identity; cycles reject |
| Source-only versus freshly built artifact-only graph | Same behavior/diagnostics; provider sources physically absent |
| Move graph to a different directory; portable artifact dependencies | Same declarations and valid logical identity; no host-path leakage |
| `check`/`build`/`run`; explicit driver inputs | Same discovery decisions and common-stage failures |

Use [the implementation plan](alpha-module-plan.md). Existing alpha correctness
and release gates remain required; the linked verification record supplies
evidence for the tested implementation scope.
