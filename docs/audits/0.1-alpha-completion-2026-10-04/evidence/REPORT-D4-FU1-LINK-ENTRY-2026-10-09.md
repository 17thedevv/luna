<!-- luna-doc-role: evidence -->

# D4-FU1 report — `#[link]` on the program entry

Branch `d4-fu1-link-leading-attribute` (from candidate `d4d980fb`).

## Retraction of the original observation

The original D4-FU1 note claimed that `#[link(...)]` as the **first item of a
file** was rejected with E0003. That is **retracted / a misdiagnosis**: the E0003
came from the invalid `extern "C" fn` spelling (the correct form is `extern fn`),
not from the attribute position. On the current candidate, `#[link]` first item +
`extern fn`, a leading comment, and a preceding item all build and run (exit 0).

## Corrected finding

```
#[link(name = "abs")]
fn main() { }          // (also fn main() -> i32)
```

- `check` → PASSED (silent)
- `build` → `E6001 Backend Error: Compiler internal invariant violated:
  Function not found: __luna_user_main` — an **ICE**

Root cause: the generator applies `#[link]` as a **symbol rename**
(`generator.rs:1651-1675`). On `main` that renames the compiler-owned entry
symbol, so the entry alias is never emitted and the backend aborts.

Classification: **front-end validation gap** (DIAG-7 — a known-layer error must
fail before the backend). **Release blocker.**

## Decision (contract)

Typed rejection in the front-end: `#[link]` must not be applied to the program
entry `main`. The generator is **not** changed to silently ignore `#[link]`, and
the check uses the front-end entry identity (`is_main`), not a backend name string.

```
error[E0005]: `#[link]` cannot be applied to the program entry `main`
```

Primary span points at the `#[link]` attribute, not the function body.

## Acceptance matrix

| Case | Result |
|---|---|
| `#[link] fn main() -> i32` | E0005 |
| `#[link] fn main()` (void) | E0005 |
| `#[link]` on helper | build + run PASS |
| `#[link]` on another valid function | semantics unchanged |
| `main` without attribute | build + run PASS |
| `#[link]` first item + `extern fn` | PASS |
| `#[link]` after another item + `extern fn` | PASS |

Negatives: `check` and `build` emit the same diagnostic identity; no E6001/ICE;
no executable; generator unchanged.

## Harness

`ffi_attribute_placement_cli.rs`. Guards: `retained_contracts_cli` 1,
`diagnostic_conformance_cli` 4, `luna-semantic` 0 failed.

## Versioning

No bump — validation only; the artifact representation is unchanged and the
rejected program never produced a valid executable
(compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**D4-FU1: CLOSED / FIXED IN TESTED SCOPE.**
