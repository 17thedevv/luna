"""Reproduce audit probes through CLI, using isolated source/artifact sysroots."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
CLI = REPO / 'luna-rs/target/debug/luna.exe'
EXTERNAL = REPO / 'luna-rs/libs/external'
FIXTURES = REPO / 'tests/luna/stdlib/audit_2026_10_02'
OUT = Path(__file__).parent / 'evidence'
OUT.mkdir(exist_ok=True)
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
selected = {Path(arg).name for arg in sys.argv[1:]}
fixtures = list(FIXTURES.glob('*.ln'))
fixtures += [Path(arg).resolve() for arg in sys.argv[1:] if Path(arg).is_file()
             and Path(arg).resolve().parent != FIXTURES]
results = json.loads((OUT / 'probes.json').read_text()) if selected and (OUT / 'probes.json').exists() else []
results = [row for row in results if row['fixture'] not in selected]
with tempfile.TemporaryDirectory(prefix='luna_stdlib_audit_') as temp:
    temp = Path(temp)
    for mode in ('source', 'artifact'):
        root = temp / mode
        dest = root / 'libs/external'
        for file in EXTERNAL.rglob('*'):
            if file.is_file() and (file.suffix == '.toml' or
                (mode == 'source' and file.suffix == '.ln') or
                (mode == 'artifact' and file.suffix in ('.llib', '.obj', '.o'))):
                target = dest / file.relative_to(EXTERNAL)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(file, target)
        env['LUNA_SYSROOT'] = str(root)
        for fixture in sorted(fixtures):
            if selected and fixture.name not in selected:
                continue
            binary = root / (fixture.stem + '.exe')
            command = [str(CLI), 'build', str(fixture), '-o', str(binary), '-q']
            row = {'mode': mode, 'fixture': fixture.name, 'command': command}
            try:
                compiled = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True, timeout=60)
                row.update(compile_exit=compiled.returncode, compile_stdout=compiled.stdout,
                           compile_stderr=compiled.stderr)
                if compiled.returncode == 0:
                    started = time.perf_counter()
                    ran = subprocess.run([str(binary), 'audit-argument'], cwd=root, env=env, capture_output=True, text=True, timeout=10)
                    row.update(run_exit=ran.returncode, stdout=ran.stdout, stderr=ran.stderr)
                    row['run_seconds'] = time.perf_counter() - started
            except subprocess.TimeoutExpired as error:
                row['timeout'] = str(error)
            results.append(row)
            print(mode, fixture.name, 'compile=', row.get('compile_exit'), 'run=', row.get('run_exit'),
                  'timeout=', 'timeout' in row, flush=True)
            (OUT / 'probes.json').write_text(json.dumps(results, indent=2), encoding='utf-8')
