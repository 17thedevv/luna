<!-- luna-doc-role: guidance -->

# Đồng thiết kế lại stdlib cho Luna 0.1.0

Ngày **2026-10-21**, Asia/Bangkok. **ĐANG THIẾT KẾ / TRIỂN KHAI PHẦN NỀN**.
Nhánh `stdlib-redesign`, dựa trên `264390da` của `stable-baseline`.

Maintainer yêu cầu thiết kế lại toàn bộ API và layout, thay cho phương án chỉ
sắp xếp mã và giữ API. Tài liệu ghi riêng quyết định đã chốt và phần đề xuất;
API mới chưa có trong alpha, chưa thiết lập stable qualification. Sau STD-D06,
maintainer yêu cầu bắt đầu triển khai: allocator hai tầng là phần đầu tiên,
được theo dõi trong [hồ sơ thực hiện](../audits/stdlib-fallible-2026-10-21/README.md).

## Quyết định đã chốt trực tiếp với maintainer

| ID | Quyết định | Hệ quả |
| --- | --- | --- |
| STD-D01 | Thiết kế lại toàn bộ API và layout | Có thể thay public surface trước stable; cần hợp đồng và bản đồ chuyển đổi |
| STD-D02 | Namespace gọn ở std | std::String, std::Vec; namespace con khi nhóm API cần nó |
| STD-D03 | Constructor thuộc type | Type::new(...), Type::with_capacity(...); thay factory có tiền tố type khi làm cùng việc |
| STD-D04 | Giữ vị trí source hiện tại | Gốc vẫn là luna-rs/libs/external; tổ chức lại bên trong |
| STD-D05 | usize cho size/index bộ nhớ | Độ dài/capacity/index của container và bộ nhớ dùng usize; fixed-width dành cho domain/format yêu cầu số bit |
| STD-D06 | Cấp phát hai tầng theo phương án B | API thường abort khi OOM; API try_* trả lỗi để caller xử lý, bảo toàn allocation/dữ liệu cũ khi thất bại |

Thẩm quyền: các câu trả lời của maintainer ngày 2026-10-21. Đây là hướng thiết
kế, chưa chứng nhận mọi API có thể triển khai. Thay API không tự đổi ownership,
lifetime, builtin str, cú pháp macro hoặc lang-item. STD-D06 cho phép thiết kế
thêm primitive cấp phát fallible; ABI cụ thể cần amendment và kiểm chứng riêng.

[Decision ledger dạng JSON](stdlib-redesign-0.1/decisions.json) ghi riêng các
direction đã được chọn và đề xuất chưa xác nhận.

[Bản đồ size/index](stdlib-redesign-0.1/size-index.md) tách các vị trí bộ nhớ
cần đổi sang usize khỏi hash, file offset và số liệu domain có độ rộng bit cố
định. Đây là kế hoạch migration tiếp theo, chưa phải API đã triển khai.

## Audit trước kế hoạch migration

[Inventory bằng parser Luna](../audits/stdlib-redesign-2026-10-21/README.md)
tại pin 264390da ghi **49 provider, 213 import trực tiếp, 2.358 record
declaration/impl/module**. Có helper private và trait method: đây không phải
2.358 public API hoặc feature đã kiểm chứng.

Mỗi record gắn với provider, file, namespace source, owner impl/trait và artifact
dự kiến qua provider. Signature/field/bound/lifetime AST phục vụ đối chiếu;
source vẫn là authority. Generated items và effective export cần snapshot
semantic riêng trước migration.

| Điểm cần thống nhất | Hiện trạng | Hướng review |
| --- | --- | --- |
| Constructor | vec_new/string_new/box_new; type khác có new; Ipv4Addr có cả hai | Một constructor công khai theo STD-D03 |
| Provider nhiều trách nhiệm | string 1.210 dòng; io 1.203; file 1.025; sync 890 | Chia theo type/protocol và dependency, không theo ngưỡng dòng |
| Thư mục và dependency | core/sync, core/time, core/random có runtime binding; alloc/thread tạo thread OS | Tách phần thuần dữ liệu khỏi OS; không coi mọi extern là OS |
| Console | Hàm byte, hàm _str, macro global và trong std | Chốt entry point theo tác vụ và alias compatibility |
| Size/index | Vec dùng u64; byte I/O và slice có usize | Đề xuất usize cho bộ nhớ/index; giữ fixed width cho domain |
| Runtime bridge | Một số provider có export extern __luna_* | Rà effective visibility/internal bridge; prefix không thay access control |
| Test | Hai gốc tests/luna/stdlib và luna-rs/tests/luna/stdlib | Một catalog acceptance; textual reference không phải kết quả chạy |

## Layout bên trong gốc hiện tại — đề xuất

```text
luna-rs/libs/external/
  sysroot.toml
  lang/     # protocol/hook được ngôn ngữ nhận diện
  core/     # foundation, primitive, trait, thuật toán không heap/OS
  alloc/    # ownership/container/text cần heap
  host/     # console, file, network, clock, lock/thread cần OS
```

File/thư mục không tạo namespace: source trong alloc vẫn định nghĩa std::Vec.
Logical provider ID, physical path và public name là ba quyết định riêng.
Component sở hữu type/protocol cùng impl/helper bảo vệ invariant. Tách helper
chung khi có consumer thật và dependency phù hợp. String với C-string view,
Duration với clock OS là các ứng viên tách trách nhiệm, chưa phải migration đã duyệt.

Lang và primitive core tạo tập bootstrap có DAG tường minh. Không giả định mọi
lang đứng trước mọi core: Try hiện liên quan Result. Toàn bộ graph, gồm bootstrap,
quyết định thứ tự. Core thuần không kéo heap/OS; alloc không kéo dịch vụ OS.
Panic/trap, memory và atomic primitive được xét theo contract cụ thể. Trait byte
I/O thuần và buffer heap không bắt buộc cùng provider với console OS.

sysroot.toml giữ mapping physical/access policy; registry compiler giữ ngữ nghĩa
hook. Relocation cần kiểm registry, builder và resolver cùng nhau. Đề xuất tách
artifact staging/cache theo target khỏi source; output/install layout và manifest
cuối cùng còn mở. Official builder sinh artifact; interface đổi phải đổi fingerprint
và rebuild dependent, không patch llib bằng tay.

## Quy ước API — đề xuất để cùng chốt

| Nhóm | Đề xuất | Điều cần chứng minh |
| --- | --- | --- |
| Tạo giá trị | Constructor trên type theo STD-D03 | Owner/method binder, inference, llib call |
| Size/index | usize cho bộ nhớ/index theo STD-D05 | Overflow trước allocation, conversion runtime ABI |
| Borrow | &Self / &rw Self và lifetime relation rõ | Exclusivity, escape, owner invalidation, borrowed aggregate |
| View | as_* mô tả view không chuyển ownership | Return bị chặn bởi lifetime input; mô tả copy đúng thực tế |
| Copy/consume | Tên phân biệt sao chép với chuyển ownership, ví dụ to_* / into_* | Clone/Copy bound, drop một lần, lỗi giữa chừng |
| Lỗi có thể xử lý | Result với error type theo domain | Cause/status, partial I/O, cleanup |
| Vắng giá trị bình thường | Option | Phân biệt không tìm thấy với I/O error/invalid input |
| Text | String sở hữu UTF-8; byte slice/Vec chứa byte bất kỳ; str giữ nghĩa hiện hành | Byte → text validation, Unicode/NUL, lifetime |
| Raw/runtime | Bridge nội bộ, safe wrapper có invariant và unsafe proof | Effective visibility, FFI/layout, source/artifact parity |

STD-D06 đã chốt bổ sung fallible allocation: constructor/thao tác thường giữ
hành vi abort khi OOM, còn biến thể try_* trả Result. Không ép mọi constructor
trả Result: Vec::new hiện tạo vector rỗng mà không cấp phát. Khi try_reserve
thất bại, vector cũ vẫn hợp lệ; API nhận ownership một giá trị cần có đường trả
lại giá trị đó khi không thể đưa nó vào container.

[Bản thiết kế allocator](stdlib-redesign-0.1/allocator.md) ghi primitive,
ownership, versioning và acceptance đề xuất. [ABI v1](../runtime/abi-v1.md)
hiện vẫn abort khi allocation overflow/OOM và không trả NULL; extension
[runtime ABI 2](../runtime/abi-v2-allocation.md) được triển khai trên nhánh và
đang kiểm chứng. Conversion naming, error taxonomy các domain còn lại, danh sách try_*
và compatibility alias còn mở. Thiết kế xuất phát từ semantics Luna, không
sao chép catalog API của ngôn ngữ khác.

## Catalog cần thiết kế lần lượt

| Domain | Contract trọng tâm |
| --- | --- |
| Protocol nền tảng | Drop/Copy, Option/Result, Iterator/Try, controlled prelude, hook identity |
| Memory/view | Alignment, ZST, initialized storage, provenance/lifetime/anchor |
| Container | Vec/Box/map/set/queue/tree; non-Copy drop, growth, borrow và move/clone |
| Text/encoding | Literal/owned text/byte/C-string; UTF-8, NUL, byte/char boundary |
| Algorithm/iterator/numeric | Naming/bounds, checked arithmetic, predicate ownership, terminal/drop, IEEE scope |
| Formatting/console | Macro/hàm/stream, error, Display/Writer; interpolation cần contract riêng |
| File/path/network | Resource ownership, path contract, partial I/O, error/timeout, cleanup |
| Sync/thread/time | Pure value và OS operation; guard ownership, concurrency, Duration/clock |
| Serialization/crypto | Input/output, invalid data, deterministic contract, resource budget và scope |

Mỗi API hiện có cần disposition: giữ, đổi tên/signature, chuyển owner, thay thế
hoặc đề xuất loại khỏi surface. Loại bỏ phải có lý do và ảnh hưởng consumer.

Console là ca đầu tiên: std::println! và println! hiện đã có, dispatch Display
bằng code thư viện, in nối tiếp giá trị, không parse {}. std::io::println nhận
&[u8], println_str nhận str. Cần chốt surface text/value và byte I/O, alias global,
stdout/stderr error, partial write/flush, arity/repetition. Compiler không chọn
stream/newline/callee theo tên macro; interpolation cần generic contract riêng.

## Đường thực hiện sau thiết kế

1. Chốt nền API: namespace/constructor/root/size-index và chính sách allocator
   hai tầng đã quyết định; ABI fallible, error type, conversion naming và
   compatibility còn mở.
2. Spec domain: signature, owner, dependency, invariant, unsafe proof, error;
   phân loại IMPLEMENT / SAFE-API-VIA-AUDITED-UNSAFE / LANGUAGE-BLOCKED /
   COMPILER-GAP / DEFERRED trước khi hứa API.
3. Snapshot semantic surface qua official builder/manifest và consumer probes;
   bổ sung generated exports/impls; map declaration cũ → API/owner mới và alias.
4. Migration component: tách relocation và đổi contract thành thay đổi review
   riêng; cập nhật import/builder/registry/package/docs và hướng dẫn chuyển mã.
5. Acceptance: CLI check/build/run, typed negatives, non-Copy drop, borrow/escape,
   error boundary; source-only và fresh artifact-only.
6. Qualification mới: sysroot/Lunar/package/installer Windows/Ubuntu,
   stale-artifact rejection, tám lane cùng pin sau khi API ổn định.

49/2.358/213 là snapshot, không phải số đích của layout mới. Alpha và focused PASS
trước redesign giữ giá trị lịch sử, không chứng nhận stdlib mới. Nhánh thiết kế
đã bắt đầu triển khai allocator; toàn bộ API/layout và qualification vẫn còn mở.

## Mở rộng theo đối chiếu C++23 — 2026-10-22

Maintainer yêu cầu đối chiếu nhóm chức năng thư viện chuẩn C++ để bổ sung gap,
đồng thời chọn math/graph/matrix/tensor CPU cơ bản trong stdlib; tính năng nâng
cao dùng thư viện riêng local/vendored. [Bảng đối chiếu và thứ tự triển khai](stdlib-redesign-0.1/cpp-library-gap-map.md)
phân biệt phần có nền, thiếu thao tác, chưa có và khác biệt semantics.
Đây không phải quyết định sao chép exception/RTTI/API deprecated của C++.
