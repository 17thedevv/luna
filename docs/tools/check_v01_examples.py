"""Record public CLI behavior separately from retained 0.1 requirements."""
from pathlib import Path
import json
import os
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tests/luna/language/spec_v01'
CLI = Path(os.environ.get('LUNA_DOCS_CLI', str(ROOT / 'luna-rs/target/debug/luna.exe')))
OUT = ROOT / 'docs/spec/0.1/evidence'
OUT.mkdir(exist_ok=True)
BASE = Path(os.environ.get('LUNA_DOCS_TEMP', tempfile.gettempdir()))
BASE.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
if os.environ.get('LUNA_DOCS_TOOLCHAIN_PATH'):
    env['PATH'] = os.environ['LUNA_DOCS_TOOLCHAIN_PATH'] + os.pathsep + env['PATH']
env.setdefault('LUNA_RUNTIME_LIB', str(ROOT / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a'))
rows = []
with tempfile.TemporaryDirectory(prefix='luna_v01_docs_', dir=BASE) as temporary:
    for mode in ('source', 'artifact'):
        root = Path(temporary) / mode
        for path in (ROOT / 'luna-rs/libs/external').rglob('*'):
            admitted = ('.ln', '.toml') if mode == 'source' else ('.llib', '.obj', '.o', '.toml')
            if path.is_file() and path.suffix in admitted:
                target = root / 'libs/external' / path.relative_to(ROOT / 'luna-rs/libs/external')
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, target)
        local_env = dict(env, LUNA_SYSROOT=str(root))
        for fixture in sorted(FIXTURES.glob('*.ln')):
            name = fixture.stem
            binary = root / (name + '.exe')
            expected = 'reject' if name.startswith('reject_') else 'accept'
            if name == 'foreach_current_parser':
                expected = 'characterization'
            started = time.perf_counter()
            compiled = subprocess.run([str(CLI), 'build', str(fixture), '-q', '-o', str(binary)],
                cwd=root, env=local_env, capture_output=True, text=True, timeout=60)
            row = dict(mode=mode, fixture=name, expectation=expected,
                       compile_exit=compiled.returncode, compile_seconds=time.perf_counter()-started)
            (OUT / f'{mode}-{name}.log').write_text(compiled.stdout+compiled.stderr, encoding='utf-8')
            if compiled.returncode == 0:
                ran = subprocess.run([str(binary)], cwd=root, env=local_env,
                                     capture_output=True, text=True, timeout=10)
                row.update(run_exit=ran.returncode, stdout=ran.stdout, stderr=ran.stderr)
            row['contract_met'] = (compiled.returncode != 0 if expected == 'reject'
                else (row.get('run_exit') == 0 if expected == 'accept' else None))
            if name == 'reject_private_field':
                row['contract_met'] = row['contract_met'] and 'error[E1003]' in compiled.stderr + compiled.stdout
            if name == 'macro_name_independent':
                row['contract_met'] = row['contract_met'] and row.get('stdout') == '' and row.get('stderr') == ''
            rows.append(row)
            (OUT / 'cli-examples.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
            print(row, flush=True)
print('The result records known gaps; it does not redefine expectations to turn the compiler green.')
