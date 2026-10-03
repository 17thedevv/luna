"""Run already-built stdlib harnesses without compiling extra Rust targets."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import shutil
from concurrent.futures import ThreadPoolExecutor, as_completed
REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_SYSROOT'] = str(REPO / 'luna-rs')
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
env['TEMP'] = env['TMP'] = 'D:/tmp/luna-audit-2026-10-02'
results_path = OUT / 'existing-suites.json'
rows = json.loads(results_path.read_text()) if results_path.exists() else []
completed = {row['suite'] for row in rows if 'exit' in row or row.get('timeout')}
rows = [row for row in rows if row['suite'] in completed]
def run_suite(binary):
    row = {'suite': binary.stem}
    logfile = OUT / (binary.stem + '.log')
    try:
        with tempfile.TemporaryDirectory(prefix='luna_suite_', dir=env['TEMP']) as directory:
            root = Path(directory)
            shutil.copytree(REPO / 'luna-rs/libs/external', root / 'libs/external')
            suite_env = dict(env, LUNA_SYSROOT=str(root))
            with logfile.open('w', encoding='utf-8') as log:
                p = subprocess.run([str(binary), '--test-threads=1'], cwd=root,
                                   env=suite_env, stdout=log, stderr=subprocess.STDOUT, timeout=600)
                row['exit'] = p.returncode
    except subprocess.TimeoutExpired:
        row['timeout'] = True
    return row
with ThreadPoolExecutor(max_workers=3) as executor:
    binaries = sorted((REPO / 'luna-rs/target/debug/deps').glob('stdlib_*.exe'))
    binaries += sorted(Path('D:/tmp/luna-audit-2026-10-02/cargo-target/debug/deps').glob('stdlib_*.exe'))
    pending = [executor.submit(run_suite, binary) for binary in
               binaries if binary.stem not in completed]
    for future in as_completed(pending):
        row = future.result()
        rows.append(row)
        print(row, flush=True)
        results_path.write_text(json.dumps(rows, indent=2), encoding='utf-8')
