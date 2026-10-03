"""Repeated CLI HashMap insertion measurements; includes process startup."""
import json
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
EXTERNAL = REPO / 'luna-rs/libs/external'
CLI = REPO / 'luna-rs/target/debug/luna.exe'
BASE = Path('D:/tmp/luna-audit-2026-10-02')
SAMPLES = 7
ROUNDS = 8
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
rows = []
rng = random.Random(20261002)
with tempfile.TemporaryDirectory(prefix='luna_hash_bench_', dir=BASE) as directory:
    directory = Path(directory)
    for mode in ('source', 'artifact'):
        root = directory / mode
        for path in EXTERNAL.rglob('*'):
            if path.is_file() and (path.suffix == '.toml' or
                (mode == 'source' and path.suffix == '.ln') or
                (mode == 'artifact' and path.suffix in ('.llib', '.obj', '.o'))):
                target = root / 'libs/external' / path.relative_to(EXTERNAL)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, target)
        local_env = dict(env, LUNA_SYSROOT=str(root))
        cases = [(n, pattern, stride) for n in (1500, 3000, 6000, 12000)
                 for pattern, stride in (('sequential', 1), ('low_bits_collision', 65536))]
        cases += [(0, 'startup_control', 1)]
        compiled = []
        for n, pattern, stride in cases:
            name = f'{pattern}_{n}'
            source = f'''import <hashmap>;
fn main() -> i32 {{
    dec rw round: i32 = 0;
    while round < {ROUNDS} {{
        dec rw map = std::hashmap_new<i32, i32>();
        dec rw i: i32 = 0;
        while i < {n} {{ map.insert(i * {stride}, i); i = i + 1; }}
        if map.len() != ({n} as u64) {{ return 1; }}
        round = round + 1;
    }}
    return 0;
}}'''
            if n == 0:
                source = 'fn main() -> i32 { return 0; }'
            fixture = root / (name + '.ln')
            binary = root / (name + '.exe')
            fixture.write_text(source, encoding='utf-8')
            p = subprocess.run([str(CLI), 'build', str(fixture), '-q', '-o', str(binary)],
                cwd=root, env=local_env, capture_output=True, text=True, timeout=60)
            if p.returncode:
                raise RuntimeError((name, p.returncode, p.stderr))
            row = dict(mode=mode, pattern=pattern, n=n, rounds=ROUNDS if n else 0,
                       source=source, samples_seconds=[])
            compiled.append((binary, row))
        # One warm-up per executable; interleave subsequent cases to reduce order bias.
        for binary, row in compiled:
            subprocess.run([str(binary)], cwd=root, env=local_env, check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=40)
        for sample in range(SAMPLES):
            order = list(compiled)
            rng.shuffle(order)
            for binary, row in order:
                started = time.perf_counter()
                p = subprocess.run([str(binary)], cwd=root, env=local_env,
                    stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=40)
                elapsed = time.perf_counter() - started
                if p.returncode:
                    raise RuntimeError((binary.name, p.returncode, p.stderr))
                row['samples_seconds'].append(elapsed)
            print(mode, 'sample', sample + 1, '/', SAMPLES, flush=True)
        for binary, row in compiled:
            samples = row['samples_seconds']
            row.update(median_seconds=statistics.median(samples),
                       minimum_seconds=min(samples), maximum_seconds=max(samples))
            rows.append(row)
            print(mode, row['pattern'], row['n'], 'median=', row['median_seconds'], flush=True)
        (OUT / 'hashmap-benchmark.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
