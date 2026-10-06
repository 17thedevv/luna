<!-- luna-doc-role: evidence -->

# A1 report — closure environment destruction

Branch `codex/a1-closure-environment-destruction`, base `bbe5ea29`, implementation `cdeb976a`.

## Progression

| Stage | Result | Meaning |
|---|---|---|
| original | `exit 3` | unused owned capture leaked; closure environment never destroyed |
| intermediate | `exit 2` | double-drop after mechanism 1 alone; **preserved debugging evidence, not an accepted checkpoint** |
| final | `exit 0` | environment destroyed exactly once on both unused and consumed paths |

The intermediate `exit 2` is retained only as debugging history. Mechanism 1
alone was semantically incomplete (dropped a moved-from source and never freed
the environment); the fix is committed as one logical change.

## Root cause

- Primary: the closure environment was not part of the ownership/drop lifecycle
  (no drop obligation, no `HeapFree`).
- Secondary: `needs_drop(Closure)` inspected the return type
  (`Closure(expr_id, params, ret)`) instead of the environment.
- Not the backend: the captured type's drop glue already existed.

## Mechanism

- Unused closure: whole-place `Drop(closure)` → `ClosureDropGlue { env_ty }`:
  load `env_ptr` (field 1 of `{ code, env }`) → drop owned (move) captures →
  `HeapFree(env)`.
- Consumed closure: `CallClosure` on a closure owning a move capture consumes
  the closure (caller local moved); the invocation moves owned captures into
  body locals and every exit path runs path-sensitive capture cleanup then
  `HeapFree(env)` exactly once.
- Identity: `ClosureDropGlue { env_ty }` mangled as
  `__luna_closure_drop_glue_<EnvTuple>` — structural, no ExprId/session id.
  The environment tuple fully determines the destruction plan (move capture →
  value type, drop iff `needs_drop(T)`; borrow capture → pointer type, never
  dropped).

## Protocol

compiler **20**, metadata **9**, MVIR **5**, format **2**. Only the compiler
protocol was bumped: older provider bodies carried the incorrect closure
ownership behavior and may lack environment cleanup. Rebuilding is explicit.

## Regressions and evidence

- 9 public regressions (6 positive `exit0`, 2 negative `E3001`, plus the
  existing `closure_capture`), passing in source and fresh artifact modes.
- Compiler-level evidence:
  - `luna-semantic/tests/closure_drop_glue_identity.rs`
  - `luna-cli/tests/closure_env_teardown_mvir.rs`

## Workspace

`cargo test --workspace --no-fail-fast` → **exit 0, 203 targets, 1329 passed /
0 failed / 1 ignored** (baseline was 1315 / 1 / 1; candidate 1328 / 1 / 1).

## Verdict

**A1 PASS — CLOSED / CONFORMANT IN TESTED SCOPE.**
