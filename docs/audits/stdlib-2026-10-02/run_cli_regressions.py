"""Run all existing standalone stdlib fixtures in source and artifact modes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
CLI = REPO / 'luna-rs/target/debug/luna.exe'
EXTERNAL = REPO / 'luna-rs/libs/external'
FIXTURES = REPO / 'luna-rs/tests/luna/stdlib'
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
env['TEMP'] = env['TMP'] = 'D:/tmp/luna-audit-2026-10-02'
rows = []
with tempfile.TemporaryDirectory(prefix='luna_cli_regression_', dir=env['TEMP']) as directory:
    directory = Path(directory)
    for mode in ('source', 'artifact'):
        root = directory / mode
        dest = root / 'libs/external'
        for file in EXTERNAL.rglob('*'):
            if file.is_file() and (file.suffix == '.toml' or
                (mode == 'source' and file.suffix == '.ln') or
                (mode == 'artifact' and file.suffix in ('.llib', '.obj', '.o'))):
                target = dest / file.relative_to(EXTERNAL)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(file, target)
        env['LUNA_SYSROOT'] = str(root)
        for index, fixture in enumerate(sorted(FIXTURES.rglob('*.ln'))):
            negative = fixture.name.startswith('reject_')
            work = root / f'case_{index}'
            work.mkdir()
            binary = work / 'main.exe'
            row = {'mode': mode, 'fixture': str(fixture.relative_to(FIXTURES)), 'negative': negative}
            try:
                command = [str(CLI), 'check' if negative else 'build', str(fixture), '-q']
                if not negative:
                    command += ['-o', str(binary)]
                p = subprocess.run(command, env=env, capture_output=True, text=True, timeout=60)
                row.update(compile_exit=p.returncode, compile_stdout=p.stdout, compile_stderr=p.stderr)
                row['pass'] = p.returncode != 0 if negative else False
                if not negative and p.returncode == 0:
                    if fixture.name == 'whole_file_io_v1.ln':
                        (work / 'source.bin').write_bytes(bytes([0, 255, 128, 1, 127, 0, 195, 169]))
                        (work / 'text-source.bin').write_bytes(b'luna text\n')
                        (work / 'empty-source.bin').write_bytes(b'')
                        (work / 'large-source.bin').write_bytes(bytes((i * 31) % 256 for i in range(8193)))
                    ran = subprocess.run([str(binary)], cwd=work, env=env, capture_output=True, text=True, timeout=15)
                    row.update(run_exit=ran.returncode, stdout=ran.stdout, stderr=ran.stderr)
                    row['pass'] = ran.returncode == 0
                    if fixture.name == 'io_runtime.ln':
                        row['pass'] &= ran.stdout == 'Hello, World!\n' and ran.stderr == 'Error!\n'
            except subprocess.TimeoutExpired as error:
                row.update(timeout=str(error), **{'pass': False})
            rows.append(row)
            print(mode, row['fixture'], 'PASS' if row['pass'] else 'FAIL', row.get('compile_exit'), row.get('run_exit'), flush=True)
            (OUT / 'cli-regressions.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
