"""Independently reconcile original CI package/build/smoke ZIPs before release.

Run from the repository root. The metadata manifest contains the original
GitHub id/name/digest/size/workflow_run fields and a local_path per artifact.
Verified distribution ZIPs are copied byte-for-byte to --assets.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import struct
import subprocess
import zipfile


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def safe_zip(raw):
    z = zipfile.ZipFile(io.BytesIO(raw))
    names = z.namelist()
    assert len(names) == len(set(names))
    assert all(not PurePosixPath(n).is_absolute() and '..' not in PurePosixPath(n).parts and '\\' not in n for n in names)
    assert z.testzip() is None
    return z


def check_streams(z, records, prefix=''):
    count = 0
    for record in records:
        for stream in ('stdout', 'stderr'):
            ref = record[stream]
            assert sha(z.read(prefix + ref['path'])) == ref['sha256']
            count += 1
    return count


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--manifest', type=Path, required=True)
    p.add_argument('--source', required=True)
    p.add_argument('--qualified', required=True)
    p.add_argument('--assets', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    targets = {'x86_64-pc-windows-gnu', 'x86_64-unknown-linux-gnu'}
    trees = {part: subprocess.check_output(['git', 'rev-parse', a.qualified + ':' + part], text=True).strip()
             for part in ('luna-rs', 'runtime', 'tests', 'CMakeLists.txt')}
    source_trees = {part: subprocess.check_output(['git', 'rev-parse', a.source + ':' + part], text=True).strip() for part in trees}
    assert trees == source_trees
    artifacts = json.loads(a.manifest.read_text(encoding='utf-8'))
    assert len(artifacts) == len({r['id'] for r in artifacts}) == 6
    builds, packages, smokes, outer = {}, {}, {}, []
    a.assets.mkdir(parents=True, exist_ok=True)
    for artifact in artifacts:
        raw = Path(artifact['local_path']).read_bytes()
        assert sha(raw) == artifact['digest'].removeprefix('sha256:')
        assert len(raw) == artifact['size_in_bytes']
        assert artifact['workflow_run']['head_sha'] == a.source
        with safe_zip(raw) as z:
            load = lambda name: json.loads(z.read(name))
            name = artifact['name']
            if name.startswith('luna-package-build-evidence-'):
                summary = load('package-build-summary.json')
                assert summary['status'] == 'PASS' and summary['source_commit'] == a.source
                assert summary['qualified_implementation'] == a.qualified and summary['code_tree_objects'] == trees
                target = summary['target']
                commands = load('commands.json')
                assert all(c['exit'] == 0 for c in commands)
                labels = {r['label']: r for r in commands}
                for label in ('clone', 'detach', 'head', 'initial-status', 'gcc', 'rustc', 'llvm',
                              'runtime-abi', 'cli-release', 'cli-version', 'sysroot', 'final-status'):
                    assert label in labels
                assert z.read('head.stdout').decode().strip() == a.source
                assert z.read('initial-status.stdout') == z.read('final-status.stdout') == b''
                assert b'--detach' in json.dumps(labels['detach']['command']).encode()
                count = check_streams(z, commands)
                builds[target] = dict(summary=summary, stream_hashes=count, commands=len(commands))
            elif name.startswith('luna-package-smoke-'):
                summary = load('package-smoke-summary.json')
                assert summary['status'] == 'PASS' and summary['source_commit'] == a.source
                assert summary['qualified_implementation'] == a.qualified and summary['providers_checked'] == 49
                assert summary['source_fallback'] is False and summary['external_luna_variables'] == []
                expected = [('version', 0), ('llvm-host', 0), ('quickstart-build', 0), ('quickstart-native', 0),
                            ('stdlib-build', 0), ('stdlib-native', 0), ('missing-provider-control', 1)]
                assert [(r['label'], r['exit']) for r in summary['steps']] == expected
                assert b'E1004' in z.read('missing-provider-control.stdout') + z.read('missing-provider-control.stderr')
                assert z.read('version.stdout').decode().strip() == 'luna 0.1.0'
                count = check_streams(z, summary['steps'])
                smokes[summary['target']] = dict(summary=summary, stream_hashes=count)
            else:
                assert name.startswith('luna-package-')
                build = load('build-summary.json')
                assert build['status'] == 'PASS' and build['source_commit'] == a.source
                assert build['qualified_implementation'] == a.qualified and build['code_tree_objects'] == trees
                inner_name = build['archive']
                payload = z.read(inner_name)
                assert sha(payload) == build['archive_sha256'] == z.read(inner_name + '.sha256').decode().split()[0]
                assert len(payload) == build['archive_bytes']
                with safe_zip(payload) as package:
                    names = package.namelist()
                    assert len({n.split('/')[0] for n in names}) == 1
                    prefix = names[0].split('/')[0] + '/'
                    manifest = json.loads(package.read(prefix + 'manifest.json'))
                    target = manifest['target']
                    assert manifest['source_commit'] == a.source and manifest['qualified_implementation'] == a.qualified
                    assert manifest['code_tree_objects'] == trees and manifest['release_tag'] == 'v0.1.0-alpha.1'
                    assert manifest['versions'] == dict(compiler=25, metadata=9, mvir=5, format=2)
                    assert manifest['provider_count'] == len(manifest['providers']) == 49
                    assert {n.removeprefix(prefix) for n in names} == set(manifest['files']) | {'manifest.json', 'SHA256SUMS'}
                    for path, digest in manifest['files'].items():
                        assert sha(package.read(prefix + path)) == digest, f'Payload hash mismatch: {path}'
                    for line in package.read(prefix + 'SHA256SUMS').decode().splitlines():
                        digest, path = line.split('  ', 1)
                        assert sha(package.read(prefix + path)) == digest
                    assert not any(n.startswith(prefix + 'libs/external/') and n.endswith(('.ln', '.ms')) for n in names)
                    assert package.read(prefix + manifest['runtime_path']).startswith(b'!<arch>\n')
                    for provider in manifest['providers']:
                        assert provider['cpu'] == 'generic' and provider['features'] == ''
                        stem = prefix + 'libs/external/' + provider['path']
                        lib, obj = package.read(stem + '.llib'), package.read(stem + '.obj')
                        assert sha(lib) == provider['library_sha256'] and sha(obj) == provider['object_sha256']
                        assert lib[:4] == b'LLIB' and struct.unpack('<HHH', lib[4:10]) == (2, 25, 5)
                        assert lib[10:74].split(b'\0')[0].decode() == target
                        assert (obj[:2] == b'\x64\x86') if 'windows' in target else (obj[:5] == b'\x7fELF\x02' and struct.unpack('<H', obj[18:20])[0] == 62)
                    manifest_sha = sha(package.read(prefix + 'manifest.json'))
                (a.assets / inner_name).write_bytes(payload)
                packages[target] = dict(name=inner_name, sha256=sha(payload), bytes=len(payload),
                                        manifest_sha256=manifest_sha, payload_files=len(manifest['files']),
                                        providers=49, cli_version=manifest['cli_version'], runtime_path=manifest['runtime_path'])
            outer.append(dict(id=artifact['id'], name=name, sha256=sha(raw), bytes=len(raw), status='PASS'))
    assert set(builds) == set(packages) == set(smokes) == targets
    for target in targets:
        assert builds[target]['summary']['archive_sha256'] == packages[target]['sha256'] == smokes[target]['summary']['archive_sha256']
        assert smokes[target]['summary']['package_manifest_sha256'] == packages[target]['manifest_sha256']
        assert smokes[target]['summary']['payload_hashes_checked'] == packages[target]['payload_files']
    result = dict(status='PASS', source_commit=a.source, qualified_implementation=a.qualified,
                  code_tree_objects=trees, artifacts=outer, builds=builds, packages=packages, smokes=smokes,
                  scope='Original CI ZIP digests/size/CRC; clean immutable source; GNU/ELF/COFF provider bytes; payload and checksum manifests; build ABI; separate native consumer logs; source-fallback negative control')
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(dict(status='PASS', packages=packages), indent=2))


if __name__ == '__main__':
    main()
