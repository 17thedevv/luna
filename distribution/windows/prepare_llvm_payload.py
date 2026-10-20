"""Produce the pinned, minimal LLVM runtime/tools ZIP used by Luna Setup."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile
import urllib.request
import zipfile

SOURCE_SHA256 = '22c5907db053026cc2a8ff96d21c0f642a90d24d66c23c6d28ee7b1d572b82e8'
SOURCE_URL = 'https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.8/clang%2Bllvm-18.1.8-x86_64-pc-windows-msvc.tar.xz'
LICENSE_URL = 'https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-18.1.8/llvm/LICENSE.TXT'
LICENSE_SHA256 = '8d85c1057d742e597985c7d4e6320b015a9139385cff4cbae06ffc0ebe89afee'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--package-dir', default='luna-0.1.0-alpha.1-x86_64-pc-windows-gnu')
    a = p.parse_args()
    assert len(PurePosixPath(a.package_dir).parts) == 1 and a.package_dir not in ('.', '..')
    with a.archive.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == SOURCE_SHA256, 'LLVM source archive SHA-256 mismatch'
    files = {}
    with tarfile.open(a.archive, 'r|xz') as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            if not member.isfile() or len(parts) != 3 or parts[1] != 'bin':
                continue
            name = parts[2]
            runtime = name.lower().endswith('.dll') and name.lower().startswith(('msvcp', 'vcruntime', 'concrt'))
            if name in ('llc.exe', 'LLVM-C.dll', 'clang.exe', 'ld.lld.exe') or runtime:
                data = archive.extractfile(member).read()
                files['toolchains/llvm/bin/' + name] = data
                if runtime:
                    # App-local copies let the original compiler run without a
                    # system-wide Visual C++ Runtime installation or elevation.
                    files['packages/' + a.package_dir + '/bin/' + name] = data
                print('Selected', name, len(data), flush=True)
    assert {'toolchains/llvm/bin/' + name for name in ('llc.exe', 'LLVM-C.dll', 'clang.exe', 'ld.lld.exe')} <= set(files), 'Missing required LLVM binary'
    files['toolchains/llvm/LICENSE.TXT'] = urllib.request.urlopen(LICENSE_URL, timeout=60).read()
    assert hashlib.sha256(files['toolchains/llvm/LICENSE.TXT']).hexdigest() == LICENSE_SHA256, 'LLVM license SHA-256 mismatch'
    provenance = dict(version='18.1.8', source_url=SOURCE_URL, source_sha256=SOURCE_SHA256,
                      license_url=LICENSE_URL, files={name: hashlib.sha256(data).hexdigest() for name, data in sorted(files.items())})
    files['toolchains/llvm/provenance.json'] = (json.dumps(provenance, indent=2, sort_keys=True) + '\n').encode()
    a.out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(a.out, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100755 if name.endswith('.exe') else 0o100644) << 16
            archive.writestr(info, data)
    digest = hashlib.sha256(a.out.read_bytes()).hexdigest()
    summary = dict(name=a.out.name, sha256=digest, bytes=a.out.stat().st_size,
                   unpacked_bytes=sum(map(len, files.values())), **provenance)
    a.out.with_suffix('.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
