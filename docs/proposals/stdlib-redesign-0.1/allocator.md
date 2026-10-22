<!-- luna-doc-role: guidance -->

# Cấp phát hai tầng cho stdlib Luna 0.1.0

Ngày **2026-10-21**, Asia/Bangkok. **ĐANG TRIỂN KHAI / KIỂM CHỨNG**.
Thuộc [RFC stdlib](../stdlib-redesign-0.1.md), quyết định **STD-D06**.

## Chính sách đã được maintainer chọn

Phương án B bổ sung khả năng xử lý lỗi cấp phát vào stdlib:

- API thường như Type::with_capacity tiếp tục abort khi OOM.
- Biến thể try_* trả Result để caller quyết định thử yêu cầu nhỏ hơn, bỏ thao
  tác hoặc giải phóng tài nguyên rồi thử lại.
- Allocation thất bại không làm mất dữ liệu/container đang có.
- Không bắt constructor không cần cấp phát, như Vec::new rỗng, trả Result.

Đây là hướng đã được chọn và maintainer đã yêu cầu triển khai. Bản implementation
đầu dùng ABI 2 và các tên API/error dưới đây; vẫn cần review/qualification trước
stable. [Hồ sơ thực hiện](../../audits/stdlib-fallible-2026-10-21/README.md)
phân biệt code đã có, bằng chứng và công việc còn lại.
Result chỉ phản ánh lỗi allocator đã trả về; không bảo đảm mọi tình huống
thiếu tài nguyên đều cho phép chương trình tiếp tục.

## Điểm xuất phát và ranh giới

Điểm xuất phát trước STD-D06: [ABI v1](../../runtime/abi-v1.md) và
[memory.c](../../../runtime/src/memory/memory.c) chỉ có allocation/reallocation
abort khi OOM; __luna_alloc không trả NULL. Không thể triển khai try_* bằng
cách gọi primitive đó rồi kiểm tra NULL hoặc bắt panic sau khi đã abort.

Runtime chỉ nhận kích thước, alignment, raw pointer và status; không biết
Vec, String, Box, Result hay Drop. Stdlib giữ invariant và chuyển status thành
lỗi theo API. Compiler không có nhánh đặc biệt cho container hay tên try_*.
Việc thêm primitive cấp phát fallible là thay đổi runtime generic riêng.
Gốc source vẫn là luna-rs/libs/external theo STD-D04.

## Primitive runtime đang triển khai

Giữ các entry point v1 và bổ sung hai entry point C theo
[contract implementation ABI 2](../../runtime/abi-v2-allocation.md):

```c
int32_t __luna_try_alloc(size_t size, size_t align, void **out_ptr);
int32_t __luna_try_realloc(void *ptr, size_t old_size, size_t align,
                           size_t new_size, void **out_ptr);
```

| Status implementation | Nghĩa | Output |
| --- | --- | --- |
| 0 | Thành công | Pointer khác NULL, kể cả zero-size sentinel |
| 1 | Allocator không cấp được block yêu cầu | NULL |
| 2 | Size hoặc phép làm tròn vượt miền size_t | NULL |
| 3 | Alignment yêu cầu không hợp lệ | NULL |

Error/status không cần heap, formatting hoặc callback người dùng. out_ptr
phải là vùng ghi hợp lệ, đúng alignment cho pointer và không chồng lên block
đầu vào; hàm đặt *out_ptr = NULL trước mọi nhánh có thể trả lỗi. Raw pointer,
old_size và align phải đúng metadata của allocation còn sống thuộc runtime.
Sai provenance, metadata hoặc double free là vi phạm unsafe precondition,
không được hứa sẽ phát hiện bằng status. Status 3 kiểm tra giá trị alignment,
không xác minh metadata của pointer.

Allocation kiểm tra alignment trước zero-size; alignment là lũy thừa hai,
trong khoảng 1..4096 như ABI hiện tại. Size 0 thành công với sentinel runtime
aligned 4096, không cấp heap và không được dereference. Dealloc sentinel là
no-op. Phép nhân số phần tử và size_of<T>, cộng capacity và làm tròn đều phải
được kiểm tra trước khi overflow. Luna dùng usize theo STD-D05; bridge ánh xạ
sang size_t theo target, không mặc định mọi target tương lai đều là 64-bit.

Reallocation có contract đề xuất:

1. ptr NULL/sentinel biểu diễn allocation rỗng (old_size = 0), thực hiện như
   try_alloc. Alignment vẫn phải hợp lệ.
2. Với block còn sống, kiểm tra request trước mọi thay đổi. new_size = 0
   giải phóng block đúng metadata và trả sentinel thành công.
3. Với size mới khác 0, lấy block mới cùng alignment. Nếu thất bại, trả lỗi,
   out_ptr NULL và **block cũ, nội dung, ownership giữ nguyên**; caller vẫn
   phải giải phóng block cũ khi không cần nữa.
4. Khi thành công, giữ min(old_size, new_size) byte, giải phóng block cũ đúng
   một lần và trả ownership block mới. Caller chỉ dùng pointer trả về sau
   thành công; không tiếp tục dùng pointer cũ.

Allocate-copy-free là chiến lược có thể chứng minh contract này; không bắt
buộc dùng C realloc. Byte copy không gọi move/drop của Luna. Stdlib phải chứng
minh việc chuyển raw storage giữ đúng anchor, initialized slots và ownership
cho type được hỗ trợ, trước khi dùng nó cho container generic.

Các entry point v1 tiếp tục contract abort hiện hành, gồm mã panic và quy tắc
zero-size/alignment. Có thể chia sẻ implementation nếu giữ được contract đó;
không đổi __luna_alloc thành hàm có thể trả NULL.

## API stdlib và ownership đề xuất

| API minh họa | Thành công | Khi allocation thất bại |
| --- | --- | --- |
| Vec<T>::try_with_capacity(n) | Result chứa Vec rỗng có capacity >= n | AllocError, không rò block |
| Vec<T>::try_reserve(additional) | Dành đủ chỗ cho len + additional, không đổi len | AllocError; giữ pointer, len, capacity và các giá trị cũ |
| Vec<T>::try_push(value) | Move value vào slot mới, tăng len một lần | Lỗi sở hữu lại value cùng nguyên nhân; vector giữ nguyên |
| Box<T>::try_new(value) | Box sở hữu value | Lỗi sở hữu lại value cùng nguyên nhân |
| String::try_reserve(additional_bytes) | Đủ byte trống, text không đổi | AllocError; giữ nguyên text và UTF-8 invariant |

Implementation dùng std::AllocFailure<T> với hai field cause và value; error
std::AllocError gồm OutOfMemory, CapacityOverflow và UnsupportedLayout. Aggregate
sở hữu nguyên nhân và T, không cấp phát thêm; caller
có thể lấy lại T hoặc drop lỗi, khiến T được drop đúng một lần. Không âm thầm
drop hoặc clone input để làm cho try_push/try_new có vẻ thành công.

AllocError tối thiểu phân biệt không cấp được bộ nhớ và capacity/size overflow;
layout không được runtime hỗ trợ cần phân loại riêng. Vec phải kiểm tra
len + additional và capacity * size_of<T> trước runtime. Growth policy không
được gây lỗi nếu một capacity vừa đủ vẫn hợp lệ chỉ vì capacity tối ưu bị
overflow. Không giữ hành vi hiện tại trả vector rỗng khi capacity overflow làm
contract mới. Zero-sized type cần quy tắc capacity/drop riêng: không suy ra
chia cho size_of<T> luôn hợp lệ hoặc zero byte nghĩa là không có giá trị sống.

Chuẩn bị mọi kiểm tra/cấp phát có thể thất bại trước khi move input hoặc đổi
metadata. Sau thành công, commit metadata và ownership đúng một lần. Nếu một
API còn có lỗi khác ngoài allocation, contract domain phải nói rõ phạm vi bảo
toàn, partial progress và cleanup. Không tự áp guarantee này cho clone, iterator,
formatting, I/O hoặc callback người dùng. String-producing API vẫn phải kiểm
UTF-8; B không thay semantics của str hay cho phép text chứa byte tùy ý.

## Versioning và migration

Implementation dùng runtime ABI revision 2; phát hành phải rebuild runtime,
stdlib và consumer từ cùng pin. ABI v1 vẫn là contract của các symbol cũ;
extension được ghi riêng, không coi nó đã có trong gói alpha.
Provider/interface fingerprint và package manifest phải ghi dependency mới.
Không tăng metadata protocol chỉ vì thêm symbol nếu format không thay đổi.

Artifact mới cần runtime có symbol tương ứng. Tooling phải từ chối bộ phối
runtime/stdlib không tương thích với lỗi hữu ích, không chờ một lỗi link khó
hiểu rồi quảng cáo là compatible. Không tái dùng qualification alpha hoặc
chuyển fingerprint bằng cách patch .llib. Không thêm allocator tùy biến,
allocator registry hoặc cơ chế tải package vào phạm vi quyết định này.

## Acceptance bắt buộc trước khi claim đã hỗ trợ

| Tầng | Điều kiện kiểm chứng |
| --- | --- |
| Runtime ABI | Alignment thường/over-aligned/invalid; zero/sentinel/null; checked overflow; grow/shrink/to-zero; failure giữ byte và ownership block cũ; dealloc đúng allocator |
| Legacy ABI | Entry point thường vẫn abort đúng typed panic; không trả NULL khi thất bại |
| Public Luna CLI | check/build/run cho mỗi API được công bố; success/error; dùng lại container sau lỗi; input trả lại dùng/drop được |
| Ownership | Non-Copy type có observable Drop; đếm allocation/deallocation và drop đúng một lần cho success, retry, lỗi bị drop, ZST và partial initialization |
| Borrow/anchor | Negative khi mutation/reallocation xung đột borrow; reference không thoát owner; chứng minh raw relocation theo contract Luna hiện hành |
| Source/artifact | Cùng fixture với source-only và fresh artifact-only, không fallback ngầm hoặc lấy source cũ |
| Package | Windows/Ubuntu dùng compiler/runtime/stdlib cùng pin; smoke và negative runtime mismatch; qualification mới sau migration |

Test lỗi phải dùng failure injection xác định trong **runtime test riêng**,
chạy qua cùng đường allocation và fixture CLI thật; không dựa vào yêu cầu RAM
khổng lồ hoặc tình trạng máy. Không ship môi trường bật failure injection trong
runtime production. Dùng budget/counter không cần heap; kiểm tra cả allocation
đầu tiên và các lần growth, cùng trường hợp không allocation thì không tiêu
budget. Cơ chế injection cụ thể là implementation test, không public API.

Primitive và các API trong bảng đã có trên nhánh, cộng String::try_with_capacity
và constructor trên type. Factory cũ tạm giữ làm cầu nối có chủ đích cho migration;
không nhân thêm alias mới. API mới nhận usize; field/size-index API cũ còn u64
trên hai target 64-bit hiện hành và sẽ được chuyển trong đợt STD-D05 riêng.
Hồ sơ thực hiện mới là nơi ghi kết quả chạy; tài liệu này không tự claim completion.
Nếu gặp thiếu machinery ownership/FFI chung, tách reproducer user-defined và
compiler gap trước khi tiếp tục; không đặc cách stdlib.

SKILL IMPACT: không đổi skill. Chính sách đã được chọn được lưu trong RFC;
ABI/API chi tiết đang kiểm chứng; không tự tạo language rule hay stable freeze.
