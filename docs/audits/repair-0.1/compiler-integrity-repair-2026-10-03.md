<!-- luna-doc-role: evidence -->

# Compiler integrity repair — 2026-10-03

Base: `6a3ec9f787523b26489e4b51531471896d4d67b2`, branch
`codex/antigravity-repair-0.1`, checkout `D:/fdlang`. This follow-up addresses
the three independently reproduced defects in that revision. It does not
implement namespace opening or `luna.toml`; the maintainer explicitly deferred
that work until a later instruction.

## Provider authority

Before: `import <slice>` with `-I` could select user source or a relocated
artifact and inherit `SysrootCanonical` just because the selected sysroot's
manifest contained the logical name. Direct/local imports correctly rejected
the same inherent slice implementation.

After: `ExternalComponentDiscovery` produces an ordinary project candidate.
`Sysroot::discover_provider` authenticates its actual canonical file path
against registered provider entries and containment within the selected
canonical external root before attaching capabilities. Canonical source,
artifacts, package entries and supported legacy candidates use that same rule.
Files copied outside that root gain no capability, including valid artifacts.
No provider-name comparison or new intrinsic is introduced.

Permanent controls:

- [CLI matrix](../../../luna-rs/crates/luna-cli/tests/compiler_trust_diagnostics_cli_parity.rs):
  rogue source under `-I` rejects E2006 with both source and artifact sysroots;
  fresh relocated slice artifact rejects E2006 with artifact dependencies;
  direct rogue source rejects; canonical slice iteration builds, links and runs
  in both modes; ordinary imported nominal inherent methods remain accepted.
- Mixed relocated artifact/source dependencies reject E6001 for a dependency
  interface fingerprint mismatch before authorization. This remains a strict
  rejection, with no source fallback or validator bypass.
- [Authority invariant](../../../luna-rs/crates/luna-driver/tests/provider_authority_tests.rs):
  a custom logical name and physical stem prove that granting authority is
  independent of stdlib names, with `.ln`, `.llib`, `.ms` and `.mlib` controls.
  The Unix-only symlink-escape assertion is not executed on this Windows host.

## Typed error identity

Previously uncoded move-analysis diagnostics now use E3006 for uninitialized
values, E3007 for use after drop, and E3001 for a conditional-move drop error.
Missing-return and local-borrow escape diagnostics have typed identities;
region diagnostics no longer embed a conflicting `error[E...]` prefix in their
message. Existing region tests assert actual codes instead of hidden strings.
The uncoded explicit-destructor call and driver I/O/backend/linker paths also
receive typed codes. Existing code numbers are preserved; new categories are
recorded in the [registry](../../diagnostics/diagnostics-v1.md).

The real [uninitialized fixture](../../../tests/luna/language/compiler_integrity/uninitialized.ln)
rejects with E3006 and primary location 3:12 in both provider modes.

Full **DIAG-1..10 remains PARTIAL**. This repair does not implement mandatory
source/global constructors, nonoptional code/span representation, structural
related labels, complete per-code conformance coverage or `DiagnosticCode::ALL`.
A registry review verifies that all 66 implemented code names and numbers match
the specification and that numerical identities are unique. The repair ledger
no longer marks the entire diagnostic contract PASS.

## Unicode rendering

Spans remain UTF-8 byte offsets. `SourceFile::get_line_col` reports one-based
Unicode scalar columns; the renderer measures Unicode display width, expands
tabs to four-column stops and clips a multi-line underline to its primary line.
Controls cover Vietnamese/accented text, wide characters, combining marks,
joined emoji, CRLF, invalid offsets and non-ASCII highlight lengths.

The real [missing-field fixture](../../../tests/luna/language/compiler_integrity/unicode_missing_field.ln)
now emits E1001 at line 2, column 61 with four carets exactly below `nope`, in
both source and artifact modes.

## Verification and limits

The final commands, counts, compiler/runtime hashes and captured logs are in
[the verification record](evidence/compiler-integrity/verification.json).
All source/artifact CLI roots are independently produced by the official
`build-sysroot` path. Source mode contains no `.llib`/`.obj`; artifact mode
contains no `.ln`. The positive slice fixture is linked and executed, rather
than only typechecked.

Evidence covers Windows x86_64 GNU with LLVM 18 and the existing runtime static
library. Native POSIX/Darwin validation, full workspace tests, full provider API
certification and all diagnostic invariants are not claimed. W8 and the native
portability gate remain PARTIAL.

Compiler change classification:

| Capability | Why the library exposed it | Generic benefit | New intrinsic/lang item | Stdlib-specific branch |
|---|---|---|---|---|
| Authenticate discovered provider authority | Slice inherent methods require a manifest capability | Any capability-bearing provider is authenticated independently of its name and import spelling | No | No |
| Preserve structured error identity | Ownership and artifact failures lacked codes | Arbitrary user declarations and compiler output failures obtain stable typed errors | No | No |
| Render byte spans correctly for Unicode | Real source snippets precede errors with Unicode | Every source diagnostic uses the same location/rendering mechanism | No | No |

SKILL IMPACT: none. Existing skills already require authentic provenance,
negative controls and fresh source/artifact validation; this task adds
implementation and permanent regression evidence for those requirements.
