<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](docs/spec/0.1/README.md).

# Luna 0.1 roadmap

Authority: [0.1 specification](docs/spec/0.1/README.md). This roadmap sequences
work; it does not add language contracts or promise completion dates.

1. Resolve P1 correctness defects in deterministic method selection, unsigned
   widening, container arithmetic, generic dedup and UTF-8 text production.
2. Repair remaining entry, metadata-inflation, iterator/ZST and ownership-contract
   defects; preserve negative ownership/borrow checks.
3. Reconcile grammar disagreements and remaining semantic decisions listed in
   [gaps](docs/spec/0.1/gaps.md); amend contract/parser/tests explicitly.
4. Remove library implementation-name/layout coupling from compiler machinery.
5. Rebuild canonical artifacts, execute source/artifact and relevant workspace
   regressions, then document verified target profiles and accepted deferrals.
6. Extend measured optimization work without weakening semantic/ABI contracts.

Formatting interpolation is deferred pending a complete generic macro contract.
No compiler mapping of print macro names, stdout/stderr prefixes or newline
suffixes is permitted. Future library/runtime capabilities are not release
promises merely because they appear in old plans.

The prior roadmap is retained in [history](docs/history/pre-0.1/ROADMAP.md).
