<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](spec/0.1/README.md).

# Luna 0.1 — compiler-required language contracts

Authority: [Luna 0.1 specification](spec/0.1/README.md). Acquisition uses an explicit manifest; recognition uses canonical declaration identity, not container spelling or layout.

| Family | Provider | Language hook identities |
|---|---|---|
| Drop | __lang_drop | drop, drop_fn |
| Option | __lang_option | option, some, none |
| Iterator | __lang_iterator | iterator |
| IntoIterator | __lang_into_iterator | into_iterator |
| Copy | copy | copy |
| Try / FromResidual / ControlFlow | try | try, try_from_output, try_branch, from_residual, from_residual_fn, control_flow, continue, break |

The baseline has six families and **16** manifest hook identities. Exact canonical paths are represented in luna-rs/crates/luna-driver/src/lang_contracts.rs. The result provider adds controlled std::OptionExt visibility with no language hook. std::option_flatten is selected ordinary visibility beside Option. These contributions do not turn Result or a helper into a compiler-native type.

## Identification, policy and identity

A hook identifies a semantic role; ownership, traits, borrow analysis and lowering enforce its policy. Renaming a declaration does not change its role. Provider-local SymbolId/DeclId are not portable semantic identity; import reconstructs fresh IDs from canonical identities.

Drop, Copy, iteration, Option advancement and generic Try/FromResidual retain their contracts. Result implements ordinary generic protocols. if/match/while are language syntax, not artificial providers. Bootstrap exposes selected qualified std paths only; it does not create unqualified root names or auto-load all ordinary core/alloc APIs.

## Conformance

Source and fresh artifact contracts must agree. Missing, duplicate, mismatched or invalid identities require deterministic diagnostics. Historical claims of zero semantic name/layout coupling are not current evidence: SliceIter coupling and dispatch/metadata defects remain in [gaps.md](spec/0.1/gaps.md). Formatting macro names are not language hooks.
