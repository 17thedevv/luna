"""Build a relocatable release package from an immutable qualified source commit.

This is distribution tooling, not a compiler or stdlib modification. LLVM 18
and the native GNU toolchain are prerequisites, deliberately not redistributed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import time
import tomllib
import zipfile


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8', newline='\n')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', required=True)
    p.add_argument('--qualified', required=True)
    p.add_argument('--target', choices=['x86_64-pc-windows-gnu', 'x86_64-unknown-linux-gnu'], required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    assert all(len(s) == 40 and all(c in '0123456789abcdef' for c in s) for s in (a.source, a.qualified))
    windows = os.name == 'nt'
    assert windows == ('windows' in a.target)
    root = a.out.resolve()
    root.mkdir(parents=True, exist_ok=False)
    evidence = root / 'evidence'
    evidence.mkdir()
    source = root / 's'
    build = root / 'b'
    dist = root / 'dist'
    dist.mkdir()
    records = []
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(build / 'cargo'), CARGO_TERM_COLOR='never',
               LUNA_SYSROOT=str(source / 'luna-rs'),
               LUNA_RUNTIME_LIB=str(build / 'runtime/libluna-runtime.a'))

    def run(label, command, cwd=root):
        started = time.monotonic()
        result = subprocess.run(list(map(str, command)), cwd=cwd, env=env, capture_output=True, timeout=7200)
        record = dict(label=label, command=list(map(str, command)), cwd=str(cwd),
                      exit=result.returncode, seconds=round(time.monotonic() - started, 3))
        for stream in ('stdout', 'stderr'):
            path = evidence / (label + '.' + stream)
            path.write_bytes(getattr(result, stream))
            record[stream] = dict(path=path.name, sha256=digest(path))
        records.append(record)
        save(evidence / 'commands.json', records)
        print(f'{label}: exit {result.returncode}', flush=True)
        if result.returncode:
            raise RuntimeError(f'{label}: {result.stderr.decode(errors="replace")[-3000:]}')
        return result.stdout.decode('utf-8', errors='replace').strip()

    summary = dict(status='FAIL', release_tag='v0.1.0-alpha.1', source_commit=a.source,
                   qualified_implementation=a.qualified, target=a.target)
    try:
        run('clone', ['git', '-c', 'core.longpaths=true', 'clone', '--no-checkout',
                      'https://github.com/17thedevv/luna.git', source])
        run('detach', ['git', '-c', 'core.longpaths=true', 'checkout', '--detach', a.source], source)
        assert run('head', ['git', 'rev-parse', 'HEAD'], source) == a.source
        assert run('initial-status', ['git', 'status', '--porcelain'], source) == ''
        trees = {}
        for part in ('luna-rs', 'runtime', 'tests', 'CMakeLists.txt'):
            actual = subprocess.check_output(['git', 'rev-parse', a.source + ':' + part], cwd=source, text=True).strip()
            qualified = subprocess.check_output(['git', 'rev-parse', a.qualified + ':' + part], cwd=source, text=True).strip()
            assert actual == qualified, f'Unqualified implementation tree: {part}'
            trees[part] = actual
        tools = dict(rustc=run('rustc', ['rustc', '-vV']), gcc=run('gcc', ['gcc', '-dumpmachine']),
                     llvm=run('llvm', [env['LLVM_CONFIG_PATH'], '--version']),
                     llvm_host=run('llvm-host', [Path(env['LLVM_HOME']) / 'bin' / ('llc.exe' if windows else 'llc'), '--version']))
        assert tools['llvm'].startswith('18.')
        if windows:
            assert tools['gcc'].startswith('x86_64') and 'mingw' in tools['gcc']
        else:
            assert tools['gcc'].startswith('x86_64') and 'linux' in tools['gcc']
        run('runtime-config', ['cmake', '-S', source / 'runtime', '-B', build / 'runtime', '-G', 'Ninja',
                              '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_C_COMPILER=gcc', '-DCMAKE_CXX_COMPILER=g++'])
        run('runtime-build', ['cmake', '--build', build / 'runtime', '--parallel'])
        run('runtime-abi', ['ctest', '--test-dir', build / 'runtime', '-R', '^RuntimeAbiTests$',
                           '--output-on-failure', '--no-tests=error'])
        run('cli-release', ['cargo', 'build', '--locked', '--release', '-p', 'luna-cli'], source / 'luna-rs')
        cli = build / ('cargo/release/luna.exe' if windows else 'cargo/release/luna')
        cli_version = run('cli-version', [cli, '--version'])
        assert cli_version == 'luna 0.1.0', 'The CLI version is distinct from the distribution prerelease tag'
        run('sysroot', [cli, 'build-sysroot', '--quiet'], source)
        package_name = 'luna-0.1.0-alpha.1-' + a.target
        package = root / package_name
        (package / 'bin').mkdir(parents=True)
        (package / 'runtime').mkdir()
        (package / 'libs/external').mkdir(parents=True)
        (package / 'examples').mkdir()
        shutil.copy2(cli, package / 'bin' / cli.name)
        # Executable-relative Windows discovery expects this filename. These
        # remain the GNU static-archive bytes produced by the audited C build.
        runtime_name = 'luna-runtime.lib' if windows else 'libluna-runtime.a'
        shutil.copy2(build / 'runtime/libluna-runtime.a', package / 'runtime' / runtime_name)
        external = source / 'luna-rs/libs/external'
        manifest_source = external / 'sysroot.toml'
        shutil.copy2(manifest_source, package / 'libs/external/sysroot.toml')
        providers = tomllib.loads(manifest_source.read_text(encoding='utf-8'))['provider']
        assert len(providers) == len(list(external.rglob('*.llib'))) == 49
        provider_records = []
        for provider in providers:
            stem = provider['path']
            lib, obj = external / (stem + '.llib'), external / (stem + '.obj')
            raw = lib.read_bytes()
            assert raw[:4] == b'LLIB' and struct.unpack('<HHH', raw[4:10]) == (2, 25, 5)
            assert raw[10:74].split(b'\0')[0].decode() == a.target
            native = obj.read_bytes()
            assert (native[:2] == b'\x64\x86') if windows else (native[:5] == b'\x7fELF\x02' and struct.unpack('<H', native[18:20])[0] == 62)
            published = json.loads(run('manifest-' + provider['name'], [cli, 'manifest', lib], source))
            contract = published['target']
            assert contract['target_triple'] == a.target and contract['cpu'] == 'generic' and contract['features'] == ''
            assert contract['pointer_width'] == 64 and contract['endianness'] == 'little'
            assert published['object_metadata']['size'] == len(native)
            assert bytes(published['object_metadata']['hash']).hex() == digest(obj)
            for item in (lib, obj):
                destination = package / 'libs/external' / item.relative_to(external)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(item, destination)
            provider_records.append(dict(name=provider['name'], path=stem,
                                         library_sha256=digest(lib), object_sha256=digest(obj),
                                         cpu=contract['cpu'], features=contract['features'],
                                         object_format=contract['object_format'], abi=contract['abi']))
        assert 'pub const SEMANTIC_METADATA_VERSION: u16 = 9;' in (source / 'luna-rs/crates/luna-llib/src/format.rs').read_text(encoding='utf-8')
        shutil.copy2(source / 'tests/luna/language/spec_v01/basic_reference.ln', package / 'examples/quickstart.ln')
        (package / 'examples/stdlib.ln').write_text(
            'import <vec>;\nfn main() -> i32 {\n'
            '    dec rw values: std::Vec<i32> = std::vec_new<i32>();\n'
            '    values.push(20);\n    values.push(22);\n'
            '    dec view = values.as_slice();\n    return view[0] + view[1] - 42;\n}\n',
            encoding='utf-8', newline='\n')
        notice = ('The source snapshot contains no project LICENSE file. This package adds no license grant.\n'
                  'Project source and qualification records: https://github.com/17thedevv/luna\n'
                  'External LLVM/GNU toolchains are prerequisites and are not bundled.\n')
        (package / 'LICENSE-NOTICE.txt').write_text(notice, encoding='utf-8', newline='\n')
        readme = f'''Luna 0.1.0-alpha.1 — {a.target}
Source commit: {a.source}
Qualified implementation: {a.qualified}
Protocol: compiler25 / metadata9 / MVIR5 / format2
The CLI reports luna0.1.0; manifest.json distinguishes this alpha distribution.

Prerequisites: LLVM18 tools and shared library (Windows: LLVM-C.dll on PATH),
native x86_64 GNU C/C++ toolchain, and the host C runtime. Windows needs the
Microsoft Visual C++ runtime for the MSVC-hosted compiler executable.
On Ubuntu24.04 install build-essential and llvm-18. Put LLVM18/bin on PATH or
set LLVM_HOME to its root. Rust/Cargo is not required to use this package.

Unpack to a new directory and add its bin directory to PATH. Keep bin, runtime
and libs/external together. No external LUNA_SYSROOT/LUNA_RUNTIME_LIB is needed;
remove old overrides before use. The compiler discovers the package relative
to its executable. Windows runtime/luna-runtime.lib is the GNU static archive,
renamed for the compiler's existing executable-relative lookup.

From a writable project directory:
  luna --version
  luna build /path/to/package/examples/quickstart.ln -o quickstart{'.exe' if windows else ''}
  ./quickstart{'.exe' if windows else ''}
  luna build /path/to/package/examples/stdlib.ln -o stdlib{'.exe' if windows else ''}
  ./stdlib{'.exe' if windows else ''}

Both examples must exit0. manifest.json and SHA256SUMS record every payload file.
Providers are artifact-only:49 canonical LLIB/object pairs, without source fallback.
Both release targets use CPU generic with no optional host features. Strict
CPU/features identity checking is retained; the package does not select host tuning.
ASan qualification applies only to the C runtime test lane, not this Release build.
Standalone comptime statements remain unsupported; VM/layout is scoped to64-bit GNU.
See the release notes for retained contracts, deferrals and diagnostic exceptions.
'''
        (package / 'README.txt').write_text(readme, encoding='utf-8', newline='\n')
        files = {path.relative_to(package).as_posix(): digest(path) for path in sorted(package.rglob('*')) if path.is_file()}
        metadata = dict(release_tag='v0.1.0-alpha.1', distribution_version='0.1.0-alpha.1', cli_version=cli_version,
                        source_commit=a.source, qualified_implementation=a.qualified, code_tree_objects=trees,
                        target=a.target, host=platform.platform(), toolchain=tools,
                        versions=dict(compiler=25, metadata=9, mvir=5, format=2),
                        runtime_path='runtime/' + runtime_name, provider_count=49, providers=provider_records,
                        license='No project license declared in source snapshot; see LICENSE-NOTICE.txt', files=files)
        save(package / 'manifest.json', metadata)
        (package / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.relative_to(package).as_posix()}\n'
                                                 for path in sorted(package.rglob('*')) if path.is_file()), encoding='utf-8', newline='\n')
        archive = dist / (package_name + '.zip')
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
            for path in sorted(package.rglob('*')):
                if not path.is_file():
                    continue
                info = zipfile.ZipInfo(package_name + '/' + path.relative_to(package).as_posix(), date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = (0o100755 if path == package / 'bin' / cli.name else 0o100644) << 16
                z.writestr(info, path.read_bytes())
        (dist / (archive.name + '.sha256')).write_text(f'{digest(archive)}  {archive.name}\n', encoding='utf-8')
        for tool in ('smoke_luna_package.py',):
            shutil.copy2(Path(__file__).with_name(tool), dist / tool)
        shutil.copy2(Path(__file__).resolve().parents[1] / 'audits/r5-missing-return-2026-10-17/prepare_windows_llvm.py',
                     dist / 'prepare_windows_llvm.py')
        assert run('final-status', ['git', 'status', '--porcelain'], source) == ''
        summary.update(status='PASS', archive=archive.name, archive_sha256=digest(archive),
                       archive_bytes=archive.stat().st_size, providers=49, code_tree_objects=trees,
                       cli_version=cli_version, runtime_sha256=digest(package / 'runtime' / runtime_name))
        save(dist / 'build-summary.json', summary)
        shutil.copytree(evidence, dist / 'build-evidence')
    except Exception as error:
        summary['failure'] = str(error)
        raise
    finally:
        save(evidence / 'package-build-summary.json', summary)
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
