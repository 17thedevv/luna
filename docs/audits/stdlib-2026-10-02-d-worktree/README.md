> **Historical D:/fdlang worktree snapshot.** Preserved during all-worktree integration on 2026-10-03. These records use their original checkout paths and runner output directories. They are not a fresh audit of the merged main. The extended current-checkout audit remains in the sibling stdlib-2026-10-02 directory.

# Audit Luna stdlib và các lỗi compiler — 2026-10-02

**Kết luận: BROKEN về tính đúng; PARTIAL về tối ưu và mức độ xác minh.** Build toàn bộ sysroot thành công, nhưng có lỗi tái hiện bằng executable ở API an toàn và lỗi compiler làm chương trình hợp lệ chạy sai. Test xanh hiện tại chưa đủ để kết luận stdlib hoạt động đúng trên toàn bộ miền dữ liệu/kiểu generic.

Phạm vi: 32 provider, 5.926 dòng Luna trong `libs/external`, compiler/ABI liên quan; HEAD `3dac3ac0e85204411bd80fac15b3294dd812e1be`. Kiểm tra trên Windows x86_64 GNU, LLVM 18, compiler debug freshly built và runtime CMake Release. Chỉ thêm báo cáo, script audit và fixture; không sửa implementation compiler, stdlib hoặc runtime.

**Bằng chứng và giới hạn kiểm tra**

| Hạng mục | Kết quả | Bằng chứng |
|---|---|---|
| Build compiler CLI | Thành công | Binary được build trong phiên audit |
| `luna build-sysroot` | 32/32 provider build thành công | [build-sysroot.log](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/build-sysroot.log) |
| Runtime ABI | 1/1 suite CTest pass, 123,10 giây | [runtime-tests.log](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/runtime-tests.log) |
| Regression Rust stdlib hiện có | 33/34 suite pass, 258 test trong các suite hoàn tất; 1 suite timeout | [existing-suites.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/existing-suites.json) |
| Fixture stdlib qua CLI | 58 fixture × 2 chế độ = 116 lượt; 109 đúng kỳ vọng, 7 lệch | [cli-regressions.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/cli-regressions.json) |
| Probe bổ sung | 22 fixture mới và 2 fixture Try có sẵn × 2 chế độ = 48 lượt | [probes.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/probes.json) |
| Dispatch biên dịch lặp lại | 2 chương trình × 8 lượt; mỗi chương trình 5 đúng, 3 stack overflow | [dispatch-repeat.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/dispatch-repeat.json) |
| Storage retry | HT5, HT6 timeout 120 giây/test; HT7 pass trong 106,19 giây | [storage-retry.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/storage-retry.json) |

Trong 116 lượt CLI có 52 lượt positive và 64 lượt negative. Bảy lượt lệch gồm 3 stack overflow thực tế và 4 lượt liên quan hai fixture cũ hỏng/lỗi thời, phân tích bên dưới. Các probe audit là chương trình mô tả hành vi đúng mong đợi; `run_exit=1` thường là bằng chứng lỗi, không phải lỗi của runner. Riêng `compiler_move_after_call.ln` là negative control và phải bị từ chối. `compiler_invalid_char.ln` là characterization của miền giá trị char, không tự quyết định semantics mới.

Hai sysroot kiểm tra CLI được tách độc lập: source-only chỉ có `.ln` và manifest; artifact-only chỉ có `.llib`, object và manifest. Artifact được build bằng đường chính thức trước khi kiểm tra. Các suite Rust chạy với bản sao sysroot riêng. Danh sách source, SHA-256 source/artifact, số lượng và kết quả được lưu trong [summary.json](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/summary.json).

Toàn bộ `cargo test -p luna-driver --tests` chưa hoàn tất: lần build rộng đầu tiên hết dung lượng ổ C. Đã chuyển target của 13 suite còn thiếu sang ổ D, build thành công và chạy riêng. Lỗi hết dung lượng là hạn chế môi trường, không được tính thành lỗi compiler. Chưa xác minh toàn bộ compiler integration tests, target khác, sanitizer, fuzzing, hoặc benchmark có nhiều mẫu thống kê.

**Các lỗi compiler đã xác nhận**

**C-GAP-01 — P1 — Chọn method không xác định, có thể tạo đệ quy vô hạn.** [typechecker.rs:5026](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-semantic/src/typechecker.rs:5026) duyệt tập impl rồi lấy ứng viên theo thứ tự gặp; fallback ở dòng 5085 cũng lấy ứng viên đầu tiên. Các bảng này là `HashMap` tại [semantic_tables.rs:163](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-semantic/src/semantic_tables.rs:163) và dòng 181. Việc chọn ứng viên chưa có thứ tự/giải quyết xung đột ổn định giữa inherent method và trait method cùng tên; nhánh không có expected return còn dừng vòng lặp trước khi bảo đảm có ứng viên phù hợp.

Probe [compiler_inherent_trait_dispatch.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/compiler_inherent_trait_dispatch.ln) chỉ dùng `Counter` và trait tự định nghĩa. Cùng một source, 8 lần compile sinh 5 executable trả 0 và 3 executable chết với `0xC00000FD` (stack overflow). [dispatch.ll:51](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/dispatch.ll:51) cho thấy trait forwarding gọi lại chính nó. Fixture HashMap owned iteration cũng có tỷ lệ quan sát 5/3; [map_owned.ll:914](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/map_owned.ll:914) thể hiện cùng kiểu đệ quy trong `IntoIterator::into_iter`.

Giai đoạn sai đầu tiên: semantic method resolution; backend phát đúng symbol đã bị chọn sai. Ảnh hưởng thực tế đến owned iteration của HashMap/HashSet. Một lượt pass ở source hay `.llib` không chứng minh tính đúng hoặc parity ổn định. Cần sửa machinery chọn method chung cho mọi kiểu, xác định quy tắc Luna về precedence/ambiguity và kiểm tra compilation lặp lại; không thêm nhánh riêng cho HashMap hoặc tên method stdlib.

**C-GAP-02 — P1 — Widening integer bỏ mất unsignedness.** [llvm_codegen.rs:1499](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-backend/src/llvm_codegen.rs:1499) dùng `build_int_cast` cho int→int. Với giá trị động `u8=255`, LLVM sinh `sext i8 ... to i64` tại [unsigned_cast.ll:31](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02/evidence/unsigned_cast.ll:31), kết quả là `18446744073709551615`, thay vì 255.

Tái hiện bằng [compiler_unsigned_cast.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/compiler_unsigned_cast.ln), [convert_unsigned_widen.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/convert_unsigned_widen.ln), và [format_unsigned_high_bit.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/format_unsigned_high_bit.ln), đều lỗi ở cả source-only và artifact-only. Probe formatting thực sự in `18446744073709551615` khi input là `255u8`. Nhánh constant cast tại dòng 1454 có đường riêng nên test chỉ dùng literal dễ bỏ sót lỗi. Stdlib Convert/Display là nạn nhân của backend; không nên chữa bằng mask riêng trong từng API. Phải chọn zero/sign extension từ kiểu nguồn và giữ signed widening đúng.

**C-GAP-03 — P2 — Entrypoint bỏ toàn bộ command-line arguments.** [llvm_codegen.rs:352](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-backend/src/llvm_codegen.rs:352) gọi user main bằng `slice_type.const_zero()` dù shim nhận `argc/argv`. [compiler_main_args.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/compiler_main_args.ln) quan sát `args.len()==0` ngay cả khi runner truyền `audit-argument`. Lỗi ở cả hai chế độ. Runtime ABI pass không bao phủ đường chuyển arguments tới Luna main. Cần nối shim với ABI arguments đã chuẩn hóa của runtime; không thêm namespace process mới.

**Các lỗi implementation stdlib đã xác nhận**

| ID / mức | Vị trí | Trigger và kết quả | Probe |
|---|---|---|---|
| S-01 / P1 | [vec.ln:39](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/vec.ln:39), grow/reserve ở 232/258 | `capacity * sizeof(T)`, `len + additional`, doubling không kiểm tra overflow. `Vec<u64>` với capacity `2^61` nhân thành 0 byte nhưng vẫn giữ capacity cực lớn. `reserve(u64::MAX)` sau một phần tử trả về bình thường vì tổng wrap về 0. | [vec_capacity_overflow.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/vec_capacity_overflow.ln), [vec_reserve_overflow.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/vec_reserve_overflow.ln) |
| S-02 / P2 | [raw_table.ln:45](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/raw_table.ln:45) | Round-up capacity lớn hơn `2^63` nhân đôi tới 0, vòng `while cap < capacity` không kết thúc. Public `hashmap_with_capacity(9223372036854775809)` timeout sau 10 giây ở cả hai chế độ. | [hashmap_capacity_overflow.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/hashmap_capacity_overflow.ln) |
| S-03 / P2 | [slice.ln:36](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/core/slice.ln:36), mutable ở 69, constructors 93/103 | Iterator dùng địa chỉ end để đếm phần tử. Với `sizeof(T)==0`, ptr=end dù slice chứa hai phần tử; shared và mutable iterator đều trả None ngay. | [slice_zst.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/slice_zst.ln), [slice_zst_mut.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/slice_zst_mut.ln) |
| S-04 / P1 | [string.ln:97](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/string.ln:97), push_char ở 197 | Safe cast tạo được `55296 as char`; encoder chấp nhận surrogate D800. `String.push_char` tạo byte không phải UTF-8 hợp lệ, rồi chính `string_from_bytes` từ chối byte của String đó. | [string_invalid_char.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/string_invalid_char.ln) |
| S-05 / P2 | [iter_adapters.ln:47](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/core/iter_adapters.ln:47), [iter_consumers.ln:114](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/core/iter_consumers.ln:114) | Filter và find truyền owned Item cho predicate rồi trả lại cùng Item. Với `Box<i32>`, compiler từ chối use-after-move bằng E3001 trong thân stdlib. | [filter_owned.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/filter_owned.ln), [find_owned.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/find_owned.ln) |
| S-06 / P1 | [vec.ln:410](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/vec.ln:410) | `dedup_i32` nằm trong `impl<T> Vec<T>` nhưng đọc `*i32`. Trên Vec<u64>, 1 và 4294967297 bị gộp vì cùng low 32 bits. | [vec_dedup_i32_generic.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/vec_dedup_i32_generic.ln) |
| S-07 / P2 | [iter_adapters.ln:457](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/core/iter_adapters.ln:457) | `Range<T: Step>` move `self.current` rồi dùng lại để step. Custom Counter có Drop, triển khai đầy đủ Step, bị E3001 tại dòng 458. | [range_owned_step.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/range_owned_step.ln) |

S-01 có hậu quả an toàn bộ nhớ: runtime nhận size đã wrap nên không thể phát hiện tích ban đầu; 0 byte trả về zero-size sentinel tại [memory.c:36](C:/Users/84387/.codex/worktrees/a250/fdlang/runtime/src/memory/memory.c:36). Vec vẫn cho phép ghi phần tử theo capacity giả. Probe chỉ quan sát metadata và tránh ghi vào bộ nhớ không đủ; heap corruption là hậu quả suy ra từ đường push, chưa cố tình thực thi. Cần checked arithmetic trước khi cấp phát và trước khi cập nhật capacity/len. RawTable cũng có các tích byte-size/load-factor chưa kiểm tra, cần xử lý cùng nguyên tắc, dù probe S-02 chỉ xác nhận vòng lặp round-up.

S-04 là vi phạm invariant String phải luôn hợp lệ UTF-8, được nêu trong thiết kế hiện hành và [formatting contract](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/docs/phase6_core_formatting_v1.md:25). Probe [compiler_invalid_char.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/compiler_invalid_char.ln) xác nhận compiler hiện cho phép integer→char ngoài miền Unicode scalar. Chưa đóng băng kết luận mọi cast này phải bị compiler từ chối: đây còn là khoảng hở contract giữa miền char và text sink. Với behavior hiện tại, safe encoder/sink phải bảo vệ UTF-8; nếu muốn char mang invariant scalar ở cấp ngôn ngữ thì cần sửa capability generic và xác định cả cast động/literal.

S-05/S-07 là sai hợp đồng generic trong thư viện; borrow checker đang từ chối đúng. Negative control độc lập [compiler_move_after_call.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/compiler_move_after_call.ln) cũng trả E3001. Không nên nới ownership để làm các hàm này pass. Filter/find cần predicate mượn Item hoặc giới hạn contract phù hợp; Range cần bước cập nhật không dùng lại giá trị đã move hoặc ràng buộc kiểu tương ứng. `Option.filter` đã dùng `fn(&T)` và là đối chiếu hữu ích.

S-06 ngoài việc cho kết quả sai còn có nguy cơ đọc vượt phần tử/vượt allocation hoặc sai alignment với kiểu nhỏ hơn i32; phần nguy cơ này được xác định từ cast con trỏ, chưa tái hiện crash. API nên chỉ nhận đúng Vec<i32>, hoặc bỏ helper đặc biệt và dùng dedup có Eq. Không chữa bằng compiler nhận diện tên Vec.

**Ranh giới kiến trúc — A-01 / P2**

[typechecker.rs:4824](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-semantic/src/typechecker.rs:4824) và dòng 4841 tìm struct bằng tên `SliceIter`/`SliceIterMut`, rồi tự dựng semantic struct hai pointer cho `.iter()`/`.iter_mut()` của slice. Đây là tri thức cụ thể về implementation stdlib, trái với boundary stdlib là consumer của language. Nó cũng buộc iterator giữ layout ptr/end, gây khó khăn khi sửa S-03 để đếm ZST theo số phần tử.

Đây là finding tĩnh đã xác nhận từ code; chưa có probe chứng minh một kiểu user cùng tên chiếm nhầm binding hoặc gây memory corruption. Cần chuyển sang machinery generic hoặc một language contract được định nghĩa rõ, với metadata/layout do provider cung cấp; không chỉ đổi chuỗi tên hay thêm nhánh ZST ở compiler.

**Đánh giá tối ưu: PARTIAL**

| Finding | Bằng chứng | Đề xuất cần đo/kiểm tra |
|---|---|---|
| O-01: hash phân bố thấp | Primitive hash chủ yếu là identity tại [hash.ln:17](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/core/hash.ln:17); RawTable dùng `hash & mask`, linear probing ở [raw_table.ln:102](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/raw_table.ln:102). 6.000 key `i*65536` so với `i`: source 0,1495/0,0297 giây; artifact 0,1314/0,0288 giây. | Pattern low bits trùng nhau tạo clustering, tổng insertion có thể O(n²). Đánh giá generic mixing/seed hoặc chiến lược probing, giữ Hash/Eq contract. |
| O-02: pipeline IR tối ưu còn mỏng | [driver lib.rs:571](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-driver/src/lib.rs:571) chỉ đăng ký constant folding và DCE. Backend tạo target machine với OptimizationLevel::Default; không thấy lời gọi chạy pipeline tối ưu LLVM IR trong code driver/backend. | Chưa có cơ sở khẳng định generic helpers được inline hay vòng byte-copy được vectorize. Benchmark generated code rồi thêm passes generic có kiểm chứng semantics. |
| O-03: cấp phát/copy dư khi đã biết độ dài | [string.ln:201](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/string.ln:201) push_str không reserve trước; [path.ln:19](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/path/path.ln:19) dựng Vec trung gian rồi validate/copy sang String; [file.ln:116](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/file/file.ln:116) read_file biết length nhưng Vec bắt đầu capacity 0. | Sau khi sửa overflow, reserve known length, giảm bản copy qua API giữ UTF-8 invariant. Không nhận trực tiếp runtime buffer vào Vec khi ABI storage chưa tương thích. |
| O-04: rehash không cần thiết | [raw_table.ln:147](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/libs/external/alloc/raw_table.ln:147) grow trước khi biết key đã tồn tại; overwrite/get_or_insert có thể cấp phát/rehash ở load threshold. clear chỉ xóa states/tombstones khi len>0. | Đo workload overwrite và remove-all→clear→insert; tránh rehash dư và làm rõ việc purge tombstones. Đây là quan sát tĩnh, chưa đo số lần alloc. |
| O-05: layout enum lớn | [llvm_codegen.rs:187](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/crates/luna-backend/src/llvm_codegen.rs:187) tối thiểu hai payload words; Option<u8> theo layout hiện tại chiếm 24 byte. Option/Result as_ref dùng payload offset 8. | Có dư địa footprint nhưng không đổi ABI/niche layout âm thầm; cần contract/metadata và source-artifact parity trước khi tối ưu. |

Thời gian O-01 chỉ là smoke comparison, mỗi cấu hình một mẫu, gồm startup process, compiler audit là debug; không phải benchmark thống kê và không chứng minh hệ số chậm cố định. Điểm tốt đã kiểm tra: Vec dùng tăng trưởng geometric; sort có heapsort fallback để tránh worst-case quicksort; RawTable cache hash và đã có rehash cùng capacity khi tombstones nhiều. Không có bằng chứng để cáo buộc tăng capacity vô hạn do tombstones trong workload thông thường.

**Ma trận bao phủ tất cả 32 provider**

PARTIAL nghĩa là có build/test/đọc implementation nhưng chưa đủ mọi contract, kiểu generic, miền biên và target; không đồng nghĩa đã phát hiện lỗi riêng trong mỗi provider. BROKEN chỉ dùng khi có lỗi runtime/compile cụ thể trong capability đó hoặc phụ thuộc compiler đã tái hiện.

| Nhóm provider | Số | Trạng thái | Bằng chứng / giới hạn |
|---|---:|---|---|
| lang: Drop, Option, Iterator, IntoIterator | 4 | PARTIAL | Autoload, visibility, move/drop/borrow regressions; C-GAP-01 ảnh hưởng conversion iteration |
| core: ptr, mem | 2 | PARTIAL | Ptr/mem acceptance pass; chưa chứng minh toàn miền raw-pointer preconditions |
| core: copy, clone, default, cmp | 4 | PARTIAL | Public surface, cmp/memory/ownership tests pass; không coi mọi primitive/generic combination đã phủ |
| core: hash | 1 | PARTIAL | Hash collections pass nhiều ca; C-GAP-02 ảnh hưởng widening, O-01 clustering |
| core: num, float | 2 | PARTIAL | Numerics, bounds, floating bit/NaN/conversion suites pass; target 64-bit hiện tại |
| core: convert | 1 | BROKEN | unsigned widening qua Convert sai do C-GAP-02 |
| core: fmt | 1 | BROKEN | Display 255u8 sai; baseline/custom writer/error propagation pass |
| core: result, try | 2 | PARTIAL | Borrow/conversion/combinator suites và generic ? chạy đúng ở cả hai chế độ |
| core: iter_adapters, iter_consumers | 2 | BROKEN | S-05, S-07; adapters/consumers regression hiện tại vẫn pass |
| core: slice | 1 | BROKEN | S-03 shared/mutable ZST; slice/index/borrow/sorting regression pass |
| alloc: global, box | 2 | PARTIAL | Runtime allocation ABI và Box ownership pass; chưa đo allocator toàn diện |
| alloc: vec | 1 | BROKEN | S-01, S-06; drop/owned iteration/depth/ownership tests pass |
| alloc: string | 1 | BROKEN | S-04; UTF-8 byte validation/truncate/clone/io tests pass |
| alloc: raw_table, hashmap, hashset | 3 | BROKEN | S-02, C-GAP-01; storage HT5/HT6 chưa hoàn tất xác minh |
| alloc: iter_collect | 1 | PARTIAL | Collect regression pass; contract tổng thể chịu lỗi iterator đầu vào/compiler |
| io: io | 1 | PARTIAL | CLI stdout/stderr đúng ở cả hai chế độ; chưa thử mọi lỗi platform I/O |
| file: file | 1 | PARTIAL | Binary/empty/8.193-byte file đọc-ghi-copy, error/NUL cases và parity pass |
| path: path | 1 | PARTIAL | Lexical slash semantics, Unicode và parity pass; không áp semantics Windows drive/UNC lên contract slash-only |

**Nợ test và điều chưa xác minh**

- [public_surface.ln:31](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/tests/luna/stdlib/phase3/public_surface.ln:31) khai báo `map/set` nhưng dùng `root_map/std_map/root_set/std_set` không tồn tại. Cả hai chế độ từ chối đúng; cần sửa fixture, không sửa compiler để nhận biến chưa khai báo.
- [reject_std_vec.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/luna-rs/tests/luna/stdlib/phase3/reject_std_vec.ln) đòi từ chối `std::Vec` dù đây là namespace canonical. Compiler chấp nhận đúng. Cần đổi/xóa expectation lỗi thời.
- Storage suite timeout 600 giây tại HT5 sau HT1–HT4 pass; retry riêng HT5/HT6 timeout 120 giây, HT7 pass. Không coi timeout là chứng minh RawTable growth hỏng. Public [hashmap_growth_control.ln](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02/hashmap_growth_control.ln) insert/get 20 phần tử qua resize chạy đúng ở cả hai chế độ. Chưa xác định nguyên nhân/phase của timeout trong đường driver nội bộ.
- Không chứng minh source/.llib parity toàn diện: các bug deterministic tái hiện ở cả hai, còn dispatch phụ thuộc thứ tự nội bộ. Cần repeat compilation và negative ownership cases sau sửa.
- Chưa chạy exhaustive arithmetic/UTF-8 corpora, memory sanitizer, platform khác, full compiler tests hay benchmark allocation/throughput. Các nhóm PARTIAL không được nâng thành COMPLETE từ số lượng test pass.

**Thứ tự xử lý đề nghị**

1. Sửa C-GAP-01 và C-GAP-02 bằng machinery generic; dùng user-defined reproducer, source/artifact và compile lặp để freeze regression.
2. Sửa S-01/S-06/S-04 vì ảnh hưởng an toàn bộ nhớ hoặc invariant safe API. Sửa arithmetic của RawTable cùng nguyên tắc và xử lý S-02.
3. Sửa C-GAP-03, ZST iterator và các contract generic Filter/find/Range. Kiểm tra ownership/drop negative cases, tránh làm borrow checker yếu đi.
4. Loại coupling SliceIter layout/name trong compiler; xây lại toàn bộ artifact bằng đường chính thức, chạy source/artifact parity và xử lý hai fixture lỗi thời.
5. Sau khi correctness ổn định, benchmark O-01–O-05 với nhiều mẫu, input distribution, allocation counts và generated IR/object; không đổi ABI chỉ để giảm kích thước.

**Tái chạy**

Các runner trong [thư mục audit](C:/Users/84387/.codex/worktrees/a250/fdlang/docs/audits/stdlib-2026-10-02) lưu JSON và log, dùng đường LLVM/MinGW/runtime của môi trường này. Chạy từ repository sau khi build CLI, runtime và sysroot:

```powershell
python docs/audits/stdlib-2026-10-02/run_cli_regressions.py
python docs/audits/stdlib-2026-10-02/run_probes.py
python docs/audits/stdlib-2026-10-02/run_probes.py luna-rs/tests/luna/language/try_codegen_canonical_std_try.ln luna-rs/tests/luna/language/try_codegen_generic_cross_module_std_try.ln
python docs/audits/stdlib-2026-10-02/repeat_dispatch.py
python docs/audits/stdlib-2026-10-02/run_existing_suites.py
python docs/audits/stdlib-2026-10-02/run_storage_retry.py
python docs/audits/stdlib-2026-10-02/summarize.py
```

`run_existing_suites.py` chạy các binary đã build và bỏ qua kết quả đã lưu; muốn retry phải chọn/xóa entry tương ứng trong JSON hoặc chạy test riêng. Fixture mới nằm trong [audit_2026_10_02](C:/Users/84387/.codex/worktrees/a250/fdlang/tests/luna/stdlib/audit_2026_10_02); chưa được nối vào CI. Chúng lưu trigger lâu dài để sửa lỗi tiếp theo, không phải tuyên bố regression hiện đã xanh.

**SKILL IMPACT: none — không sửa skill trong nhiệm vụ audit.** Có đề xuất cập nhật nhỏ: `luna-lang-contracts` còn liệt kê bốn contract, trong khi manifest/code hiện có thêm Copy và Try, cùng controlled prelude OptionExt. Báo cáo chỉ ghi độ lệch hiện hành; không tự thay đổi semantics hoặc trạng thái freeze trong skill.
