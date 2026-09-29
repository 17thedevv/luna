---
name: luna-skill-maintenance
description: Maintain Luna repository agent skills using confirmed reusable knowledge, minimal edits, and explicit conflict checks. Use at the end of substantial compiler/language/stdlib work or when a skill is stale or contradictory; not for routine edits.
---

# Luna Skill Maintenance

## Purpose

Keep `.agents/skills/**/SKILL.md` aligned with confirmed, reusable Luna
architecture, semantics, and workflows. Skills are durable operational
guidance—not task reports, design authority, or a second language
specification.

## When to review skills

At the close of substantial work, briefly ask:

- Did the task disprove or refine an assumption in an existing skill?
- Did it establish a reusable rule, acceptance gate, or recurring anti-pattern?
- Was a contract explicitly approved/frozen that an existing skill should
  reference?
- Did a recommended workflow demonstrably fail or cause avoidable risk?

If none apply, make no skill changes and report `SKILL IMPACT: none`.
Do not trigger recursive skill audits for trivial tasks.

## Evidence and authority

Promote a lesson only when supported by at least one reliable source, such as
an approved/frozen contract, explicit maintainer instruction, current
architecture, permanent regression, or repeated demonstrated behavior. Label
it as a **current fact**, **frozen invariant**, **recommended workflow**, or
**known limitation** as appropriate.

Do not promote guesses, temporary state, test counts, incidental paths,
one-off debugging notes, unapproved proposals, or a workaround that has not
been accepted as design. A skill cannot create semantics or freeze a feature.
If a change would decide or alter language semantics, stop and request design
authority first.

## Minimal maintenance workflow

1. Find the narrowest skill that owns the reusable guidance. Search related
   skills for overlap before editing.
2. Classify the change as **CORRECTION**, **REFINEMENT**, **NEW DURABLE RULE**,
   **DEPRECATION**, or **CROSS-SKILL RECONCILIATION**.
3. Edit only the affected section(s), preserve useful guidance, and avoid
   duplicate copies of a rule. Prefer one authoritative owner with brief
   cross-references from dependent skills.
4. Check names, syntax, namespace examples, maturity labels, and status
   wording against current Luna conventions. Distinguish proposed,
   implemented, verified, and frozen states.
5. Re-read the changed skill(s) in full and search for contradictions with
   closely related skills. Run the repository's skill validator if one
   exists; do not invent a validation mechanism.
6. Report `SKILL IMPACT` briefly, including files changed or why no update was
   warranted.

Do not rewrite unrelated skills or historical specifications to make them
look current. If an existing conflict cannot be resolved from authorized
evidence, report both sides and request maintainer direction instead of
silently choosing one.

## Creation threshold

Create a separate skill only for a distinct, recurring domain or workflow
with enough durable guidance to justify independent activation. For a single
bug or one implementation, update the owning skill only if the lesson is
generalizable; otherwise leave it in the task's tests/report.

Follow repository conventions for skill names, frontmatter, folder layout,
and optional resources. Keep the description concise and discriminating so
automatic skill selection activates it only for relevant work. Do not add
templates, scripts, metadata, or other files without a concrete use.

## Cross-skill relationships

Respect domain-specific skills as owners of their subject matter (grammar,
semantic compliance, stdlib architecture/design, testing, compiler research,
and audit). A maintenance pass should reconcile cross-references, not copy
entire workflows into every skill.

`luna-language-capability-validation` governs evidence for claims that a
language/compiler feature is supported. This skill governs whether confirmed
lessons from such work belong in durable agent guidance. For example, a
temporary generic-method failure is not a language rule; only after the
contract and capability are confirmed should durable guidance be updated.

## Scope and authorization

Only modify skills when the current task authorizes skill maintenance. If a
task does not authorize edits, report the proposed minimal update instead.
When authorized, still preserve scope: do not use a skill change to expand
permission for unrelated compiler, runtime, stdlib, or external-system work.
