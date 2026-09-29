# STD-ARCH-01 — Compatibility Contract Matrix

Status: canonical surfaces and deliberately preserved compatibility paths, as of
the current Luna stdlib/runtime baseline.

## Surfaces

| Surface | Canonical | Legacy read accepted | Canonical write |
| :--- | :--- | :--- | :--- |
| Source | `.ln` | `.ms` (read-only; probed after `.ln`) | `.ln` only |
| Artifact | `.llib` (magic `LLIB`) | `.mlib` (reader also accepts magic `MLIB`) | `.llib` only |
| Provider identity | manifest logical ID (e.g. `core/panic`) | no legacy aliases | canonical only |
| Runtime ABI | `__luna_*` | none found in active runtime source | canonical ABI |

## Resolver precedence

- discovery: `.llib` → `.ln` → `package/package.ln` → `.mlib` → `.ms` → `package/package.ms`
- local import: `.llib` → `.ln` → `.mlib` → `.ms`

Canonical representations always shadow legacy. The state machine distinguishes
**not found** (may advance to the next permitted representation) from **found but
invalid** (stop — no fallback).

## Preserved compatibility paths (deliberate, not accidental)

- **`.mlib` artifact read** — the reader accepts `MLIB` magic and `.mlib` files.
  A canonical `.llib` missing `AstInterface` is rejected (`InvalidArtifact`), but a
  legacy `.mlib` without `AstInterface` may still reconstruct via `SemanticMetadata`.
- **`.ms` source read** — historical Mellis source extension, probed after `.ln`.
- **`MELLIS_SYSROOT`** — `LUNA_SYSROOT` is preferred; the legacy env var is still read.
- **Legacy `emit_mlib` option field** — compatibility spelling only. It selects
  canonical `.llib` emission; explicit `.mlib` output paths are rejected. No
  current writer or CLI route emits `.mlib`.
- **`Mlib*` type names / `MLIB_*` constants** — legacy names remain in parts of
  the internal serialization representation; the active reader/writer APIs use
  `LlibReader` / `LlibWriter`, while compatibility aliases remain available.
  These internal names do not select or emit `.mlib` artifacts.
- **`__mellis_*` runtime ABI** — no active declarations or definitions remain in
  `runtime/`; those names occur only in historical documentation and are not a
  compatibility ABI.

## Frozen invariants

| Invariant | Statement |
| :--- | :--- |
| **COMPAT-CANONICAL-01** | Canonical source is `.ln`; canonical artifact is `.llib` / magic `LLIB`. |
| **COMPAT-WRITE-01** | Every current library-writing path emits `.llib` only. `.mlib` is read-only compatibility; explicit `.mlib` output is rejected. |
| **COMPAT-READ-01** | Legacy formats are read only through explicitly preserved compatibility paths. |
| **COMPAT-PRECEDENCE-01** | Canonical representations take precedence over legacy representations. |
| **COMPAT-INVALID-01** | An existing invalid canonical artifact never falls back to legacy or source. |
| **COMPAT-ABI-01** | Active runtime ABI names use the `__luna_*` prefix; no `__mellis_*` ABI is retained. |
| **DEAD-PATH-01** | Every remaining legacy-looking code path has an identified live purpose. |

## Provider identity note

`core/panic` is the canonical logical provider ID (frozen in Phase 4A). The `/` is
part of the logical ID, not a physical path, and there is deliberately no `panic`
alias. This is intentional architecture — do not rename or normalize it.
