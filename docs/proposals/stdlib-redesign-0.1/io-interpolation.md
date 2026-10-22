<!-- luna-doc-role: guidance -->

# Nội suy $tenbien cho I/O — contract thiết kế

Ngày 2026-10-23. **Cú pháp $tenbien được maintainer chọn; implementation chưa có.**
Grammar/semantics chi tiết dưới đây là proposal cần kiểm chứng, chưa tự thay
[macro contract 0.1](../../spec/0.1/macros.md). Không sửa compiler trong đợt thiết kế.

## Surface

```luna
dec name = "Luna";
dec count: usize = 4;
std::println!("Hello $name, count=$count");
```

Template là normal string literal ở macro có capability opt-in. Nội suy chỉ
ở context đó; `dec text = "$name";` vẫn là ordinary str chứa chữ `$name`.
Không thay `str` thành String, không cấp phát ngầm cho mọi literal.

| Nội dung template | Nghĩa đề xuất |
| --- | --- |
| `$name` | Lookup identifier name theo scope/hygiene của literal; format giá trị bằng Display |
| `$$` | Một ký tự `$` literal |
| `$$name` | Text `$name`, không lookup name |
| `$$$name` | Dấu `$` rồi giá trị name |
| `{}` hoặc `{name}` | Text bình thường, không placeholder |
| `$` cuối literal hoặc `$5` | Diagnostic tại vị trí dấu $, dùng $$ nếu muốn dollar literal |
| `${expression}` | Chưa hỗ trợ; diagnostic, không âm thầm đoán grammar |
| `$object.field` | Nội suy object rồi text `.field`; không ngầm nhận field expression |
| Raw string literal | Literal hoàn toàn, không nội suy, vẫn có thể output chữ $name |

Identifier dùng đúng lexer Luna, kể cả quy tắc chữ số sau ký tự đầu; không tạo
regex identifier khác. `$name_suffix` là một tên đầy đủ, không đoán thành name.
Muốn format field/kết quả hàm, tính một local trước:

```luna
dec total = compute_total();
std::println!("total=$total");
```

Không hỗ trợ `$name:precision`, named argument, dynamic format spec, Debug hay
expression interpolation trong đợt đầu. Không thêm backslash escape `\$` khi
lexer hiện hành chưa có contract đó. Escape thông thường của string vẫn giữ
nghĩa và source span theo lexer.

`std::println!(template_variable)` không tự tìm biến trong text ở runtime.
Template phải là literal ở site còn giữ scope/source metadata; runtime str
được ghi bằng write_str, owned String bằng write_text. Không eval code từ input.
Macro nhận literal từ caller phải giữ nguồn gốc/hygiene của literal qua forwarding.
Literal được viết trong body một macro giữ lexical origin ở definition; không
vô tình capture một biến trùng tên ở caller.

## Evaluation, ownership và failure

1. Writer expression của write!/writeln! được đánh giá đúng một lần trước các
   giá trị trong template; borrow target tồn tại trong lời gọi.
2. Text/placeholder đi từ trái sang phải. Mỗi occurrence gọi Display một lần;
   tên lặp hai lần được format hai lần, không yêu cầu Clone/Copy.
3. Placeholder mượn binding bằng &T, không move String hay giá trị non-Copy.
   Loan không sống quá lời gọi; không giữ reference trong một packet thoát scope.
4. Name resolution và Display bounds được kiểm khi compile, kể cả artifact-only.
   Undefined/private/missing-Display là lỗi trước khi có output.
5. Console macro render staging trước lock/write như [I/O design](io-api.md):
   formatting/OOM error không xuất prefix. write!/writeln! lên writer của caller
   có thể đã xuất prefix khi sink/formatter lỗi; không hứa rollback.
6. Khi sink/formatter thất bại, dừng format occurrence phía sau; không gọi tiếp
   Display hoặc ghi newline. Những biến đã được caller tính trước macro không
   được hoàn tác side effect.
7. println!/writeln! thêm một LF sau khi render template thành công; không đoán
   stream/newline từ tên macro trong compiler.

Không có implicit conversion str -> String, Vec<u8> -> text, hay arbitrary T ->
Display. Unicode literal/str phải được decode đúng; String giữ invariant UTF-8.
Nội suy không tự thêm float formatting hoặc một format spec language.

## Compiler boundary và prerequisite

Machinery declarative macro hiện có chỉ capture literal như một expression/token;
nó chưa cung cấp capability generic biến `$name` bên trong text thành một
reference có source span và lexical scope. Không được sửa bằng nhánh
`if macro_name == println` hoặc thay regex string thành lời gọi std::io.

Đề xuất capability **opt-in literal segmentation cho macro**, độc lập tên library:

- Input giữ raw/cooked literal cùng span map và lexical origin.
- Opt-in yêu cầu structured segments Text / NameReference; raw literal không
  sinh NameReference. Ordinary string expression không tự opt-in.
- Identifier là syntax có hygiene/scope thật, không phải lookup runtime theo chuỗi.
- Macro/library transcriber quyết định sink, Display dispatch, Result và LF.
- Không cần compiler biết std::String, std::fmt::Display hay stdout.
- Cùng capability phải chạy được cho một user-defined macro với tên khác, sink
  không I/O và trait/callback bình thường do library định nghĩa.

Exact macro fragment spelling/API opt-in **chưa được chọn**. Cần grammar RFC và
reproducer trước implementation; không tự thêm một keyword, intrinsic,
`#[format_macro]` hay hardcoded library callee vào grammar/skill hiện hành.
Đây là feature mới (LANGUAGE-BLOCKED), không mặc định là C-GAP sửa bug.

Source .ln và macro từ .llib phải giữ nguyên segments, literal origin, visibility,
trait bounds và borrow semantics. Nếu representation/metadata đổi, bump version
đúng phần và từ chối artifact cũ; không dùng file tồn tại để quyết định hợp lệ.

## Acceptance trước khi gọi là supported

| Nhóm | Cases |
| --- | --- |
| Text/escape | Rỗng, Unicode, $$, $$$name, braces literal, raw literal, cooked escape/spans |
| Lookup | Local, parameter, shadowing, qualified/imported macro, forwarding literal, definition-site literal |
| Reject | Unknown/private name, $ cuối chuỗi, $5, ${expr}, dynamic template, missing Display |
| Ownership | Repeated non-Copy name, move/use sau gọi, borrow không escape, target expression chỉ một lần |
| Ordering | User Display ghi observable counters theo thứ tự, failing sink không format phần sau |
| Generic boundary | Macro user-defined có tên/sink/protocol khác, không cần import io hay biết stdout |
| Artifact | Fresh source-only/artifact-only, relocated user provider, stale macro metadata reject |
| Platform | Exact UTF-8 stdout/stderr/file bytes, LF, error Result và staging/flush policy |

Các ví dụ trong tài liệu là design examples. Không thêm chúng vào executable
quickstart trước khi generic capability và I/O acceptance thật sự PASS.
