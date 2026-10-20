<!-- luna-doc-role: evidence -->

# FIND-ASYNC-PROVIDER-01 report — exported async provider `.llib` corruption

Branch `fix-async-provider-zero-param-export`. Implementation commit **13082834**
(authored by the maintainer; a parallel local attempt was discarded as a
duplicate — 13082834 also removes the stale opcode-15 arm and adds MVIR wire unit
tests).

## Symptom (reproduced twice: A2-FU2, D3)

Importing a `.llib` that exports a **non-generic `async fn`** failed with
`Invalid semantic metadata: CorruptedData`. The "zero-parameter" name was a red
herring.

## Control matrix (freeze)

| Variant | Before | Now |
|---|---|---|
| `async fn f()` (zero param) | FAIL | PASS |
| `async fn f(x: i32)` | **FAIL** | PASS |
| `async fn f()` (void) | FAIL | PASS |
| generic `async fn f<T>` | PASS | PASS |
| sync `fn f()` (zero param) | PASS | PASS |

The trigger was **non-generic exported async** (a materialized async body), not
zero parameters. The A2-FU2 workaround happened to use *generic* async, which
never materializes a body and therefore never hit the bug.

## First mismatch (byte/field precision)

- **Writer:** `HeapFree` is emitted as **opcode 17** with a single operand.
- **Reader:** opcode **17** decoded `Eq` (two operands) — a stray duplicate of
  opcode 8 — while `HeapFree` sat at opcode **15**, which the writer never emits.

Only async-lowered bodies emit `HeapFree` (the A2 future-environment free), so
each occurrence drifted the reader by one operand → garbage
`block_count = 83886080` → allocation abort / `CorruptedData`.

## Fix

The reader's opcode 17 decodes `MlibInstruction::HeapFree { value }`; the stale
opcode-15 `HeapFree` arm is removed. Unit tests `heap_free_decodes_at_writer_opcode_17`
and `stale_heap_free_opcode_15_is_not_accepted` pin the wire tag.

Avoided non-fixes: no dummy parameter, no zero-parameter special-case, no source
fallback, no diagnostic re-mapping, no validation removal.

## Versioning

No bump: the serialized schema and the writer's emitted bytes are unchanged; the
decoder's wrong arm was corrected, so previously-broken artifacts become readable
rather than invalidated (compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**FIND-ASYNC-PROVIDER-01: CLOSED / FIXED IN TESTED SCOPE** at `13082834`.
