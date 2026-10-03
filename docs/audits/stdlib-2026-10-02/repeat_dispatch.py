"""Check determinism of method dispatch by compiling identical inputs repeatedly."""
import json
import os
from pathlib import Path
import subprocess
REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
WORK = Path('D:/tmp/luna-audit-2026-10-02/dispatch_repeat')
WORK.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_SYSROOT'] = str(REPO / 'luna-rs')
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
CLI = REPO / 'luna-rs/target/debug/luna.exe'
rows = []
for fixture in (REPO / 'tests/luna/stdlib/audit_2026_10_02/compiler_inherent_trait_dispatch.ln',
                REPO / 'luna-rs/tests/luna/stdlib/phase3/hashmap_owned_iteration.ln'):
    for attempt in range(8):
        binary = WORK / f'{fixture.stem}_{attempt}.exe'
        p = subprocess.run([str(CLI), 'build', str(fixture), '-q', '-o', str(binary)],
                           env=env, capture_output=True, text=True, timeout=60)
        row = dict(fixture=fixture.name, attempt=attempt, compile_exit=p.returncode,
                   compile_stderr=p.stderr)
        if p.returncode == 0:
            ran = subprocess.run([str(binary)], env=env, capture_output=True, text=True, timeout=10)
            row.update(run_exit=ran.returncode, stdout=ran.stdout, stderr=ran.stderr)
        rows.append(row)
        print(row['fixture'], attempt, row.get('run_exit'), flush=True)
        (OUT / 'dispatch-repeat.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
