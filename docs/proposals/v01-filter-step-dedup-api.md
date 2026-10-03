# Decision Note: Public Filter, Step, and Dedup API Contracts

Date: 2026-10-03
Status: PROPOSED / PENDING DECISION
Authors: Antigravity

---

## 1. Context & Scope

During the stdlib audit of 2026-10-02, three API-level ownership/typing findings were identified in collection and iterator contracts:
1. **S-05**: `Filter<I, Item>` (`core/iter_adapters.ln`) and `iter_find` (`core/iter_consumers.ln`) consume `Item` by value through their predicate (`fn(Item) -> bool`), causing borrowck rejection (E3001 Use of moved value) when used with non-Copy owned items (e.g. `Box<T>`).
2. **S-07**: `Range<T>` and `RangeInclusive<T>` (`core/iter_adapters.ln`) advance using the `Step` trait.
3. **S-06**: `Vec<T>.dedup_i32()` in `alloc/vec.ln` reinterpreted arbitrary element pointers as `*i32` when deduplicating.

Per Luna rules (Section 14), we must not arbitrarily add `Copy` bounds or drop APIs to force compilation. Instead, this document compares existing contracts with implementation signatures and outlines the architectural migration paths.

---

## 2. Finding S-05: `Filter` and `iter_find` Predicate Ownership

### Current Implementation Signatures
- `export struct Filter<I, Item> { ... private pred: fn(Item) -> bool }`
- `export fn iter_filter<I: std::Iterator<Item>, Item>(iter: I, pred: fn(Item) -> bool) -> Filter<I, Item>`
- `export fn iter_find<FindIter: std::Iterator<FindItem>, FindItem>(iter: FindIter, pred: fn(FindItem) -> bool) -> std::Option<FindItem>`

### The Semantic Problem
When `Item` is an owned, non-`Copy` type (such as `std::Box<i32>` or `std::Vec<u8>`):
1. `Filter::next` receives `opt = self.iter.next() -> Some(x)`.
2. Calling `pred(x)` consumes `x` by moving it into the predicate function.
3. If `pred(x)` returns `true`, `Filter::next` attempts to return `Option::Some(x)`.
4. The borrow checker correctly rejects this with `error[E3001]: Use of moved value 'x'`.
5. Independent negative control `compiler_move_after_call.ln` confirms that `E3001` is language-normative behavior.

### Comparison with Existing Working Precedent
In `lang/option.ln`:
```luna
export fn filter(self: Self, pred: fn(&T) -> bool) -> std::Option<T>
```
`Option::filter` takes `pred: fn(&T) -> bool`. It borrows `&x` to test the predicate, and yields `x` by value if true.

### Evaluation of Options

| Option | Signature | Pros | Cons |
|---|---|---|---|
| **A (Recommended): Borrowed Predicate** | `pred: fn(&Item) -> bool` | Clean ownership; identical to `Option::filter` and standard systems language conventions (`FnMut(&Item) -> bool`); supports non-Copy owned items. | Breaking change for code expecting `fn(Item) -> bool` on borrowed iterators (`Item = &T` becomes `&&T`). |
| **B: Constrain to Copy** | `impl<I: Iterator<Item>, Item: std::Copy> ...` | Preserves `fn(Item) -> bool` signature. | Violates generic collection requirements: impossible to filter vectors of owned strings, boxes, or nested containers. |
| **C: Dual Predicate Forms** | `iter_filter_ref` vs `iter_filter` | Non-breaking backward compatibility. | Duplicates adapter types and surface area. |

### Migration Note for Option A
- If Option A is adopted, callers filtering borrowed iterators (e.g. `slice.iter()` where `Item = &T`) write `fn(&&T) -> bool` or use dereferencing.
- Callers filtering owned iterators (e.g. `vec.into_iter()` where `Item = Box<i32>`) write `fn(&Box<i32>) -> bool`.

---

## 3. Finding S-07: `Range` and `Step` Ownership

### Current Implementation
In `core/iter_adapters.ln`:
```luna
export trait Step {
    fn step_forward(self: &Self) -> Self;
    fn step_less(self: &Self, other: &Self) -> bool;
    fn step_eq(self: &Self, other: &Self) -> bool;
    fn step_greater(self: &Self, other: &Self) -> bool;
}
```
And in `Range::next`:
```luna
if self.current.step_less(&self.end) {
    dec next_val = self.current.step_forward();
    dec prev = std::mem::replace(&rw self.current, next_val);
    return std::Option::Some(prev);
}
```

### Analysis & Resolution
- In earlier audit runs, it was hypothesized that `Range` might move `self.current` and fail borrow check on non-Copy steps.
- However, our verification of `range_owned_step.ln` demonstrates that using `std::mem::replace` with `step_forward(&self)` works correctly without any `Copy` restriction on `Counter`.
- `range_owned_step.ln` compiles and exits with code 0 on the current compiler baseline.
- **Status**: CLOSED as VERIFIED; no API change required for `Step`.

---

## 4. Finding S-06: `Vec<T>.dedup_i32` Type Safety

### Current Implementation
In `alloc/vec.ln`:
- `dedup_i32` was written as an optimization/utility specifically for `i32` elements.
- When invoked on generic `Vec<T>` where `T` was not `i32`, it previously reinterpreted elements as `i32` without size or alignment checks.
- S-06 fix guarded this with `std::mem::size_of<T>() == (4 as u64)`.

### Analysis & Options
- A method named `dedup_i32` on a generic `Vec<T>` is inherently a historical convenience method.
- **Option 1**: Retain `dedup_i32` with runtime safety assertion `size_of<T>() == 4` (status quo).
- **Option 2 (Recommended)**: Deprecate `dedup_i32` and introduce a generic `dedup(self: &rw Self)` once trait bound `T: std::Eq` is fully unified in inherent methods.
