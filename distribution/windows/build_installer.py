"""Build the small online installer; payload URLs and hashes come only from the lock."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--iscc', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--lock', type=Path, default=Path(__file__).with_name('installer-lock.json'))
    p.add_argument('--test-base-url', help='Test-build only: serve locked archives from a local HTTP fixture')
    a = p.parse_args()
    source = Path(__file__).resolve().parent
    lock = json.loads(a.lock.read_text(encoding='utf-8'))
    assert lock['schema'] == 1 and lock['distribution_version'] == '0.1.0-alpha.1'
    a.out = a.out.resolve()
    generated = a.out / 'generated'
    generated.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env['LUNA_PACKAGE_DIR'] = lock['luna']['root']
    rustc = shutil.which('rustc') or str(Path.home() / '.cargo/bin/rustc.exe')
    for source_name, output_name in [('launcher.rs', 'luna.exe'), ('linker.rs', 'luna-cc.exe')]:
        subprocess.run([rustc, str(source / source_name), '--edition=2021', '-O', '-C', 'target-feature=+crt-static',
                        '-C', 'strip=symbols', '-o', str(generated / output_name)], env=env, check=True)
    lines = []
    for key, prefix in [('luna', 'Luna'), ('llvm', 'LLVM'), ('gnu', 'GNU')]:
        row = lock[key]
        assert re.fullmatch('[0-9a-f]{64}', row['sha256'])
        url = row['url']
        assert url.startswith('https://') and '"' not in url
        if a.test_base_url:
            assert a.test_base_url.startswith('http://127.0.0.1:'), 'Fixture URLs must be loopback only'
            url = a.test_base_url.rstrip('/') + '/' + row['name']
        lines += [f'#define {prefix}Url "{url}"', f'#define {prefix}SHA256 "{row["sha256"]}"',
                  f'#define {prefix}Unpacked {row["unpacked_bytes"]}']
    (generated / 'payloads.iss').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    shutil.copyfile(a.lock, generated / 'installer-lock.json')
    subprocess.run([str(a.iscc.resolve()), '/Qp', '/DGeneratedDir=' + str(generated),
                    '/DOutputDir=' + str(a.out), str(source / 'luna.iss')], check=True)
    exe = a.out / 'lunainstaller.exe'
    digest = hashlib.sha256(exe.read_bytes()).hexdigest()
    (a.out / 'lunainstaller.exe.sha256').write_text(digest + '  lunainstaller.exe\n', encoding='utf-8')
    report = dict(file=exe.name, sha256=digest, bytes=exe.stat().st_size, test_build=bool(a.test_base_url),
                  lock_sha256=hashlib.sha256(a.lock.read_bytes()).hexdigest(),
                  source_commit=subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
                  workflow_run=os.environ.get('GITHUB_RUN_ID'),
                  rustc=subprocess.check_output([rustc, '--version'], text=True).strip())
    (a.out / 'build-summary.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
