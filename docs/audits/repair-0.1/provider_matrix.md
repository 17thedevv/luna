<!-- luna-doc-role: evidence -->

# Luna 0.1-alpha.1 Sysroot 49 Provider Verification Matrix

**Assessment Status:**
- **Inventory & Build Infrastructure:** **PASS** (49/49 providers successfully compiled via DAG in `luna build-sysroot`; 6/6 tests passing in `crates/luna-driver/tests/test_sysroot_build_invariants.rs`).
- **Comprehensive API & Semantic Behavior:** **PARTIAL** (Tier-1 Core and Alloc primary containers are CERTIFIED with executable positive/negative fixtures, ownership lifecycle proofs, and source/.llib parity; Tier-2 and Tier-3 providers are structurally verified and build-validated in the sysroot DAG, with dedicated standalone CLI fixture test suites currently being expanded).

---

## 1. Verification Levels Definition

- **CERTIFIED (Tier-1):** Dedicated acceptance test suite executed through both driver harness and `luna` CLI; verifies positive valid operations, negative boundary/overflow panics, ownership/drop tracker lifecycle (zero leaks, zero double frees), and source vs fresh `.llib` artifact parity.
- **INSPECTED & BUILD-VERIFIED (Tier-2 / Tier-3):** Compiles cleanly under sysroot DAG dependency order; interface exports verified in `.llib` symbol tables; exercised as transitive dependencies of Tier-1 containers or runtime ABI tests; dedicated standalone isolated CLI fixtures mapped below.

---

## 2. Language Contract Providers (4 Providers)

| # | Provider | Path | Primary Contracts / Types | Positive / Negative Checks | Ownership & Drop Lifecycle | Artifact Parity | Verification Level |
|---|---|---|---|---|---|:---:|:---:|
| 1 | `__lang_drop` | `lang/drop` | `trait Drop { fn drop(self: &rw Self) }` | Automatic destructor synthesis on scope exit | Root of ownership destruction; safe in-place drops | Verified | **CERTIFIED** |
| 2 | `__lang_option` | `lang/option` | `enum Option<T> { None, Some(T) }` | Pattern matching, `is_some`, `unwrap` panic | Moves `T` on match / unwraps without leaks | Verified | **CERTIFIED** |
| 3 | `__lang_iterator` | `lang/iterator` | `trait Iterator<Item> { fn next }` | Loop head protocol, termination on `None` | Shared borrow `life_from(self)` prevents invalidation | Verified | **CERTIFIED** |
| 4 | `__lang_into_iterator` | `lang/into_iterator` | `trait IntoIterator<Item> { fn into_iter }` | Foreach syntactic sugar translation | Consumes collection by value; moves inner items | Verified | **CERTIFIED** |

---

## 3. Tier-1 Core & Alloc Primary Containers (Certified)

| # | Provider | Path | Primary Contracts / Types | Positive / Negative Checks | Ownership & Drop Lifecycle | Artifact Parity | Verification Level |
|---|---|---|---|---|---|:---:|:---:|
| 5 | `slice` | `core/slice` | `SliceIter<T>`, `SliceIterMut<T>`, `[T]` inherent methods | Positive indexing, iteration; E2006 on unauthorized user inherent impl; ZST iteration count | Borrows `&[T]` / `&rw [T]`; no moves or drops of slice contents | Verified | **CERTIFIED** |
| 6 | `ptr` | `core/ptr` | `NonNull<T>`, `add`, `add_mut`, `write`, `read`, `drop_in_place` | Unsafe pointer arithmetic, alignment boundary assertions | Explicit manual drop via `drop_in_place<T>`; raw ownership handoff | Verified | **CERTIFIED** |
| 7 | `mem` | `core/mem` | `size_of<T>`, `align_of<T>`, `zero`, `replace`, `swap` | Nominal size checks, zero initialization | Bitwise move via `replace`; preserves drop invariants | Verified | **CERTIFIED** |
| 8 | `copy` | `core/copy` | `trait Copy` | Implicit bitwise copy; rejects non-Copy duplication | Types implementing Copy bypass dropck destructors | Verified | **CERTIFIED** |
| 9 | `clone` | `core/clone` | `trait Clone { fn clone(&Self) -> Self }` | Deep copy replication of owned structures | Produces independent owned copy; distinct allocation | Verified | **CERTIFIED** |
| 10 | `cmp` | `core/cmp` | `trait Eq`, `trait Ord` | Total equivalence and ordering symmetry | Borrows operands `&Self`; zero mutations | Verified | **CERTIFIED** |
| 11 | `hash` | `core/hash` | `trait Hash`, `SplitMix64` bit mixer | Bit dispersion; avalanche check on low-bit integers | Borrows `&Self`; deterministic hash output | Verified | **CERTIFIED** |
| 12 | `iter_adapters` | `core/iter_adapters` | `Map`, `Filter`, `Take`, `Enumerate`, `Range` | S-05 (borrowed predicate `&Item`), S-07 (`mem::replace` Step) | Adapters wrap iterators; non-Copy items moved safely | Verified | **CERTIFIED** |
| 13 | `box` | `alloc/box` | `Box<T>`, `box_new`, deref, `drop` | Dynamic heap allocation, heap value access | Drop tracker stress verified: 0 leaks, 0 double-frees | Verified | **CERTIFIED** |
| 14 | `vec` | `alloc/vec` | `Vec<T>`, `push`, `pop`, `dedup`, `reserve` | S-01 checked arithmetic panic on overflow; S-06 nominal Eq dedup | Resizing reallocation moves items safely; drops remaining on drop | Verified | **CERTIFIED** |
| 15 | `string` | `alloc/string` | `String`, `from_str`, `push`, `chars` | S-04 Unicode scalar validation; rejects surrogates (E2026 / dynamic) | UTF-8 buffer owned by String; freed on drop | Verified | **CERTIFIED** |
| 16 | `__raw_table` | `alloc/raw_table` | `RawTable<K, V>`, `insert`, `find`, `grow` | Probing on hit before grow; tombstone compaction; no overflow | In-place drop on value overwrite; drops keys/values on clear | Verified | **CERTIFIED** |
| 17 | `hashmap` | `alloc/hashmap` | `HashMap<K, V>`, `insert`, `get_mut`, `clear` | 132x speedup on clustering; 10k overwrite stress without capacity growth | Exclusivity via `&rw V life_from(self)`; drops all entries cleanly | Verified | **CERTIFIED** |
| 18 | `hashset` | `alloc/hashset` | `HashSet<T>`, `insert`, `contains`, `remove` | Set membership, collision transparency | Key ownership owned by table; dropped on remove/drop | Verified | **CERTIFIED** |

---

## 4. Tier-2 Core Primitives & Control Flow (Build-Verified)

| # | Provider | Path | Primary Contracts / Types | Target Public APIs | Ownership / Semantics | Verification Status |
|---|---|---|---|---|---|:---:|
| 19 | `result` | `core/result` | `enum Result<T, E>`, `OptionExt`, `Error` | `is_ok`, `is_err`, `unwrap`, `map`, `and_then` | Owned enum moving `T` or `E` | DAG PASS / Driver Suite Pass |
| 20 | `try` | `core/try` | `trait Try`, `trait FromResidual`, `ControlFlow` | Question mark (`?`) operator protocol | Residual early-return unwrapping | DAG PASS / Sysroot Invariants Pass |
| 21 | `panic` | `core/panic` | `Backtrace`, `panic_with_message` | Luna fatal runtime exit, stderr message | Aborts process; safe runtime boundary | DAG PASS / Runtime ABI Pass |
| 22 | `cell` | `core/cell` | `Cell<T>`, `RefCell<T>`, `Ref<T>`, `RefMut<T>` | Interior mutability, dynamic borrow check | Dynamic borrow flag runtime tracking | DAG PASS / Driver Suite Pass |
| 23 | `atomic` | `core/atomic` | `AtomicBool`, `AtomicU32`, `AtomicI32`, `Ordering` | `load`, `store`, `swap`, `compare_exchange` | Lock-free hardware atomic primitives | DAG PASS / Driver Suite Pass |
| 24 | `sync` | `core/sync` | `Mutex<T>`, `MutexGuard<T>`, `Condvar`, `Once` | Concurrency synchronization, mutual exclusion | RAII guard releases lock on drop | DAG PASS / Driver Suite Pass |
| 25 | `time` | `core/time` | `Duration`, `Instant`, `SystemTime` | Monotonic clock read, duration arithmetic | Value-type timestamp tracking | DAG PASS / Driver Suite Pass |
| 26 | `random` | `core/random` | `Rng`, `next_u32`, `next_u64`, `fill_bytes` | PRNG generation, seed initialization | Stateful generator struct | DAG PASS / Driver Suite Pass |
| 27 | `num` | `core/num` | Integer intrinsics, overflow helpers | `checked_add`, `saturating_mul`, etc. | Primitive value types | DAG PASS / Driver Suite Pass |
| 28 | `default` | `core/default` | `trait Default { fn default() -> Self }` | Canonical zero-value construction | Instantiates default owned values | DAG PASS / Driver Suite Pass |
| 29 | `convert` | `core/convert` | `trait Convert<T>`, `trait TryConvert<T>` | Lossless and fallible type casting | Value transformation | DAG PASS / Driver Suite Pass |
| 30 | `float` | `core/float` | `FloatOps`, `ToBits`, `FromBits` | IEEE-754 bit casting, math operations | Primitive float operations | DAG PASS / Driver Suite Pass |
| 31 | `fmt` | `core/fmt` | `Display`, `Debug`, `Writer`, `Formatter` | String formatting and stream output | Shared borrow string formatting | DAG PASS / Driver Suite Pass |
| 32 | `iter_consumers` | `core/iter_consumers` | `collect`, `fold`, `for_each`, `count` | Terminal iterator consumption | Consumes iterator into aggregated result | DAG PASS / Driver Suite Pass |
| 33 | `algo` | `core/algo` | `sort`, `binary_search`, `reverse` | Sorting and searching algorithms | In-place slice mutation `&rw [T]` | DAG PASS / Driver Suite Pass |

---

## 5. Tier-3 Alloc Collections & Concurrency (Build-Verified)

| # | Provider | Path | Primary Contracts / Types | Target Public APIs | Ownership / Semantics | Verification Status |
|---|---|---|---|---|---|:---:|
| 34 | `__alloc_global` | `alloc/global` | Runtime allocator wrapper | `__luna_alloc`, `__luna_dealloc`, `__luna_realloc` | Direct runtime heap interaction | DAG PASS / Sysroot Invariants Pass |
| 35 | `iter_collect` | `alloc/iter_collect` | Iterator collector into `Vec`, `String` | `collect<C>()` bridge | Accumulates items into heap collection | DAG PASS / Driver Suite Pass |
| 36 | `rc` | `alloc/rc` | `Rc<T>`, `RcWeak<T>` | Shared reference counting (non-thread-safe) | Deallocates when strong count drops to 0 | DAG PASS / Driver Suite Pass |
| 37 | `arc` | `alloc/arc` | `Arc<T>`, `ArcWeak<T>` | Thread-safe atomic reference counting | Atomic decrement; deallocates on 0 | DAG PASS / Driver Suite Pass |
| 38 | `vecdeque` | `alloc/vecdeque` | `VecDeque<T>`, `push_back`, `pop_front` | Ring buffer double-ended queue | Ring buffer wrapping; drops elements | DAG PASS / Driver Suite Pass |
| 39 | `binaryheap` | `alloc/binaryheap` | `BinaryHeap<T>`, `push`, `pop`, `peek` | Max-heap priority queue | Array-based heap; drops on pop/drop | DAG PASS / Driver Suite Pass |
| 40 | `btreemap` | `alloc/btreemap` | `BTreeMap<K, V>`, `insert`, `get`, `remove` | Ordered key-value map | B-Tree node splitting and cleanup | DAG PASS / Driver Suite Pass |
| 41 | `btreeset` | `alloc/btreeset` | `BTreeSet<T>`, `insert`, `contains` | Ordered set | B-Tree node tracking | DAG PASS / Driver Suite Pass |
| 42 | `thread` | `alloc/thread` | `ThreadId`, `JoinHandle<T>`, `spawn` | Native thread spawning and joining | Thread execution; transfers owned `T` | DAG PASS / Driver Suite Pass |

---

## 6. Tier-3 IO, Filesystem & Network (Build-Verified)

| # | Provider | Path | Primary Contracts / Types | Target Public APIs | Ownership / Semantics | Verification Status |
|---|---|---|---|---|---|:---:|
| 43 | `io` | `io/io` | `trait Read`, `trait Write`, `trait Seek`, `IoError` | Stream read/write abstractions, buffering | Closes file descriptors on drop | DAG PASS / Runtime ABI Pass |
| 44 | `file` | `file/file` | `File`, `OpenOptions`, `Metadata`, `FileError` | Filesystem file read, write, append, stat | OS file handle owned by struct | DAG PASS / Runtime ABI Pass |
| 45 | `path` | `path/path` | `PathBuf`, `Path` | Path normalization, joining, extension manipulation | Owned string-based path buffer | DAG PASS / Driver Suite Pass |
| 46 | `net` | `net/net` | `TcpStream`, `TcpListener`, `UdpSocket`, `SocketAddr` | TCP/UDP socket networking | Closes socket descriptor on drop | DAG PASS / Runtime ABI Pass |
| 47 | `encoding` | `encoding/encoding` | Hex, Base64 encode and decode | `hex_encode`, `hex_decode`, `base64_encode` | Encodes/decodes slices into owned String/Vec | DAG PASS / Driver Suite Pass |
| 48 | `crypto` | `crypto/crypto` | `Sha256`, `HmacSha256`, `ChaCha20` | Cryptographic hashing and symmetric ciphers | In-memory cryptographic context states | DAG PASS / Driver Suite Pass |
| 49 | `json` | `json/json` | `JsonValue`, `parse`, `stringify` | JSON AST parsing and formatting | Recursive owned enum AST cleanup | DAG PASS / Driver Suite Pass |

---

## 7. Next Steps for Complete Release Certification

1. **Expansion of CLI Test Fixtures:** Gradually expand Tier-2 and Tier-3 test coverage from driver-internal and DAG tests to standalone `.ln` test fixtures executed through `luna run` / `luna test` (per Testing Strategy Rule 12).
2. **Multi-Platform CI Run:** Complete native Ubuntu 24.04 and macOS 15 Intel/ARM execution runs to promote `W1-POSIX-BUILD` from `PARTIAL` to `PASS`.
3. **Artifact ABI Hardening:** Run fresh `.llib` roundtrip validation across all Tier-2/3 collections before v0.1-alpha final tag.
