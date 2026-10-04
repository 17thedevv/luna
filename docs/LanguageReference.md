<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](spec/0.1/README.md).

# Tham khảo Luna 0.1

Tài liệu chuẩn: [Spec Luna 0.1](spec/0.1/README.md). Giữ contract đã định nghĩa dù implementation chưa hoàn tất; xem [khoảng hở](spec/0.1/gaps.md).

## Ví dụ cơ bản

```luna
struct Point { x: i32, y: i32, };
fn add(a: i32, b: i32) -> i32 { return a + b; }
fn main() -> i32 {
    dec point = Point { x: 10, y: 20 };
    dec rw result = add(point.x, point.y);
    result = result + 12;
    return result - 42;
}
```

- Binding: dec, dec rw; const cần được đánh giá ở compile-time.
- Field cách nhau bằng dấu phẩy; struct kết thúc bằng dấu chấm phẩy sau contract.
- Declaration mặc định private; field mặc định public theo Visibility-02, private ẩn field. Truy cập kiểm tra cả type và field.
- Reference: &T / &rw T; raw pointer: *T / *rw T.
- Generic: identity<i32>(42); match dùng ->; await dùng future.await.
- Lifetime: life_from(source), requires life(a) >= life(b).
- Macro: macro twice { (@x: expr) => { @x * 2 } }, gọi twice!(21).

## Provider và namespace

```luna
import <vec>;
fn main() -> i32 {
    dec rw values = std::vec_new<i32>();
    values.push(42);
    return values[0] - 42;
}
```

import nạp provider; module định nghĩa namespace; :: truy cập namespace; using std as library đặt alias cục bộ; using std mở các tên trực tiếp có quyền truy cập để dùng không cần tiền tố. Các directive này chỉ dùng ở cấp file/module; tên xung đột báo E1008. Vec là std::Vec, không suy alloc::Vec hay std::collections::Vec từ tên file. Bootstrap chỉ lộ path được chỉ định dưới std.

| Chủ đề | Spec |
|---|---|
| Kiểu, literal, binding, hàm, struct, enum, trait, control flow | [Syntax](spec/0.1/syntax.md) |
| Ownership, borrow, lifetime, unsafe, FFI, closure, async, comptime | [Semantics](spec/0.1/semantics.md) |
| Macro, hygiene và giới hạn compiler/stdlib | [Macros](spec/0.1/macros.md) |
| Module, visibility, bootstrap và artifact | [Modules](spec/0.1/modules.md) |
| Thư viện chuẩn | [Stdlib](spec/0.1/stdlib.md) |
| Runtime ABI và targets | [Runtime](spec/0.1/runtime.md) |
| Diagnostic, evidence và release gates | [Conformance](spec/0.1/conformance.md) |

Các ví dụ cũ dùng mut, use, mod, $, @<...>, prefix await hoặc where outlives không phải syntax hiện hành. Các kết quả grammar cũ trong V01-GRAMMAR-02 là bằng chứng có ngày; đối chiếu với sổ cái sửa chữa và kiểm chứng hiện tại trước khi kết luận về implementation.

## Cấu hình provider tùy chọn

```toml
schema = 1
[providers]
geo = "../shared/geometry"
```

File `luna.toml` gần entry file nhất được chọn cho toàn bộ lần biên dịch, kể cả
dependency bắc cầu. `import <geo>;` tìm provider tại stem đã cấu hình; provider
vẫn tự khai báo namespace, ví dụ `geometry`. Đường dẫn tương đối tính từ file
TOML, không từ thư mục chạy lệnh. `--config FILE` chọn rõ file, `--no-config`
tắt cấu hình. Import đường dẫn cũ vẫn hoạt động. Không tự import, build lại
artifact, tải package hoặc trộn config của dependency. Xem
[PROVIDER-CONFIG-v1](spec/0.1/provider-config-v1.md) và
[NAMESPACE-USING-v1](spec/0.1/namespace-using-v1.md).
