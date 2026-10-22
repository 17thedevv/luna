<!-- luna-doc-role: guidance -->

# Bộ lang item và compiler contract của Luna

Ngày 2026-10-23. **Thiết kế, chưa triển khai hoặc freeze bộ mới.** Maintainer yêu
cầu thiết kế bộ lang item đầy đủ sau API I/O. Tài liệu này bao phủ identity,
signature/shape, ownership/safety, bootstrap, artifact và acceptance; không biến
toàn bộ catalog thành điều kiện bắt buộc cho stable 0.1.0.

Authority hiện hành vẫn là [spec 0.1](../../spec/0.1/README.md) và các amendment
đã chốt. [Catalog máy đọc được](lang-items-catalog.json) phân biệt core target,
profile mở rộng và mục đã có registry. [Inventory hiện tại](lang-items-current.json)
được acquire tại 25c9e26d: **24 entry = 22 production + 2 test**; 22 literal
annotation trong stdlib; 8 bootstrap entry gồm 7 compiler-contract family và
1 controlled prelude; 19 binding trong bootstrap. Ba memory hook đến qua import
ptr/mem, không phải ba binding auto-visible còn thiếu.

Hai acquisition inventory phải byte-identical; source SHA-256 và Git blob SHA
được ghi riêng. Inventory được giữ nguyên byte qua Git để catalog tham chiếu đúng
file hash; static reference không chứng minh support hay conformance.

## 1. Phân loại đúng trước khi thêm item

Identity và operation là hai trục: một function có thể vừa dùng lang-item
identity, vừa là intrinsic. Sáu memory/wrapping function hiện có thuộc trường hợp
đó. Không ép chúng vào hai bảng không liên hệ với nhau.

| Nhóm | Khi nào compiler cần biết | Ví dụ |
| --- | --- | --- |
| Lang contract | Type/trait tham gia invariant hoặc desugaring ngôn ngữ | Drop, Copy, Iterator, Try |
| Lang member hook | Một member/variant/associated type có vai trò xác định | drop_fn, iterator_next, some, try_output |
| Intrinsic primitive | Operation không thể thay bằng arbitrary user body mà giữ semantics | drop_in_place, raw-slice construction, modular arithmetic |
| Compiler-native syntax/rule | Semantics có sẵn trong AST/type/IR, không cần một declaration thư viện | sizeof, alignof, cast, borrow, match, checked primitive arithmetic |
| Ordinary library protocol | Compiler không tự gọi hoặc suy luật từ tên trait | Display, Writer, Read, Write, Clone, Hash, Convert, Error |
| Runtime ABI slot | Cơ chế native với prototype/status đã version | allocation, panic/trap, startup, OS I/O |

Result/Infallible, Vec/String/Box, allocator wrappers, File, Matrix, Tensor và
Graph không nhận lang item. Option có identity vì iteration cần variant roles;
Result thực hiện Try như một ordinary enum. `?` không hardcode Result/Error.

Nội suy `$name` là generic macro/literal capability trong
[thiết kế I/O](io-interpolation.md), không là lang item của println, Display,
stdout hoặc một String formatter. Display/Writer tiếp tục là ordinary traits.

## 2. Core target: 26 identity

Core target giữ 22 key production hiện tại, bổ sung đúng 4 role chưa được tag:
iterator_next, into_iter_fn, try_output, try_residual. Hai associated-type role
cần thêm target kind AssociatedType vào resolver/registry; annotation đó **chưa
được compiler hiện tại hỗ trợ**. Không đổi Iterator sang associated types chỉ
để giống một ngôn ngữ khác.

| Family | Key | Target/owner | Signature/shape chính |
| --- | --- | --- | --- |
| Drop | drop | Trait | std::Drop, không generic parameter |
| Drop | drop_fn | Method của drop | drop(&rw Self) -> void, safe, không async/variadic |
| Copy | copy | Marker trait | Không method; eligibility theo move/field/Drop rules |
| Option | option | Enum | Option<T>, một type parameter |
| Option | some / none | Variant của option | Some(T) một payload; None không payload |
| Iterator | iterator | Trait | Iterator<Item>, một type parameter |
| Iterator | iterator_next (mới) | Method của iterator | next(&rw Self) -> Option<Item>, life_from(self) |
| IntoIterator | into_iterator | Trait | IntoIterator<Item, IntoIter>, hai type parameter |
| IntoIterator | into_iter_fn (mới) | Method của into_iterator | into_iter(Self) -> IntoIter; IntoIter implements Iterator<Item> |
| Try | try | Trait | Hai associated type Output và Residual |
| Try | try_output / try_residual (mới) | AssociatedType của try | Vai trò chuẩn hóa projection, không lookup bằng text Output/Residual |
| Try | try_from_output | Static method của try | from_output(Output) -> Self |
| Try | try_branch | Method của try | branch(Self) -> ControlFlow<Residual, Output> |
| Residual | from_residual | Trait | FromResidual<R> |
| Residual | from_residual_fn | Static method của from_residual | from_residual(R) -> Self |
| ControlFlow | control_flow | Enum | ControlFlow<B, C>, hai type parameter |
| ControlFlow | break / continue | Variant của control_flow | Break(B), Continue(C) |
| Raw memory | drop_in_place | Unsafe intrinsic function | <T>(*rw T) -> void |
| Raw memory | slice_from_raw_parts | Unsafe intrinsic function | <T>(*T, usize) -> &[T] |
| Raw memory | slice_from_raw_parts_mut | Unsafe intrinsic function | <T>(*rw T, usize) -> &rw [T] |
| Modular arithmetic | wrapping_add / wrapping_sub / wrapping_mul | Intrinsic function | <T>(T, T) -> T, integer-only, modulo 2^width |

Những key some/none/break/continue đang có được giữ để tránh rename identity
không cần thiết. Tên public canonical mặc định vẫn dưới std; binding theo role
là semantic identity đã resolve, không là phép so sánh short name trong compiler.

### Ownership và shape chi tiết

**Drop/Copy.** Drop chỉ chạy cho owned state còn initialized, chưa move; overwrite
drop giá trị cũ một lần. DropFn không trở thành một phương thức mà safe code có
thể gọi thủ công qua alias/callback để rồi compiler drop lần nữa. Method tên drop
trên một ordinary type không tự mang vai trò DropFn. Copy không cùng tồn tại với
Drop và không che giấu field không đủ điều kiện Copy. Clone vẫn explicit. Copy
của reference handle không cho phép bypass luật exclusive access của Luna;
không nhập quy tắc reference/Copy từ ngôn ngữ khác.

Explicit source call tới DropFn, kể cả qua function value/unsafe block, không
được bypass cleanup accounting. Hủy raw storage có API unsafe drop_in_place với
obligation riêng; không dùng tên drop để trao quyền compiler-only invocation.

**Option/ControlFlow.** Variant role gắn vào owner đã resolve, đúng generic binder
và payload. Không dùng ordinal 0/1, tên Some/None hoặc tên Break/Continue như
nghĩa ngôn ngữ. Reorder enum có thể đổi ABI/interface fingerprint: source rebuild
phải giữ behavior theo role và stale dependent phải reject, không reinterpret
artifact cũ. Không thay ABI enum thành một layout special-case.

**Iterator.** Giữ Item ở generic slot hiện tại; IntoIterator có Item/IntoIter.
Catalog ghi role của binder theo declaration identity, không theo tên biến T.
into_iter consume iterable đúng một lần; next mượn iterator độc quyền trong lời
gọi, trả owned Item hoặc borrowed Item có provenance hợp lệ. None kết thúc lần
for hiện tại; không tự buộc mọi ordinary Iterator thành fused iterator. Break,
return, lỗi và early exit drop iterator/remaining state đúng một lần. Không hứa
lending iterator/GAT bằng việc thêm một annotation.

**Try.** Operand được đánh giá một lần và consume vào branch. Continue trả Output;
Break chuyển đúng Residual sang FromResidual của return type. Không thêm E: Error,
implicit conversion mọi error, hoặc clone residual. Associated role là declaration
của đúng Try trait; các type khác có member tên Output không được dùng thay.
Metadata giữ các binder, impl substitution, return provenance và safety bình thường.

**Raw memory.** T phải có layout dùng được ở operation đó. drop_in_place nhận
địa chỉ valid/aligned, initialized/live T và chuyển obligation thành đã drop;
không tự deallocate storage. Slice count là usize, theo element, không theo byte.
Caller chứng minh readable/initialized bounds và mutable exclusivity khi cần.
Unsafe không tạo lifetime tùy ý; raw origin/owner anchor và safe reference loans
tiếp tục theo contract đã chốt. Không tự viết life_from(raw_pointer) như một
borrow root giả. Direct và callable-value paths phải cùng kiểm unsafe admission.

**Wrapping.** Chỉ integer, kể cả generic instance sau substitution. Float, char,
bool, pointer và nominal cùng layout integer không tự hợp lệ. Ordinary arithmetic
vẫn checked; lang item này không đổi + thành wrapping theo build mode. VM/native
phải dùng cùng width, signedness và modulo semantics.

## 3. Profile mở rộng, chưa được bật

Catalog có **97 identity đề xuất = 26 core + 71 optional**. Đây là số role trong
bản thiết kế, không là 97 feature đã hỗ trợ hoặc một release gate đếm số entry.

| Profile | Số identity | Vai trò |
| --- | --- | --- |
| core | 26 | Bộ vừa mô tả; gồm cả memory chỉ đăng ký khi import provider |
| operators | 56 | Arithmetic/bit/assignment/comparison/index/deref hooks |
| callable | 6 | Fn/FnMut/FnOnce và call hooks |
| async | 5 | Future/Poll và poll/ready/pending hooks |
| ranges | 2 | Constructor hooks cho range expression |
| concurrency | 2 | Send/Sync marker candidates, cần contract riêng |

Những profile này là **target protocol proposal**. Async/closure/range có native
machinery hiện hữu không có nghĩa các source lang-item protocol mới đã có. Không
thu hẹp semantics baseline vì profile mới còn thiếu implementation.

### Operators

- Binary arithmetic: Add/Sub/Mul/Div/Rem, mỗi trait có một hook tương ứng.
- Bitwise: BitAnd/BitOr/BitXor/Shl/Shr. Unary: Neg và BitNot cho `~`.
- Mười compound assignment trait riêng: AddAssign ... ShrAssign, nhận &rw Self,
  return void. Không tự biến a += b thành a = a + b với non-Copy ownership khác.
- Eq/eq hook dùng **std::Eq hiện có**, borrow hai Self -> bool. != gọi eq một
  lần rồi negate bool. Ord/cmp dùng **std::Ord hiện có**, borrow hai Self -> i32;
  relational operators dùng dấu âm/0/dương, không nhập một Ordering enum mới.
- Index<Idx, Elem>/IndexMut<Idx, Elem> trả &Elem/&rw Elem life_from(self).
  Mutable contract kế thừa shared contract cùng binder roles. Index không hợp
  lệ phải fail có kiểm soát; lookup vắng giá trị bình thường vẫn dùng get/Option.
- Deref<Target>/DerefMut<Target> trả borrow life_from(self). Đợt đầu là explicit
  dereference; recursive method auto-deref, implicit unsize và coercion mới để RFC
  riêng, có cycle/ambiguity rules. Không derive raw-to-safe permission từ tên Deref.

Operator trait binary có Rhs và Output generic roles; unary có Output. Tên type
và method canonical ghi trong JSON; compiler chọn impl bằng trait/hook identity.
Owned operands move như normal calls, borrowed operands giữ loan bình thường.
Không auto-clone để tránh move error. LHS place/index expression của assignment
được evaluate một lần, thứ tự/effects theo normal Luna evaluation contract.

Primitive path vẫn có checked arithmetic và IEEE float comparison hiện hành;
không route primitive ops trở lại library impl rồi recurse. Builtin char operator
restrictions giữ nguyên. &&/||/boolean !, ordinary assignment, address-of,
raw dereference và post-increment/decrement hiện hành không bị overload ngầm.
Overloaded post-inc/dec cần riêng contract old-value ownership, chưa có lang ID.

Eq/Ord hiện vẫn là ordinary library protocols khi profile operators chưa bật.
Float NaN không tự được gán một Ord total order. Không đổi tên và API comparator
hiện có chỉ để khớp catalog của Rust/C++.

### Callable

Đề xuất FnOnce<Args, Output>::call_once(Self, Args) -> Output;
FnMut::call_mut(&rw Self, Args) -> Output; Fn::call(&Self, Args) -> Output.
Fn kế thừa FnMut, FnMut kế thừa FnOnce với cùng binder roles. Args là tuple
argument pack do call syntax tạo, có arity/type chính xác. Empty tuple không
đồng nhất với void; không thêm variadic generic syntax như một workaround.

Consuming capture chỉ có FnOnce; call consume environment và captures đúng một
lần, never-called closure cũng drop captures. FnMut cần exclusive capability,
Fn chỉ cho shared access. FnOnce không có nghĩa callable phải chạy đúng một lần;
nó có thể không bao giờ chạy, nhưng không được gọi lại sau khi bị consume.

Generated closure impl và user-defined callable dùng cùng checked trait machinery.
Fn pointer/extern/unsafe signatures vẫn có identity và safety riêng: unsafe fn
không tự implement safe call contract. Tuple packing, named/default arguments và
opaque callbacks phải giữ thứ tự đánh giá và arity checks hiện hành.

Output không được mượn owned capture đã consume; borrowed captures giữ external
provenance. Lending output mượn trực tiếp environment theo từng invocation là
prerequisite lifetime/associated-type design riêng, không được hứa bởi 6 tag này.

### Async

Đề xuất std::Future<Output>, Future::poll(&rw Self, &std::task::Context) ->
std::Poll<Output>; Poll có Ready(Output) và Pending. Năm role là future,
future_poll, poll, poll_ready, poll_pending. Context/Waker/executor là ordinary
library types/policy. Compiler lấy context parameter type từ checked poll hook,
forward như normal value, không inspect fields hoặc construct Waker theo tên.
Context chỉ được mượn trong một poll; không giữ &Context trong state qua Pending.
Muốn giữ scheduling capability, library cung cấp owned Waker với ownership và
lifetime bình thường; compiler không tự clone một callback hoặc kéo dài loan.

Giữ baseline: future owns environment; move chuyển obligation; external loans
hợp lệ qua suspension; **self-referential future bị cấm**. &rw Self ở poll không
cho địa chỉ ổn định sau khi return, không lén thêm Pin/Unpin. Nếu muốn cho self
reference, phải có RFC immovability/provenance riêng trước khi đổi rule.

Pending không move output; Ready chuyển Output một lần và await không poll lại.
Poll sau completion của generated future phải deterministic failure, không đọc
uninitialized state hoặc double-drop. Cancellation dùng generic Drop của state
còn live, cascade child futures; không tạo future_drop lang hook song song Drop.
Executor lựa chọn scheduling; compiler không chọn runtime theo tên async type.

Hiện SemanticType::Future và nhận diện short type name Future là đường khác;
không biến trait Future thành replacement âm thầm cho source Future<T> cũ.
Migration cần source compatibility, callable ABI, environment ownership,
artifact/MVIR/VM/native parity trước khi bật profile.

### Range expressions

Hai candidate range_new/range_inclusive_new trỏ vào ordinary constructor có
hai endpoint cùng T, trả iterator/range value với IntoIterator proof. Step và
Range<T>/RangeInclusive<T> vẫn là library type/protocol, không có compiler biết
field current/end/exhausted hoặc một RangeU32 chuyên biệt. Constructor identity
đủ cho desugaring; không thêm range struct lang item chỉ để nhận biết layout.
Generic bodies loaded từ .llib phải được instantiate như normal calls. Inclusive
MAX không tính successor gây overflow; giữ contract terminal Option đã adopted.

### Concurrency candidates

send/sync là marker candidate **disabled**, chưa là bảo đảm thread safety hiện
tại. Trước khi đăng ký cần rule structural propagation, manual impl safety,
raw pointer/borrow exceptions, scoped-thread lifetimes và negative shared-state
tests. Copy không suy ra Send/Sync. Một annotation không sửa data race hoặc tạo
lifetime dài hơn. Sized hiện là compiler-native admission rule, không tự thêm
implicit Sized bound; Unpin không cần cho async baseline cấm self-reference.

## 4. Manifest, trust và validation

Một catalog declarative là SSOT sinh enum/key lookup, target validators,
bootstrap binding và diagnostic descriptions. Đây là target architecture;
file JSON proposal chưa được compiler load và chưa thay manifest Rust hiện tại.

Mỗi record bắt buộc có: stable key, role/target, owning contract, generic roles,
signature/receiver/safety, profile/required-when, trusted provider binding,
canonical identity, semantic version, ABI/parity implications và negative tests.
Provider ID, physical path, public namespace và lang identity là bốn trường riêng.

Pipeline đích:

1. Chọn toolchain/profile/manifest hợp lệ. Không scan file để tìm #[lang] rồi
   tự nạp; include path hoặc file tên stdlib không tự cho quyền định nghĩa hook.
2. Resolve provider/source/artifact bằng provenance và dependency identity bình
   thường. Trusted compiler/core context là admission privilege, không property
   suy từ tên module hoặc một annotation trong user file.
3. Resolve marker và symbol, kiểm target kind + đúng owner + generic binder roles.
4. Typecheck đầy đủ signature, unsafe/async/variadic/ABI và canonical lifetime.
   Đúng target kind thôi chưa đủ, hai declaration cùng machine layout không đủ.
5. Kiểm complete family, exclusivity, duplicate và profile membership trước lower.
   Member/variant rời owner hoặc missing required item là diagnostic có origin.
6. Resolve normal impl theo trait member identity và checked substitutions;
   frontend ghi semantic plan, mono/MVIR/backend chỉ consume plan đó.

Core default bảo toàn bootstrap hiện có, thêm role metadata của iterator/Try.
Memory hooks vẫn đến từ explicit ptr/mem; không auto-visible tất cả core.
Result provider chỉ đóng góp controlled std::OptionExt; option_flatten hiện cũng
là controlled visible helper, không phải lang item. Qualification/autoload là
hai việc khác nhau. Optional profile chỉ load khi được enable rõ; không làm
mọi chương trình phụ thuộc ops/future/thread/I/O.

Duplicate cùng key với cùng canonical declaration qua diamond import là
idempotent; khác canonical declaration phải reject và chỉ cả hai origin.
Không silently giữ entry đầu tiên. TestTrait/TestStruct chỉ tồn tại trong
test registry/profile, không thành supported production attributes.

Giữ stable string key làm semantic identity; enum ordinal chỉ là index nội bộ,
không là stable wire ID. Canonical public path mặc định xác định trong manifest,
không dùng short-name fallback. Trusted replacement profile trong test có thể
rename symbol/method/variant để chứng minh compiler dùng identity, không spelling.
Public no-core/replacement-toolchain CLI không được tự tạo bởi thiết kế này.

## 5. Source/artifact, ABI và VM

Source marker và canonical binding phải được giữ qua compile/export/import.
Nếu có fast index trong metadata, nó là dữ liệu derive từ contract/AST và phải
được validate; không là authority thứ hai. Không serialize session-local
SymbolId/DeclId/LifetimeId; dùng provider/declaration canonical identity cùng
dependency/interface fingerprint, owner/member role và canonical lifetime.

Import phải kiểm profile/schema/compiler/target identity trước khi publish hook.
Reorder role hoặc thêm tag không cho phép reinterpret object cũ. Signature,
receiver, return lifetime, safety hoặc enum layout thay đổi phải đổi interface
identity và reject stale dependent. Body-only change giữ interface vẫn relink.

Hiện registry/Interface có lang map nội bộ, nhưng metadata_decoder tạo lang_items
rỗng và bootstrap/AST có đường reconstruct riêng. Không tuyên bố metadata v9 đã
có một portable role table hoàn chỉnh. Thêm target kind/role schema phải có
artifact admission versioning phù hợp; không bump version chỉ vì viết proposal.
Không tự gán compiler 27 hay một runtime ABI mới trong đợt này.

Native và comptime VM dùng cùng resolved identities, checked shape, Copy/Drop
và integer semantics. Một ordinary trait tên Copy/Drop/Future không được thay
thế binding qua string comparison. VM không I/O/extern chỉ vì wrapper có tag.
Language/profile thiếu role phải có diagnostic sớm, không fallback sang layout,
symbol đầu tiên, ordinary function cùng tên hoặc một runtime extern tưởng tượng.

## 6. Những thứ chủ động không thêm vào lang registry

| Cơ chế | Quyết định |
| --- | --- |
| sizeof/alignof, raw pointer ops, explicit cast, char validity | Giữ native semantic primitive/rule; size_of wrapper là ordinary API |
| IntrinsicKind 11 variant hiện tại | Enum không chứng minh 11 public primitive; audit từng path, không tạo declaration chỉ để đủ enum |
| Type reflection queries | Compiler capability/RFC riêng nếu thiếu; không ordinary function đoán bằng type name |
| Allocation/deallocation/OOM, panic/trap | Runtime ABI + safe library policy; không tag allocator/Box/String tên cụ thể |
| Entry/main/args/startup | Entry contract và runtime bridge hiện hành; không thêm public process provider |
| Unsize/dyn vtable | Native Rule I/coercion metadata; không lén thêm CoerceUnsized/Unsize trait semantics |
| Lifetime/anchor/capture effects | Canonical semantic contracts; không là callbacks hoặc runtime user functions |
| Display/Writer/I/O, Clone/Default/Hash/Convert/Error | Ordinary traits, normal import/impl resolution |
| SliceIter và collection layout | Library implementation; không tạo lang item cho một iterator struct chỉ để giữ compiler coupling |
| &&/||/!, match/if/while/break/return, assignment | Native control/evaluation rules; không thành arbitrary eager method dispatch |

## 7. Các điểm review đã thấy trong implementation

[Inventory](lang-items-current.json) ghi source span và digest cho từng điểm;
đây là static observations, không tự gọi là defect đã reproduce:

- MVIR drop fallback tìm symbol short name Drop; VM Copy bound so short name.
- Type lowering có nhánh short name Future; async còn native state machine riêng.
- Iterator protocol chọn theo một required method; tag member cụ thể chưa có.
- Try projection lookup text Output/Residual; associated role marker chưa có.
- Bootstrap raw injection là một đường cần hợp nhất với full shape/duplicate
  admission, không chỉ giả định trusted path tương đương validation.
- Hai dummy test key vẫn nằm trong table cùng production entries.

Đối với vòng for trên array/slice: giữ đúng IntoIterator/Iterator semantics và
borrow contract; ordinary sequence impl có thể là dependency bootstrap có kiểm
soát. Không fix coupling bằng một SliceIter lang item. Việc bỏ native fast path
hoặc thay source impl cần independent acceptance, không nằm trong docs-only task.

## 8. Acceptance và thứ tự triển khai

| Gate | Yêu cầu |
| --- | --- |
| Inventory/schema | Unique keys, owner/profile closure, exact source annotations, no test key trong production manifest |
| Registration | Unknown/untrusted/wrong target/owner/generic arity/ABI/unsafe/lifetime reject; missing family và duplicate có đúng span |
| Identity controls | Fake Drop/Copy/Future, renamed trusted hooks, extra trait methods, reordered variants/methods không thay semantic role |
| Ownership | Non-Copy observable drop, partial move, overwrite, loop/break/return, never-called and one-shot closure |
| Try/iteration | User-defined impl, borrow-carrying Item/Residual, failed bound, wrong projection/variant, single evaluation |
| Memory/wrapping | usize 32/64 configured checks, real native target scope, unsafe callable paths, provenance/escape, width/sign/range negatives |
| Optional protocols | User-defined number/container/callable/future; no library-name branch, no auto-clone, invalid mutability/arity/cancellation controls |
| Artifact | Source-only/fresh artifact-only check/build/native; relocated provider, stale interface/compiler/schema rejection, diamond imports |
| VM/native | Same role and ownership semantics; no admitted backend UB or runtime escape, known unrelated comptime gaps kept separate |
| Release | Full Windows/Ubuntu workspace, canonical sysroot, Lunar/package/installer at final implementation pin |

Thứ tự: (1) central schema + validation/trust/test isolation; (2) core iterator
hooks/Try associated roles và bỏ name fallbacks với regression riêng;
(3) source/artifact/VM parity; (4) từng optional profile theo feature RFC;
(5) stable qualification. Native async/callable/range migration là thay đổi
ngôn ngữ/ABI riêng; không trộn vào structural provider relocation.

Không triển khai hook bằng code stdlib sai semantics rồi bổ sung compiler magic
để nó chạy. Bug phải có independent user-type reproducer trước C-GAP; feature
mới là feature proposal. Tài liệu này không claim 26 hoặc 97 identity đã PASS.

SKILL IMPACT: CORRECTION giới hạn vào inventory hiện hành của luna-lang-contracts:
bổ sung wrapping arithmetic và phân biệt memory explicit-import. Không đưa các
role/profile mới còn là proposal vào skill như supported/frozen contract.
Ví dụ marker được đổi sang Drop/DropFn đã có; Iterator giữ generic Item thực tế.
