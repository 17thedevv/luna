<!-- luna-doc-role: guidance -->

# Đối chiếu thư viện chuẩn C++ và lộ trình mở rộng Luna

Ngày 2026-10-22. Đây là bản đồ chức năng để triển khai theo yêu cầu maintainer,
không phải cam kết tương thích C++ hay tuyên bố Luna đã có thư viện tương đương.
Mốc đối chiếu cố định là bản thảo C++23 N4950, không trộn các đề xuất C++26.
Nguồn: [bảng header N4950](https://timsong-cpp.github.io/cppwp/n4950/headers),
[container](https://timsong-cpp.github.io/cppwp/n4950/containers),
[numerics](https://timsong-cpp.github.io/cppwp/n4950/numerics),
[bản PDF WG21](https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2023/n4950.pdf).

Phía Luna được đối chiếu với source của 50 provider trong
[manifest](../../../luna-rs/libs/external/sysroot.toml), không chỉ tên file.
[Snapshot parser tại 28802f12](../../audits/stdlib-usize-2026-10-22/evidence/windows-28802f12/source-surface-receipt.json)
ghi 2.411 khai báo và 219 import tường minh, gồm cả khai báo triển khai.
“Có nền” nghĩa là tìm thấy triển khai liên quan, chưa chứng minh đủ API, độ
phức tạp, điều kiện biên hoặc parity. Không tính phần trăm coverage bằng số
header/provider: provider, namespace và năng lực ngôn ngữ là các đơn vị khác nhau.

## Bảng chức năng

| Nhóm C++23 | Luna hiện có trong source | Phần thiếu hoặc cần mở rộng |
| --- | --- | --- |
| array / tuple / span | Array, tuple, slice của ngôn ngữ; ptr/mem/slice | Đối chiếu view và lifetime; không cần thêm bản sao chỉ để khớp tên C++ |
| vector / deque / priority queue | Vec, VecDeque, BinaryHeap | Hoàn thiện constructor, usize, try_* và kiểm thử từng thao tác |
| stack / queue | Vec và VecDeque cung cấp thao tác nền | Thiết kế adaptor nếu có invariant/use case riêng; không tạo wrapper rỗng |
| ordered map/set | BTreeMap, BTreeSet; quan hệ/đại số BTreeSet PASS focused Windows source/artifact | Removal, entry và range cần audit riêng; qualification đầy đủ vẫn mở |
| unordered map/set | HashMap, HashSet, RawTable; HashSet::is_disjoint PASS focused Windows source/artifact | Audit rộng hơn về hash/collision/drop và fallible growth |
| list / forward_list | Chưa có collection node tương ứng | Chỉ triển khai sau contract cursor, invalidation và nhu cầu thao tác node |
| flat/multi associative containers | Chưa có collection chuyên biệt | FlatMap/FlatSet và duplicate-key collection cần contract riêng |
| bit / bitset | Toán bit và 10 utility u64 mới PASS focused Windows source/artifact | Packed bit collection và các width khác chưa có trong lát này |
| memory / memory_resource | Box, Rc, Arc, Weak, Layout; ABI 2; Vec/Box/String try_* | Allocator tùy biến/arena/pool chưa có contract; không giả lập allocator bằng compiler branch |
| optional / expected / variant | Option, Result, enum có payload | Phần tương ứng là kiểu của Luna; không cần nhân đôi Optional/Expected/Variant |
| any / typeinfo / typeindex | Chưa có Any/type erasure/RTTI API tương ứng | Cần quyết định contract reflection/type identity trước khi thêm |
| concepts / type_traits / utility | Trait bounds, generic, comptime, move/drop | Chưa có catalog truy vấn type; không đưa luật C++ vào semantics Luna |
| iterator / ranges / generator | Iterator, IntoIterator, adapters/consumers, range | Audit adapter, double-ended/exact-size, collect và borrow; không xem mọi range là hoàn chỉnh |
| algorithm / execution | slice sort/search; algo partition, bounds, rotate | Merge/set algorithms, permutation, scans và parallel execution còn thiếu hoặc chưa có contract |
| numeric / numbers | checked/saturating/wrapping, gcd/lcm, iterator sum/product/reduce; pi/e | Inner product, prefix scan, numeric sequence và catalog hằng số còn thiếu |
| cmath / floating environment | float có sin/cos/sqrt/pow/log và IEEE helpers | Thiếu nhiều hàm đặc biệt, complex và floating environment; phải xác định sai số/NaN/Inf |
| complex / valarray | Chưa có type tương ứng | Complex và dense numeric array là các đợt thiết kế/triển khai mới |
| random | Rng, seed/entropy, scalar range, fill/shuffle | Distribution và reproducibility contract chưa tương đương catalog C++ |
| string / string_view / charconv | String, str, CStr/CString, UTF-8, conversion helpers | Parser/formatter số và Unicode cần đối chiếu chi tiết; không xem str của Luna là string_view C++ |
| format / print / streams | Display/Debug/Writer, console macros, Read/Write/Seek, buffer/cursor | Chưa có format-string grammar, width/precision catalog hay stream ecosystem đầy đủ |
| regex / locale / Unicode | Chưa có regex/locale engine | Thiết kế regex và Unicode riêng; không gắn locale C++ vào String mặc định |
| filesystem | File/OpenOptions, metadata, directory operations, lexical path | Recursive traversal, permission/symlink/watch và lỗi platform cần audit/thiết kế |
| chrono | Duration, Instant, SystemTime | Calendar, timezone database, parse/format ngày giờ còn thiếu |
| atomic / mutex / thread | Atomic types, Mutex/RwLock/Condvar/Once, semaphore/channel, scoped thread | Latch/barrier, cancellation/stop token và future/promise cần contract riêng |
| coroutine | Nền async của compiler/runtime | Chưa có catalog executor/generator công khai được qualification đầy đủ |
| system_error / source_location / stacktrace | Error theo domain, panic/diagnostic | Error-code liên miền, source location và stacktrace API còn thiếu |
| exception / C compatibility / deprecated headers | Result/panic/FFI của Luna | Không nhập exception model, C varargs hay API deprecated chỉ để đủ tên header |

## Thứ tự thực hiện

1. **Nền bộ nhớ và collection (lát đầu PASS focused Windows):** STD-D05 usize; constructor
   trên type; giữ ownership/UTF-8/overflow; hoàn thiện quan hệ và phép tập hợp
   BTreeSet; bổ sung HashSet::is_disjoint và utility bit u64. Đây là một lát
   triển khai, không phải đóng toàn bộ nhóm container/bit của C++.
   [Kết quả tại 28802f12](../../audits/stdlib-usize-2026-10-22/README.md)
   tách rõ kiểm thử thành công và các gate còn mở.
2. **Toán và mảng CPU:** thiết kế Complex, numeric folds/scans và constants;
   Matrix/Tensor sở hữu bộ nhớ liên tục, shape/index usize, kiểm tra overflow,
   indexing/reshape, rồi elementwise và matmul CPU. Chốt kiểu số, storage order,
   shape errors, borrow/view, allocation failure và IEEE behavior trước từng API.
3. **Graph:** chốt directed/undirected, node/edge identity, duplicate/removal,
   weight và invalidation; sau đó adjacency, traversal và path algorithms.
4. **Text và host:** conversion/formatting, regex/Unicode, filesystem, calendar
   và concurrency theo các contract riêng, với cùng qualification hai platform.
5. **Collection chuyên biệt và tối ưu:** BitSet, flat/multi/node containers,
   arena/pool, parallel/vectorized execution sau khi API nền ổn định.

Graph và owning Matrix/Tensor là phần mở rộng maintainer đã chọn, không phải
header còn thiếu trong bảng C++23. `mdspan` là view nhiều chiều, không thay thế
toàn bộ tensor engine. NumPy, GPU và AI nâng cao không phải mục tiêu clone vào
stdlib; thư viện riêng có thể dùng local/vendored dependency của Lunar, không
cần package registry hoặc server tự host.

## Điều kiện đóng từng gap

Mỗi đợt phải có signature, namespace/owner, generic bounds, unit của size/index,
ownership/borrow, lỗi/overflow và allocation policy. Sau đó triển khai bằng Luna
thông thường, có CLI check/build/native cho source-only và fresh artifact-only,
negative controls và stale-artifact rejection. Có source hoặc build được provider
không đủ để chuyển cả dòng thành PASS. Compiler defect phải có reproducer độc
lập, sửa machinery chung và regression; không thêm type-specific intrinsic.

Toàn bộ stable, package/installer và tám lane vẫn phải được qualification ở
final pin. Bảng này không biến thư viện C++ thành authority của ngôn ngữ Luna.
