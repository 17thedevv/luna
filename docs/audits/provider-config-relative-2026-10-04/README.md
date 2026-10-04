<!-- luna-doc-role: evidence -->

# Relative-only provider configuration — 2026-10-04

Base: `4a5adf42a4fdb48b640acaa14c097adb675a9b60`, branch
`codex/antigravity-repair-0.1`. The maintainer amended
[PROVIDER-CONFIG-v1](../../spec/0.1/provider-config-v1.md) to forbid absolute
machine-specific paths in project configuration before 0.1 freeze.
Status: **IMPLEMENTED / CLI SOURCE-ARTIFACT MATRIX PASS** on Windows x86_64 GNU.
This record does not close previous workspace correctness gaps or certify
release conformance or other native targets.

## Contract and implementation

Every provider value in the selected `luna.toml` must be a relative file stem.
Relative values, including `../` and `../../`, resolve from that configuration's
absolute parent directory. Discovery cwd and transitive importer location do not
change the selected base. Forward `/` is the canonical recommended separator.

The CLI loader rejects rooted POSIX/Windows paths, drive-qualified paths
(including drive-relative `C:foo` and bare `C:`), UNC and device/extended paths
with E6008. Lexical foreign-prefix checks prevent host-specific `is_absolute()`
behavior from accepting Windows roots on Unix. Home prefixes, `$`/`%` marker
characters and control characters reject too, even in unused entries.
No filesystem read or environment expansion is needed for that validation.

After validation, the loader joins the stem to the selected config directory.
Absolute `--config FILE`, `-I`/`--search-path` arguments and internal typed driver
inputs remain supported. The driver, provider identity, artifact validation,
parser, semantic phases, borrow checking and backend are unchanged. No machine
local override format is added.

## Verification

The existing public [CLI harness](../../../luna-rs/crates/luna-cli/tests/alpha_modules_cli.rs)
uses 57 permanent Luna fixture files with source-only and freshly built
artifact-only sysroots; custom provider sources are absent during artifact
consumption. The former absolute-provider positive case now requires E6008.

Eighteen distinct forbidden path strings are checked through `check`, `build`
and `run` in both modes, including unused mappings and source snippets identifying
the invalid config value. An actual existing absolute provider is also rejected;
a TOML-escaped newline is rejected as a control character. This gives 116 path
rejection assertions across both modes. Relative `../` and `../../` values are
verified through all three commands, using absolute explicit config filenames
and a different cwd (12 positive command assertions). Existing nearest-config,
relocation, alias, transitive lookup, strict artifact rejection and no-config
behavior remain covered by the matrix.

| Command | Actual result | Evidence |
|---|---|---|
| `cargo test --manifest-path luna-rs/Cargo.toml -p luna-cli --test alpha_modules_cli -j2 -- --nocapture` | PASS; one Rust orchestration test, source/artifact matrix, 87.73 s | [CLI log](evidence/cli.log) |
| `cargo check --manifest-path luna-rs/Cargo.toml --workspace -j2` | PASS; existing warnings remain | [Check log](evidence/check.log) |
| Documentation generation and local validation | Recorded separately | [Documentation log](evidence/documentation.log) |

[verification.json](evidence/verification.json) records exact tested-file and
log hashes, CLI/runtime identity, environment and command exits. Tests ran on
Windows; foreign prefixes are covered lexically, not presented as a Unix native
build. The older [module implementation record](../alpha-modules-2026-10-03/README.md)
retains its original logs and source hashes, including the formerly permitted
absolute-value policy. Full workspace tests were not repeated for this bounded
CLI-only change; the earlier workspace failures remain open.

SKILL IMPACT: **none**. Existing skills defer to the versioned contract; none
contains an independent absolute-provider policy needing correction.
