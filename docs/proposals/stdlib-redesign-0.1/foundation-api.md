<!-- luna-doc-role: guidance -->

# First C++ comparison implementation slice

This is the implementation contract for the first library slice authorized by
the maintainer's 2026-10-22 request. It does not freeze all stdlib APIs or adopt
C++ semantics. The [functional gap map](cpp-library-gap-map.md) records the larger
backlog; verification lives in the [audit](../../audits/stdlib-usize-2026-10-22/README.md).

The implementation at 28802f12 passes focused Windows check/build/native
source/artifact matrices, including negative bounds and old-artifact rejection.
Full platform/workspace and final distribution qualification remain open.

## Set operations

All types remain directly in std. BTreeSet<T> adds:

```luna
impl<T: std::Ord> BTreeSet<T> {
    export fn is_subset(self: &Self, other: &Self) -> bool;
    export fn is_superset(self: &Self, other: &Self) -> bool;
    export fn is_disjoint(self: &Self, other: &Self) -> bool;
}
impl<T: std::Ord + std::Clone> BTreeSet<T> {
    export fn union_owned(self: &Self, other: &Self) -> Self;
    export fn intersection_owned(self: &Self, other: &Self) -> Self;
    export fn difference_owned(self: &Self, other: &Self) -> Self;
    export fn symmetric_difference_owned(self: &Self, other: &Self) -> Self;
}
```

HashSet<T> adds is_disjoint under its existing Hash + Eq bound, complementing
its existing relations and owned-result methods. Relations do not allocate and
require no Clone. Empty is a subset of every set and disjoint from every set.
Equality of ordered-set elements follows cmp == 0; hash sets use Hash/Eq.

Owned operations borrow both inputs and clone elements into independent results.
They do not move out of or mutate an input; output length is usize. Duplicates
are eliminated by the existing collection insertion contract; no clone or owner
may leak. BTreeSet iteration remains ordered. For equivalence classes containing
different representations, union processes self then other, preserving the
existing key when insertion finds an equivalent key; intersection/difference
clone from self. Symmetric difference clones each side's exclusive elements.

These initial algorithms use iteration and existing lookup/insertion: they do
not claim C++ sorted-range algorithms' linear complexity. They allocate under
the ordinary aborting policy. A later recoverable variant must define its error
and ownership contract explicitly rather than returning a partial result.

## Fixed-width bit utilities

Import the existing num provider and call std::bit functions. This source
placement does not define a new public provider or make provider names namespaces.

| Signature | Behavior |
| --- | --- |
| count_ones_u64(u64) -> u32 | Number of set bits, 0..64 |
| leading_zeros_u64(u64) -> u32 | High zero run; zero returns 64 |
| trailing_zeros_u64(u64) -> u32 | Low zero run; zero returns 64 |
| rotate_left_u64(u64, u32) -> u64 | Rotate modulo 64, preserving all bits |
| rotate_right_u64(u64, u32) -> u64 | Inverse rotation, count modulo 64 |
| has_single_bit_u64(u64) -> bool | True exactly for nonzero powers of two |
| bit_width_u64(u64) -> u32 | Required binary digits; zero returns 0 |
| bit_floor_u64(u64) -> u64 | Greatest power of two <= input; zero returns 0 |
| bit_ceil_u64(u64) -> Option<u64> | Least representable power of two >= input; 0/1 return Some(1), overflow returns None |
| swap_bytes_u64(u64) -> u64 | Reverse eight bytes, independent of host endianness |

Left-shift pieces are masked to their admitted width before shifting: Luna
checks bit loss, so rotation must not depend on implicit truncation.

These are total, allocation-free scalar operations using existing integer
machinery. u32 describes bounded properties/rotation counts of a fixed u64,
not memory length/index. No implicit u64/usize conversion, new intrinsic or
all-integer-width coverage is promised by this slice.

## Constructor migration slice — 2026-10-23

The confirmed STD-D03 convention now has type-owned constructors for the
shared-ownership, interior-cell, time, random, path, crypto and existing I/O
buffer/cursor types:

```luna
std::Rc<T>::new(value)
std::Arc<T>::new(value)
std::Cell<T>::new(value)
std::RefCell<T>::new(value)
std::Duration::new(secs, nanos)
std::Rng::new(seed)
std::PathBuf::new()
std::Sha256::new()
std::HmacSha256::new(key)
std::ChaCha20::new(key, nonce, counter)
std::io::Cursor<T>::new(inner)
std::io::BufReader<R>::new(inner)
std::io::BufReader<R>::with_capacity(inner, capacity)
std::io::BufWriter<W>::new(inner)
std::io::BufWriter<W>::with_capacity(inner, capacity)
```

Each method forwards its arguments once to the provider's existing factory;
owned inputs move once, while borrowed crypto inputs keep their existing copy
behavior. Allocation, initialization, representation and borrow behavior are
unchanged.
The I/O wrappers preserve the current default capacity and zero-capacity
behavior; the separate I/O policies remain proposals.
The older `std::rc_new`, `std::arc_new`, `std::cell_new` and
other `*_new`/`*_with_capacity` entry points remain migration bridges. Removing
or deprecating those aliases requires the remaining compatibility and migration
decision. No fallible constructor or new allocation contract is introduced by
this slice.
