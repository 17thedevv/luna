"""E1 freeze matrix recorder for checked integer arithmetic.

Compiles and runs every reducer in `tests/luna/language/checked_arithmetic/` and
records the observed behavior. Exit 0 means the operation completed with no trap
(the pre-E1 wrapped/UB behavior); an abnormal/nonzero exit means a trap; a
nonzero compile exit means a compile diagnostic.

Run it once before the fix (Stage 1 baseline) and once after (Stage 2), and
compare the two JSON files: the diff is the evidence.

Usage:
  python docs/tools/record_checked_arithmetic.py <stage-label> [--cli PATH]
"""
from pathlib import Path
import argparse
import json
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tests/luna/language/checked_arithmetic'
OUT = ROOT / 'docs/audits/0.1-alpha-completion-2026-10-04/evidence'


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label', help="Evidence label, e.g. stage1-baseline or stage2-checked")
    parser.add_argument('--cli', default=str(ROOT / 'luna-rs/target/debug/luna.exe'))
    parser.add_argument('--sysroot', default=str(ROOT / 'luna-rs'))
    parser.add_argument('--timeout', type=int, default=120)
    return parser.parse_args()


def main():
    args = parse_args()
    env = os.environ.copy()
    env['LUNA_SYSROOT'] = args.sysroot
    rows = []
    with tempfile.TemporaryDirectory(prefix='luna_e1_arith_') as tmp:
        tmp = Path(tmp)
        for fixture in sorted(FIXTURES.glob('*.ln')):
            name = fixture.stem
            exe = tmp / f'{name}.exe'
            row = {'fixture': name}
            try:
                build = subprocess.run([args.cli, 'build', str(fixture), '-q', '-o', str(exe)],
                                       env=env, capture_output=True, text=True, timeout=args.timeout)
                combined = (build.stdout or '') + (build.stderr or '')
                row['compile_exit'] = build.returncode
                row['diagnostics'] = [line for line in combined.splitlines() if 'error' in line.lower()][:4]
                if build.returncode == 0 and exe.exists():
                    try:
                        run = subprocess.run([str(exe)], capture_output=True, text=True, timeout=15)
                        row['run_exit'] = run.returncode
                        row['run_stderr'] = (run.stderr or '')[:400]
                        row['trap'] = run.returncode != 0
                    except subprocess.TimeoutExpired:
                        row.update(run_exit=-1, trap=True, timeout=True)
                else:
                    row.update(run_exit=None, trap=None)
            except subprocess.TimeoutExpired:
                row.update(compile_exit=-1, timeout=True)
            row['outcome'] = classify(row)
            rows.append(row)
            print(f"{name}: compile={row.get('compile_exit')} run={row.get('run_exit')} "
                  f"outcome={row['outcome']} {row.get('diagnostics')}", flush=True)

    (OUT / f'e1-{args.label}.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
    trapped = sum(1 for row in rows if row.get('trap'))
    clean = sum(1 for row in rows if row.get('trap') is False)
    failed = sum(1 for row in rows if row.get('compile_exit') not in (0, None))
    print(f"\nrows={len(rows)} trapped={trapped} completed_without_trap={clean} compile_failed={failed}")


def classify(row):
    """Fine-grained outcome: distinguishes a real trap from an unexpected crash.

    A non-zero exit alone is not evidence of a trap: the runtime signature (a
    Luna panic mentioning the arithmetic failure) or a known trap status must
    confirm it.
    """
    if row.get('compile_exit') not in (0, None):
        return 'COMPILE_ERROR'
    if row.get('timeout'):
        return 'TIMEOUT'
    run = row.get('run_exit')
    if run == 0:
        return 'NO_TRAP'
    text = (row.get('run_stderr') or '').lower()
    if 'overflow' in text or 'division by zero' in text or 'divide by zero' in text or 'shift' in text:
        return 'TRAP'
    # Windows status codes: integer divide by zero / illegal instruction.
    if run in (0xC0000094, 0xC000001D, 0xC0000409) or run < 0:
        return 'TRAP'
    return 'UNEXPECTED_EXIT'


if __name__ == '__main__':
    main()
