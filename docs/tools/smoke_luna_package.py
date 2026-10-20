"""Verify and run an extracted distribution in a separate job with no checkout."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import struct
import subprocess
import time
import zipfile


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive', type=Path, required=True)
    p.add_argument('--expected-source', required=True)
    p.add_argument('--expected-pin', required=True)
    p.add_argument('--target', required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    root = a.out.resolve()
    root.mkdir(parents=True, exist_ok=False)
    evidence = root / 'evidence'
    evidence.mkdir()
    archive = a.archive.resolve()
    summary = dict(status='FAIL', source_commit=a.expected_source, qualified_implementation=a.expected_pin,
                   target=a.target, archive=archive.name, archive_sha256=sha(archive),
                   environment='Separate hosted job; no source checkout; external LUNA variables cleared',
                   workflow_run=os.environ.get('GITHUB_RUN_ID'), steps=[])
    env = {k: v for k, v in os.environ.items() if not k.startswith('LUNA_')}
    env.pop('CC', None)
    try:
        expected = archive.with_suffix(archive.suffix + '.sha256').read_text().split()[0]
        assert summary['archive_sha256'] == expected, 'Package ZIP digest mismatch'
        install = root / 'install'
        install.mkdir()
        with zipfile.ZipFile(archive) as z:
            names = z.namelist()
            assert len(set(names)) == len(names)
            assert all(not PurePosixPath(n).is_absolute() and '..' not in PurePosixPath(n).parts and '\\' not in n for n in names)
            assert len({n.split('/')[0] for n in names}) == 1
            assert z.testzip() is None
            z.extractall(install)
        package = next(install.iterdir())
        manifest = json.loads((package / 'manifest.json').read_text(encoding='utf-8'))
        assert manifest['source_commit'] == a.expected_source and manifest['qualified_implementation'] == a.expected_pin
        assert manifest['target'] == a.target and manifest['release_tag'] == 'v0.1.0-alpha.1'
        assert manifest['versions'] == dict(compiler=25, metadata=9, mvir=5, format=2)
        assert manifest['provider_count'] == len(manifest['providers']) == 49
        payload = {x.relative_to(package).as_posix() for x in package.rglob('*') if x.is_file()}
        assert payload == set(manifest['files']) | {'manifest.json', 'SHA256SUMS'}

        def verify_files():
            for name, digest in manifest['files'].items():
                assert sha(package / name) == digest, f'Payload mismatch: {name}'
            for line in (package / 'SHA256SUMS').read_text().splitlines():
                digest, name = line.split('  ', 1)
                assert sha(package / name) == digest, f'Checksum mismatch: {name}'

        verify_files()
        external = package / 'libs/external'
        assert len(list(external.rglob('*.llib'))) == len(list(external.rglob('*.obj'))) == 49
        assert not list(external.rglob('*.ln')) and not list(external.rglob('*.ms'))
        for provider in manifest['providers']:
            lib = external / (provider['path'] + '.llib')
            obj = external / (provider['path'] + '.obj')
            raw = lib.read_bytes()
            assert sha(lib) == provider['library_sha256'] and sha(obj) == provider['object_sha256']
            assert raw[:4] == b'LLIB' and struct.unpack('<HHH', raw[4:10]) == (2, 25, 5)
            assert raw[10:74].split(b'\0')[0].decode() == a.target
            native = obj.read_bytes()
            assert (native[:2] == b'\x64\x86') if os.name == 'nt' else (native[:5] == b'\x7fELF\x02' and struct.unpack('<H', native[18:20])[0] == 62)
        cli = package / ('bin/luna.exe' if os.name == 'nt' else 'bin/luna')
        cli.chmod(cli.stat().st_mode | stat.S_IXUSR)
        programs = root / 'programs'
        programs.mkdir()

        def run(label, command, expected_exit=0):
            begin = time.monotonic()
            result = subprocess.run(list(map(str, command)), cwd=programs, env=env, capture_output=True, timeout=180)
            record = dict(label=label, command=list(map(str, command)), cwd=str(programs), exit=result.returncode,
                          expected_exit=expected_exit, seconds=round(time.monotonic() - begin, 3))
            for stream in ('stdout', 'stderr'):
                path = evidence / (label + '.' + stream)
                path.write_bytes(getattr(result, stream))
                record[stream] = dict(path=path.name, sha256=sha(path))
            summary['steps'].append(record)
            assert result.returncode == expected_exit, f'{label}: {result.stderr.decode(errors="replace")[-2000:]}'
            print(f'{label}: exit {result.returncode}', flush=True)
            return result

        version = run('version', [cli, '--version'])
        assert version.stdout.decode().strip() == manifest['cli_version'] == 'luna 0.1.0'
        for name in ('quickstart', 'stdlib'):
            fixture = programs / (name + '.ln')
            shutil.copyfile(package / 'examples' / fixture.name, fixture)
            exe = programs / (name + ('.exe' if os.name == 'nt' else ''))
            run(name + '-build', [cli, 'build', fixture, '--quiet', '--no-config', '-o', exe])
            run(name + '-native', [exe])
        # A missing packaged provider must reject, proving no external source
        # or artifact root silently supplies it in the clean consumer job.
        vec = next(p for p in manifest['providers'] if p['name'] == 'vec')
        library = external / (vec['path'] + '.llib')
        hidden = library.with_suffix('.withheld')
        library.rename(hidden)
        try:
            rejection = run('missing-provider-control', [cli, 'check', programs / 'stdlib.ln', '--no-config'], expected_exit=1)
            assert b'E1004' in rejection.stderr + rejection.stdout, 'Missing provider must retain E1004'
        finally:
            hidden.rename(library)
        verify_files()
        summary.update(status='PASS', package_manifest_sha256=sha(package / 'manifest.json'),
                       payload_hashes_checked=len(manifest['files']), providers_checked=49,
                       source_fallback=False, external_luna_variables=[],
                       runtime_path=manifest['runtime_path'], cli_version=manifest['cli_version'])
    except Exception as error:
        summary['failure'] = str(error)
        raise
    finally:
        (evidence / 'package-smoke-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
