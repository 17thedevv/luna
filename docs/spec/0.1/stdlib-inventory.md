# Luna 0.1 provider inventory

Generated from [sysroot.toml](../../../luna-rs/libs/external/sysroot.toml). Provider identity is not a namespace or a completeness claim.

Current manifest: **50 providers**. The [original audit manifest](evidence/sysroot-audit-baseline.toml) has 32; later additions are implementation inventory, not automatic spec adoption or conformance. Public/internal visibility controls import access, separately from exported declarations.

Current evidence: all 49 fresh bundle/object hashes and target headers are preserved by [E1 provenance](../../audits/0.1-alpha-completion-2026-10-04/evidence/e1-final5-sysroot-provenance-2026-10-13.json). [R4 provider reconciliation](../../audits/0.1-alpha-completion-2026-10-04/r4/README.md) records bounded D3 graph/API evidence; builds do not certify every public API or advertised target.

| Provider | Source component | Import visibility | Contract family | Historical audit boundary | Current build evidence |
|---|---|---|---|---|---|
| `result` | [core/result.ln](../../../luna-rs/libs/external/core/result.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `ptr` | [core/ptr.ln](../../../luna-rs/libs/external/core/ptr.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `slice` | [core/slice.ln](../../../luna-rs/libs/external/core/slice.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `mem` | [core/mem.ln](../../../luna-rs/libs/external/core/mem.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `cell` | [core/cell.ln](../../../luna-rs/libs/external/core/cell.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `atomic` | [core/atomic.ln](../../../luna-rs/libs/external/core/atomic.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `sync` | [core/sync.ln](../../../luna-rs/libs/external/core/sync.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `cmp` | [core/cmp.ln](../../../luna-rs/libs/external/core/cmp.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `hash` | [core/hash.ln](../../../luna-rs/libs/external/core/hash.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `clone` | [core/clone.ln](../../../luna-rs/libs/external/core/clone.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `copy` | [core/copy.ln](../../../luna-rs/libs/external/core/copy.ln) | public | copy | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `iter_adapters` | [core/iter_adapters.ln](../../../luna-rs/libs/external/core/iter_adapters.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `iter_consumers` | [core/iter_consumers.ln](../../../luna-rs/libs/external/core/iter_consumers.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `algo` | [core/algo.ln](../../../luna-rs/libs/external/core/algo.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `time` | [core/time.ln](../../../luna-rs/libs/external/core/time.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `try` | [core/try.ln](../../../luna-rs/libs/external/core/try.ln) | public | try | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `io` | [io/io.ln](../../../luna-rs/libs/external/io/io.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `file` | [file/file.ln](../../../luna-rs/libs/external/file/file.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `path` | [path/path.ln](../../../luna-rs/libs/external/path/path.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `net` | [net/net.ln](../../../luna-rs/libs/external/net/net.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `random` | [core/random.ln](../../../luna-rs/libs/external/core/random.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `encoding` | [encoding/encoding.ln](../../../luna-rs/libs/external/encoding/encoding.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `crypto` | [crypto/crypto.ln](../../../luna-rs/libs/external/crypto/crypto.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `json` | [json/json.ln](../../../luna-rs/libs/external/json/json.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `num` | [core/num.ln](../../../luna-rs/libs/external/core/num.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `default` | [core/default.ln](../../../luna-rs/libs/external/core/default.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `convert` | [core/convert.ln](../../../luna-rs/libs/external/core/convert.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `float` | [core/float.ln](../../../luna-rs/libs/external/core/float.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `fmt` | [core/fmt.ln](../../../luna-rs/libs/external/core/fmt.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `alloc_error` | [alloc/error.ln](../../../luna-rs/libs/external/alloc/error.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `box` | [alloc/box.ln](../../../luna-rs/libs/external/alloc/box.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `vec` | [alloc/vec.ln](../../../luna-rs/libs/external/alloc/vec.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `string` | [alloc/string.ln](../../../luna-rs/libs/external/alloc/string.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `hashmap` | [alloc/hashmap.ln](../../../luna-rs/libs/external/alloc/hashmap.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `hashset` | [alloc/hashset.ln](../../../luna-rs/libs/external/alloc/hashset.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `iter_collect` | [alloc/iter_collect.ln](../../../luna-rs/libs/external/alloc/iter_collect.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `rc` | [alloc/rc.ln](../../../luna-rs/libs/external/alloc/rc.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `arc` | [alloc/arc.ln](../../../luna-rs/libs/external/alloc/arc.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `vecdeque` | [alloc/vecdeque.ln](../../../luna-rs/libs/external/alloc/vecdeque.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `binaryheap` | [alloc/binaryheap.ln](../../../luna-rs/libs/external/alloc/binaryheap.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `btreemap` | [alloc/btreemap.ln](../../../luna-rs/libs/external/alloc/btreemap.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `btreeset` | [alloc/btreeset.ln](../../../luna-rs/libs/external/alloc/btreeset.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `thread` | [alloc/thread.ln](../../../luna-rs/libs/external/alloc/thread.ln) | public | ordinary provider | later addition; outside original audit | E1 fresh bundle verified; API scope per R4 |
| `panic` | [core/panic.ln](../../../luna-rs/libs/external/core/panic.ln) | public | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__alloc_global` | [alloc/global.ln](../../../luna-rs/libs/external/alloc/global.ln) | internal | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__raw_table` | [alloc/raw_table.ln](../../../luna-rs/libs/external/alloc/raw_table.ln) | internal | ordinary provider | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__lang_drop` | [lang/drop.ln](../../../luna-rs/libs/external/lang/drop.ln) | internal | drop | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__lang_option` | [lang/option.ln](../../../luna-rs/libs/external/lang/option.ln) | internal | option | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__lang_iterator` | [lang/iterator.ln](../../../luna-rs/libs/external/lang/iterator.ln) | internal | iterator | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |
| `__lang_into_iterator` | [lang/into_iterator.ln](../../../luna-rs/libs/external/lang/into_iterator.ln) | internal | into_iterator | original 32-provider audit | E1 fresh bundle verified; API scope per R4 |

Component builds and API conformance are separate. See [stdlib.md](stdlib.md) and [gaps.md](gaps.md).
