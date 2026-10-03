"""Record public CLI behavior separately from retained 0.1 requirements."""
from pathlib import Path
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tests/luna/language/spec_v01'

def parse_args():
    parser = argparse.ArgumentParser(description="Record public CLI behavior or gate against 0.1 requirements.")
    parser.add_argument('--gate', action='store_true', default=(os.environ.get('LUNA_DOCS_GATE') == '1'),
                        help="Exit nonzero if any contract is not met.")
    parser.add_argument('--cli', default=os.environ.get('LUNA_DOCS_CLI'),
                        help="Path to luna executable.")
    parser.add_argument('--runtime-lib', default=os.environ.get('LUNA_RUNTIME_LIB'),
                        help="Path to luna runtime static library.")
    parser.add_argument('--out', default=os.environ.get('LUNA_DOCS_EVIDENCE_OUT'),
                        help="Directory to write logs and json evidence.")
    parser.add_argument('--timeout', type=int, default=int(os.environ.get('LUNA_DOCS_TIMEOUT', '60')),
                        help="Compile timeout in seconds.")
    return parser.parse_args()

def find_cli(configured):
    if configured:
        return Path(configured)
    ext = '.exe' if os.name == 'nt' else ''
    candidates = [
        ROOT / f'luna-rs/target/debug/luna{ext}',
        ROOT / f'luna-rs/target/release/luna{ext}',
    ]
    for c in candidates:
        if c.exists():
            return c
    return candidates[0]

def find_runtime_lib(configured):
    if configured:
        return Path(configured)
    candidates = [
        ROOT / 'build/runtime-win/libluna-runtime.a',
        ROOT / 'build/runtime/libluna-runtime.a',
        ROOT / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a',
    ]
    for c in candidates:
        if c.exists():
            return c
    return candidates[0]

args = parse_args()
CLI = find_cli(args.cli)
OUT = Path(args.out if args.out else (ROOT / 'docs/spec/0.1/evidence'))
OUT.mkdir(parents=True, exist_ok=True)
BASE = Path(os.environ.get('LUNA_DOCS_TEMP', tempfile.gettempdir()))
BASE.mkdir(parents=True, exist_ok=True)

env = os.environ.copy()
if os.environ.get('LUNA_DOCS_TOOLCHAIN_PATH'):
    env['PATH'] = os.environ['LUNA_DOCS_TOOLCHAIN_PATH'] + os.pathsep + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(find_runtime_lib(args.runtime_lib))

rows = []
bin_ext = '.exe' if os.name == 'nt' else ''

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
            binary = root / f'{name}{bin_ext}'
            expected = 'reject' if name.startswith('reject_') else 'accept'
            if name == 'foreach_current_parser':
                expected = 'characterization'
            started = time.perf_counter()
            row = dict(mode=mode, fixture=name, expectation=expected)
            try:
                compiled = subprocess.run([str(CLI), 'build', str(fixture), '-q', '-o', str(binary)],
                    cwd=root, env=local_env, capture_output=True, text=True, timeout=args.timeout)
                row.update(compile_exit=compiled.returncode, compile_seconds=time.perf_counter() - started)
                (OUT / f'{mode}-{name}.log').write_text(compiled.stdout + compiled.stderr, encoding='utf-8')
                if compiled.returncode == 0:
                    try:
                        ran = subprocess.run([str(binary)], cwd=root, env=local_env,
                                             capture_output=True, text=True, timeout=10)
                        row.update(run_exit=ran.returncode, stdout=ran.stdout, stderr=ran.stderr)
                    except subprocess.TimeoutExpired:
                        row.update(run_exit=-1, timeout=True, stderr="Execution timed out after 10s")
                row['contract_met'] = (compiled.returncode != 0 if expected == 'reject'
                    else (row.get('run_exit') == 0 if expected == 'accept' else None))
                if name == 'reject_private_field':
                    row['contract_met'] = bool(row['contract_met'] and 'error[E1003]' in compiled.stderr + compiled.stdout)
                if name == 'macro_name_independent':
                    row['contract_met'] = bool(row['contract_met'] and row.get('stdout') == '' and row.get('stderr') == '')
            except subprocess.TimeoutExpired:
                row.update(compile_exit=-1, compile_seconds=time.perf_counter() - started, timeout=True, contract_met=False)
                (OUT / f'{mode}-{name}.log').write_text("Compilation timed out after {args.timeout}s", encoding='utf-8')

            rows.append(row)
            (OUT / 'cli-examples.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
            print(row, flush=True)

unmet = [r for r in rows if r.get('expectation') in ('accept', 'reject') and not r.get('contract_met')]
if args.gate:
    if unmet:
        print(f"\n[GATE FAILED] {len(unmet)} contract(s) unmet:")
        for u in unmet:
            print(f"  - mode={u['mode']} fixture={u['fixture']} expected={u['expectation']}")
        sys.exit(1)
    else:
        print("\n[GATE PASSED] All conformance contracts satisfied.")
        sys.exit(0)
else:
    print('\nThe result records known gaps; it does not redefine expectations to turn the compiler green.')
