"""Record focused CLI rejections for the adopted E1 diagnostic amendment.

CLI evidence checks identity/parity; the structural phase-ownership suite
establishes actual compiler ownership. Historical blocker evidence is immutable.
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
    parser.add_argument('--sysroot', type=Path, default=ROOT / 'luna-rs')
    parser.add_argument('--baseline', action='store_true',
                        help='Reproduce the historical E0005 blocker with an old CLI')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Evidence output already exists; choose a new acquisition label')
    cli = args.cli.resolve()
    cases = [
        ('reserved_declaration', 'tests/luna/language/diagnostic_ownership/reserved_intrinsic_declaration.ln', 'E1009'),
        ('reserved_link', 'tests/luna/language/diagnostic_ownership/reserved_intrinsic_link.ln', 'E1009'),
        ('link_on_main', 'tests/luna/language/ffi_attributes/invalid_link_placement.ln', 'E2032'),
    ]
    if args.baseline:
        cases = [(name, fixture, 'E0005') for name, fixture, _ in cases]
    else:
        for name, expected in [('unknown_attribute', 'E0005'), ('malformed_attribute', 'E0005'),
                               ('invalid_application', 'E2032'), ('macro_as_value', 'E2001'),
                               ('invalid_integer_syntax', 'E0003'),
                               ('macro_rule_mismatch', 'E1010'), ('macro_recursion', 'E1010'),
                               ('unsupported_comptime_statement', 'E4005')]:
            cases.append((name, f'tests/luna/language/diagnostic_ownership/{name}.ln', expected))
    for name in ('comptime_division_by_zero', 'comptime_shift_out_of_range',
                 'comptime_signed_overflow', 'comptime_unary_neg_min'):
        cases.append((name, f'tests/luna/language/checked_arithmetic/{name}.ln', 'E4005'))
    env = os.environ.copy()
    env['LUNA_SYSROOT'] = str(args.sysroot.resolve())
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
                    provider_mode='selected sysroot; parity separately verified by CLI harness',
                    sysroot=env['LUNA_SYSROOT'],
                    llvm_runtime_root=env.get('LLVM_SYS_180_PREFIX'),
                    diagnostic_contract='DIAG-8; E0005 is owned by Parser',
                    verdict=(('BLOCKED: historical out-of-phase E0005 reproduced' if args.baseline
                              else 'PASS: adopted diagnostic identities; structural ownership verified separately')
                             if reproduced else 'NOT VERIFIED: acquisition failed'),
                    rows=rows)
    args.output.write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    return 0 if reproduced else 1


if __name__ == '__main__':
    raise SystemExit(main())
