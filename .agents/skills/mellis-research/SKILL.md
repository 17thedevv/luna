---
name: mellis-research
description: Research protocol for modifying the Mellis compiler
---

<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](../../../docs/spec/0.1/README.md).

# Mellis Research Protocol

Never begin implementation of a non-trivial Mellis task immediately.

## Workflow

First:
- locate relevant source files
- locate existing tests
- inspect current implementation
- inspect related architecture docs
- identify existing abstractions
- identify the exact failing invariant

Then produce a short implementation plan.
Only after that modify code.

## Source of Truth

When instructions conflict, use this precedence:
1. Luna 0.1 specification and adopted language contracts
2. Adopted runtime/ABI contracts
3. Current compiler implementation (conformance evidence, not a semantic override)
4. Current tests and verified behavior
5. Architecture documentation
6. This skill
7. Previous walkthroughs, plans, or agent claims

Never assume a previous implementation status is still valid.
Never infer a feature from a roadmap.
Inspect the repository before changing code.

## Do not trust these as current state

The following are historical and may be stale:
- walkthroughs
- old roadmap documents
- old status files
- previous agent messages
- generated summaries

Always re-verify current state from source and tests.
