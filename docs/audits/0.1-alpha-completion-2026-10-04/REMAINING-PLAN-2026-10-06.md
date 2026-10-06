<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Kế hoạch cho phần công việc còn lại. Baseline và
> adopted amendment vẫn là authority; tài liệu này không phải nghiệm thu
> compiler, không tự adopt quyết định ngôn ngữ mới và không tuyên bố release.

# Kế hoạch hoàn thành phần còn lại — Luna 0.1-alpha.1

Lập ngày 2026-10-06, đối chiếu [kế hoạch gốc 2026-10-04](README.md), [ledger thực thi](EXECUTION.md),
các gate chuyên đề ([CAST-GATE](CAST-GATE.md), [CALL-ARGUMENTS-GATE](CALL-ARGUMENTS-GATE.md),
[MEMORY-CALLABLES](MEMORY-CALLABLES.md), [MONO-CALLEE-ROLE](MONO-CALLEE-ROLE.md),
[RAW-SLICE-VIEWS](RAW-SLICE-VIEWS.md)), [gap register](../../spec/0.1/gaps.md),
[conformance gates](../../spec/0.1/conformance.md) và [repair ledger](../repair-0.1/README.md).

Mốc ghi nhận lúc lập kế hoạch: branch `codex/antigravity-repair-0.1`, HEAD `3601d12e`.
Đây là kế hoạch, không phải bằng chứng; mọi số liệu phải được chạy lại trên exact SHA
trước khi dùng làm nghiệm thu.

Micro-backlog thực thi (task ID, DoD template, execution sequence ~250–300 task) ở
[MICRO-BACKLOG-2026-10-06.md](MICRO-BACKLOG-2026-10-06.md).

## 1. Trạng thái nền

- Artifact/representation hiện tại: `LLIB_COMPILER_VERSION = 15`, `SEMANTIC_METADATA_VERSION = 7`,
  `MVIR_VERSION = 4`, `FORMAT_VERSION = 2`; 49 provider; runtime Windows x86_64 GNU.
- Verdict workspace đầy đủ gần nhất được pin (e3b4402): **FAIL exit101 — 1311 pass / 3 fail /
  1 ignored / 4 target fail**. CI Ubuntu e3b4402 cũng fail. Lỗi test dai dẳng và xác định
  duy nhất còn lại là `generic_drop_cli::closure_capture` exit 3 ở cả source lẫn artifact mode
  (owned capture không được destroy). Các verdict cũ hơn (9e45ca9 1297/2, de9977d 1290/1,
  39a2a9b 1262/34, 20f682f…) giữ nguyên ranh giới revision.
- R0 gần xong; R1 đóng được nhiều reducer nhưng còn vài đường chưa chứng minh; R2 phần lớn
  triển khai nhưng chưa đóng; R3, R4, R5 chưa hoàn tất.
- Nguyên tắc bất biến: không trừ focused PASS khỏi một lần workspace FAIL; mỗi thay đổi
  representation phải bump version và rebuild canonical sysroot qua đường chính thức; không
  sửa expectation để hấp thụ lỗi; decision method/overflow/target phải được maintainer adopt.

## 2. Quyết định maintainer cần chốt (blocking)

- **D1 — Closure callable policy.** Hành vi khi một closure gọi lại sau khi đã consume một
  owned capture: cho phép, reject (E-code nào), hay fail runtime. Chặn A1/A2.
- **D2 — V01-DESIGN-03 runtime integer overflow.** Nghĩa signed/unsigned arithmetic,
  shift/division/overflow, và debug/release consistency; không mặc định LLVM wrap là normative.
  Chặn E1 và một phần D4.
- **D3 — Target matrix cuối cùng cho 0.1-alpha.1.** Windows GNU (bắt buộc vì có native-crash
  history), Ubuntu (+ASan), macOS native compiler?, freestanding có được quảng bá? Chặn G/R5.
- **D4 — Phạm vi.** Xác nhận giữ toàn bộ NAMESPACE-USING-v1, PROVIDER-CONFIG-v1,
  MODULE-CONST-STORAGE-v1, CALL-ARGUMENTS-v1 trong 0.1; phần nào (nếu có) chuyển 0.2.

## 3. Chi tiết từng phần công việc còn lại

Mỗi task dưới đây là một root-cause task độc lập, giao theo thứ tự dependency R0 → R1 → R2 →
R3/R4 → R5. Mỗi task phải có: immutable SHA, original failure, reduced `.ln` reproducer, root
cause, contract justification, commands/results và remaining gaps.

### 3.0 Bảng tổng hợp task còn lại

| ID | Mức | Trạng thái | Blocker | Bằng chứng hiện tại | Điều kiện đóng |
|---|---|---|---|---|---|
| R0-HARNESS | P1 | mở | – | [EXECUTION](EXECUTION.md) R0 | Oracle theo manifest; mọi failure có reproducer; không nhiễm canonical |
| A1 Closure env destruction | P0 | BROKEN | D1 | `generic_drop_cli` exit3 | Harness PASS ở source + artifact |
| A2 Async suspended cleanup | P1 | PARTIAL | – | `future_initial_cancel` | Conditional/projected suspension pass |
| A3 Partial aggregate cleanup | P1 | PARTIAL | – | `partial_aggregate_cleanup` | Enum-pattern/indexed/projected pass |
| A4 Cast + poison containment | P1 | PARTIAL | – | [CAST-GATE](CAST-GATE.md) | Poison không sinh secondary; imported-provider cast |
| A5 Raw slice/view | P1 | scoped PASS | – | [RAW-SLICE-VIEWS](RAW-SLICE-VIEWS.md) | CStr misuse probes + opaque coverage |
| A6 Unary logical Not | P2 | BROKEN | – | `boolean_not_observation.ln` | Contract + fixture |
| C1 CALL-ARGUMENTS-v1 | P1 | IMPLEMENTED_ON_SIDE_BRANCH | – | nhánh `codex/call-arguments-0.1` | Reconcile/audit/verify + version gate |
| C2 Per-impl-header metadata | P1 | audit mở | – | [EXECUTION](EXECUTION.md) | Fingerprint/stale/invariance controls |
| C3 Projection/fallback + 64-frame | P2 | OPEN | – | [EXECUTION](EXECUTION.md) | Graph sâu hơn pass, diagnostic rõ |
| C4 Comptime execution dependency | P2 | conservative | – | [EXECUTION](EXECUTION.md) | Matrix phân loại đầy đủ |
| C5 Freshness classification | P1 | mở | – | test freshness | Norm + test thống nhất |
| D1 DIAG-1..10 | P1 | PARTIAL | – | [repair ledger](../repair-0.1/README.md) | Full contract + registry parity |
| D2 Expression spans | P1 | BROKEN (một phần) | – | [CAST-GATE](CAST-GATE.md) | Span đúng cho 4 shape |
| D3 Providers W8 | P1 | PARTIAL | D3 | [provider_matrix](../repair-0.1/provider_matrix.md) | Tier-2/3 map đầy đủ |
| D4 Retained contracts | P1 | PARTIAL | D2 | [conformance](../../spec/0.1/conformance.md) | Map không requirement bị che |
| E1 Runtime overflow | P1 | decision | D2 | [gaps](../../spec/0.1/gaps.md) | Norm + fixture |
| E2 Formal grammar | P2 | mở | – | [gaps](../../spec/0.1/gaps.md) | Productions + reconcile |
| F R4 Ledger/Docs | P1 | chưa | – | [Status.md](../../../Status.md) | Ledger tách trường; docs đồng bộ |
| G R5 Release gate | – | chưa | D3 | full workspace FAIL | Clean checkout + CI xanh + tag |

### A. R0 — ổn định baseline và harness (đóng R0)

**R0-BASELINE.** Ghi exact SHA, runtime hash, compiler hash, Rust/LLVM version, target, manifest,
số provider và commands. Dùng `CARGO_TARGET_DIR` riêng khi so baseline/candidate. Build runtime
và 49 provider qua `luna build-sysroot`. Tách source-only và artifact-only root; không fallback source.

**R0-HARNESS.**
- Việc cần làm: (a) chuyển các oracle hardcode số provider (`36`, `35/36` trong Whole-File I/O và
  path suite) sang đọc từ manifest/provider set thực tế; (b) rà mọi oracle phiên bản metadata
  (v3/v5/v7) và protocol theo version hiện tại, giữ incompatible-version rejection; (c) rà lại
  `E_RAW_STORAGE_ANCHOR_FIELD`/`COPY` trong struct-lifetime suite — kiểm tra bằng typed identity +
  reason/span, và xác nhận fixture parse trước khi coi rejection là semantic PASS; (d) xác nhận
  không test nào ghi artifact vào canonical sysroot dùng chung (HT7/String parity đã tách); (e)
  kiểm soát temp (TEMP/TMP trên D), tránh StorageFull và child treo vô hạn.
- Điều kiện đóng: mỗi failure có reproducer/command rõ; stale assertion có giải thích oracle cũ sai
  và replacement đo đúng invariant; harness không gây nhiễm artifact/canonical state.
- Rủi ro: oracle drift lặp lại mỗi lần bump protocol — nên centralize hằng số version dùng chung.

### B. R1 — soundness, crash và ownership

**A1 — Closure environment destruction (P0, release blocker).**
- Hiện trạng: `generic_drop_cli::closure_capture` exit 3 ở cả hai mode. Fixture: owned capture
  trong closure không được gọi (`dec unused = move || { resources::consume(value); };`) không được
  destroy khi closure ra khỏi scope, nên counter không đạt 2.
- Root-cause area: sinh MVIR cleanup cho environment của closure; tương tác với move/drop accounting
  của capture; backend drop glue. Tham chiếu [MEMORY-CALLABLES](MEMORY-CALLABLES.md) và checkpoint
  closure trong [EXECUTION](EXECUTION.md).
- Việc cần làm: (1) sinh drop glue cho closure environment theo từng capture đã move (owned), chạy
  đúng thứ tự; (2) không tạo loan/provenance giả; (3) phụ thuộc D1 cho semantics gọi lại sau consume;
  (4) thêm positive control (unused owned capture bị destroy) và negative (double destroy / use-after-move).
- Điều kiện đóng: `cargo test -p luna-cli --test generic_drop_cli -- --nocapture` PASS toàn bộ;
  positives đếm đúng số lần drop; negatives giữ nguyên; chạy source + fresh artifact.
- Files dự kiến: `luna-semantic` (closure env/borrowck facts), `luna-mvir/src/generator.rs`,
  backend drop call.

**A2 — Async suspended-state cleanup.**
- Hiện trạng: initial/suspended cleanup đã có control (16/16 driver + `future_initial_cancel`), nhưng
  conditional/projected suspension state chưa được chứng minh.
- Việc cần làm: fixture future với suspend tại nhánh điều kiện và field projected; xác nhận cleanup
  đúng khi cancel ở trạng thái đã/ chưa init từng phần; quy về user-defined generic resource.
- Điều kiện đóng: positive + negative (E3001/E3005) trong source/artifact.

**A3 — Partial/aggregate cleanup còn mở.**
- Hiện trạng: struct/tuple partial cleanup, return transfer, conditional init đã có; enum-pattern
  partial cleanup, indexed moves, projected/partially-moved aggregate cleanup chưa chứng minh.
- Việc cần làm: mở rộng fixture nested tuple + enum-pattern + indexed move; giữ nguyên user destructor
  indivisible và cấm move field khỏi Drop owner.
- Điều kiện đóng: matrix positives đúng số drop + negatives (E3001) ở cả hai mode.

**A4 — Cast correctness + poison containment.**
- Hiện trạng: protocol-15 cast admission đã sửa 5 lỗi native (shared→mutable, nominal/privacy,
  unsafe callable erasure, referent reinterpret, raw→safe). Còn: private-member access sau poisoned
  cast sinh secondary diagnostic (DIAG-5 containment); coverage imported user-provider cast bodies
  và cross-target execution.
- Việc cần làm: đảm bảo poisoned Type trở thành `Error`/poison và không sinh chẩn đoán phụ; thêm
  fixture provider nhập khẩu chứa cast body; test relocated artifact.
- Điều kiện đóng: không còn secondary diagnostic sau poison; 5 reducer giữ reject; identity-cast
  giữ đúng một drop và E3001.

**A5 — Raw slice/view hoàn tất.**
- Hiện trạng: memory raw-slice regression đã pass, view/provenance CLI 12 native + 68 rejection.
  Còn: safe raw-pointer CStr construction misuse probes; coverage borrowed-header/mixed/opaque.
- Việc cần làm: probe misuse cho CStr safe raw constructors; mở rộng opaque callback/view return.
- Điều kiện đóng: positive + negative ở source/artifact; không nới safety để làm xanh.

**A6 — Unary logical Not.**
- Hiện trạng: `boolean_not_observation.ln` bị E2012; đây là gap contract coverage độc lập.
- Việc cần làm: quyết định contract theo syntax đã adopt, triển khai/reject đúng, thêm fixture.
- Điều kiện đóng: Not trên bool có contract + test; không phá các rejection hiện có.

### C. R2 — artifacts, method policy và feature đã adopt

**C1 — CALL-ARGUMENTS-v1 (named/default) — feature lớn nhất còn lại.**
- Hiện trạng: positional-arity prerequisite đã sửa (HEAD `3601d12e`); chưa có harness
  `call_arguments_cli.rs`. Contract: [call-arguments-v1](../../spec/0.1/call-arguments-v1.md);
  baseline 11 case trong [CALL-ARGUMENTS-GATE](CALL-ARGUMENTS-GATE.md).
- Việc cần làm theo 7 giai đoạn: (1) binding plan arg→ordinal, validate unknown/duplicate/
  positional-after-named, dùng trait signature cho trait call; (2) mang plan qua provider
  extract/inject, mono, MVIR — lower theo source order nhưng truyền theo parameter order, giữ
  effect order và move; (3) public param names + capabilities vào canonical interface, bump
  schema/compat epoch, reject stale-dependent, có control unchanged-body/binder-rename; (4) thêm
  AST default expr + relocation + lexical resolution trong defining scope (chỉ earlier parameters);
  (5) omission entry materialize đúng pattern, evaluate default theo declaration order, giữ
  full-arity ABI, không dùng zero/null stand-in; (6) portability qua relocated artifact-only
  provider + trait role map theo ordinal; (7) full CLI matrix + workspace candidate.
- Điều kiện đóng: named và default cùng đạt contract đầy đủ (không chỉ parser/literal); parity
  source/fresh-artifact; giữ evaluation order và ownership/escape negatives.
- Files dự kiến: `luna-parser` (label `=`, default expr), `luna-semantic` (binding plan, signature),
  `luna-mvir`/mono (lowering, omission entry), `luna-llib` (public metadata).

**C2 — Per-impl-header canonical metadata audit.**
- Hiện trạng: compiler9 đã giữ individual checked impl headers; còn nguy cơ grouping theo nominal
  head thay thế checked header; fingerprint công khai cần audit.
- Việc cần làm: audit identity/fingerprint theo từng impl header; đảm bảo thay đổi public bound
  invalidates dependent artifacts; binder/order/body invariance.
- Điều kiện đóng: reader/decoder retention invariants + stale rejection đúng; không phụ thuộc session ID.

**C3 — Associated projection/fallback + giới hạn bound prover.**
- Hiện trạng: broader associated projection/fallback domains chưa verify; bound prover có giới hạn
  64-frame báo là "unsatisfied".
- Việc cần làm: mở rộng domain lookup associated projection; audit diagnostic cho giới hạn 64-frame
  (đảm bảo graph hợp lệ sâu hơn không bị chặn sai).
- Điều kiện đóng: positive sâu hơn pass; diagnostic rõ ràng, deterministic.

**C4 — Comptime execution dependency precision.**
- Hiện trạng: comptime dependency selection còn conservative.
- Việc cần làm: làm chính xác hơn mà không invalidation quá mức; giữ fail-closed.
- Điều kiện đóng: matrix public signature/private body/generic body/comptime body/target/compiler.

**C5 — R2-FRESHNESS classification.**
- Hiện trạng: cần norm cho phân biệt interface identity vs execution/body identity vs target/compiler.
- Việc cần làm: chốt policy (private non-inlined body giữ validity; materialized generic/comptime/inlined
  cần execution dependency); matrix day đủ; chỉ đổi validator/test trên căn cứ contract.
- Điều kiện đóng: `test_dependency_implementation_only_change_preserves_validity` + controls pass.

### D. R3 — diagnostics và 49-provider conformance

**D1 — DIAG-1..10 đầy đủ.**
- Hiện trạng: V01-DIAG-01 PARTIAL; typed codes đã có một phần; còn optional code/span fields,
  structured related labels, constructor enforcement, registry parity.
- Việc cần làm: map mỗi requirement → production constructor → focused negative CLI fixture →
  evidence; poison containment; deterministic ordering/dedup; phân biệt lỗi source cần span với
  I/O/internal không có location (không tạo span giả).
- Điều kiện đóng: toàn bộ adopted diagnostic contract có implementation/evidence; registry + docs đồng bộ.

**D2 — Expression spans.**
- Hiện trạng (CAST-GATE): E2002 deref, E2012 negation, E2026 known-invalid char cast trỏ 1:1 thay
  vì offending expression; tuple projection E2003 thiếu primary location; scalar index là control đúng.
- Việc cần làm: sửa `get_expr_span_for_diag` bổ sung các shape còn thiếu; `TupleIndex` constructors
  mang span và trả poison Error; giữ code identity + poison containment; không dùng AST fallback file-start.
- Điều kiện đóng: `expression_diagnostic_spans_cli` phủ các shape trên với vị trí đúng.

**D3 — R3-PROVIDERS (W8 Tier-2/3).**
- Hiện trạng: Tier-1 Core/Alloc CERTIFIED; Tier-2/3 còn map trong [provider_matrix](../repair-0.1/provider_matrix.md).
- Việc cần làm: map public API/nhóm contract → Luna fixture → ownership/negative → source/artifact →
  target; ưu tiên resources/concurrency/file/net, collections còn lại, error/borrow API; re-evaluate
  row CERTIFIED nếu counterexample mới thuộc phạm vi.
- Điều kiện đóng: contract map không còn requirement BROKEN/PARTIAL bị che; UNKNOWN ghi rõ.

**D4 — R3-RETAINED-CONTRACTS.**
- Phạm vi: adopted lifetime, FFI, dyn restrictions, closure escape, async cancellation/init state,
  comptime containment/materialization, char/UTF-8, entrypoint, backend fail-closed.
- Việc cần làm: reuse suite/fixture đã có, chỉ bổ sung khoảng hở có nghĩa; kiểm tra performance ở
  workload đã đo, không thêm feature/optimizer mới.
- Điều kiện đóng: contract map không có requirement bị che; profile supported có coverage tương ứng.

### E. Spec debt

**E1 — V01-DESIGN-03 runtime overflow (theo D2).** Chốt nghĩa signed/unsigned arithmetic,
shift/division/overflow và debug/release consistency; cập nhật semantics + fixture; không adopt
LLVM wrap mặc nhiên.

**E2 — V01-DESIGN-04 formal grammar.** Hoàn thiện/cross-check productions cho surface đã adopt;
không hợp thức syntax legacy chỉ vì parser accept; reconcile V01-GRAMMAR-01/02/03 giữa gap register
và repair ledger (PASS ở [repair ledger](../repair-0.1/README.md) nhưng còn bị reproduce trong
[gaps](../../spec/0.1/gaps.md)) bằng một lần chạy CLI mới.

### F. R4 — provenance, status và tài liệu release

**R4-LEDGER.** Tách tối thiểu: `implementation_commit` (nullable), `verified_at_commit`,
`contract_status`, `implementation_status`, `evidence` (command/mode/target/log + result),
`remaining_gaps`. Case fix đã có trong merge-base ghi `preexisting_at_baseline`. Audit old
`status.json`/evidence giữ trong history; migration bảo toàn dữ liệu gốc.

**R4-CURRENT-DOCS.** Đồng bộ `Status.md`, current gaps, spec chapters, repair/provider ledger,
README/quick start, limitations, runtime/artifact compatibility và generated site. Bảng current có
baseline → current → exact evidence boundary; audit `3dac3ac` chuyển thành mục lịch sử. Giữ BLOCKED
tới gate cuối. Khôi phục hoặc sửa tham chiếu tới `docs/agent-handoff/antigravity-repair-plan-2026-10-03.md`
(đang được repair README tham chiếu nhưng không tồn tại).

### G. R5 — release gate

Chọn immutable candidate SHA; từ clean checkout build runtime/compiler/sysroot; chạy focused
regressions + workspace với `--no-fail-fast` để triage rồi release run phải exit0; module/config
57-fixture matrix, formatting/Unicode, soundness/parity mới và docs phải giữ xanh. Lưu exit mọi
command; không che failure bằng pipeline logger. CI matrix theo target đã quảng bá (D3): Windows
GNU bắt buộc lấy lại native evidence, Ubuntu ASan, macOS native compiler cần coverage tương ứng.
Lưu `tested_commit` tách khỏi commit report; tag `0.1-alpha.1` trỏ exact verified release commit.
Merge chỉ sau gate; nếu merge tạo tree khác thì chạy lại.

## 4. Thứ tự thực hiện và dependency

1. Chốt D1–D4 (maintainer).
2. **R0-BASELINE + R0-HARNESS** (A) — ổn định oracle để negative fixture chạm đúng semantic phase.
3. **R1 (B)**: A1 trước (test workspace fail duy nhất), song song A4/D2; rồi A2, A3, A5, A6.
4. **R2 (C)**: C1 sau khi pipeline ổn định; C2/C3/C4/C5 song song theo file (không cùng sửa shared pipeline).
5. **R3 (D)** song song với C khi đã có contract ổn định.
6. **E** chốt sau D2 (overflow) và trước F/G.
7. **R4 (F)** sau khi code ổn định.
8. **R5 (G)** cuối, trên exact SHA; tag chỉ sau khi pass.

## 5. Lệnh chuẩn và bảng truy vết

```text
# Build canonical sysroot (49 providers)
cargo run -p luna-cli -- build-sysroot

# CLI focused harnesses (ví dụ)
cargo test -p luna-cli --test generic_drop_cli -- --nocapture
cargo test -p luna-cli --test cast_contract_cli -- --nocapture
cargo test -p luna-cli --test method_resolution_cli -- --nocapture

# Triage / release
cargo test --workspace --no-fail-fast
python docs/tools/validate_v01_docs.py
python docs/tools/check_v01_examples.py
```

- Version cần đối chiếu mỗi lần bump: `luna-rs/crates/luna-llib/src/format.rs`
  (`LLIB_COMPILER_VERSION`, `SEMANTIC_METADATA_VERSION`, `MVIR_VERSION`, `FORMAT_VERSION`).
- Artifact rejection/rebuild là thao tác tường minh của build tool; không auto rebuild.

## 6. Quy tắc nghiệm thu

- Một task chỉ PASS khi compiler/library đúng contract và evidence đúng layer (CLI `.ln` ở public
  boundary; Rust test chỉ bổ sung). Không nhận báo cáo chỉ ghi "N/N PASS", `cargo check` hay số provider.
- Không đổi invalid thành expected success, không ignorer/xfail để có bảng xanh, không nới safety
  check. Khi phát hiện mới: thêm finding, không sửa expectation để hấp thụ lỗi.
- Mọi kết quả gắn SHA + target + mode; focused PASS không đổi verdict workspace cũ; verdict mới
  phải chạy trên exact candidate.
