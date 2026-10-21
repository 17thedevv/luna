#[path = "support/stdlib.rs"]
mod support;

use std::process::Command;
use support::{ProviderModes, render, workspace_root};

#[test]
fn documented_text_and_byte_printing_have_source_artifact_parity() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/stdlib/stable_io");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in ["literal_text", "owned_text_bytes"] {
            let source = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &source);
            assert!(check.status.success(), "{mode}/{name}: {}", render(&check));
            let (exe, build) = modes.build(root, &source, &format!("{name}_{mode}"));
            assert!(build.status.success(), "{mode}/{name}: {}", render(&build));
            assert!(
                build.stderr.is_empty(),
                "unexpected compiler stderr: {}",
                render(&build)
            );
            let run = Command::new(exe).output().unwrap();
            assert!(run.status.success(), "{mode}/{name}: {}", render(&run));
            assert_eq!(
                String::from_utf8(run.stdout).unwrap().replace("\r\n", "\n"),
                "hello world\n"
            );
            assert!(run.stderr.is_empty());
        }
        let source = fixtures.join("reject_literal_bytes.ln");
        let check = modes.check(root, &source);
        let (exe, build) = modes.build(root, &source, &format!("invalid_literal_{mode}"));
        assert!(!exe.exists());
        for result in [check, build] {
            assert!(!result.status.success());
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("error[E2001]"),
                "{}",
                render(&result)
            );
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("expected &[u8], got str"),
                "{}",
                render(&result)
            );
            assert!(!String::from_utf8_lossy(&result.stderr).contains("SemanticTypeId("));
            assert!(!String::from_utf8_lossy(&result.stderr).contains("DEBUG"));
        }
    }
}
