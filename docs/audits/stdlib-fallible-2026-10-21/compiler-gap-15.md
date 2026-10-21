<!-- luna-doc-role: evidence -->

# C-GAP-15: inferred associated owners and callable bounds

2026-10-21, Asia/Bangkok. Generic compiler repair; no stable release claim.

After C-GAP-13, explicitly supplied owner arguments worked. Omitting them left
the selected impl's binders rigid, so ordinary argument/result inference failed.
Separately, taking `Restricted<u8>::create` as a function value bypassed the
impl's `Ready` bound, although the equivalent direct call rejected E2021.

The typechecker now creates fresh inference variables for the selected checked
impl header when owner arguments are omitted. Fixed parts of a specialized self
pattern remain fixed. Method binders have their own variables, and explicit
method arguments compose with the owner substitution instead of replacing it.
Calls retain those existing variables so arguments and expected results constrain
the same instance later used by monomorphization.

Associated callable bound obligations are checked after body inference. A direct
call updates the same obligation; materializing an unused function value still
has to satisfy its bounds. A generic caller must prove its own declared bound.
This does not add syntax or infer an owner from an unrelated method parameter.

The [CLI matrix](../../../luna-rs/crates/luna-cli/tests/associated_owner_generic_cli.rs)
passes native Windows source-only and freshly built artifact-only modes. It
retains explicit-owner controls and adds argument inference, expected result
inference, independent method arguments, specialized self patterns, expected
callable types, later indirect calls and bounded generic forwarding. Missing
explicit/inferred/caller bounds reject E2021 in both check and build. Broad
workspace regression and Ubuntu reacquisition are tracked separately.

Compiler Change
- Capability: ordinary associated-owner inference and bounds on callable values.
- Why stdlib exposed it: `Box::new(value)` and `Vec::new()` with contextual types.
- Why it is generic: checked impl and method binder identities govern every type.
- User-defined types benefiting: Storage, Optional, Pair and Restricted.
- Tests: public source/artifact CLI matrix above, including native executions.
- New intrinsic/lang_item?: NO.
- Stdlib-specific branch?: NO.

SKILL IMPACT: none. Existing generic binder, callable identity and source/artifact
validation rules already require these controls.
