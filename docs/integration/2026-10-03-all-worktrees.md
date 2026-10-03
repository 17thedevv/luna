# All-worktree integration — 2026-10-03

User scope: commit, push and merge all local/fetched branches and uncommitted work from every Git worktree into main. This integration preserves work; it is not a language or release freeze.

## Preserved worktrees

| Worktree | Branch | Snapshot commit | Files committed |
|---|---|---|---|
| `C:\Users\84387\.codex\worktrees\a250\fdlang` | `codex/integrate-all-worktrees-2026-10-03` | `ceeed0a5fde6` | 256 |
| `D:\fdlang` | `codex/phase6-formatting-closure` | `43c5ed6cbf7d` | 325 |
| `C:\Users\84387\.codex\worktrees\phase5-audit\fdlang` | `codex/phase6-formatting` | `d7fe694a2575` | 1 |
| `C:\Users\84387\.codex\worktrees\phase6-final-verification\fdlang` | `codex/worktree-verification-snapshot-2026-10-03` | `9662ab7dddd8` | 1 |
| `C:\Users\84387\Documents\Codex\2026-09-18\luna-sem-maturity-01-phase-0-2` | `sem-maturity-01-phase-0` | `d13ba2475ceb` | 0 |

## Reconciliation

- The current 0.1 reference and extended audit remain the active entry points.
  D:/fdlang audit records and fixtures are retained in the sibling directories
  ending in d-worktree / d_worktree; runner paths and results remain historical.
- Phase 6 closure supersedes the older formatting implementation/harness.
  Its earlier worktree notes are preserved under docs/history/pre-0.1.
  The maintainer freeze remains limited to the recorded 0394776 baseline.
- The original recovery/pattern-matching history contains two 296 MB build
  blobs and cannot be pushed to GitHub. The user approved a filtered derivative:
  codex/recovery-source-only-2026-10-03 retains all 26 commits while removing
  4,312 generated build paths. Retained file blobs/modes are compared with the
  original tree at every commit; their contents are unchanged. The original
  recovery ref and raw integration history remain local. The derivative is
  merged with the ours strategy, retaining current Luna sources rather than
  restoring obsolete C++/Mellis trees. See recovery-commit-map.json.
- Browser profile/cache files, build directories, temporary copied sysroots and
  generated preview screenshots remain local and ignored. Source, permanent
  tests, website assets, audit runners and intentional evidence are committed.
- Every eligible local/fetched remote branch tip is checked as an ancestor of
  the final integration tip. The two local archival refs containing the original
  oversized blobs are explicitly excluded from push. No existing remote history
  is rewritten, no force push is used, and no original branch is deleted.

## Evidence boundary

The merged manifest contains 49 providers, compared with the original audited
32. Their presence does not certify the 17 added providers or all their APIs.
The imported Macro Engine contains hardcoded output-name/callee mappings;
V01-ARCH-02 records the conflict with the retained compiler/library boundary.
Some original compiler/grammar defects have candidate fixes in the other
branches. Prior 3dac3ac evidence is not silently marked current or closed.

Current validation results are recorded in integration-validation.json beside
this document. Original audit/example evidence remains associated with its old
revision. Workspace compilation and selected tests are distinct from a full
workspace, source/artifact, all-provider and all-target acceptance run.

## Checks completed on the merged implementation

- cargo check --workspace --all-targets: exit 0.
- Lexer/parser/AST: 41 tests passed across 10 binaries; selected semantic
  target-width/nominal-order regressions: 5 passed across two binaries.
- Fresh CMake runtime and Luna CLI builds: exit 0. Runtime ABI CTest: 1/1.
- Official build-sysroot: all 49 providers build, exit 0.
- Fresh source/artifact CLI characterization: 16 attempts; eight meet contracts,
  six still expose the three grammar defects, two characterize parser behavior.
- Website: 33 pages build from a frozen-lockfile install. Initial prerender
  resolved an unrelated ancestor cookie package; explicitly declaring the
  already-locked cookie 2.0.1 dependency makes the local build pass.
- Documentation links, inventories and whitespace are checked before commit.

Full workspace acceptance, exhaustive generic domains, every new provider and
all native targets have not been certified by this integration.
