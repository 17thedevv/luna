# Luna 0.1-alpha.1 Sysroot 49 Provider Verification Matrix

| # | Provider | Path | Visibility | Contract / Trait / Type | Test Suite / Coverage | DAG Status |
|---|---|---|---|---|---|:---:|
| 1 | result | core/result | public | enum Result, trait OptionExt, trait Error | sysroot_build_invariants, result_tests | PASS |
| 2 | ptr | core/ptr | public | NonNull | sysroot_build_invariants, ptr_tests | PASS |
| 3 | slice | core/slice | public | SliceIter, SliceIterMut | sysroot_build_invariants, slice_tests | PASS |
| 4 | mem | core/mem | public | Layout | sysroot_build_invariants, mem_tests | PASS |
| 5 | cell | core/cell | public | Cell, RefCell, Ref, RefMut | sysroot_build_invariants, cell_tests | PASS |
| 6 | atomic | core/atomic | public | enum Ordering, AtomicBool, AtomicU32, AtomicI32 (+2 more) | sysroot_build_invariants, atomic_tests | PASS |
| 7 | sync | core/sync | public | MutexGuard, Mutex, Condvar, Once (+9 more) | sysroot_build_invariants, sync_tests | PASS |
| 8 | cmp | core/cmp | public | trait Eq, trait Ord | sysroot_build_invariants, cmp_tests | PASS |
| 9 | hash | core/hash | public | trait Hash | sysroot_build_invariants, hash_tests | PASS |
| 10 | clone | core/clone | public | trait Clone | sysroot_build_invariants, clone_tests | PASS |
| 11 | copy | core/copy | public | trait Copy | sysroot_build_invariants, copy_tests, lang_contract(copy) | PASS |
| 12 | iter_adapters | core/iter_adapters | public | Map, Filter, Enumerate, Take (+16 more) | sysroot_build_invariants, iter_adapters_tests | PASS |
| 13 | iter_consumers | core/iter_consumers | public | primitives / intrinsics | sysroot_build_invariants, iter_consumers_tests | PASS |
| 14 | algo | core/algo | public | primitives / intrinsics | sysroot_build_invariants, algo_tests | PASS |
| 15 | time | core/time | public | Duration, Instant, SystemTimeError, SystemTime | sysroot_build_invariants, time_tests | PASS |
| 16 | try | core/try | public | trait FromResidual, trait Try, enum ControlFlow, enum Infallible (+4 more) | sysroot_build_invariants, try_tests, lang_contract(try) | PASS |
| 17 | io | io/io | public | enum IoError, enum SeekFrom, trait Read, trait Write (+9 more) | sysroot_build_invariants, io_tests | PASS |
| 18 | file | file/file | public | enum FileError, File, OpenOptions, Metadata (+7 more) | sysroot_build_invariants, file_tests | PASS |
| 19 | path | path/path | public | PathBuf | sysroot_build_invariants, path_tests | PASS |
| 20 | net | net/net | public | enum NetError, enum Shutdown, Ipv4Addr, SocketAddrV4 (+5 more) | sysroot_build_invariants, net_tests | PASS |
| 21 | random | core/random | public | Rng | sysroot_build_invariants, random_tests | PASS |
| 22 | encoding | encoding/encoding | public | enum HexError, enum Base64Error | sysroot_build_invariants, encoding_tests | PASS |
| 23 | crypto | crypto/crypto | public | Sha256, HmacSha256, ChaCha20 | sysroot_build_invariants, crypto_tests | PASS |
| 24 | json | json/json | public | enum JsonValue, JsonKeyValue, enum JsonError | sysroot_build_invariants, json_tests | PASS |
| 25 | num | core/num | public | primitives / intrinsics | sysroot_build_invariants, num_tests | PASS |
| 26 | default | core/default | public | trait Default | sysroot_build_invariants, default_tests | PASS |
| 27 | convert | core/convert | public | enum TryConvertError, trait Convert, trait TryConvert | sysroot_build_invariants, convert_tests | PASS |
| 28 | float | core/float | public | trait ToBits, trait FromBits, trait FloatOps | sysroot_build_invariants, float_tests | PASS |
| 29 | fmt | core/fmt | public | enum FmtError, trait Writer, trait Display, trait Debug | sysroot_build_invariants, fmt_tests | PASS |
| 30 | box | alloc/box | public | Box | sysroot_build_invariants, box_tests | PASS |
| 31 | vec | alloc/vec | public | Vec, VecIntoIter | sysroot_build_invariants, vec_tests | PASS |
| 32 | string | alloc/string | public | String, StringChars, StringIntoIter, enum NulError (+4 more) | sysroot_build_invariants, string_tests | PASS |
| 33 | hashmap | alloc/hashmap | public | MapIntoIter, MapIter, Keys, Values (+1 more) | sysroot_build_invariants, hashmap_tests | PASS |
| 34 | hashset | alloc/hashset | public | SetIntoIter, SetIter, HashSet | sysroot_build_invariants, hashset_tests | PASS |
| 35 | iter_collect | alloc/iter_collect | public | primitives / intrinsics | sysroot_build_invariants, iter_collect_tests | PASS |
| 36 | rc | alloc/rc | public | Rc, RcWeak | sysroot_build_invariants, rc_tests | PASS |
| 37 | arc | alloc/arc | public | Arc, ArcWeak | sysroot_build_invariants, arc_tests | PASS |
| 38 | vecdeque | alloc/vecdeque | public | VecDequeIter, VecDequeIntoIter, VecDeque | sysroot_build_invariants, vecdeque_tests | PASS |
| 39 | binaryheap | alloc/binaryheap | public | BinaryHeapIntoIter, BinaryHeap | sysroot_build_invariants, binaryheap_tests | PASS |
| 40 | btreemap | alloc/btreemap | public | BTreeMapIter, BTreeKeys, BTreeValues, BTreeMapIntoIter (+1 more) | sysroot_build_invariants, btreemap_tests | PASS |
| 41 | btreeset | alloc/btreeset | public | BTreeSetIter, BTreeSetIntoIter, BTreeSet | sysroot_build_invariants, btreeset_tests | PASS |
| 42 | thread | alloc/thread | public | enum ThreadError, ThreadId, JoinHandle, Scope | sysroot_build_invariants, thread_tests | PASS |
| 43 | panic | core/panic | public | Backtrace | sysroot_build_invariants, panic_tests | PASS |
| 44 | __alloc_global | alloc/global | internal | primitives / intrinsics | sysroot_build_invariants, __alloc_global_tests | PASS |
| 45 | __raw_table | alloc/raw_table | internal | enum ProbeResult, RawTable, RawTableIntoIter, RawTableIter | sysroot_build_invariants, __raw_table_tests | PASS |
| 46 | __lang_drop | lang/drop | internal | trait Drop | sysroot_build_invariants, __lang_drop_tests, lang_contract(drop) | PASS |
| 47 | __lang_option | lang/option | internal | enum Option | sysroot_build_invariants, __lang_option_tests, lang_contract(option) | PASS |
| 48 | __lang_iterator | lang/iterator | internal | trait Iterator | sysroot_build_invariants, __lang_iterator_tests, lang_contract(iterator) | PASS |
| 49 | __lang_into_iterator | lang/into_iterator | internal | trait IntoIterator | sysroot_build_invariants, __lang_into_iterator_tests, lang_contract(into_iterator) | PASS |
