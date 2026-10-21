<!-- luna-doc-role: evidence -->

# Stdlib redesign — current source inventory

Acquired 2026-10-21 (Asia/Bangkok), before designing the migration. Source pin:
`264390dacfd8f35ea53d4e5639ca4018cfc64f4d`.

This is a **pre-expansion source map**, not resolved public API, artifact
validation, correctness acceptance or release qualification. No stdlib source,
compiler semantics or runtime ABI was changed by this audit.

The [full inventory](source-inventory.json) maps each library declaration to
its source provider/component, explicit namespace, impl/trait owner and expected
artifact paths through its provider. Parameters/fields/contracts remain nested
metadata; local variables are not migration units. Raw source SHA-256 records
cover acquired checkout bytes, including its line endings. Read
[verification](verification.json) for limits and acquisition checks.

## Method and boundary

All manifest source files are parsed with Luna's own lexer/parser. Parser
diagnostics fail the acquisition instead of silently skipping a file. Logical
imports are matched through current manifest identities/aliases; the explicit
graph is checked for cycles and topological consistency. Implicit bootstrap
edges are separate: consult the current
[contract registry](../../../luna-rs/crates/luna-driver/src/lang_contracts.rs).
Its wrapping-arithmetic entry is included when reviewing the bootstrap surface;
an older six-family summary alone is insufficient.

Declared visibility is not effective export visibility. Derived/generated items,
reexports and effective public reachability need a semantic/export snapshot
before migration. Expected `.llib`/`.obj` paths are not claims that those bundles
exist or are valid. Fixture references below are textual import matches across
both test roots (including comments/negative fixtures), not coverage or executions.

## Snapshot

- Providers: **49**; declaration/impl/module records: **2,358**.
- Explicit import declarations: **213**; fixture files indexed: **957**.
- Extern declaration occurrences: **137**; spelled public: **55**.
  These counts do not establish effective exports or a visibility defect.
- Two independent inventory invocations produce byte-identical output. All
  49 source digests are rechecked; malformed input fails without partial JSON.
- Existing parser build warnings remain; no warning-free compiler claim is made.

## Provider → component → dependency → artifact index

The provider table joins to `declarations` by provider ownership in the JSON;
each record has a unique provider-local AST key. Source links point to the
acquired implementation, and artifact stems are relative to its sysroot root.

| Provider | Component source | Access | Lines | Records | Direct imports | Fixture references | Artifact stem |
| --- | --- | --- | ---: | ---: | ---: | ---: | --- |
| `result` | [core/result.ln](../../../luna-rs/libs/external/core/result.ln) | public | 154 | 24 | 2 | 43 | `core/result.llib/.obj` |
| `ptr` | [core/ptr.ln](../../../luna-rs/libs/external/core/ptr.ln) | public | 102 | 19 | 0 | 18 | `core/ptr.llib/.obj` |
| `slice` | [core/slice.ln](../../../luna-rs/libs/external/core/slice.ln) | public | 775 | 52 | 7 | 17 | `core/slice.llib/.obj` |
| `mem` | [core/mem.ln](../../../luna-rs/libs/external/core/mem.ln) | public | 106 | 19 | 1 | 28 | `core/mem.llib/.obj` |
| `cell` | [core/cell.ln](../../../luna-rs/libs/external/core/cell.ln) | public | 144 | 23 | 4 | 1 | `core/cell.llib/.obj` |
| `atomic` | [core/atomic.ln](../../../luna-rs/libs/external/core/atomic.ln) | public | 353 | 62 | 2 | 2 | `core/atomic.llib/.obj` |
| `sync` | [core/sync.ln](../../../luna-rs/libs/external/core/sync.ln) | public | 890 | 110 | 5 | 4 | `core/sync.llib/.obj` |
| `cmp` | [core/cmp.ln](../../../luna-rs/libs/external/core/cmp.ln) | public | 201 | 69 | 0 | 9 | `core/cmp.llib/.obj` |
| `hash` | [core/hash.ln](../../../luna-rs/libs/external/core/hash.ln) | public | 48 | 32 | 1 | 4 | `core/hash.llib/.obj` |
| `clone` | [core/clone.ln](../../../luna-rs/libs/external/core/clone.ln) | public | 20 | 31 | 0 | 6 | `core/clone.llib/.obj` |
| `copy` | [core/copy.ln](../../../luna-rs/libs/external/core/copy.ln) | public | 22 | 18 | 0 | 3 | `core/copy.llib/.obj` |
| `iter_adapters` | [core/iter_adapters.ln](../../../luna-rs/libs/external/core/iter_adapters.ln) | public | 807 | 139 | 5 | 25 | `core/iter_adapters.llib/.obj` |
| `iter_consumers` | [core/iter_consumers.ln](../../../luna-rs/libs/external/core/iter_consumers.ln) | public | 502 | 29 | 3 | 10 | `core/iter_consumers.llib/.obj` |
| `algo` | [core/algo.ln](../../../luna-rs/libs/external/core/algo.ln) | public | 255 | 21 | 6 | 1 | `core/algo.llib/.obj` |
| `time` | [core/time.ln](../../../luna-rs/libs/external/core/time.ln) | public | 551 | 87 | 6 | 5 | `core/time.llib/.obj` |
| `try` | [core/try.ln](../../../luna-rs/libs/external/core/try.ln) | public | 136 | 36 | 3 | 6 | `core/try.llib/.obj` |
| `io` | [io/io.ln](../../../luna-rs/libs/external/io/io.ln) | public | 1203 | 118 | 5 | 24 | `io/io.llib/.obj` |
| `file` | [file/file.ln](../../../luna-rs/libs/external/file/file.ln) | public | 1025 | 113 | 7 | 9 | `file/file.llib/.obj` |
| `path` | [path/path.ln](../../../luna-rs/libs/external/path/path.ln) | public | 347 | 29 | 3 | 5 | `path/path.llib/.obj` |
| `net` | [net/net.ln](../../../luna-rs/libs/external/net/net.ln) | public | 748 | 101 | 7 | 4 | `net/net.llib/.obj` |
| `random` | [core/random.ln](../../../luna-rs/libs/external/core/random.ln) | public | 193 | 26 | 3 | 1 | `core/random.llib/.obj` |
| `encoding` | [encoding/encoding.ln](../../../luna-rs/libs/external/encoding/encoding.ln) | public | 305 | 23 | 4 | 3 | `encoding/encoding.llib/.obj` |
| `crypto` | [crypto/crypto.ln](../../../luna-rs/libs/external/crypto/crypto.ln) | public | 576 | 37 | 5 | 1 | `crypto/crypto.llib/.obj` |
| `json` | [json/json.ln](../../../luna-rs/libs/external/json/json.ln) | public | 827 | 46 | 4 | 2 | `json/json.llib/.obj` |
| `num` | [core/num.ln](../../../luna-rs/libs/external/core/num.ln) | public | 842 | 100 | 1 | 4 | `core/num.llib/.obj` |
| `default` | [core/default.ln](../../../luna-rs/libs/external/core/default.ln) | public | 27 | 35 | 1 | 5 | `core/default.llib/.obj` |
| `convert` | [core/convert.ln](../../../luna-rs/libs/external/core/convert.ln) | public | 273 | 102 | 1 | 4 | `core/convert.llib/.obj` |
| `float` | [core/float.ln](../../../luna-rs/libs/external/core/float.ln) | public | 308 | 174 | 2 | 3 | `core/float.llib/.obj` |
| `fmt` | [core/fmt.ln](../../../luna-rs/libs/external/core/fmt.ln) | public | 250 | 73 | 1 | 19 | `core/fmt.llib/.obj` |
| `box` | [alloc/box.ln](../../../luna-rs/libs/external/alloc/box.ln) | public | 62 | 9 | 4 | 9 | `alloc/box.llib/.obj` |
| `vec` | [alloc/vec.ln](../../../luna-rs/libs/external/alloc/vec.ln) | public | 765 | 68 | 11 | 60 | `alloc/vec.llib/.obj` |
| `string` | [alloc/string.ln](../../../luna-rs/libs/external/alloc/string.ln) | public | 1210 | 100 | 15 | 57 | `alloc/string.llib/.obj` |
| `hashmap` | [alloc/hashmap.ln](../../../luna-rs/libs/external/alloc/hashmap.ln) | public | 225 | 49 | 9 | 25 | `alloc/hashmap.llib/.obj` |
| `hashset` | [alloc/hashset.ln](../../../luna-rs/libs/external/alloc/hashset.ln) | public | 273 | 43 | 9 | 12 | `alloc/hashset.llib/.obj` |
| `iter_collect` | [alloc/iter_collect.ln](../../../luna-rs/libs/external/alloc/iter_collect.ln) | public | 65 | 4 | 7 | 6 | `alloc/iter_collect.llib/.obj` |
| `rc` | [alloc/rc.ln](../../../luna-rs/libs/external/alloc/rc.ln) | public | 206 | 21 | 7 | 2 | `alloc/rc.llib/.obj` |
| `arc` | [alloc/arc.ln](../../../luna-rs/libs/external/alloc/arc.ln) | public | 202 | 21 | 7 | 2 | `alloc/arc.llib/.obj` |
| `vecdeque` | [alloc/vecdeque.ln](../../../luna-rs/libs/external/alloc/vecdeque.ln) | public | 448 | 47 | 10 | 2 | `alloc/vecdeque.llib/.obj` |
| `binaryheap` | [alloc/binaryheap.ln](../../../luna-rs/libs/external/alloc/binaryheap.ln) | public | 221 | 33 | 9 | 2 | `alloc/binaryheap.llib/.obj` |
| `btreemap` | [alloc/btreemap.ln](../../../luna-rs/libs/external/alloc/btreemap.ln) | public | 719 | 60 | 11 | 2 | `alloc/btreemap.llib/.obj` |
| `btreeset` | [alloc/btreeset.ln](../../../luna-rs/libs/external/alloc/btreeset.ln) | public | 131 | 31 | 9 | 2 | `alloc/btreeset.llib/.obj` |
| `thread` | [alloc/thread.ln](../../../luna-rs/libs/external/alloc/thread.ln) | public | 226 | 32 | 5 | 2 | `alloc/thread.llib/.obj` |
| `panic` | [core/panic.ln](../../../luna-rs/libs/external/core/panic.ln) | public | 213 | 30 | 0 | 7 | `core/panic.llib/.obj` |
| `__alloc_global` | [alloc/global.ln](../../../luna-rs/libs/external/alloc/global.ln) | internal | 7 | 3 | 0 | 1 | `alloc/global.llib/.obj` |
| `__raw_table` | [alloc/raw_table.ln](../../../luna-rs/libs/external/alloc/raw_table.ln) | internal | 703 | 34 | 9 | 1 | `alloc/raw_table.llib/.obj` |
| `__lang_drop` | [lang/drop.ln](../../../luna-rs/libs/external/lang/drop.ln) | internal | 7 | 3 | 0 | 0 | `lang/drop.llib/.obj` |
| `__lang_option` | [lang/option.ln](../../../luna-rs/libs/external/lang/option.ln) | internal | 115 | 16 | 1 | 0 | `lang/option.llib/.obj` |
| `__lang_iterator` | [lang/iterator.ln](../../../luna-rs/libs/external/lang/iterator.ln) | internal | 8 | 3 | 1 | 0 | `lang/iterator.llib/.obj` |
| `__lang_into_iterator` | [lang/into_iterator.ln](../../../luna-rs/libs/external/lang/into_iterator.ln) | internal | 6 | 3 | 0 | 0 | `lang/into_iterator.llib/.obj` |

## Replay

Requires the development Rust toolchain; this is not an end-user dependency.
From the repository root, build the independent audit helper:

```powershell
cargo build --offline --locked --manifest-path docs/tools/stdlib-surface/Cargo.toml
python docs/tools/inventory_stdlib.py --parser docs/tools/stdlib-surface/target/debug/luna-stdlib-surface.exe --out NEW-INVENTORY.json
```

Use the configured target directory when CARGO_TARGET_DIR is set, and omit
`.exe` on POSIX. The script refuses to overwrite an acquired snapshot and
requires tracked stdlib/parser sources to match HEAD. For this acquisition the
helper was built in D:/luna-stable-build/cargo. Its binary hash and the parser
source tree are recorded in the inventory.

The [redesign RFC](../../proposals/stdlib-redesign-0.1.md) contains the human
decisions and proposed layout/API rules. This index does not adopt those rules
or assign old declarations to future owners automatically.

SKILL IMPACT: none at this design checkpoint. Proposed layout/API rules remain
in the RFC until their domain contracts are adopted; existing language/compiler
boundary guidance still applies.
