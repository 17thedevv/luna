# Mellis v1.0 — Official Roadmap: The Self-Hosting Milestone

Mục tiêu tối thượng của Mellis v1.0 không chỉ là một trình biên dịch hoàn thiện, mà là một ngôn ngữ **Self-Hosting**.
Định nghĩa hoàn thành (Definition of Done) cho v1.0:
1. **Stage 0**: Trình biên dịch C++ (hiện tại).
2. **Stage 1**: Trình biên dịch Mellis được viết bằng ngôn ngữ Mellis, biên dịch bởi Stage 0.
3. **Stage 2**: Trình biên dịch Mellis biên dịch lại chính mã nguồn của nó (bởi Stage 1).
4. **Xác thực**: `stage1 == stage2` (Tối thiểu về mặt ngữ nghĩa/behavior, lý tưởng nhất là deterministic output artifact).

---

## Phase 0 — Compiler Soundness (Nền tảng C++)
*Mục tiêu: Đảm bảo compiler C++ (Stage 0) hoàn toàn vững chắc để đủ sức biên dịch Stage 1.*

- **0.1 Error Propagation (`?`)**:
  - `TryExpr`, tương thích `Option`/`Result`, MVIR lowering.
- **0.2 Lifetime Validation**:
  - Đảm bảo an toàn bộ nhớ tuyệt đối cho `borrow`, `return references`, `closure captures`.
- **0.3 Escape Analysis**:
  - Cấm hoàn toàn việc rò rỉ con trỏ cục bộ (return &local, capture).
- **0.4 Destructuring / Patterns (Hoàn thành)**:
  - Ổn định tuple, struct, match, move, borrow.
- **0.5 Intrinsic Framework**:
  - Rà soát hệ thống, loại bỏ code thừa, chuẩn bị cho runtime.
- **0.6 Generics / Traits / MLib**:
  - Đóng module và liên kết cross-module hoàn thiện.

---

## Phase 1 — Runtime + Core (Môi trường tối thiểu)
*Mục tiêu: Đưa các tính năng cơ bản vào thư viện chuẩn, giúp Mellis chạy mượt mà độc lập.*

- **1.1 Process & Startup**:
  - C FFI: `exit`, `args`, `env`, `current_dir`.
  - Chuỗi khởi động: `entry` → `runtime init` → `main` → `shutdown`.
- **1.2 Hosted Profiles**: Windows và Linux.
- **1.3 Thư viện `core` & `alloc`**:
  - `core`: `Option`, `Result`, `slice`, `str`, `iter`, `cmp`, các Trait chuẩn (`Copy`, `Drop`, v.v.).
  - `alloc`: `Box`, `Vec`, `String` hoàn chỉnh (`String` sử dụng `str` / UTF-8 invariant).

---

## Phase 2 — Self-hosting Prerequisites (Hệ sinh thái OS)
*Mục tiêu: Trình biên dịch Mellis cần thao tác file, path, build process. Các tính năng này trở thành bắt buộc cho v1.0.*

- **2.1 `std::env` (Môi trường)**
  - `var`, `var_or`, `has`, `set_var`, `remove_var`.
  - `vars` (Iterator/Collection toàn bộ biến môi trường).
  - `current_dir`, `set_current_dir`, `args`.
- **2.2 `std::process` (Tiến trình)**
  - Hỗ trợ spawn, run, status, exit để build tools có thể gọi `llvm`, `linker`, `assembler`.
- **2.3 `std::fs` (Hệ thống tệp)**
  - Thao tác: `read_file`, `write_file`, `exists`, `create_dir`, `remove`, `rename`, `metadata`.
- **2.4 `std::path` (Đường dẫn)**
  - Abstraction `Path`, `join`, `parent`, `file_name`, `extension`, `normalize`.
- **2.5 Deterministic MLib Loading**: Tải module mượt mà và chính xác từ file system.

---

## Phase 3 — Mellis Compiler Rewrite (Viết lại bằng Mellis)
*Mục tiêu: Chuyển đổi toàn bộ source code C++ sang Mellis (`.ms`).*

- `lexer.ms`
- `parser.ms`
- `AST/` (khai báo các struct và enum)
- `resolver.ms`
- `typechecker.ms`
- `MVIR/` (generation và lowering)
- `llvm_backend.ms` (hoặc giao tiếp FFI với LLVM C API)

---

## Phase 4 — Bootstrap (The v1.0 Gate)
*Mục tiêu: Hoàn tất chu trình Self-Hosting và xác thực.*

1. **Stage 0 Build**: C++ compiler `mellis-stage0` biên dịch toàn bộ mã nguồn ở Phase 3.
2. **Stage 1 Generation**: Sinh ra `mellis-stage1` (trình biên dịch viết bằng Mellis, chạy dưới dạng mã máy).
3. **Stage 2 Generation**: Dùng `mellis-stage1` biên dịch lại chính mã nguồn ở Phase 3 để sinh ra `mellis-stage2`.
4. **Verification**: Đảm bảo `mellis-stage1 == mellis-stage2` (đạt mức identity compiler).

🚀 **Mellis v1.0 Public Release** chính thức được phát hành với cấu trúc:
- Trình biên dịch 100% tự chủ (Self-hosted).
- Công cụ CLI đi kèm: `mellis build`, `mellis run`, `mellis test`.
- Package chuẩn `std` bao gồm đầy đủ `fs`, `path`, `env`, `process`.
