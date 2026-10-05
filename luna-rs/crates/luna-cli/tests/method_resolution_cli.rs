#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn method_applicability_and_binder_identity_survive_provider_modes_and_order() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/method_policy");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for reversed in [false, true] {
            let order = if reversed { "reversed" } else { "forward" };
            let project = root.join(order);
            fs::create_dir(&project).unwrap();
            for provider in [
                "provider",
                "multi_impl_provider",
                "private_provider",
                "trait_provider",
                "bound_provider",
                "concrete_inherent_provider",
            ] {
                let input_name = if reversed && provider == "multi_impl_provider" {
                    "multi_impl_provider_reversed"
                } else if reversed && provider == "concrete_inherent_provider" {
                    "concrete_inherent_provider_reversed"
                } else {
                    provider
                };
                let source = project.join(format!("{provider}.ln"));
                fs::copy(fixtures.join(format!("{input_name}.ln")), &source).unwrap();
                if mode == "artifact" {
                    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
                        .arg("build")
                        .arg(&source)
                        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                        .arg(source.with_extension("llib"))
                        .env("LUNA_SYSROOT", root)
                        .output()
                        .unwrap();
                    assert!(
                        build.status.success(),
                        "{mode}/{order}/{provider}: {}",
                        render(&build)
                    );
                    fs::remove_file(source).unwrap();
                }
            }
            // Relocate after publishing and forbid adjacent provider sources.
            let relocated = root.join(format!("relocated_{order}"));
            fs::rename(project, &relocated).unwrap();
            for name in [
                "qualified_traits",
                "nested_callable_role",
                "inapplicable_inherent_bound",
                "inapplicable_inherent_arity",
                "inapplicable_inherent_type",
                "immutable_receiver_fallback",
                "private_inherent_fallback",
                "imported_inherent_local_trait",
                "imported_qualified",
                "multi_impl_consumer",
                "method_binders_independent",
                "result_inference",
                "generic_inherent_bound_fallback",
                "stdlib_element_bounds",
                "dynamic_regular_drop",
                "qualified_generic",
                "bound_argument_applicability",
                "recursive_impl_bound_valid",
                "array_impl_pattern",
                "mutable_reference_binding",
                "associated_impl_bound_valid",
                "rigid_trait_argument_valid",
                "reference_head_coherence",
                "concrete_inherent_dispatch",
                "concrete_inherent_consumer",
            ] {
                let source = relocated.join(format!("{name}.ln"));
                fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
                let check = modes.check(root, &source);
                if !check.status.success() {
                    failures.push(format!("{mode}/{order}/{name} check: {}", render(&check)));
                }
                let (exe, build) =
                    modes.build(root, &source, &format!("method_{mode}_{order}_{name}"));
                if !build.status.success() {
                    failures.push(format!("{mode}/{order}/{name} build: {}", render(&build)));
                    continue;
                }
                let run = Command::new(exe)
                    .current_dir(std::env::temp_dir())
                    .output()
                    .unwrap();
                if !run.status.success() {
                    failures.push(format!("{mode}/{order}/{name} native: {}", render(&run)));
                } else {
                    eprintln!("PASS {mode}/{order}/{name}: check, build, native exit0");
                }
            }
            for (name, code) in [
                ("ambiguous_traits", "E1008"),
                ("ambiguous_traits_reversed", "E1008"),
                ("ambiguous_expected_result", "E1008"),
                ("ambiguous_generic_bounds", "E1008"),
                ("imported_ambiguous", "E1008"),
                ("private_only", "E1003"),
                ("method_unconstrained", "E2001"),
                ("multi_impl_wrong_specialization", "E2001"),
                ("stdlib_no_element_bound", "E2021"),
                ("dynamic_wrong_arity", "E2001"),
                ("generic_missing_bound", "E2021"),
                ("recursive_impl_bound_missing", "E2021"),
                ("qualified_impl_bound_missing", "E2021"),
                ("method_impl_bound_missing", "E2021"),
                ("cyclic_impl_bound", "E2021"),
                ("array_impl_wrong_length", "E2001"),
                ("array_trait_overlap", "E2005"),
                ("immutable_reference_binding", "E2023"),
                ("actual_destructor", "E2031"),
                ("associated_impl_bound_missing", "E2021"),
                ("associated_caller_bound_missing", "E2001"),
                ("rigid_trait_argument_missing", "E2021"),
            ] {
                let source = relocated.join(format!("{name}.ln"));
                fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
                let check = modes.check(root, &source);
                let (_, build) =
                    modes.build(root, &source, &format!("method_{mode}_{order}_{name}"));
                for (command, output) in [("check", check), ("build", build)] {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if output.status.success() || !stderr.contains(&format!("error[{code}]")) {
                        failures.push(format!(
                            "{mode}/{order}/{name} {command}, expected {code}: {}",
                            render(&output)
                        ));
                    } else {
                        eprintln!("PASS {mode}/{order}/{name} {command}: {code}");
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
