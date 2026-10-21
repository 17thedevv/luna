"""Record stable baseline provenance and migrate historical findings without promoting PASS."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/stable/0.1.0'


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    if (OUT / 'baseline.json').exists():
        raise SystemExit('Baseline already recorded; do not overwrite provenance')
    old = json.loads((ROOT / 'docs/audits/0.1-alpha-completion-2026-10-04/r4/release-ledger.json').read_text())
    legacy = json.loads((ROOT / 'docs/audits/0.1-alpha-completion-2026-10-04/r4/legacy-status.snapshot.json').read_text())
    closure_path = ROOT / 'docs/audits/r5-cpu-baseline-2026-10-20/release-closure.json'
    closure = json.loads(closure_path.read_text())
    format_source = (ROOT / 'luna-rs/crates/luna-llib/src/format.rs').read_text()
    versions = {name: int(re.search(r'pub const ' + constant + r': u16 = (\d+)', format_source)[1]) for name, constant in [
        ('compiler', 'LLIB_COMPILER_VERSION'), ('metadata', 'SEMANTIC_METADATA_VERSION'),
        ('mvir', 'LLIB_MVIR_VERSION'), ('format', 'LLIB_FORMAT_VERSION')]}
    scopes = []
    for record in old['records']:
        scopes.append(dict(id='SCOPE-' + record['id'], requirement=record['title'],
                           historical_status=record['implementation_status'],
                           historical_tested_commit=record.get('verified_at_commit'),
                           historical_closure_commit=record.get('closure_commit'),
                           historical_evidence=record.get('evidence', []),
                           historical_limits=record.get('remaining_gaps', []),
                           suites=record.get('suites', []),
                           stable_status='ACCEPTED_DEFERRAL' if record['implementation_status'] == 'DEFERRED_NON_BLOCKING' else 'REQUALIFICATION_REQUIRED',
                           current_execution_evidence=[],
                           targets=['x86_64-pc-windows-gnu', 'x86_64-unknown-linux-gnu']))
    for record in legacy:
        scopes.append(dict(id='LEGACY-' + record['id'], requirement=record['contract'],
                           historical_status=record['status'], historical_record=record,
                           stable_status='RECONCILE_WITH_CURRENT_REGRESSION', current_execution_evidence=[]))
    items = [
        ('STABLE-001', 'S0', 'P1', 'Reconcile current publication/status pages and immutable SHA provenance', 'OPEN', ['docs/spec/0.1/README.md', 'docs/spec/0.1/gaps.md', 'docs/spec/0.1/conformance.md']),
        ('STABLE-002', 'S0', 'P1', 'Reconcile retained grammar coverage and V01-DESIGN-04 without adopting parser-only syntax', 'AUDIT_REQUIRED', ['docs/grammar.ebnf', 'docs/spec/0.1/syntax.md']),
        ('STABLE-003', 'S1', 'P2', 'Remove unconditional importer DEBUG stderr while preserving source diagnostics', 'CONFIRMED', ['luna-rs/crates/luna-driver/src/importer.rs']),
        ('STABLE-004', 'S1', 'P2', 'Retire invalid empty ignored fixture with explicit replacement coverage', 'CONFIRMED', ['luna-rs/crates/luna-driver/tests/sem_maturity_phase2b.rs']),
        ('STABLE-005', 'S2', 'P1', 'Qualify Lunar with the current candidate compiler and fresh stdlib', 'OPEN', ['docs/tools/smoke_lunar.py']),
        ('STABLE-006', 'S3', 'P1', 'Adopt maintainer-selected GPL-3.0-only; preserve third-party notices and corresponding source provenance', 'AUTHORIZED', []),
        ('STABLE-007', 'S3', 'P1', 'Make CLI/distribution versions agree and bundle Lunar in both native packages', 'OPEN', ['docs/tools/package_luna_release.py', 'luna-rs/crates/luna-cli/Cargo.toml']),
        ('STABLE-008', 'S3', 'P1', 'Build versioned Windows installer with candidate Luna and Lunar; upgrade/uninstall and hash controls', 'OPEN', ['distribution/windows/build_installer.py', 'distribution/windows/luna.iss']),
        ('STABLE-009', 'S3', 'P1', 'Publish consistent install/reference/download docs and verify live website', 'OPEN', ['docs/installation.md', 'luna-web']),
        ('STABLE-010', 'S4', 'P1', 'Reacquire eight exact-pin qualification lanes and verify all archives', 'NOT_STARTED', []),
        ('STABLE-011', 'S4', 'P1', 'Clean Windows/Ubuntu package + installer consumers using shipped compiler/Lunar', 'NOT_STARTED', []),
        ('STABLE-012', 'S5', 'P1', 'Exercise three external RC projects, reconcile defects and maintainer release review', 'NOT_STARTED', []),
        ('STABLE-013', 'S0', 'P1', 'Document source/API/ABI/artifact compatibility policy for maintainer review', 'OPEN', []),
        ('STABLE-014', 'S1', 'P1', 'Run bounded malformed parser/import/LLIB corpus; retain budgets and failures', 'NOT_STARTED', []),
    ]
    backlog = dict(schema=1, owner='17thedevv', status='IN_PROGRESS_NOT_QUALIFIED',
                   evidence_policy='Historical scoped PASS is input evidence only, never a stable candidate PASS.',
                   items=[dict(id=i, milestone=m, priority=p, requirement=q, status=s,
                               owner='17thedevv', source_paths=paths, fix_commit=None,
                               evidence=[], blocks=['STABLE_RELEASE']) for i, m, p, q, s, paths in items])
    baseline = dict(schema=1, date='2026-10-21', status='INTEGRATED_NOT_QUALIFIED',
                    source_commit=git('rev-parse', 'HEAD'),
                    parents=git('show', '-s', '--format=%P', 'HEAD').split(),
                    source_trees={part: git('rev-parse', 'HEAD:' + part) for part in ['luna-rs', 'runtime', 'tests']},
                    versions=versions, provider_count=len(re.findall(r'^\[\[provider\]\]', (ROOT / 'luna-rs/libs/external/sysroot.toml').read_text(), re.M)),
                    alpha=dict(status=closure['status'], source_record=str(closure_path.relative_to(ROOT)).replace('\\', '/'),
                               record_sha256=hashlib.sha256(closure_path.read_bytes()).hexdigest(),
                               historical_packaged_source=closure['tag']['peeled_commit'], historical_qualified_pin=closure['qualification']['tested_implementation_pin']),
                    history_mapping=[dict(old='b5cb77c3286f583c43c41d0e56f4136903ebb842', current='297d6e1a0b208baa748c461d59fa8ed4d0533b2e'),
                                     dict(old='cd9cd1884d2e49c0f03979597ca73e92043b7fe7', current='a3080712a458fc581c13c2b1ac5313c759c05f1c')],
                    license_decision=dict(spdx='GPL-3.0-only', authority='Human maintainer reply 2026-10-21'),
                    qualified_lanes=0, stable_tag=None)
    for mapping in baseline['history_mapping']:
        mapping['current_tree'] = git('rev-parse', mapping['current'] + '^{tree}')
    save(OUT / 'baseline.json', baseline)
    save(OUT / 'backlog.json', backlog)
    save(OUT / 'scope-matrix.json', dict(schema=1, status='REQUALIFICATION_REQUIRED', records=scopes))
    print(json.dumps(dict(baseline=baseline['source_commit'], versions=versions, providers=baseline['provider_count'],
                          backlog=len(items), historical_scope_records=len(scopes)), indent=2))


if __name__ == '__main__':
    main()
