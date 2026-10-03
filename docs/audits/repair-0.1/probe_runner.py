"""Standardized probe runner for repair-0.1 audit."""
from pathlib import Path
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
FIXTURES = REPO / 'tests/luna/stdlib/audit_2026_10_02'
EXTERNAL = REPO / 'luna-rs/libs/external'
OUT = Path(__file__).parent / 'evidence'
OUT.mkdir(parents=True, exist_ok=True)

def parse_args():
    parser = argparse.ArgumentParser(description="Run audit reproducers and probes in isolated sysroots.")
    parser.add_argument('--cli', default=os.environ.get('LUNA_CLI'), help="Path to luna CLI binary.")
    parser.add_argument('--runtime-lib', default=os.environ.get('LUNA_RUNTIME_LIB'), help="Path to runtime library.")
    parser.add_argument('--out', default=os.environ.get('LUNA_EVIDENCE_OUT', str(OUT)), help="Evidence output dir.")
    parser.add_argument('--timeout', type=int, default=int(os.environ.get('LUNA_TIMEOUT', '60')), help="Timeout seconds.")
    parser.add_argument('fixtures', nargs='*', help="Optional specific fixtures to run.")
    return parser.parse_args()

def find_cli(configured):
    if configured:
        return Path(configured)
    ext = '.exe' if os.name == 'nt' else ''
    candidates = [
        REPO / f'luna-rs/target/debug/luna{ext}',
        REPO / f'luna-rs/target/release/luna{ext}',
    ]
    for c in candidates:
        if c.exists():
            return c
    return candidates[0]

def find_runtime(configured):
    if configured:
        return Path(configured)
    candidates = [
        REPO / 'build/runtime-win/libluna-runtime.a',
        REPO / 'build/runtime/libluna-runtime.a',
        REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a',
    ]
    for c in candidates:
        if c.exists():
            return c
    return candidates[0]

def main():
    args = parse_args()
    cli = find_cli(args.cli)
    runtime = find_runtime(args.runtime_lib)
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    if os.environ.get('LUNA_TOOLCHAIN_PATH'):
        env['PATH'] = os.environ['LUNA_TOOLCHAIN_PATH'] + os.pathsep + env['PATH']
    env['LUNA_RUNTIME_LIB'] = str(runtime)

    selected = {Path(arg).stem: Path(arg) for arg in args.fixtures} if args.fixtures else {}
    if not selected:
        fixture_paths = sorted(FIXTURES.glob('*.ln'))
    else:
        fixture_paths = []
        for name, p in selected.items():
            if p.is_file():
                fixture_paths.append(p.resolve())
            else:
                target = FIXTURES / f'{name}.ln'
                if target.is_file():
                    fixture_paths.append(target)

    results = []
    bin_ext = '.exe' if os.name == 'nt' else ''

    with tempfile.TemporaryDirectory(prefix='luna_repair_probe_') as temp:
        temp_dir = Path(temp)
        for mode in ('source', 'artifact'):
            sysroot = temp_dir / mode
            ext_dest = sysroot / 'libs/external'
            for file in EXTERNAL.rglob('*'):
                if file.is_file() and (file.suffix == '.toml' or
                    (mode == 'source' and file.suffix == '.ln') or
                    (mode == 'artifact' and file.suffix in ('.llib', '.obj', '.o'))):
                    target = ext_dest / file.relative_to(EXTERNAL)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(file, target)

            env['LUNA_SYSROOT'] = str(sysroot)
            for fixture in fixture_paths:
                binary = sysroot / f'{fixture.stem}{bin_ext}'
                command = [str(cli), 'build', str(fixture), '-o', str(binary), '-q']
                row = {
                    'mode': mode,
                    'fixture': fixture.name,
                    'command': [str(c) for c in command]
                }
                started = time.perf_counter()
                try:
                    compiled = subprocess.run(command, cwd=sysroot, env=env,
                                              capture_output=True, text=True, timeout=args.timeout)
                    compile_time = time.perf_counter() - started
                    row.update(compile_exit=compiled.returncode, compile_seconds=compile_time)
                    log_file = out_dir / f'{mode}-{fixture.stem}.log'
                    log_file.write_text(compiled.stdout + compiled.stderr, encoding='utf-8')

                    if compiled.returncode == 0:
                        run_start = time.perf_counter()
                        try:
                            ran = subprocess.run([str(binary), 'audit-argument'],
                                                 cwd=sysroot, env=env,
                                                 capture_output=True, text=True, timeout=10)
                            row.update(run_exit=ran.returncode, run_seconds=time.perf_counter() - run_start,
                                       stdout=ran.stdout, stderr=ran.stderr)
                        except subprocess.TimeoutExpired:
                            row.update(run_exit=-1, run_seconds=time.perf_counter() - run_start,
                                       timeout=True, stderr="Execution timed out after 10s")
                except subprocess.TimeoutExpired:
                    row.update(compile_exit=-1, compile_seconds=time.perf_counter() - started,
                               timeout=True, compile_stderr="Compilation timed out")

                results.append(row)
                print(f"[{mode}] {fixture.name:<35} compile={row.get('compile_exit')} "
                      f"run={row.get('run_exit')} timeout={'timeout' in row}", flush=True)

    json_path = out_dir / 'probes.json'
    json_path.write_text(json.dumps(results, indent=2), encoding='utf-8')
    print(f"\nSaved {len(results)} probe outcomes to {json_path}")

if __name__ == '__main__':
    main()
