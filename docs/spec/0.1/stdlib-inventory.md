# Luna 0.1 provider inventory

Generated from [sysroot.toml](../../../luna-rs/libs/external/sysroot.toml). Provider identity is not a namespace or a completeness claim.

Current manifest: **49 providers**. The [original audit manifest](evidence/sysroot-audit-baseline.toml) has 32; later additions are implementation inventory, not automatic spec adoption or conformance. Public/internal visibility controls import access, separately from exported declarations.

| Provider | Source component | Import visibility | Contract family | Evidence boundary |
|---|---|---|---|---|
| `result` | [core/result.ln](../../../luna-rs/libs/external/core/result.ln) | public | ordinary provider | original 32-provider audit |
| `ptr` | [core/ptr.ln](../../../luna-rs/libs/external/core/ptr.ln) | public | ordinary provider | original 32-provider audit |
| `slice` | [core/slice.ln](../../../luna-rs/libs/external/core/slice.ln) | public | ordinary provider | original 32-provider audit |
| `mem` | [core/mem.ln](../../../luna-rs/libs/external/core/mem.ln) | public | ordinary provider | original 32-provider audit |
| `cell` | [core/cell.ln](../../../luna-rs/libs/external/core/cell.ln) | public | ordinary provider | later addition; unverified |
| `atomic` | [core/atomic.ln](../../../luna-rs/libs/external/core/atomic.ln) | public | ordinary provider | later addition; unverified |
| `sync` | [core/sync.ln](../../../luna-rs/libs/external/core/sync.ln) | public | ordinary provider | later addition; unverified |
| `cmp` | [core/cmp.ln](../../../luna-rs/libs/external/core/cmp.ln) | public | ordinary provider | original 32-provider audit |
| `hash` | [core/hash.ln](../../../luna-rs/libs/external/core/hash.ln) | public | ordinary provider | original 32-provider audit |
| `clone` | [core/clone.ln](../../../luna-rs/libs/external/core/clone.ln) | public | ordinary provider | original 32-provider audit |
| `copy` | [core/copy.ln](../../../luna-rs/libs/external/core/copy.ln) | public | copy | original 32-provider audit |
| `iter_adapters` | [core/iter_adapters.ln](../../../luna-rs/libs/external/core/iter_adapters.ln) | public | ordinary provider | original 32-provider audit |
| `iter_consumers` | [core/iter_consumers.ln](../../../luna-rs/libs/external/core/iter_consumers.ln) | public | ordinary provider | original 32-provider audit |
| `algo` | [core/algo.ln](../../../luna-rs/libs/external/core/algo.ln) | public | ordinary provider | later addition; unverified |
| `time` | [core/time.ln](../../../luna-rs/libs/external/core/time.ln) | public | ordinary provider | later addition; unverified |
| `try` | [core/try.ln](../../../luna-rs/libs/external/core/try.ln) | public | try | original 32-provider audit |
| `io` | [io/io.ln](../../../luna-rs/libs/external/io/io.ln) | public | ordinary provider | original 32-provider audit |
| `file` | [file/file.ln](../../../luna-rs/libs/external/file/file.ln) | public | ordinary provider | original 32-provider audit |
| `path` | [path/path.ln](../../../luna-rs/libs/external/path/path.ln) | public | ordinary provider | original 32-provider audit |
| `net` | [net/net.ln](../../../luna-rs/libs/external/net/net.ln) | public | ordinary provider | later addition; unverified |
| `random` | [core/random.ln](../../../luna-rs/libs/external/core/random.ln) | public | ordinary provider | later addition; unverified |
| `encoding` | [encoding/encoding.ln](../../../luna-rs/libs/external/encoding/encoding.ln) | public | ordinary provider | later addition; unverified |
| `crypto` | [crypto/crypto.ln](../../../luna-rs/libs/external/crypto/crypto.ln) | public | ordinary provider | later addition; unverified |
| `json` | [json/json.ln](../../../luna-rs/libs/external/json/json.ln) | public | ordinary provider | later addition; unverified |
| `num` | [core/num.ln](../../../luna-rs/libs/external/core/num.ln) | public | ordinary provider | original 32-provider audit |
| `default` | [core/default.ln](../../../luna-rs/libs/external/core/default.ln) | public | ordinary provider | original 32-provider audit |
| `convert` | [core/convert.ln](../../../luna-rs/libs/external/core/convert.ln) | public | ordinary provider | original 32-provider audit |
| `float` | [core/float.ln](../../../luna-rs/libs/external/core/float.ln) | public | ordinary provider | original 32-provider audit |
| `fmt` | [core/fmt.ln](../../../luna-rs/libs/external/core/fmt.ln) | public | ordinary provider | original 32-provider audit |
| `box` | [alloc/box.ln](../../../luna-rs/libs/external/alloc/box.ln) | public | ordinary provider | original 32-provider audit |
| `vec` | [alloc/vec.ln](../../../luna-rs/libs/external/alloc/vec.ln) | public | ordinary provider | original 32-provider audit |
| `string` | [alloc/string.ln](../../../luna-rs/libs/external/alloc/string.ln) | public | ordinary provider | original 32-provider audit |
| `hashmap` | [alloc/hashmap.ln](../../../luna-rs/libs/external/alloc/hashmap.ln) | public | ordinary provider | original 32-provider audit |
| `hashset` | [alloc/hashset.ln](../../../luna-rs/libs/external/alloc/hashset.ln) | public | ordinary provider | original 32-provider audit |
| `iter_collect` | [alloc/iter_collect.ln](../../../luna-rs/libs/external/alloc/iter_collect.ln) | public | ordinary provider | original 32-provider audit |
| `rc` | [alloc/rc.ln](../../../luna-rs/libs/external/alloc/rc.ln) | public | ordinary provider | later addition; unverified |
| `arc` | [alloc/arc.ln](../../../luna-rs/libs/external/alloc/arc.ln) | public | ordinary provider | later addition; unverified |
| `vecdeque` | [alloc/vecdeque.ln](../../../luna-rs/libs/external/alloc/vecdeque.ln) | public | ordinary provider | later addition; unverified |
| `binaryheap` | [alloc/binaryheap.ln](../../../luna-rs/libs/external/alloc/binaryheap.ln) | public | ordinary provider | later addition; unverified |
| `btreemap` | [alloc/btreemap.ln](../../../luna-rs/libs/external/alloc/btreemap.ln) | public | ordinary provider | later addition; unverified |
| `btreeset` | [alloc/btreeset.ln](../../../luna-rs/libs/external/alloc/btreeset.ln) | public | ordinary provider | later addition; unverified |
| `thread` | [alloc/thread.ln](../../../luna-rs/libs/external/alloc/thread.ln) | public | ordinary provider | later addition; unverified |
| `panic` | [core/panic.ln](../../../luna-rs/libs/external/core/panic.ln) | public | ordinary provider | original 32-provider audit |
| `__alloc_global` | [alloc/global.ln](../../../luna-rs/libs/external/alloc/global.ln) | internal | ordinary provider | original 32-provider audit |
| `__raw_table` | [alloc/raw_table.ln](../../../luna-rs/libs/external/alloc/raw_table.ln) | internal | ordinary provider | original 32-provider audit |
| `__lang_drop` | [lang/drop.ln](../../../luna-rs/libs/external/lang/drop.ln) | internal | drop | original 32-provider audit |
| `__lang_option` | [lang/option.ln](../../../luna-rs/libs/external/lang/option.ln) | internal | option | original 32-provider audit |
| `__lang_iterator` | [lang/iterator.ln](../../../luna-rs/libs/external/lang/iterator.ln) | internal | iterator | original 32-provider audit |
| `__lang_into_iterator` | [lang/into_iterator.ln](../../../luna-rs/libs/external/lang/into_iterator.ln) | internal | into_iterator | original 32-provider audit |

Component builds and API conformance are separate. See [stdlib.md](stdlib.md) and [gaps.md](gaps.md).
