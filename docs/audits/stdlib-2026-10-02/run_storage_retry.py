"""Bounded individual retries of the storage cases left after the suite timeout."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
BINARY = next((REPO / 'luna-rs/target/debug/deps').glob('stdlib_hashmap_storage_acceptance_tests-*.exe'))
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
env['TEMP'] = env['TMP'] = 'D:/tmp/luna-audit-2026-10-02'
rows = []
for test in ('test_ht5_resize_growth_and_relocation',
             'test_ht6_drop_tracker_stress_soundness', 'test_ht7_source_vs_llib_parity'):
    with tempfile.TemporaryDirectory(prefix='luna_storage_retry_', dir=env['TEMP']) as directory:
        root = Path(directory)
        shutil.copytree(REPO / 'luna-rs/libs/external', root / 'libs/external')
        with (OUT / (test + '.log')).open('w', encoding='utf-8') as log:
            process = subprocess.Popen([str(BINARY), '--exact', test, '--test-threads=1', '--nocapture'],
                cwd=root, env=dict(env, LUNA_SYSROOT=str(root)), stdout=log, stderr=subprocess.STDOUT)
            row = {'test': test, 'timeout_seconds': 120}
            try:
                row['exit'] = process.wait(timeout=120)
            except subprocess.TimeoutExpired:
                # Stop only this runner and the subprocesses it created.
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                    capture_output=True, check=False)
                process.wait()
                row['timeout'] = True
    rows.append(row)
    print(row, flush=True)
    (OUT / 'storage-retry.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
