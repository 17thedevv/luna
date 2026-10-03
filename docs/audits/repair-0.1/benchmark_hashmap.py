"""Repeated CLI HashMap insertion measurements; before/after optimization verification."""
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
CLI = REPO / 'luna-rs/target/debug/luna.exe'
SAMPLES = 7
ROUNDS = 8
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env.get('PATH', '')
rows = []
rng = random.Random(20261003)

with tempfile.TemporaryDirectory(prefix='luna_hash_bench_') as directory:
    root = Path(directory)
    cases = [(n, pattern, stride) for n in (1500, 3000, 6000, 12000)
             for pattern, stride in (('sequential', 1), ('low_bits_collision', 65536))]
    cases += [(0, 'startup_control', 1)]
    compiled = []
    print("Compiling benchmarks...")
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
            cwd=root, env=env, capture_output=True, text=True, timeout=60)
        if p.returncode:
            raise RuntimeError((name, p.returncode, p.stderr))
        row = dict(pattern=pattern, n=n, rounds=ROUNDS if n else 0,
                   samples_seconds=[])
        compiled.append((binary, row))

    print("Warm-up pass...")
    for binary, row in compiled:
        subprocess.run([str(binary)], cwd=root, env=env, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=40)

    print("Running timed samples...")
    for sample in range(SAMPLES):
        order = list(compiled)
        rng.shuffle(order)
        for binary, row in order:
            started = time.perf_counter()
            p = subprocess.run([str(binary)], cwd=root, env=env,
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=40)
            elapsed = time.perf_counter() - started
            if p.returncode:
                raise RuntimeError((binary.name, p.returncode, p.stderr))
            row['samples_seconds'].append(elapsed)
        print(f"Sample {sample + 1}/{SAMPLES} complete", flush=True)

    print("\n--- Benchmark Results ---")
    for binary, row in compiled:
        samples = row['samples_seconds']
        med = statistics.median(samples)
        mn = min(samples)
        mx = max(samples)
        row.update(median_seconds=med, minimum_seconds=mn, maximum_seconds=mx)
        rows.append(row)
        print(f"{row['pattern']:<20} n={row['n']:<6} median={med:.5f}s (min={mn:.5f}s, max={mx:.5f}s)")

    out_file = OUT / 'hashmap-benchmark.json'
    out_file.write_text(json.dumps(rows, indent=2), encoding='utf-8')
    print(f"\nWrote benchmark results to {out_file}")
