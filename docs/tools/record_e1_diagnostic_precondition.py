"""Record E1-FU-DIAG-01 reproductions without changing diagnostic ownership.

Successful reproduction of E0005 is evidence of the existing violation, not
DIAG-8 conformance. Compiler phase attribution is established by the source
trace in REPORT-E1-DIAGNOSTIC-BLOCKER-2026-10-12.md.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path,
                        default=ROOT / 'luna-rs/target/debug/luna.exe')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    cases = [
        ('reserved_declaration', 'tests/luna/language/diagnostic_ownership/reserved_intrinsic_declaration.ln', 'E0005'),
        ('reserved_link', 'tests/luna/language/diagnostic_ownership/reserved_intrinsic_link.ln', 'E0005'),
        ('link_on_main', 'tests/luna/language/ffi_attributes/invalid_link_placement.ln', 'E0005'),
    ]
    for name in ('comptime_division_by_zero', 'comptime_shift_out_of_range',
                 'comptime_signed_overflow', 'comptime_unary_neg_min'):
        cases.append((name, f'tests/luna/language/checked_arithmetic/{name}.ln', 'E4005'))
    env = os.environ.copy()
    env['LUNA_SYSROOT'] = str(ROOT / 'luna-rs')
    rows = []
    with tempfile.TemporaryDirectory(prefix='luna_e1_diag_') as temporary:
        work = Path(temporary)
        for name, fixture, expected in cases:
            for command in ('check', 'build'):
                executable = work / f'{name}.exe'
                argv = [str(cli), command, str(ROOT / fixture), '--quiet']
                if command == 'build':
                    argv += ['-o', str(executable)]
                result = subprocess.run(argv, cwd=work, env=env,
                                        capture_output=True, text=True, timeout=120)
                codes = re.findall(r'error\[(E\d+)\]', result.stderr)
                reproduced = (result.returncode == 1 and codes == [expected]
                              and not executable.exists())
                rows.append(dict(case=name, command=command, fixture=fixture,
                                 exit=result.returncode, diagnostic_codes=codes,
                                 stdout=result.stdout, stderr=result.stderr,
                                 executable_published=executable.exists(),
                                 expected_rejection_reproduced=reproduced))
                print(f'{name}/{command}: exit={result.returncode} codes={codes} '
                      f'reproduced={reproduced}', flush=True)
    sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    reproduced = all(row['expected_rejection_reproduced'] for row in rows)
    evidence = dict(task='E1 diagnostic precondition audit', source_sha=sha,
                    cli=str(cli), cli_sha256=hashlib.sha256(cli.read_bytes()).hexdigest(),
                    provider_mode='existing sysroot; not a fresh-artifact parity gate',
                    llvm_runtime_root=env.get('LLVM_SYS_180_PREFIX'),
                    diagnostic_contract='DIAG-8; E0005 is owned by Parser',
                    verdict=('BLOCKED: E0005 emitted outside Parser; reproduction is not conformance'
                             if reproduced else 'NOT VERIFIED: reproduction acquisition failed'),
                    rows=rows)
    args.output.write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    return 0 if reproduced else 1


if __name__ == '__main__':
    raise SystemExit(main())
