//! DIAG-3: duplicate declarations retain their previous binding structurally.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, path::Path, process::{Command, Output}};
use support::{render, workspace_root, ProviderModes};

fn marked_line(path: &Path, marker: &str) -> u32 {
    fs::read_to_string(path).unwrap().lines().position(|line| line.contains(marker)).unwrap() as u32 + 1
}

fn locations(output: &Output) -> Vec<(String, u32, u32)> {
    String::from_utf8_lossy(&output.stderr).lines().filter_map(|line| {
        let location = line.trim_start().strip_prefix("-->")?.trim();
        let mut parts = location.rsplitn(3, ':');
        let col = parts.next()?.parse().ok()?;
        let row = parts.next()?.parse().ok()?;
        Some((parts.next()?.replace('\\', "/"), row, col))
    }).collect()
}

fn assert_duplicate(output: &Output, file: &Path) -> Vec<(String, u32, u32)> {
    assert_conflict(output, "E1002", file, file)
}

fn assert_conflict(output: &Output, code: &str, file: &Path, previous_file: &Path) -> Vec<(String, u32, u32)> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{}", render(output));
    assert_eq!(stderr.lines().filter(|line| line.starts_with("error[")).count(), 1, "{}", render(output));
    assert!(stderr.contains(&format!("error[{code}]")), "{}", render(output));
    let sites = locations(output);
    assert_eq!(sites.len(), 2, "{}", render(output));
    let filename = file.to_string_lossy().replace('\\', "/");
    assert_eq!(sites[0].0, filename, "{sites:?}");
    assert_eq!(sites[1].0, previous_file.to_string_lossy().replace('\\', "/"), "{sites:?}");
    assert_eq!(sites[0].1, marked_line(file, "// invalid definition"), "{sites:?}");
    // Artifact display paths carry the source text of the corresponding .ln.
    let previous_source = previous_file.with_extension("ln");
    let previous_line = if previous_source.exists() { marked_line(&previous_source, "// previous definition") }
        else { marked_line(&workspace_root().parent().unwrap().join("tests/luna/language/diagnostic_declarations").join(previous_source.file_name().unwrap()), "// previous definition") };
    assert_eq!(sites[1].1, previous_line, "{sites:?}");
    sites
}

fn assert_primary_errors(output: &Output, file: &Path, multiple: bool) -> Vec<(String, u32, u32)> {
    assert_eq!(output.status.code(), Some(1), "{}", render(output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let source = fs::read_to_string(file).unwrap();
    let expected = if multiple { vec![marked_line(file, "// first invalid contract"), marked_line(file, "// second invalid contract")] }
        else { vec![marked_line(file, if source.contains("// invalid return contract") { "// invalid return contract" } else { "// invalid expression" })] };
    assert_eq!(stderr.lines().filter(|line| line.starts_with("error[")).count(), expected.len(), "{}", render(output));
    assert_eq!(stderr.matches("error[E2001]").count(), expected.len(), "{}", render(output));
    let sites = locations(output);
    assert_eq!(sites.iter().map(|site| site.1).collect::<Vec<_>>(), expected, "{}", render(output));
    assert!(sites.iter().all(|site| site.0 == file.to_string_lossy().replace('\\', "/") && site.2 > 1), "{sites:?}");
    sites
}

fn assert_candidates(output: &Output, code: &str, file: &Path, provider: &Path, markers: &[&str]) -> Vec<(String, u32, u32)> {
    assert_eq!(output.status.code(), Some(1), "{}", render(output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().filter(|line| line.starts_with("error[")).count(), 1, "{}", render(output));
    assert!(stderr.contains(&format!("error[{code}]")), "{}", render(output));
    let sites = locations(output);
    assert_eq!(sites.len(), markers.len() + 1, "{}", render(output));
    assert_eq!(sites[0].0, file.to_string_lossy().replace('\\', "/"));
    assert_eq!(sites[0].1, marked_line(file, "// invalid definition"));
    let source = workspace_root().parent().unwrap().join("tests/luna/language/diagnostic_declarations")
        .join(provider.with_extension("ln").file_name().unwrap());
    for (site, marker) in sites[1..].iter().zip(markers) {
        assert_eq!(site.0, provider.to_string_lossy().replace('\\', "/"), "{sites:?}");
        assert_eq!(site.1, marked_line(&source, marker), "{sites:?}");
    }
    sites
}

#[test]
fn declaration_conflicts_keep_previous_locations_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/diagnostic_declarations");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("diagnostic_declarations");
        fs::create_dir(&project).unwrap();
        for provider in ["first", "second", "overlap_provider", "candidates_provider", "foreign_provider"] {
            let file = project.join(format!("{provider}.ln"));
            fs::copy(fixtures.join(format!("{provider}.ln")), &file).unwrap();
            if mode == "artifact" {
                let output = Command::new(env!("CARGO_BIN_EXE_luna")).arg("build").arg(&file)
                    .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                    .arg(file.with_extension("llib")).env("LUNA_SYSROOT", root).output().unwrap();
                assert!(output.status.success(), "{}", render(&output));
                fs::remove_file(file).unwrap();
            }
        }
        for (name, code, provider, markers) in [
            ("ambiguous_candidates", "E2019", "candidates_provider", vec!["// first candidate", "// second candidate"]),
            ("orphan_trait_impl", "E2006", "foreign_provider", vec!["// first candidate", "// second candidate"]),
            ("orphan_inherent_impl", "E2006", "foreign_provider", vec!["// second candidate"]),
        ] {
            let file = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &file).unwrap();
            let provider = project.join(format!("{provider}.{}", if mode == "source" { "ln" } else { "llib" }));
            let first = assert_candidates(&modes.check(root, &file), code, &file, &provider, &markers);
            assert_eq!(assert_candidates(&modes.check(root, &file), code, &file, &provider, &markers), first);
            let (exe, output) = modes.build(root, &file, &format!("{mode}_{name}"));
            assert!(!exe.exists());
            assert_eq!(assert_candidates(&output, code, &file, &provider, &markers), first);
            eprintln!("PASS {mode}/{name}: primary and structural candidate/definition locations");
        }
        for name in ["overlapping_impls", "imported_overlap"] {
            let file = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &file).unwrap();
            let previous = if name == "imported_overlap" { project.join(if mode == "source" { "overlap_provider.ln" } else { "overlap_provider.llib" }) } else { file.clone() };
            let first = assert_conflict(&modes.check(root, &file), "E2005", &file, &previous);
            assert_eq!(assert_conflict(&modes.check(root, &file), "E2005", &file, &previous), first);
            let (exe, output) = modes.build(root, &file, &format!("{mode}_{name}"));
            assert!(!exe.exists());
            assert_eq!(assert_conflict(&output, "E2005", &file, &previous), first);
            eprintln!("PASS {mode}/{name}: conflicting implementations retain both source locations");
        }
        for name in ["duplicate_functions", "duplicate_fields", "duplicate_variants", "duplicate_aliases", "duplicate_imports"] {
            let file = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &file).unwrap();
            let first = assert_duplicate(&modes.check(root, &file), &file);
            assert_eq!(assert_duplicate(&modes.check(root, &file), &file), first);
            let (exe, output) = modes.build(root, &file, &format!("{mode}_{name}"));
            assert!(!exe.exists());
            assert_eq!(assert_duplicate(&output, &file), first);
            eprintln!("PASS {mode}/{name}: primary and previous binding, repeated check/build parity");
        }
        for name in ["bare_return_function", "bare_return_method", "bare_return_closure", "bare_return_multiple",
            "tuple_mismatch", "empty_tuple_mismatch", "array_mismatch", "empty_array_mismatch", "returned_tuple_mismatch"] {
            let file = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &file).unwrap();
            let multiple = name == "bare_return_multiple";
            let first = assert_primary_errors(&modes.check(root, &file), &file, multiple);
            assert_eq!(assert_primary_errors(&modes.check(root, &file), &file, multiple), first);
            let (exe, output) = modes.build(root, &file, &format!("{mode}_{name}"));
            assert!(!exe.exists());
            assert_eq!(assert_primary_errors(&output, &file, multiple), first);
            eprintln!("PASS {mode}/{name}: real expression/declaration anchors, repeated check/build parity");
        }
        let file = project.join("controls.ln");
        fs::copy(fixtures.join("controls.ln"), &file).unwrap();
        assert!(modes.check(root, &file).status.success());
        let (exe, output) = modes.build(root, &file, &format!("{mode}_controls"));
        assert!(output.status.success(), "{}", render(&output));
        let run = Command::new(exe).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/controls: distinct declarations and one imported binding execute natively");
    }
}
