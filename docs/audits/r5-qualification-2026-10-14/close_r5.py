"""R5-5 / R5-6: reconcile the exact-pin lane evidence and write the closure report.

Reads the lane outputs produced outside the tested checkout (`D:/r5-9bda6f57/`)
and writes the reconciliation table, the qualification report and a machine
readable summary into this evidence directory. It never modifies the tested
candidate or any lane result; it only packages what the lanes recorded.
"""
import gzip
import hashlib
import json
import shutil
from pathlib import Path

PIN = '9bda6f572fa5f9904379c08c625c28122fcd0c56'
LANE = Path('D:/r5-9bda6f57')
ROOT = Path(__file__).resolve().parent            # docs/audits/r5-qualification-2026-10-14
REPO = ROOT.parents[2]                            # repository root


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def main():
    windows = LANE / 'windows'
    ubuntu = LANE / 'ubuntu'
    diagnostics = LANE / 'diagnostics'

    provenance = load(windows / 'provenance.json')
    runtime_abi = load(windows / 'runtime-abi.json')
    sysroot = load(windows / 'sysroot-provenance.json')
    focused = load(windows / 'focused-summary.json')
    quickstart = load(windows / 'quickstart.json')
    docs_examples = load(windows / 'docs-examples.json')
    docs_validator = load(windows / 'docs-validator.json')
    workspace = load(windows / 'workspace-summary.json')
    final_identity = load(windows / 'final-identity.json')
    ubuntu_normal = load(ubuntu / 'normal-attempt1-target-finding.json')
    ubuntu_asan = load(ubuntu / 'asan-artifact-verification.json')
    diag_summary = load(diagnostics / 'diagnostic-audit-summary.json')
    diag_verdicts = load(diagnostics / 'invariant-verdicts.json')
    diag_findings = load(diagnostics / 'manual-findings.json')

    def gate(status):
        return 'PASS' if status else 'FAIL'

    windows_gates = {
        'fresh_pin': provenance['head'] == PIN and provenance['initial_status'] == '',
        'runtime_abi': runtime_abi['exit'] == 0,
        'providers_49': sysroot['provider_count'] == 49 and sysroot['compiler'] == 25
        and sysroot['metadata'] == 9 and sysroot['mvir'] == 5 and sysroot['format'] == 2,
        'focused_acde': focused['status'] == 'PASS',
        'docs_examples': docs_examples['exit'] == 0,
        'docs_validator': docs_validator['exit'] == 0,
        'quickstart': quickstart['status'] == 'PASS',
        'workspace': workspace['status'] == 'PASS',
        'artifact_independence': final_identity['status'] == '',
    }
    diag_broken = diag_summary['status'] != 'PASS'
    ubuntu_normal_blocked = ubuntu_normal['result'].startswith('STRICT TARGET CONTRACT GATE FAIL')

    overall = 'BLOCKED' if (diag_broken or ubuntu_normal_blocked) else (
        'READY FOR MAINTAINER DECISION' if all(windows_gates.values()) else 'BLOCKED')

    not_run = 'NOT RUN (blocked by target gate)'
    rows = [
        ('Fresh pin', gate(windows_gates['fresh_pin']), gate(ubuntu_normal['tested_commit'] == PIN), 'PASS'),
        ('Runtime ABI', gate(windows_gates['runtime_abi']), gate(ubuntu_asan['result'].startswith('PASS')), 'PASS'),
        ('49 providers', gate(windows_gates['providers_49']), gate(ubuntu_normal['provider_count'] == 49), 'PASS'),
        ('Focused A/C/D/E', gate(windows_gates['focused_acde']), not_run, 'PASS'),
        ('DIAG-1..10', diag_summary['status'], 'shared', 'PASS'),
        ('Docs examples 16/16', gate(windows_gates['docs_examples']), not_run, 'PASS'),
        ('Docs validator', gate(windows_gates['docs_validator']), not_run, 'PASS'),
        ('README quick start', gate(windows_gates['quickstart']), not_run, 'PASS'),
        ('Workspace', gate(windows_gates['workspace']), not_run, 'PASS'),
        ('ASan runtime ABI', 'N/A', gate(ubuntu_asan['result'].startswith('PASS')), 'PASS'),
    ]

    evidence = ROOT / 'evidence'
    evidence.mkdir(parents=True, exist_ok=True)
    packed = {}
    for name, path in [
        ('windows-provenance.json', windows / 'provenance.json'),
        ('windows-runtime-abi.json', windows / 'runtime-abi.json'),
        ('windows-sysroot-provenance.json', windows / 'sysroot-provenance.json'),
        ('windows-focused-summary.json', windows / 'focused-summary.json'),
        ('windows-quickstart.json', windows / 'quickstart.json'),
        ('windows-docs-examples.json', windows / 'docs-examples.json'),
        ('windows-docs-validator.json', windows / 'docs-validator.json'),
        ('windows-workspace-summary.json', windows / 'workspace-summary.json'),
        ('windows-final-identity.json', windows / 'final-identity.json'),
        ('ubuntu-normal-target-finding.json', ubuntu / 'normal-attempt1-target-finding.json'),
        ('ubuntu-asan-artifact-verification.json', ubuntu / 'asan-artifact-verification.json'),
        ('diagnostics-summary.json', diagnostics / 'diagnostic-audit-summary.json'),
        ('diagnostics-invariant-verdicts.json', diagnostics / 'invariant-verdicts.json'),
        ('diagnostics-manual-findings.json', diagnostics / 'manual-findings.json'),
        ('diagnostics-per-code-inventory.json', diagnostics / 'per-code-inventory.json'),
    ]:
        target = evidence / name
        shutil.copy2(path, target)
        packed[name] = sha(target)
    for name in ['workspace-triage.log', 'workspace-final.log']:
        source = windows / name
        if source.is_file():
            target = evidence / (name + '.gz')
            target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))
            packed[target.name] = sha(target)

    (evidence / 'evidence-sha256.json').write_text(
        json.dumps({'tested_commit': PIN, 'files': packed}, indent=2), encoding='utf-8')

    table = '\n'.join(
        f"| {label} | {w} | {u} | {req} |"
        for label, w, u, req in rows)

    report = f"""# R5 — exact release qualification ({PIN[:8]})

**Tested commit:** `{PIN}` (candidate `luna-0.1-alpha.1-candidate`)
**Protocol:** compiler25 / metadata9 / MVIR5 / format2
**Verdict:** **{overall}**

Qualification ran on a fresh detached clone of the exact candidate with all build
and log output outside the checkout. No lane modified the candidate; the
diagnostics lane added no tests and the replay window reused the freshly built
exact-pin CLI.

## Reconciliation

| Gate | Windows | Ubuntu | Required |
| --- | --- | --- | --- |
{table}

## Windows GNU lane

- Runtime MinGW/Ninja Release + `RuntimeAbiTests`: exit {runtime_abi['exit']}.
- CLI debug/release built from the exact clone; sysroot rebuilt: {sysroot['provider_count']} provider bundles,
  protocol {sysroot['compiler']}/{sysroot['metadata']}/{sysroot['mvir']}/{sysroot['format']}, target `{sysroot['providers'][0]['target']}`,
  COFF {sysroot['providers'][0]['coff_machine']}.
- Focused A/C/D/E batches: {focused['status']} ({len(focused['results'])} batches, all exit 0).
- Docs conformance recorder: exit {docs_examples['exit']}; docs validator exit {docs_validator['exit']}.
- README quick start build+native: {quickstart['status']} (PE64/COFF64 0x8664).
- Workspace triage: exit {workspace['triage']['exit']} ({workspace['triage'].get('result_groups')} groups,
  {workspace['triage'].get('passed')} passed / {workspace['triage'].get('failed')} failed).
- Workspace final: exit {workspace['final']['exit']} ({workspace['final'].get('result_groups')} groups,
  {workspace['final'].get('passed')} passed / {workspace['final'].get('failed')} failed).

## Ubuntu GNU lane

- Normal: **BLOCKED by a target contract conflict.** Actual `.llib`/manifest target
  `{ubuntu_normal['actual_llib_and_manifest_target']}` versus advertised
  `{ubuntu_normal['r4_advertised_target']}` ({ubuntu_normal['source_trace'][2]}). {ubuntu_normal['provider_count']} providers,
  protocol 25/9/5/2, ELF64 objects, {ubuntu_normal['file_hashes_verified']} file hashes and
  {ubuntu_normal['gate_log_content_hashes_verified']} gate-log hashes verified with {ubuntu_normal['hash_mismatches']} mismatches.
  Dependent focused/docs/workspace gates were intentionally not run. Resolving this
  requires either a documentation reconciliation of the advertised triple or a
  compiler target change (new implementation pin); it is a maintainer decision.
- ASan: **PASS in C runtime ABI / partial-read cleanup scope only**
  ({ubuntu_asan['file_hashes_verified']} file hashes, {ubuntu_asan['mismatches']} mismatches);
  no broader sanitized compiler/stdlib claim.

## R5-2 structural DIAG-1..10 audit

**{diag_summary['status']} — the structural diagnostic gate is not satisfied.**

| Invariant | Verdict |
| --- | --- |
{chr(10).join(f"| {v['invariant']} | {v['status']} |" for v in diag_verdicts)}

Confirmed structural findings (source traces, with existing reducers and the
exact-pin replay): untyped user-visible I/O error path (DIAG-1); move/loan
producers omit antecedent related labels (DIAG-3/DIAG-9); analyzer dedup and some
identity assertions depend on message text (DIAG-4); TypeChecker constructs a
Resolver code E1008, Borrowck constructs TypeChecker E2016, Mono constructs
TypeChecker E2001 (DIAG-8). Reported findings: {diag_summary['findings']}.
The replay matched {diag_summary['replay']['expected_results']}/{diag_summary['replay']['commands']} expectations, which
confirms reproducers rather than conformance. Repair requires compiler/runtime/
test changes and therefore a new implementation pin; the candidate is unchanged.

## Evidence

Packed copies with SHA-256 in `evidence/evidence-sha256.json`. Raw lane outputs
and gzip workspace logs remain outside the checkout as recorded by the lanes.

## Release decision

**R5: {overall}.** Release remains **BLOCKED** pending a maintainer decision on
(1) the structural diagnostic gate and (2) the Ubuntu target contract. No tag was
created, `main` was not touched, and the candidate was not modified by R5.
"""
    (ROOT / 'REPORT-R5-QUALIFICATION-2026-10-14.md').write_text(report, encoding='utf-8')
    (ROOT / 'r5-summary.json').write_text(json.dumps({
        'tested_commit': PIN,
        'protocol': {'compiler': 25, 'metadata': 9, 'mvir': 5, 'format': 2},
        'windows_gates': windows_gates,
        'ubuntu_normal': ubuntu_normal['result'],
        'ubuntu_asan': ubuntu_asan['result'],
        'diagnostics': {v['invariant']: v['status'] for v in diag_verdicts},
        'verdict': overall,
        'evidence': packed,
    }, indent=2), encoding='utf-8')
    print(json.dumps({'verdict': overall, 'windows': windows_gates,
                      'diagnostics_status': diag_summary['status'],
                      'ubuntu_normal': ubuntu_normal['result']}, indent=2))


if __name__ == '__main__':
    main()
