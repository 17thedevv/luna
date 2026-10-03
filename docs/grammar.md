<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](spec/0.1/README.md).

# Luna 0.1 grammar guide

The current contract is [syntax.md](spec/0.1/syntax.md); formal productions are in [grammar.ebnf](grammar.ebnf). This replaces the mixed historical grammar that advertised removed syntax.

Use dec rw, comma-separated struct fields and a terminating struct semicolon, <T> generic arguments, inline module, import, using path as alias, match ->, macro captures @ and rule =>, postfix .await, life_from, requires life(...), and requires anchor(field) = self. Fields default public under the approved Visibility-02 amendment; containing type access is checked independently.

The EBNF states its coverage explicitly. Complete lexical, pattern, attribute-admission and macro-fragment details are not claimed exhaustive by one file. Parser acceptance does not amend the contract. See [gaps.md](spec/0.1/gaps.md) for field delimiters, foreach and receiver shorthand.

Agent skills reference this versioned baseline and cannot independently change it. Syntax amendments update the specification, grammar, parser and positive/negative tests together. This documentation task changes no parser implementation.
