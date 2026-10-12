"""E1 freeze matrix recorder for checked integer arithmetic.

Compiles and runs every reducer in `tests/luna/language/checked_arithmetic/` and
records the observed behavior. TRAP requires Luna's arithmetic panic signature;
crashes, signals, timeouts and ICEs never count as arithmetic traps. Positive
controls and wrapping reducers must return zero after checking their result.

Run it once before the fix (Stage 1 baseline) and once after (Stage 2), and
compare the two JSON files: the diff is the evidence.

Usage:
  python docs/tools/record_checked_arithmetic.py <stage-label> [--cli PATH]
"""
from pathlib import Path
import argparse
from collections import Counter
import hashlib
import json
import os
import re
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
    parser.add_argument('--profile', choices=['debug', 'release'], default='debug')
    parser.add_argument('--provider-mode', choices=['existing', 'source', 'fresh-artifact'], default='existing')
    return parser.parse_args()


def main():
    args = parse_args()
    inputs = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'luna-rs', 'runtime'],
        cwd=ROOT, text=True).splitlines()
    input_digest = hashlib.sha256()
    for name in sorted(set(inputs)):
        path = ROOT / name
        if path.is_file():
            input_digest.update(name.encode() + b'\0' + path.read_bytes() + b'\0')
    output = OUT / f'e1-{args.label}.json'
    metadata = output.with_suffix('.meta.json')
    if output.exists() or metadata.exists():
        raise SystemExit('Evidence already exists; choose a new label to preserve history.')
    env = os.environ.copy()
    env['LUNA_SYSROOT'] = args.sysroot
    rows = []
    with tempfile.TemporaryDirectory(prefix='luna_e1_arith_') as tmp:
        tmp = Path(tmp)
        for fixture in sorted(FIXTURES.glob('*.ln')):
            name = fixture.stem
            exe = tmp / f'{name}.exe'
            row = {'fixture': name, 'expected': expected(name), 'run_exit': None,
                   'source_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest()}
            try:
                build = subprocess.run([args.cli, 'build', str(fixture), '-q', '-o', str(exe), '-I', args.sysroot],
                                       cwd=tmp, env=env, capture_output=True, text=True, timeout=args.timeout)
                combined = (build.stdout or '') + (build.stderr or '')
                row['compile_exit'] = build.returncode
                row['compile_stderr'] = combined
                row['diagnostics'] = re.findall(r'^error\[E\d+\].*', combined, re.M)
                if build.returncode == 0 and exe.exists():
                    try:
                        run = subprocess.run([str(exe)], cwd=tmp, capture_output=True, text=True, timeout=15)
                        row['run_exit'] = run.returncode
                        row['run_stderr'] = run.stderr or ''
                    except subprocess.TimeoutExpired:
                        row.update(run_exit=None, timeout=True)
                else:
                    row.update(run_exit=None, trap=None)
            except subprocess.TimeoutExpired:
                row.update(compile_exit=-1, timeout=True)
            row['outcome'] = classify(row)
            row['trap'] = row['outcome'] == 'TRAP'
            row['contract_met'] = row['outcome'] == row['expected']
            if row['expected'] == 'COMPILE_ERROR':
                row['contract_met'] &= (len(row.get('diagnostics', [])) == 1
                                        and row['diagnostics'][0].startswith('error[E4005]'))
            rows.append(row)
            print(f"{name}: compile={row.get('compile_exit')} run={row.get('run_exit')} "
                  f"outcome={row['outcome']} {row.get('diagnostics')}", flush=True)

    output.write_text(json.dumps(rows, indent=2), encoding='utf-8')
    trapped = sum(1 for row in rows if row.get('trap'))
    clean = sum(1 for row in rows if row.get('trap') is False)
    failed = sum(1 for row in rows if row.get('compile_exit') not in (0, None))
    print(f"\nrows={len(rows)} trapped={trapped} completed_without_trap={clean} compile_failed={failed}")
    summary = dict(source_sha=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                   tracked_diff_sha256=hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD', '--', 'luna-rs', 'runtime'], cwd=ROOT)).hexdigest(),
                   cli_sha256=hashlib.sha256(Path(args.cli).read_bytes()).hexdigest(),
                   source_input_sha256=input_digest.hexdigest(),
                   cli=args.cli, sysroot=args.sysroot, profile=args.profile, provider_mode=args.provider_mode,
                   rows=len(rows), outcomes=dict(Counter(row['outcome'] for row in rows)),
                   contract_met=all(row['contract_met'] for row in rows))
    metadata.write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(json.dumps(summary, indent=2), flush=True)
    return 0 if summary['contract_met'] else 1


def classify(row):
    """Fine-grained outcome: distinguishes a real trap from an unexpected crash.

    A non-zero exit alone is not evidence of a trap. Runtime ABI codes L003
    (division by zero) and L005 (integer overflow) require matching panic text.
    """
    if row.get('timeout'):
        return 'UNEXPECTED_EXIT'
    compiler = row.get('compile_stderr', '')
    if row.get('compile_exit') != 0:
        if ('error[E6001]' in compiler or 'internal compiler error' in compiler.lower()
                or 'panicked at' in compiler or 'invariant violated' in compiler):
            return 'ICE'
        return 'COMPILE_ERROR' if re.search(r'error\[E\d+\]', compiler) else 'UNEXPECTED_EXIT'
    run = row.get('run_exit')
    if run == 0:
        return 'PASS' if row['fixture'].startswith('positive_') else 'WRAP'
    text = row.get('run_stderr') or ''
    if run is not None and re.search(r'luna: PANIC \[(?:L003\] division by zero|L005\] integer overflow)\b', text):
        return 'TRAP'
    return 'UNEXPECTED_EXIT'


def expected(name):
    if name.startswith('comptime_'):
        return 'COMPILE_ERROR'
    if name.startswith('wrapping_'):
        return 'WRAP'
    return 'PASS' if name.startswith('positive_') else 'TRAP'


if __name__ == '__main__':
    raise SystemExit(main())
