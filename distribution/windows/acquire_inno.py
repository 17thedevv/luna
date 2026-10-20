"""Acquire and verify the pinned Inno Setup compiler for a build job."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.request


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    a.out = a.out.resolve()
    a.out.mkdir(parents=True, exist_ok=True)
    lock = json.loads(Path(__file__).with_name('installer-lock.json').read_text())['inno_setup']
    archive = a.out / lock['name']
    with urllib.request.urlopen(lock['url'], timeout=120) as response, archive.open('wb') as output:
        while data := response.read(1024 * 1024):
            output.write(data)
    with archive.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == lock['sha256'], 'Inno Setup SHA-256 mismatch'
    subprocess.run([str(archive), '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER',
                    '/NOICONS', '/DIR=' + str(a.out / 'compiler')], check=True)
    assert (a.out / 'compiler/ISCC.exe').is_file()


if __name__ == '__main__':
    main()
