# Kế hoạch hoàn thiện Luna 0.1-alpha.1 — 2026-10-04

Đối chiếu bản review do maintainer cung cấp với branch
`codex/antigravity-repair-0.1`, HEAD
`c8d0559500e78ef85c96ac842e824e639917cbad`; merge-base với main là
`97d5e8402cefca4cd7f99ffb031a5917d73acb1c`. Working tree sạch trước khi viết
kế hoạch. Bản review kết thúc ở `4a5adf4`; HEAD hiện tại thêm amendment
relative-only của `luna.toml`.

Đây là kế hoạch và kết quả đối chiếu source/evidence, không phải nghiệm thu
compiler. Lượt này không chạy lại workspace hoặc tạo reproducer thực thi mới,
không sửa code, không merge/tag, không tự adopt một quyết định ngôn ngữ mới.
Các log cũ giữ nguyên ranh giới revision. Mục tiêu release là **0.1-alpha.1**
theo [baseline 0.1](../../spec/0.1/README.md), không phải tuyên bố stable 0.1.

## 1. Kết luận và phạm vi

Giữ hai feature đã được maintainer yêu cầu: NAMESPACE-USING-v1 và
PROVIDER-CONFIG-v1, gồm relative-only file values. Không chuyển chúng sang 0.2
chỉ vì là feature mới. Chúng đã có ma trận CLI trong phạm vi Windows GNU;
cần giữ các regression này khi sửa core. Dừng bổ sung feature sau đây:
re-export, package tooling, config merging/local overrides, syntax mới,
const generics và optimizer mới. Refactor pipeline lớn không phải điều kiện
release độc lập; chỉ thực hiện phần tối thiểu nếu root cause đòi hỏi.

Chưa đủ bằng chứng merge toàn bộ nhánh như một candidate đã nghiệm thu, và
chưa đủ để tag. Ưu tiên soundness, crash và artifact correctness; sau đó đóng
contract/diagnostics/provider coverage và lấy regression cuối cùng. Không
đánh dấu lỗi an toàn là deferred để có bảng xanh. Contract đã adopt vẫn được
giữ như maintainer yêu cầu; muốn đổi phạm vi phải có quyết định riêng.

## 2. Nhận xét nào còn đúng tại HEAD

| Nhận xét trong review | Đối chiếu hiện tại | Hệ quả cho kế hoạch |
|---|---|---|
| Review branch đến `4a5adf4` | Mốc đó đúng cho review; HEAD hiện là `c8d0559` | Mọi nghiệm thu cuối phải gắn SHA mới, không dùng SHA review làm release evidence |
| Các repair core/macro/slice/iterator đã cải thiện thực chất | Các thay đổi và regression có trong branch; không cần giao lại toàn bộ repair cũ | Giữ regression; đóng các counterexample còn mở, không làm lại feature đã hoàn thành |
| `GenericParam => false` trong `needs_drop` | Vẫn tồn tại cùng TODO ở semantic `lib.rs` | Rủi ro cần kiểm chứng ưu tiên P0; riêng dòng này chưa chứng minh mọi instance runtime mất drop |
| Generic drop không có đường xử lý concrete | Không đúng nếu suy diễn như vậy: mono có `discover_drop_obligations`, substitution và generator đọc `instance.symbol_types` | Audit cả trước và sau mono, cache, move/Copy checks, cleanup; không sửa bằng một dòng `false => true` |
| Ledger ghi nhầm `fix_commit` | Đúng: `d950f7f` sửa macro và proposal, không sửa resolver/entry bridge; `eb72083` sửa compound assignment | Tách implementation provenance và verification provenance; không đoán commit sửa gốc |
| Method collision chưa adopt | Đúng: semantics/gaps còn decision required; proposal vẫn historical | Cần quyết định normative và acceptance cho toàn bộ collision policy |
| Method collision chỉ thiếu tài liệu | Chưa đủ: source vẫn chọn candidate đầu sau sort theo ID; các nhánh local/external tìm riêng, một nhánh còn dùng expected return | Audit/fix resolution sau khi adopt; deterministic order không đồng nghĩa ambiguity đúng |
| Cân nhắc đưa using/config sang 0.2 | Maintainer đã yêu cầu chúng cho alpha và đã triển khai | Giữ trong 0.1; không mở rộng tiếp |
| Absolute config values được phép | Đã lỗi thời sau `c8d0559` | Relative-only đã được xác minh; không giao lại sửa này |
| Runtime POSIX chỉ có Windows evidence | Đã có native Ubuntu/macOS ABI và Ubuntu ASan PASS trên `4a5adf4`, cũng PASS trong snapshot `c8d0559` | Cập nhật W1 theo bằng chứng native, nhưng không suy thành toàn bộ compiler/stdlib trên POSIX đã pass |
| CI `4a5adf4` còn đang chạy | Đã kết thúc: runtime/ASan/formatting PASS; Whole-File I/O và full workspace FAIL | Ghi kết quả hoàn tất; full workspace không xanh |
| Whole-File I/O hardcode provider count 36 | Đúng: bốn assertion `36` vẫn có; path suite cũng giữ `35/36` | Sửa oracle theo manifest và provider set thực tế, không chỉ thay bằng hardcode `49` |
| Module commit đã sửa raw-storage assertion cũ | Không đúng ở source hiện tại: còn `E_RAW_STORAGE_ANCHOR_FIELD` và `E_RAW_STORAGE_ANCHOR_COPY` trong struct lifetime suite | Sửa kiểm tra typed identity và reason/span; kiểm tra fixture parse đúng trước khi xem rejection là semantic PASS |
| Ba driver regressions, native crash, borrowed parity còn mở | Báo cáo cuối không đóng chúng | Điều tra từng root cause; baseline `b330ee3` chỉ chứng minh chúng có trước module feature, không chứng minh có trước toàn bộ repair branch |
| Diagnostics và 49-provider certification còn partial | Ledger hiện vẫn PARTIAL; build 49/49 không chứng minh tất cả API đúng | Hai workstream riêng, giữ explicit contract/evidence map |
| Status/gaps khiến người đọc nhầm lịch sử với hiện tại | Có notice baseline và update nhưng bảng đầu vẫn phản ánh `3dac3ac` | Tách current status khỏi audit bất biến, kèm revision/target và evidence hiện tại |

Nguồn implementation trọng yếu:
[needs_drop](../../../luna-rs/crates/luna-semantic/src/lib.rs),
[mono](../../../luna-rs/crates/luna-semantic/src/mono.rs),
[MVIR generator](../../../luna-rs/crates/luna-mvir/src/generator.rs),
[method resolution](../../../luna-rs/crates/luna-semantic/src/typechecker.rs),
[whole-file tests](../../../luna-rs/crates/luna-driver/tests/whole_file_io_v1_acceptance_tests.rs),
[struct lifetime tests](../../../luna-rs/crates/luna-driver/tests/struct_lifetime_contract_acceptance_tests.rs).
Đây là đối chiếu vùng source liên quan, không phải line-by-line audit toàn repo.

## 3. Evidence hiện tại và giới hạn

- [Module audit](../alpha-modules-2026-10-03/README.md): 57 fixture files qua CLI
  source-only/fresh-artifact-only; cuối đợt có 48 driver regressions pass, 3 fail;
  storage/HashSet 19/19 sau rebuild canonical. Không có full final-tree PASS.
- [Relative-only audit](../provider-config-relative-2026-10-04/README.md): matrix
  hiện pass, workspace check pass; bounded CLI change không chạy lại full tests.
- [Repair ledger](../repair-0.1/status.json): W1, DIAG và W8 vẫn PARTIAL; các
  claim CERTIFIED trong provider matrix cần đối chiếu với counterexample mới.
- [CI snapshot](evidence/ci-snapshot.json), đọc lúc 2026-10-04 05:26:41 UTC:
  metadata run/job/step trực tiếp GitHub, không tải job logs trong lượt này.
  `4a5adf4`: hai workflow hoàn tất FAIL; ABI Ubuntu/macOS, ASan Ubuntu và
  formatting Ubuntu/macOS PASS; workspace Ubuntu và Whole-File I/O fail.
  `c8d0559`: ABI/ASan pass; Whole-File I/O cả hai OS fail; formatting Ubuntu
  pass, formatting macOS và hai workspace jobs còn chạy tại snapshot. Không
  suy nguyên nhân từng failure từ conclusion; cần đọc log khi thực thi.

CI run links:
[4a5adf4 platform](https://github.com/17thedevv/luna/actions/runs/37175356920),
[4a5adf4 formatting](https://github.com/17thedevv/luna/actions/runs/37175356940),
[c8d0559 platform](https://github.com/17thedevv/luna/actions/runs/37179483526),
[c8d0559 formatting](https://github.com/17thedevv/luna/actions/runs/37179483561).

Không dùng số fail của snapshot trước alias/cache correction làm số fail HEAD.
Không trừ các targeted PASS khỏi một lần workspace FAIL rồi đổi verdict.
Không sử dụng điểm đánh giá 8/10 hay số lượng commit làm tiêu chí nghiệm thu.

## 4. Thứ tự công việc và điều kiện đóng

### Bước 0 — tái lập baseline và ổn định harness

**R0-BASELINE / R0-HARNESS.** Ghi exact SHA, runtime hash, Rust/LLVM/target,
manifest và commands. Dùng checkout/build output riêng, `CARGO_TARGET_DIR`
riêng khi so baseline và candidate; không dùng chung cache build hai revision.
Build runtime và 49 providers qua đường chính thức. Tách source-only và
artifact-only roots; không fallback vào source. Các test tạo artifacts phải
làm trong thư mục thuộc test, không viết vào canonical sysroot dùng chung như
HT7 từng làm. CLI đang chạy và build thay thế executable Windows chạy tuần tự.

Đọc log CI đã hoàn tất; lập failure inventory theo test name, phase, target,
mode, root cause dự kiến và evidence. Sửa stale oracle nhỏ trước để negative
fixtures chạm đúng semantic phase: provider set lấy từ manifest; typed code
không dựa substring message cũ; copied sysroot phải giữ layout/trust đúng;
literal và punctuation phải đúng contract. Không tăng timeout để giấu vòng
lặp, không bỏ test, không đổi chương trình invalid thành expected success.

**Đóng khi:** mỗi failure có reproducer/command rõ ràng; stale assertions có
giải thích oracle cũ sai và replacement đo đúng invariant. Harness không gây
nhiễm artifact/canonical state. Bước này không được gọi là compiler đã sửa.

### Bước 1 — soundness và crash (ưu tiên cao nhất)

**R1-GENERIC-DROP — P0 audit, sửa khi có counterexample.** Trace:
typecheck generic → substitution → instantiated symbol/expr types → drop
obligations/glue → MVIR cleanup → borrowck move/drop elimination → backend.
Kiểm tra các call site `needs_drop` trước mono, Copy/Drop incompatibility,
partial move dưới owner có Drop, conditional initialization, overwrite và
cache lifecycle. `Unknown` không được trở thành bằng chứng `NoDrop`; nếu cần
model riêng cho generic analysis thì thiết kế nghĩa theo từng consumer.
Không trả `true` blanket cho GenericParam hoặc tắt borrow checking để chạy.

Ma trận CLI với user-defined tracked resource: generic consume/return/move,
nested struct/enum/tuple/array, branch/match, overwrite, early return, loop
exits, closure captures; thêm cancellation nếu generic đi vào future. Có
hai concrete instantiations, impl-/method-level substitutions và cross-provider
source/fresh-artifact. Positive chứng minh đúng số/lần drop; negatives reject
use-after-move, double ownership, Copy+Drop và partial moves bị cấm. Artifact
generic body được consume ở process khác; không phụ thuộc ID của session cũ.

**Đóng khi:** evidence chứng minh mọi đường trong matrix; nếu defect có thật,
generic fix + reproducer được giữ vĩnh viễn. Nếu dòng `false` có nghĩa an toàn
trong một phase, chứng minh giới hạn bằng invariant và test, không chỉ thêm
comment. Không có unresolved drop/type obligation xuống executable MVIR.

**R1-COLLECT-CRASH — P0 triage.** Reproduce
`stdlib_iterator_collect_acceptance_tests` case C11, trước hết trên Windows
GNU nơi có access violation `-1073741819`. Giảm thành `.ln` CLI; so source với
fresh artifacts; kiểm tra IR và resource counters. Debug stack/provenance,
enum layout, moved/initialized state và generated drop. Dùng instrumented
runtime/ASan trên target phù hợp khi giúp phân định nguyên nhân; runtime ASan
PASS hiện có không chứng minh toàn bộ generated Luna code đã được sanitize.
Không mặc định crash do GenericParam; ghi rõ có chung root cause hay không.

**Đóng khi:** original + reduced program chạy đúng output/exit/cleanup;
không còn native crash trên target tái hiện; regression và root-cause note.

**R1-BORROW-PARITY — P1.** Điều tra
`stdlib_option_result_borrow_acceptance_tests` source/artifact parity với
`LocalBorrowEscape`. So declared/inferred lifetime relation, loan origin,
metadata remap và consumer lifetime; giảm về user-defined generic enum/API.
Nếu fixture invalid, reject cả hai modes với đúng reason; nếu fixture valid,
sửa machinery generic để cả hai modes chạy được. Không đổi thành raw pointer,
bỏ relation hoặc nới escape checker để làm positive xanh.

**Đóng khi:** có positive/negative controls, đúng diagnostic origin và parity
trong source/canonical artifact modes; branch này không làm mất safety check.

### Bước 2 — artifacts, method policy và spec debt

**R2-ASSOC / R2-MIXED — P1.** Đóng từng test:
`test_associated_type_round_trip_canonical_llib` và
`test_mixed_source_artifact_graph_parity`. Inspect canonical trait/self/associated
type identity, serialization/injection, substitution và dependency graph.
Giữ impl-/trait-/method-level mappings; không persist session-local IDs.
Ma trận pure-source, pure-artifact, mixed, relocated root, alias != provider
identity, reversed imports, multiple concrete instances và diamond dependency.
Negative stale/corrupt/interface mismatch vẫn reject, không auto rebuild.

**Đóng khi:** hai original tests + public CLI reproducer pass; invalid
artifacts reject đúng reason; no source fallback trong artifact-only run.

**R2-FRESHNESS — P1 contract/implementation reconciliation.** Test còn mở:
`test_dependency_implementation_only_change_preserves_validity`. Phân biệt
interface identity, execution/body identity và source/target/compiler identity.
Private non-inlined body change có thể giữ consumer interface validity, trong
khi materialized generic/comptime/inlined bodies cần execution dependency.
Đây là phân loại cần chứng minh với contract đang adopt, không phải đề xuất
bỏ mọi execution fingerprint. Lập matrix: public signature, private runtime
body, generic body, comptime value/body, target/compiler/features, missing hoặc
corrupt sidecar. Quyết định chính sách nào chưa adopt phải ghi riêng trước
khi đổi validator/test; không mặc định code hiện tại hoặc test hiện tại đúng.

**Đóng khi:** norm và implementation đồng nhất, fingerprint invalidation
precise, ABI/code thiếu hoặc sai vẫn fail closed; test hiện tại được sửa hoặc
giữ trên căn cứ contract và controls cụ thể.

**R2-METHOD-POLICY — P1 decision rồi implementation.** Đề xuất Option A:
applicable inherent method ưu tiên; không có inherent thì nhiều applicable
trait methods dẫn tới ambiguity; qualification chọn trait rõ ràng. Không tự
adopt trong kế hoạch này. Chốt nghĩa applicable/accessible/in-scope, generic
bounds, receiver forms, expected return có được lọc candidate hay không và
khả năng qualification theo cú pháp Luna. Kiểm tra cả source-local candidates
lẫn imported metadata candidates trước khi chọn; không để trait local thắng
inherent imported vì hai storage được search riêng.

Sau quyết định: update normative semantics/gaps/grammar nếu cần; triển khai
candidate collection/ranking/ambiguity ở mọi method lookup path. Candidate
diagnostic dùng canonical identity, không chọn theo SymbolId/import order.
CLI tests inherent-vs-trait, trait-vs-trait, explicit qualification, missing
bounds/private methods, local/imported combinations, generics, receiver và
return context; đảo import/declaration order trong source/artifact modes.

**Đóng khi:** policy đã adopt, ambiguity reject đúng mã/related spans và mọi
lookup path/parity tuân thủ. C-GAP-01 deterministic PASS chỉ là subcase,
không tự đóng toàn bộ policy.

**R2-SPEC-DEBT.** Cùng bước này rà V01-DESIGN-03 runtime overflow và
V01-DESIGN-04 formal grammar: quyết định nghĩa signed/unsigned arithmetic,
shift/division/overflow, debug/release consistency theo Luna; không adopt LLVM
wrap mặc nhiên. Hoàn thiện/cross-check productions cho surface đã adopt,
không hợp thức syntax legacy chỉ vì parser accept. Nếu thiếu quyết định,
trình phương án cụ thể cho maintainer; chưa thay semantics khi chỉ được giao
lập kế hoạch. Các contract đã tồn tại vẫn là requirements.

### Bước 3 — diagnostics và 49-provider conformance

**R3-DIAG.** Giữ đủ DIAG-1..10: stable typed codes, source primary spans,
structured related labels, origin/phase trace, poison containment, deterministic
ordering/dedup và registry parity. Map mỗi requirement → production constructors
→ focused negative CLI fixture → evidence. Phân biệt lỗi source cần span với
I/O/internal errors không có source location; không tạo span giả. Audit
optional fields/constructor APIs để thiếu code/span bắt được ở đúng boundary.
Kiểm chứng parser/import/macro/type/borrow/mono/backend errors, UTF-8/tabs và
artifact-imported declaration diagnostics. Message text không là identity.

**Đóng khi:** toàn bộ adopted diagnostic contract có implementation/evidence;
registry và docs đồng bộ. Partial đẹp hơn trước không đổi thành full PASS.

**R3-PROVIDERS.** Lấy manifest hiện tại làm inventory; map mỗi public API hoặc
nhóm contract tương đương → Luna fixture → ownership/negative cases phù hợp →
source/artifact → target. Internal provider có dependency/provenance contract,
không phải yêu cầu public import. Ưu tiên resources/concurrency/file/net,
remaining collections và error/borrow APIs; re-evaluate các row CERTIFIED nếu
counterexample hiện tại thuộc phạm vi của chúng. Hoàn thiện Tier-2/3 và các
gaps Tier-1, không coi import/build smoke là semantic certification.

**R3-RETAINED-CONTRACTS.** Bao gồm acceptance hiện tại cho adopted lifetime,
FFI, dyn restrictions, closure escape, async cancellation/initialized state,
comptime containment/materialization, char/UTF-8, entrypoint và backend
fail-closed. Reuse suite/fixtures đã có, chỉ bổ sung khoảng hở có ý nghĩa.
Không thêm feature để sửa matrix và không biến historical FROZEN thành PASS.
Kiểm tra performance ở workload đã đo và timeout/root cause đã phát hiện;
không yêu cầu optimizer mới hoặc benchmark tất cả API trước alpha.

**Đóng khi:** contract map không có requirement đang BROKEN/PARTIAL bị che;
mọi supported provider/profile có coverage phù hợp. UNKNOWN ghi rõ, không
chuyển thành SUPPORTED qua số lượng providers build được.

### Bước 4 — provenance, status và tài liệu release

**R4-LEDGER.** Tách tối thiểu:
`implementation_commit` (commit sửa thực sự, nullable khi chưa truy được),
`verified_at_commit`, `contract_status`, `implementation_status`,
`evidence` (command/mode/target/log và result), `remaining_gaps`.
Case fix đã có trong merge-base phải ghi `preexisting_at_baseline` nếu chưa
truy được introducing commit; không gán re-verification commit làm fix.
Nếu nhiều commits thực hiện repair thì lưu cả danh sách, không ép một hash.
Audit old status.json/evidence giữ trong history; migration phải bảo toàn dữ
liệu gốc hoặc có change record, không sửa các dated log thành kết quả mới.

**R4-CURRENT-DOCS.** Đồng bộ Status.md, current gaps, spec chapters,
repair/provider ledger, README/quick start, limitations, runtime/artifact
compatibility và generated site. Current table có baseline → current → exact
evidence boundary; audit `3dac3ac` chuyển thành mục lịch sử rõ ràng. W1 cập
nhật native ABI evidence; không suy là POSIX stdlib full PASS. Giữ syntax
`using math;` / `using math as m;`, config relative-only và optional discovery;
không lẫn namespace/provider/package hay hứa package manager.

**Đóng khi:** mọi current claim trace được đến SHA/target/suite tương ứng;
validator docs pass; root status đọc được mà không cần hợp nhất nhiều audit.
Không tự chuyển BLOCKED sang READY trước gate cuối.

### Bước 5 — exact candidate, merge và tag

**R5-RELEASE-GATE.** Chọn immutable candidate SHA sau các sửa trên. Từ clean
checkout, build runtime/compiler, canonical sysroot, rồi chạy focused
regressions và workspace; module/config57-fixture matrix, formatting/Unicode,
new soundness/parity cases và docs cũng phải giữ xanh. Lưu exits của mọi
command; không che failure bằng pipeline logger. Triage run dùng
`--no-fail-fast`, release run phải exit0, không timeout/crash hay bỏ safety tests.
CI matrix chính thức ghi supported targets và limitations; Windows GNU có
native crash history nên bắt buộc lấy lại evidence Windows, không thay bằng
Ubuntu-only PASS. ABI/ASan/Whole-File I/O/formatting và workspace phải xanh
trên các targets/jobs được quảng bá. macOS native compiler claims cần coverage
tương ứng; không suy từ runtime ABI. Freestanding chưa có evidence không được
quảng bá conformance. Phạm vi target cuối cần maintainer chốt, không auto defer.

**Đóng khi:** zero known unresolved critical/high correctness gaps trong
adopted scope; diagnostics/provider/contract map hoàn tất; required CI/workspace
xanh trên exact candidate; clean checkout build/run và public quick start pass;
supported matrix, limitations và release notes nhất quán với evidence.

Merge chỉ sau gate này. Nếu merge tạo tree khác candidate hoặc thêm sửa,
build/regression lại theo thay đổi và lấy CI trên merge result trước tag.
Tag `0.1-alpha.1` trỏ exact verified release commit; packaging/runtime/sysroot
được build từ đó và kiểm tra trên môi trường sạch. Lưu `tested_commit` tách
khỏi commit lưu report để tránh self-referential SHA. Không cần force-push,
squash/rewrite lịch sử hay thay artifact identity policy để hoàn thành.

## 5. Quy tắc giao việc và nghiệm thu

Giao từng root-cause task theo thứ tự dependency: R0 → R1 → R2 → R3/R4 → R5.
R1 tasks có thể chung nguyên nhân nhưng phải đóng từng reproducer; không
chạy nhiều agent cùng sửa shared pipeline/sysroot mà thiếu phân công file.
Mỗi task có commit nhỏ, original failure, reduced reproducer, root cause,
contract justification, actual commands/results và remaining gaps. Không
nhận báo cáo chỉ ghi "23/23 PASS", cargo check hoặc số providers.

Một task chỉ PASS khi compiler/library đúng contract và evidence đúng layer.
Nếu phát hiện mới, thêm finding thay vì sửa expectation để hấp thụ lỗi.
Các quyết định method/overflow và scope thay đổi cần maintainer adopt; việc
debug/fix theo contract đã có không cần xin lại quyền cho từng thao tác thường.
Không commit/push/merge chỉ vì kế hoạch này được viết; lượt hiện tại chỉ tạo
tài liệu kế hoạch và snapshot evidence.

Kết quả mong đợi cuối: **READY FOR RELEASE REVIEW trên exact SHA**, sau đó
maintainer chốt version/tag. Kế hoạch này không thay quyền freeze hoặc
[release gates](../../spec/0.1/conformance.md).
