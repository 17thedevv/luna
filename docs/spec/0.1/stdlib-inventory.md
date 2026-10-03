# Luna 0.1 provider inventory

Generated from [sysroot.toml](../../../luna-rs/libs/external/sysroot.toml). Provider identity is not a namespace or a completeness claim.

Baseline: **32 providers**. Public/internal visibility controls import access, separately from exported declarations.

| Provider | Source component | Import visibility | Contract family |
|---|---|---|---|
| `result` | [core/result.ln](../../../luna-rs/libs/external/core/result.ln) | public | ordinary provider |
| `ptr` | [core/ptr.ln](../../../luna-rs/libs/external/core/ptr.ln) | public | ordinary provider |
| `slice` | [core/slice.ln](../../../luna-rs/libs/external/core/slice.ln) | public | ordinary provider |
| `mem` | [core/mem.ln](../../../luna-rs/libs/external/core/mem.ln) | public | ordinary provider |
| `cmp` | [core/cmp.ln](../../../luna-rs/libs/external/core/cmp.ln) | public | ordinary provider |
| `hash` | [core/hash.ln](../../../luna-rs/libs/external/core/hash.ln) | public | ordinary provider |
| `clone` | [core/clone.ln](../../../luna-rs/libs/external/core/clone.ln) | public | ordinary provider |
| `copy` | [core/copy.ln](../../../luna-rs/libs/external/core/copy.ln) | public | copy |
| `iter_adapters` | [core/iter_adapters.ln](../../../luna-rs/libs/external/core/iter_adapters.ln) | public | ordinary provider |
| `iter_consumers` | [core/iter_consumers.ln](../../../luna-rs/libs/external/core/iter_consumers.ln) | public | ordinary provider |
| `try` | [core/try.ln](../../../luna-rs/libs/external/core/try.ln) | public | try |
| `io` | [io/io.ln](../../../luna-rs/libs/external/io/io.ln) | public | ordinary provider |
| `file` | [file/file.ln](../../../luna-rs/libs/external/file/file.ln) | public | ordinary provider |
| `path` | [path/path.ln](../../../luna-rs/libs/external/path/path.ln) | public | ordinary provider |
| `num` | [core/num.ln](../../../luna-rs/libs/external/core/num.ln) | public | ordinary provider |
| `default` | [core/default.ln](../../../luna-rs/libs/external/core/default.ln) | public | ordinary provider |
| `convert` | [core/convert.ln](../../../luna-rs/libs/external/core/convert.ln) | public | ordinary provider |
| `float` | [core/float.ln](../../../luna-rs/libs/external/core/float.ln) | public | ordinary provider |
| `fmt` | [core/fmt.ln](../../../luna-rs/libs/external/core/fmt.ln) | public | ordinary provider |
| `box` | [alloc/box.ln](../../../luna-rs/libs/external/alloc/box.ln) | public | ordinary provider |
| `vec` | [alloc/vec.ln](../../../luna-rs/libs/external/alloc/vec.ln) | public | ordinary provider |
| `string` | [alloc/string.ln](../../../luna-rs/libs/external/alloc/string.ln) | public | ordinary provider |
| `hashmap` | [alloc/hashmap.ln](../../../luna-rs/libs/external/alloc/hashmap.ln) | public | ordinary provider |
| `hashset` | [alloc/hashset.ln](../../../luna-rs/libs/external/alloc/hashset.ln) | public | ordinary provider |
| `iter_collect` | [alloc/iter_collect.ln](../../../luna-rs/libs/external/alloc/iter_collect.ln) | public | ordinary provider |
| `panic` | [core/panic.ln](../../../luna-rs/libs/external/core/panic.ln) | public | ordinary provider |
| `__alloc_global` | [alloc/global.ln](../../../luna-rs/libs/external/alloc/global.ln) | internal | ordinary provider |
| `__raw_table` | [alloc/raw_table.ln](../../../luna-rs/libs/external/alloc/raw_table.ln) | internal | ordinary provider |
| `__lang_drop` | [lang/drop.ln](../../../luna-rs/libs/external/lang/drop.ln) | internal | drop |
| `__lang_option` | [lang/option.ln](../../../luna-rs/libs/external/lang/option.ln) | internal | option |
| `__lang_iterator` | [lang/iterator.ln](../../../luna-rs/libs/external/lang/iterator.ln) | internal | iterator |
| `__lang_into_iterator` | [lang/into_iterator.ln](../../../luna-rs/libs/external/lang/into_iterator.ln) | internal | into_iterator |

Component builds and API conformance are separate. See [stdlib.md](stdlib.md) and [gaps.md](gaps.md).
