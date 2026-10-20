"""Validate R4 documentation provenance without running language qualification.

Only reads the repository. --require-unchanged-implementation is the R4-only
closing gate; R5 implementation repairs need their own newly pinned evidence.
"""
from pathlib import Path
import argparse
import collections
import gzip
import hashlib
import io
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
R4 = ROOT / 'docs/audits/0.1-alpha-completion-2026-10-04/r4'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--require-unchanged-implementation', action='store_true')
args = parser.parse_args()
errors = []
blob_cache = {}


def require(condition, message):
    if not condition:
        errors.append(message)


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def git(*arguments):
    return subprocess.check_output(['git', *arguments], cwd=ROOT)


def repository_bytes(name):
    """Index content is portable; permit only checkout EOL changes in text."""
    blob = blob_cache[name] if name in blob_cache else git('show', ':' + name)
    raw = existing_path(name).read_bytes()
    text = Path(name).suffix not in ('.gz', '.obj', '.llib', '.exe', '.a', '.lib', '.png', '.zip') and b'\0' not in blob
    require(raw == blob or text and raw.replace(b'\r\n', b'\n') == blob.replace(b'\r\n', b'\n'), 'Worktree content differs from repository content: ' + name)
    return blob


def prime_blob_cache(names):
    names = sorted(set(names))
    result = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT,
        input=''.join(':' + name + '\n' for name in names).encode(), capture_output=True, check=True)
    stream = io.BytesIO(result.stdout)
    for name in names:
        header = stream.readline().decode().strip().split()
        if len(header) != 3 or header[1] != 'blob':
            raise ValueError('Evidence must be staged/tracked before validation: ' + name)
        data = stream.read(int(header[2]))
        if stream.read(1) != b'\n':
            raise ValueError('Invalid Git batch framing')
        blob_cache[name] = data


def schema_check(value, rule, schema, label):
    if '$ref' in rule:
        target = schema
        for part in rule['$ref'].removeprefix('#/').split('/'):
            target = target[part]
        return schema_check(value, target, schema, label)
    if 'anyOf' in rule:
        # This schema uses anyOf only for nullable full commit strings.
        require(value is None or isinstance(value, str) and re.fullmatch(r'[0-9a-f]{40}', value), label + ': invalid nullable commit')
        return
    types = dict(object=dict, array=list, string=str, integer=int, boolean=bool, null=type(None))
    accepted = rule.get('type')
    if accepted:
        names = accepted if isinstance(accepted, list) else [accepted]
        if not any(type(value) is types[name] for name in names):
            require(False, label + ': invalid type')
            return
    if 'const' in rule:
        require(value == rule['const'], label + ': wrong constant')
    if isinstance(value, str) and 'pattern' in rule:
        require(re.fullmatch(rule['pattern'], value) is not None, label + ': invalid pattern')
    if isinstance(value, dict):
        for key in rule.get('required', []):
            require(key in value, label + ': missing ' + key)
        for key, child in rule.get('properties', {}).items():
            if key in value:
                schema_check(value[key], child, schema, label + '.' + key)
    if isinstance(value, list):
        require(len(value) >= rule.get('minItems', 0), label + ': too few items')
        if 'items' in rule:
            for i, child in enumerate(value):
                schema_check(child, rule['items'], schema, f'{label}[{i}]')


def existing_path(name):
    path = (ROOT / name).resolve()
    require(path.is_relative_to(ROOT) and path.is_file(), 'Missing or out-of-repository evidence: ' + name)
    return path


def ancestor(commit, base):
    require(re.fullmatch(r'[0-9a-f]{40}', commit) is not None, 'Non-exact commit: ' + commit)
    require(git('rev-parse', commit + '^{commit}').decode().strip() == commit, 'Unresolvable commit: ' + commit)
    result = subprocess.run(['git', 'merge-base', '--is-ancestor', commit, base], cwd=ROOT, capture_output=True)
    require(result.returncode == 0, 'Canonical record is outside base ancestry: ' + commit)


try:
    ledger = read(R4 / 'release-ledger.json')
    schema = read(R4 / 'release-ledger.schema.json')
    schema_check(ledger, schema, schema, 'ledger')
    base = ledger['base_commit']
    tested = ledger['implementation_tested_commit']
    ancestor(tested, base)
    require(not git('diff', tested, base, '--', 'luna-rs', 'runtime', 'tests').strip(), 'R4 base differs from tested implementation')
    if args.require_unchanged_implementation:
        require(not git('diff', base, '--', 'luna-rs', 'runtime', 'tests').strip(), 'Implementation/runtime/tests changed during R4')
        extra = git('ls-files', '--others', '--exclude-standard', '--', 'luna-rs', 'runtime', 'tests').strip()
        require(not extra, 'New nonignored implementation/runtime/test files during R4')
    ids = [row['id'] for row in ledger['records']]
    require(len(ids) == len(set(ids)), 'Duplicate record IDs')
    required_ids = {f'A{i}' for i in range(1, 7)} | {f'C{i}' for i in range(1, 6)} | {f'D{i}' for i in range(1, 5)} | {'A2-FU1','A2-FU2','C4-FU1','D1-FU1','D1-FU2','D4-FU1','FIND-ASYNC-PROVIDER-01','R5-HARNESS-01','E1','E2'}
    require(required_ids == set(ids), 'Missing or unexpected wave/follow-up IDs')
    evidence = {row['path']: row for row in ledger['evidence']}
    require(len(evidence) == len(ledger['evidence']), 'Duplicate evidence paths')
    protected = read(R4 / 'evidence/preservation-manifest.json')
    migration = ledger['legacy_migration']
    prime_blob_cache(list(evidence) + [row['path'] for row in protected['files']] + [migration['source'], migration['snapshot']])
    archives = 0
    for name, row in evidence.items():
        p = existing_path(name)
        if not p.is_file():
            continue
        require(row['hash_basis'] == 'git_blob_bytes', 'Ambiguous hash basis: ' + name)
        require(digest(repository_bytes(name)) == row['sha256'], 'Repository evidence hash mismatch: ' + name)
        if row.get('acquired_bytes_archive'):
            archived = gzip.decompress(existing_path(row['acquired_bytes_archive']).read_bytes())
            require(digest(archived) == row['acquired_workspace_sha256'], 'Acquired byte hash mismatch: ' + name)
        if p.suffix == '.gz':
            gzip.decompress(p.read_bytes())
            archives += 1
        if p.suffix == '.json':
            read(p)
        if row['verified_at_commit']:
            ancestor(row['verified_at_commit'], base)
    gate_path = ROOT / 'docs/audits/0.1-alpha-completion-2026-10-04/evidence/e1-workspace-final6-2026-10-13.json'
    gate = read(gate_path)
    require(gate['source_sha'] == tested and gate['three_consecutive_passes'] and len(gate['runs']) == 3, 'Invalid final6 qualification summary')
    suite_sets = []
    for row in gate['runs']:
        p = gate_path.parent / row['log_archive']
        raw = gzip.decompress(p.read_bytes())
        require(digest(raw) == row['log_sha256'], 'Raw final6 log hash mismatch: ' + p.name)
        text = raw.decode('utf-8', errors='replace')
        totals = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        sums = [sum(int(x[i]) for x in totals) for i in range(3)]
        require(sums == [row['passed'], row['failed'], row['ignored']] == [1404, 0, 1], 'Final6 result counts mismatch')
        require(row['exit'] == 0 and row['head_before'] == row['head_after'] == tested and row['code_diff_empty_after'], 'Final6 identity/exit failure')
        suite_sets.append(set(re.findall(r'Running tests[\\/]([^ ]+)\.rs', text)))
    for row in ledger['records']:
        ancestor(row['closure_commit'], base)
        for pin in row['implementation_commits']:
            ancestor(pin, base)
        require(row['implementation_commit'] == (row['implementation_commits'][0] if row['implementation_commits'] else None), 'Implementation chain mismatch: ' + row['id'])
        for path in row['evidence']:
            require(path in evidence, 'Unregistered evidence reference: ' + path)
        for suite in row['suites']:
            require(all(suite in names for names in suite_sets), 'Suite absent from a final6 run: ' + suite)
        if row['verified_at_commit']:
            require(row['verified_at_commit'] == tested, 'Current verification pin mismatch: ' + row['id'])
    matrix_rows = 0
    for profile in ('debug', 'release'):
        for mode in ('source', 'artifact'):
            stem = f'e1-final5-{profile}-{mode}-2026-10-13'
            rows = read(gate_path.parent / (stem + '.json'))
            meta = read(gate_path.parent / (stem + '.meta.json'))
            require(meta['source_sha'] == tested and meta['contract_met'] and len(rows) == meta['rows'] == 59, 'Matrix metadata mismatch: ' + stem)
            require(all(x['contract_met'] for x in rows), 'Matrix failure: ' + stem)
            require(dict(collections.Counter(x['outcome'] for x in rows)) == dict(COMPILE_ERROR=4, TRAP=34, PASS=9, WRAP=12), 'Matrix outcome mismatch: ' + stem)
            matrix_rows += len(rows)
    migration = ledger['legacy_migration']
    source = existing_path(migration['source'])
    snapshot = existing_path(migration['snapshot'])
    require(repository_bytes(migration['source']) == repository_bytes(migration['snapshot']), 'Legacy repository snapshot is not byte-identical')
    require(digest(repository_bytes(migration['source'])) == migration['source_sha256'], 'Legacy ledger repository hash mismatch')
    require(digest(gzip.decompress(existing_path(migration['snapshot_raw_archive']).read_bytes())) == migration['source_workspace_sha256'], 'Legacy acquired bytes changed')
    original = read(source)
    migrated = read(existing_path(migration['output']))
    require(len(original) == len(migrated['records']) == migration['record_count'] == 24, 'Legacy record count changed')
    require([x['historical_record'] for x in migrated['records']] == original, 'Legacy fields lost or rewritten')
    require(all(x['implementation_commit'] is None and x['verified_at_commit'] is None and x['preexisting_at_baseline'] for x in migrated['records']), 'Historical provenance was fabricated')
    for row in read(R4 / 'evidence/historical-path-resolution.json'):
        require(row['accessible'], 'Historical local reference was unavailable at reconciliation: ' + row['original_reference'])
        if 'archive' in row:
            require(digest(gzip.decompress(existing_path(row['archive']).read_bytes())) == row['raw_sha256'], 'Historical raw archive hash mismatch')
    protected = read(R4 / 'evidence/preservation-manifest.json')
    for row in protected['files']:
        p = existing_path(row['path'])
        if p.is_file():
            require(digest(repository_bytes(row['path'])) == row['repository_sha256'], 'Existing saved repository evidence changed: ' + row['path'])
    targets = ledger['target_qualification']
    require(len(targets) == 3 and targets[0]['status'] == 'VERIFIED_IN_TESTED_SCOPE', 'Unexpected Windows qualification')
    require(all(x['status'] == 'NOT_VERIFIED' and not x['evidence'] and x['verified_at_commit'] is None for x in targets[1:]), 'Ubuntu qualification was invented')
except Exception as exc:
    errors.append(f'{type(exc).__name__}: {exc}')

print(json.dumps(dict(task='R4 documentation provenance validation', records=len(ledger['records']) if 'ledger' in globals() else 0,
    evidence_files=len(evidence) if 'evidence' in globals() else 0, readable_gz=archives if 'archives' in globals() else 0,
    arithmetic_acquisitions=matrix_rows if 'matrix_rows' in globals() else 0,
    preserved_evidence_files=len(protected['files']) if 'protected' in globals() else 0,
    release_status='BLOCKED_PENDING_R5', implementation_tree_required=args.require_unchanged_implementation,
    errors=errors), indent=2))
sys.exit(1 if errors else 0)
