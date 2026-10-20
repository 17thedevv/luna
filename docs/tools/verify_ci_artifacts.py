"""Verify downloaded R5 ZIP bytes and the evidence inside them, without executing it.

The input manifest contains GitHub artifact metadata (id/name/digest/size and
workflow_run) plus local_path for each downloaded ZIP. Signed URLs are not saved.
This verifies acquired evidence; it does not grant release or freeze authority.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import struct
import subprocess
import zipfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def workspace_totals(log):
    text = log.decode('utf-8', errors='replace')
    totals = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    assert totals and all(t[0] == 'ok' and int(t[2]) == 0 for t in totals)
    for suite in ('missing_return_cli', 'native_math_link_cli', 'move_origin_join_cli',
                  'diagnostic_declaration_locations_cli'):
        assert suite in text, f'Missing workspace regression suite: {suite}'
    return dict(result_groups=len(totals), passed=sum(int(t[1]) for t in totals),
                failed=sum(int(t[2]) for t in totals), ignored=sum(int(t[3]) for t in totals),
                ignored_lines=[line for line in text.splitlines() if '... ignored' in line])


def require_cpu_baseline(rows, required):
    if required:
        assert all(r.get('cpu') == 'generic' and r.get('features') == '' for r in rows), 'Provider CPU baseline mismatch'


def verify_windows(z, artifact, pin, expected, generic_cpu=False):
    load = lambda name: json.loads(z.read(name))
    summary = load('ci-summary.json')
    provenance = load('provenance.json')
    identity = load('final-identity.json')
    assert summary['tested_commit'] == provenance['tested_commit'] == identity['tested_commit'] == pin
    assert provenance['head'] == identity['head'] == pin
    assert provenance['initial_status'] == identity['status'] == '' and identity['clean']
    assert summary['status'] == 'PASS_IN_EXECUTED_SCOPE' and summary['failure'] is None
    assert summary['harness_commit'] == artifact['workflow_run']['head_sha']
    assert int(summary['workflow_run']) == artifact['workflow_run']['id']
    # This command log proves the detached checkout separately from the clean status.
    detach = load('acquisition-detach.json')
    assert '--detach' in detach['command'] and pin in detach['command'] and detach['exit'] == 0
    checked = {}
    records = {}

    def check(name, digest):
        data = z.read(name)
        assert sha(data) == digest, f'Log digest mismatch: {name}'
        assert name not in checked or checked[name] == digest, f'Conflicting digest: {name}'
        checked[name] = digest
        return data

    for name in z.namelist():
        if not name.endswith('.json'):
            continue
        data = load(name)
        if not isinstance(data, dict) or not data.get('log_sha256'):
            continue
        assert data['tested_commit'] == pin, f'Wrong record pin: {name}'
        log = check(data['log'], data['log_sha256'])
        if data.get('log_archive'):
            compressed = check(data['log_archive'], data['archive_sha256'])
            assert gzip.decompress(compressed) == log
        if name.startswith(('harness-comptime/', 'strict-bounds/')):
            continue
        assert data['exit'] == 0, f'Failed acquisition gate: {name}'
        records[data['label']] = data
    required = {'acquisition-clone', 'acquisition-detach', 'runtime-config', 'runtime-build',
                'runtime-abi', 'cli-debug', 'cli-release', 'sysroot', 'provenance-rustc',
                'provenance-llvm', 'provenance-gcc', 'provenance-cmake', 'provenance-ninja'}
    workspace = None
    boundary = None
    if summary['lane'] in ('triage', 'final'):
        label = 'workspace-' + summary['lane']
        required.add(label)
        record = records[label]
        workspace = workspace_totals(z.read(record['log']))
        for key in ('result_groups', 'passed', 'failed', 'ignored'):
            assert workspace[key] == record[key], f'Wrong recorded workspace total: {key}'
        assert '--workspace' in record['command'] and '--locked' in record['command']
        assert ('--no-fail-fast' in record['command']) == (summary['lane'] == 'triage')
    else:
        assert summary['lane'] == 'normal'
        required.update({'docs-examples', 'docs-validator', 'quickstart-build', 'quickstart-native',
                         'comptime/qualification', 'strict-bounds'})
        for name, count in [('focused-summary.json', 12), ('diagnostics-summary.json', 4)]:
            stage = load(name)
            assert stage['tested_commit'] == pin and stage['status'] == 'PASS'
            assert len(stage['results']) == count
            for record in stage['results']:
                assert records[record['label']] == record
                required.add(record['label'])
        comptime = load('comptime-summary.json')
        assert comptime['tested_commit'] == pin
        assert (comptime['pass_count'], comptime['fail_count'], comptime['characterization_count']) == (88, 0, 8)
        assert len(comptime['gates']) == 96
        assert sum(g['result'] == 'PASS' for g in comptime['gates']) == 88
        assert sum(g['result'] == 'CHARACTERIZED' for g in comptime['gates']) == 8
        for event in comptime['events']:
            assert event['tested_commit'] == pin and event['classification'] == 'completed'
            for stream in ('stdout', 'stderr'):
                parts = PureWindowsPath(event[stream]['path']).parts
                relative = PurePosixPath(*parts[parts.index('windows') + 1:]).as_posix()
                check(relative, event[stream]['sha256'])
        bounds = load('strict-bounds/summary.json')
        assert bounds['tested_commit'] == pin and bounds['status'] == 'PASS'
        assert {(r['profile'], r['mode']) for r in bounds['rows']} == {
            ('debug', 'source'), ('debug', 'artifact'), ('release', 'source'), ('release', 'artifact')}
        for row in bounds['rows']:
            prefix = f"strict-bounds/{row['profile']}-{row['mode']}/"
            stderr = z.read(prefix + 'run.stderr')
            assert row['status'] == 'PASS' and row['build_exit'] == 0 and row['native_exit'] != 0
            assert b'luna: PANIC [L002]' in stderr and b'index 3, length 2' in stderr
            provider_files = [n for n in z.namelist() if n.startswith(prefix + 'sysroot/')]
            if row['mode'] == 'source':
                assert not any(n.endswith(('.llib', '.obj')) for n in provider_files)
            else:
                assert not any(n.endswith('.ln') for n in provider_files)
        boundary = dict(comptime_passed=88, comptime_characterized=8, strict_bounds_rows=4,
                        trap='L002: index 3, length 2',
                        native_exits=[r['native_exit'] for r in bounds['rows']])
    assert required <= records.keys(), f'Missing required gates: {required - records.keys()}'
    sysroot = load('sysroot-provenance.json')
    assert sysroot['tested_commit'] == pin and sysroot['provider_count'] == len(sysroot['providers']) == 49
    assert len({r['path'] for r in sysroot['providers']}) == 49
    assert tuple(sysroot[k] for k in ('compiler', 'metadata', 'mvir', 'format')) == (25, 9, 5, 2)
    require_cpu_baseline(sysroot['providers'], generic_cpu)
    for row in sysroot['providers']:
        assert row['target'] == 'x86_64-pc-windows-gnu' and row['coff_machine'] == '0x8664'
        if summary['lane'] == 'normal':
            for profile in ('debug', 'release'):
                prefix = f'strict-bounds/{profile}-artifact/sysroot/libs/external/'
                lib = z.read(prefix + row['path'] + '.llib')
                obj = z.read(prefix + row['path'] + '.obj')
                assert sha(lib) == row['library_sha256'] and sha(obj) == row['object_sha256']
                assert lib[:4] == b'LLIB' and struct.unpack('<HHH', lib[4:10]) == (2, 25, 5)
                assert lib[10:74].split(b'\0')[0] == b'x86_64-pc-windows-gnu'
                assert struct.unpack('<H', obj[:2])[0] == 0x8664
    return dict(artifact_id=artifact['id'], name=artifact['name'], workflow_run=artifact['workflow_run'],
                archive_sha256=expected, size_in_bytes=artifact['size_in_bytes'], local_path=artifact['local_path'],
                tested_commit=pin, harness_commit=summary['harness_commit'], status='PASS',
                checks=dict(recorded_log_hashes=len(checked), acquisition_gates=len(records)),
                gates=len(records), workspace=workspace, boundary=boundary,
                providers=dict(count=49, target='x86_64-pc-windows-gnu', object='COFF 0x8664',
                               versions=dict(compiler=25, metadata=9, mvir=5, format=2),
                               archived_bundles_checked=98 if summary['lane'] == 'normal' else 0),
                code_tree_identity=provenance['source_tree_objects'])


def verify(artifact, pin, generic_cpu=False):
    archive = Path(artifact['local_path'])
    raw = archive.read_bytes()
    expected = artifact['digest'].removeprefix('sha256:')
    assert sha(raw) == expected, f'{archive.name}: ZIP digest mismatch'
    assert len(raw) == artifact['size_in_bytes'], f'{archive.name}: ZIP size mismatch'
    checks = dict(file_hashes=0, compressed_log_hashes=0, raw_log_hashes=0)
    with zipfile.ZipFile(archive) as z:
        names = z.namelist()
        assert len(set(names)) == len(names), 'Duplicate ZIP members'
        assert all(not PurePosixPath(n).is_absolute() and '..' not in PurePosixPath(n).parts
                   and '\\' not in n for n in names), 'Unsafe ZIP member'
        assert z.testzip() is None, 'ZIP CRC failure'
        if 'ci-summary.json' in names:
            return verify_windows(z, artifact, pin, expected, generic_cpu)
        load = lambda name: json.loads(z.read(name))
        inventory = load('evidence-hashes.json')
        assert set(inventory) == set(names) - {'evidence-hashes.json'}, 'Incomplete file inventory'
        for name, digest in inventory.items():
            assert sha(z.read(name)) == digest, f'Inventory digest mismatch: {name}'
            checks['file_hashes'] += 1
        provenance = load('provenance.json')
        assert provenance['tested_commit'] == pin, 'Wrong provenance pin'
        assert provenance['initial_status'] == '', 'Dirty initial checkout'
        assert provenance['detached'] is True, 'Checkout was not detached'
        assert provenance['harness_commit'] == artifact['workflow_run']['head_sha']
        assert int(provenance['workflow_run']) == artifact['workflow_run']['id']
        identity = load('final-identity.json')
        assert identity['tested_commit'] == pin and identity['clean'] is True
        assert identity['status'] == '', 'Dirty final checkout'
        summary = load('summary.json')
        assert summary['tested_commit'] == pin and summary['failure'] is None
        assert summary['status'] == 'PASS_IN_EXECUTED_LANE_SCOPE'
        assert not summary['failed_gates']
        gates = load('gates.json')
        assert len(gates) == summary['gate_count'] and gates, 'Incomplete gate list'
        workspace = None
        repeated_identical_logs = []
        for gate in gates:
            label = gate['gate']
            record = load(label + '.json')
            differences = {k for k in record.keys() | gate.keys() if record.get(k) != gate.get(k)}
            if differences:
                # The workspace harness repeats code-tree-identity after acquire,
                # overwriting only the elapsed-time field in the per-gate file.
                # Both indexed acquisitions must retain identical content/exits.
                assert differences == {'seconds'} and sum(g['gate'] == label for g in gates) == 2, f'Conflicting gate record: {label}'
                repeated_identical_logs.append(label)
            assert gate['tested_commit'] == pin, f'Wrong gate pin: {label}'
            assert gate['status'] == 'PASS', f'Failed gate: {label}'
            detached = label == 'detached' and gate['command'] == ['git', 'symbolic-ref', '-q', 'HEAD']
            assert gate['exit'] == (1 if detached else 0), f'Unexpected exit: {label}'
            compressed = z.read(gate['log'])
            assert sha(compressed) == gate['compressed_sha256'], f'Compressed log mismatch: {label}'
            checks['compressed_log_hashes'] += 1
            log = gzip.decompress(compressed)
            assert sha(log) == gate['log_sha256'], f'Raw log mismatch: {label}'
            checks['raw_log_hashes'] += 1
            if label in ('workspace-triage', 'workspace-final'):
                workspace = workspace_totals(log)
        providers = None
        if 'sysroot-provenance.json' in names:
            sysroot = load('sysroot-provenance.json')
            assert sysroot['tested_commit'] == pin
            rows = sysroot['providers']
            assert len(rows) == 49 and len({r['path'] for r in rows}) == 49
            require_cpu_baseline(rows, generic_cpu)
            for r in rows:
                assert (r['compiler'], r['metadata'], r['mvir'], r['format']) == (25, 9, 5, 2)
                assert r['target'] == r['manifest_target'] == 'x86_64-unknown-linux-gnu'
                assert r['elf_machine'] == 62 and r['native_elf64_little_endian']
                assert r['embedded_equals_sidecar'] and r['native_integrity_valid'] and r['valid']
            providers = dict(count=49, target='x86_64-unknown-linux-gnu', object='ELF64 x86_64',
                             versions=dict(compiler=25, metadata=9, mvir=5, format=2),
                             basis='Recorded provider headers and object identities in hash-verified acquisition')
        return dict(artifact_id=artifact['id'], name=artifact['name'], workflow_run=artifact['workflow_run'],
                    archive_sha256=expected, size_in_bytes=len(raw), local_path=str(archive),
                    tested_commit=pin, harness_commit=provenance['harness_commit'],
                    status='PASS', checks=checks, gates=len(gates), workspace=workspace, providers=providers,
                    repeated_identical_log_records=repeated_identical_logs,
                    code_tree_identity=provenance['code-tree-identity'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--pin', required=True)
    parser.add_argument('--require-generic-cpu', action='store_true',
                        help='Require generic CPU and empty optional features on every recorded provider')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    artifacts = json.loads(args.manifest.read_text(encoding='utf-8'))
    assert artifacts, 'Empty manifest'
    assert len({a['id'] for a in artifacts}) == len(artifacts), 'Duplicate artifact IDs'
    results = []
    errors = []
    for artifact in artifacts:
        try:
            results.append(verify(artifact, args.pin, args.require_generic_cpu))
        except Exception as exc:
            errors.append(dict(name=artifact['name'], error=str(exc)))
    repo = Path(__file__).resolve().parents[2]
    tree = subprocess.check_output(['git', 'ls-tree', args.pin, 'CMakeLists.txt', 'luna-rs', 'runtime', 'tests'],
                                   cwd=repo, text=True)
    objects = {line.split('\t')[1]: line.split()[2] for line in tree.strip().splitlines()}
    for result in results:
        identity = result['code_tree_identity']
        if isinstance(identity, str):
            if identity.strip() != tree.strip():
                errors.append(dict(name=result['name'], error='Wrong acquired source tree'))
        else:
            if identity != {k: objects[k] for k in ('luna-rs', 'runtime', 'tests')}:
                errors.append(dict(name=result['name'], error='Wrong acquired source tree'))
    report = dict(tested_commit=args.pin, status='FAIL' if errors else 'PASS', artifacts=results, errors=errors,
                  required_cpu_baseline='generic;features=' if args.require_generic_cpu else None,
                  scope='ZIP SHA-256/size/CRC; inventory; clean exact pin; gate exits; compressed and raw log digests; workspace totals; recorded provider identities',
                  release='PENDING COMPLETE EIGHT-LANE RECONCILIATION')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8', newline='\n')
    for r in results:
        print(f"{r['name']}: PASS {r['checks']} workspace={r['workspace']}")
    for error in errors:
        print(f"{error['name']}: FAIL {error['error']}")
    return 1 if errors else 0


if __name__ == '__main__':
    raise SystemExit(main())
