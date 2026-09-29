# Raw Pointer Unsafe Semantics — FROZEN

## Status: FROZEN v1.0

---

## Core Distinction

Two fundamentally different raw pointer mechanisms:

### 1. RAW-STORAGE-ANCHOR (From Place Provenance)

```mellis
struct Buffer { data: i32 }

fn get_ref(b: &Buffer) -> &i32 {
    return life_from(&b.data, b);  // AnchoredTo(b)
}
```

**Properties:**
- `RawOrigin = FromPlace(owner)` where owner is known allocation
- Lifetime derived from `life_from(owner)`
- May escape according to normal lifetime rules
- **PASSES**: can return &T, store in aggregate, etc.

### 2. UNSAFE-RAW-ACCESS (Unknown Provenance)

```mellis
fn helper(data: *rw T) -> i32 {
    unsafe {
        return *data;  // Transient, local
    }
}
```

**Properties:**
- `RawOrigin = Unknown`
- Creates `UnsafeRawRoot` abstract identity
- **Only**: temporary safe loan for local access
- **NOT**: FromPlace, AnchoredTo, or provenance carrier
- **NOT**: allowed to escape function validity domain

---

## Semantic Rules

### Allowed (Transient Local Access)

```mellis
// Case 1: direct deref
fn read(data: *rw i32) -> i32 { unsafe { return *data; } }

// Case 2: temp to non-escaping helper
fn use(data: *rw i32) -> i32 {
    unsafe { return use_temp(raw_to_safe(data)); }
}
fn use_temp(r: &i32) -> i32 { *r }
```

### Forbidden (Escaping Provenance)

```mellis
// Case 3: return &T from unknown raw
fn evil(p: *T) -> &T {          // ❌ MUST REJECT
    unsafe { return raw_to_safe(p); }
}

// Case 4: aggregate containing &T
fn wrap(p: *T) -> Wrapper {    // ❌ MUST REJECT
    unsafe {
        return Wrapper { ref: raw_to_safe(p) };
    }
}

// Case 5: escaping lifetime
fn leak(p: *T) -> &T {          // ❌ MUST REJECT
    unsafe { return raw_to_safe(p); }
}
```

### Conflict Rules

```mellis
// Case 7: local loans from same UnsafeRawRoot must conflict
fn conflict(p: *rw i32) -> i32 {
    unsafe {
        dec r1 = raw_to_safe(p);  // UnsafeRawRoot(p)
        dec r2 = raw_to_safe(p);  // Same root!
        return *r1 + *r2;         // ❌ MUST REJECT (conflict)
    }
}
```

### Borrow Semantics Still Apply

```mellis
// Unsafe does NOT disable borrow semantics
fn bad_mutation(data: *rw i32) {
    unsafe {
        dec r = raw_to_safe(data);
        *data = 42;  // ❌ MUST REJECT (mutation while & live)
    }
}
```

---

## Formal Specification

### UnsafeRawRoot

```
UnsafeRawRoot {
    source_value: function-local identity
}
```

**Invariant:** Loans derived from `UnsafeRawRoot`:
1. Are valid only within the function's unsafe validity domain
2. Cannot establish `ReturnEffect::BorrowsFrom`
3. Cannot be carried in `BorrowsCarried`
4. Cannot be stored in aggregates returned from function
5. Must conflict when derived from same source value

### Loan Classification

| Origin | May Escape? | May Conflict? | Example |
|--------|-------------|---------------|---------|
| `FromPlace(owner)` | Yes (via lifetime) | Yes | `&b.data` where `b: &Buffer` |
| `AnchoredTo(owner)` | Yes (via lifetime) | Yes | `life_from(&b.data, b)` |
| `UnsafeRawRoot(Unknown)` | **NO** | **YES** | `raw_to_safe(p)` where `p: *T` |

---

## Acceptance Test Matrix

| # | Program | Expected | Actual | Status |
|---|---------|----------|--------|--------|
| 1 | `unsafe { *data }` | PASS | PASS | ✅ |
| 2 | `raw_to_safe(data)` → non-escaping helper | PASS | PASS | ✅ |
| 3 | `raw_to_safe(data)` → return &T | REJECT | PASS | ❌ GAP |
| 4 | `raw_to_safe(data)` → Wrapper { ref: ... } | REJECT | PASS | ❌ GAP |
| 5 | `raw_to_safe(data)` → escaping lifetime | REJECT | PASS | ❌ GAP |
| 6 | `life_from(&b.data, b)` → return &T | PASS | PASS | ✅ |
| 7 | two `raw_to_safe(p)` → conflict | REJECT | PASS | ❌ GAP |
| 8 | sorting helper pattern | PASS | PASS | ✅ |

**Gaps:** Cases 3, 4, 5, 7 — UnsafeRawRoot loans incorrectly allowed to escape

---

## Gap Assignment

**C-GAP-12:** UnsafeRawRoot Escape Detection
**Severity:** P1
**Location:** borrow_analysis.rs - `ReturnEffect` / `BorrowsCarried` handling for UnsafeRawRoot loans

---

*Canonical source for raw pointer unsafe semantics*
