"""Locate storage timeout by replaying the same pre-MVIR pipeline stages."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
BASE = Path('D:/tmp/luna-audit-2026-10-02')
HELPER = BASE / 'cargo-target/debug/audit-phase-probe.exe'
test_source = (REPO / 'luna-rs/crates/luna-driver/tests/stdlib_hashmap_storage_acceptance_tests.rs').read_text()
def original(name):
    section = test_source.split('fn ' + name + '()', 1)[1]
    return re.search(r'let src = r#"(.*?)"#;', section, re.S).group(1)
ht5 = original('test_ht5_resize_growth_and_relocation')
ht6 = original('test_ht6_drop_tracker_stress_soundness')
imports = ht5.split('fn main()', 1)[0]
no_get = ht5[:ht5.index('// Verify all 20 elements')] + 'return 0; }'
public5 = ht5.replace('import <__raw_table>;', '').replace('raw_table_with_capacity', 'std::hashmap_with_capacity')
public5 = re.sub(r'// Tombstones must be 0 after resize.*?return 5;\s*}', '', public5, flags=re.S)
cases = [
    ('raw_create_only', imports + 'fn main() -> i32 { dec table = raw_table_with_capacity<i32,i32>(8 as u64); if table.len() != (0 as u64) { return 1; } return 0; }', True),
    ('raw_growth_without_get', no_get, True),
    ('raw_ht5_exact', ht5, True),
    ('public_ht5_equivalent', public5, False),
    ('raw_ht6_exact', ht6, True),
]
followup = '--bootstrap-comparison' in sys.argv
if followup:
    cases = [('raw_ht5_without_bootstrap', ht5, 'internal'),
             ('raw_ht5_with_bootstrap', ht5, 'internal-bootstrap')]
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
rows = []
with tempfile.TemporaryDirectory(prefix='luna_phase_', dir=BASE) as directory:
    directory = Path(directory)
    for mode in ('source', 'artifact'):
        root = directory / mode
        for path in (REPO / 'luna-rs/libs/external').rglob('*'):
            if path.is_file() and (path.suffix == '.toml' or
                (mode == 'source' and path.suffix == '.ln') or
                (mode == 'artifact' and path.suffix in ('.llib', '.obj', '.o'))):
                target = root / 'libs/external' / path.relative_to(REPO / 'luna-rs/libs/external')
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, target)
        for name, source, internal in cases:
            fixture = root / (name + '.ln')
            fixture.write_text(source, encoding='utf-8')
            switch = internal if isinstance(internal, str) else 'internal'
            command = [str(HELPER), str(fixture), str(root)] + ([switch] if internal else [])
            started = time.perf_counter()
            row = dict(mode=mode, case=name, internal=internal, timeout_seconds=30)
            logpath = OUT / f'phase-{mode}-{name}.log'
            with logpath.open('w', encoding='utf-8') as log:
                process = subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
                try:
                    row['exit'] = process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    row['timeout'] = True
            row['seconds'] = time.perf_counter() - started
            row['log'] = logpath.name
            logtext = logpath.read_text(encoding='utf-8')
            row['last_phase'] = re.findall(r'^PHASE (\S+)', logtext, re.M)[-1] if 'PHASE ' in logtext else None
            rows.append(row)
            output_name = 'phase-bootstrap-comparison.json' if followup else 'phase-probe.json'
            (OUT / output_name).write_text(json.dumps(rows, indent=2), encoding='utf-8')
            print(row, flush=True)
