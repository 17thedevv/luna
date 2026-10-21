"""Provision the pinned published compiler for CI tests of Lunar (not a Lunar command)."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import urllib.request
import zipfile

BASE = 'https://github.com/17thedevv/luna/releases/download/v0.1.0-alpha.1/'


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = args.out.resolve()
    root.mkdir(parents=True, exist_ok=False)
    if os.name == 'nt':
        name = 'lunainstaller.exe'
        digest = 'adb15aa0c50a8d72bccaa447cfabd23c68010da547672c0db2c9a3184e7af5fd'
    else:
        name = 'luna-0.1.0-alpha.1-x86_64-unknown-linux-gnu.zip'
        digest = '86884370d1bea203dddcececf442ea6340600d2edeaa8609d384632878b5c3e4'
    archive = root / name
    urllib.request.urlretrieve(BASE + name, archive)
    actual = hashlib.sha256(archive.read_bytes()).hexdigest()
    if actual != digest:
        raise RuntimeError(f'Published compiler digest mismatch: {actual}')
    if os.name == 'nt':
        install = root / 'compiler'
        subprocess.run([str(archive), '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-',
                        '/NOICONS', '/TASKS=', f'/DIR={install}', f'/LOG={root / "installer.log"}'], check=True, timeout=600)
        compiler = install / 'bin/luna.exe'
    else:
        install = root / 'compiler'
        with zipfile.ZipFile(archive) as bundle:
            for name in bundle.namelist():
                path = PurePosixPath(name)
                if path.is_absolute() or '..' in path.parts or '\\' in name:
                    raise RuntimeError(f'Unsafe ZIP path: {name}')
            bundle.extractall(install)
        compiler, = install.glob('*/bin/luna')
        compiler.chmod(0o755)
    subprocess.run([str(compiler), '--version'], check=True)
    with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as file:
        file.write(f'LUNAR_SMOKE_COMPILER={compiler}\n')
        if os.name != 'nt':
            package = compiler.parent.parent
            manifest = json.loads((package / 'manifest.json').read_text(encoding='utf-8'))
            file.write(f'LUNA_RUNTIME_LIB={package / manifest["runtime_path"]}\n')
    print(f'Provisioned verified compiler {compiler}')


if __name__ == '__main__':
    main()
