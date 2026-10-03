"""Generic compiler metadata reproduction using user-defined local providers."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
BASE = Path('D:/tmp/luna-audit-2026-10-02')
FIXTURES = REPO / 'tests/luna/language/audit_2026_10_02/provider_bounds'
HELPER = BASE / 'cargo-target/debug/audit-phase-probe.exe'
CLI = REPO / 'luna-rs/target/debug/luna.exe'
env = os.environ.copy()
env['PATH'] = 'D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
env['LUNA_RUNTIME_LIB'] = str(REPO / 'build/audit/host/x86_64-pc-windows-gnu/Release/libluna-runtime.a')
rows = []
def copy_sysroot(root, mode):
    for path in (REPO / 'luna-rs/libs/external').rglob('*'):
        suffixes = ('.ln', '.toml') if mode == 'source' else ('.llib', '.obj', '.o', '.toml')
        if path.is_file() and path.suffix in suffixes:
            target = root / 'libs/external' / path.relative_to(REPO / 'luna-rs/libs/external')
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)

def measure(root, mode, depths):
    env['LUNA_SYSROOT'] = str(root)
    for depth in depths:
        main = root / f'main_{depth}.ln'
        main.write_text(f'import "p{depth}";\nfn main() -> i32 {{ return probe{depth}::tag() - {depth}; }}\n', encoding='utf-8')
        row = dict(depth=depth, mode=mode, metadata=[])
        phase = subprocess.run([str(HELPER), str(main), str(root)], cwd=root,
            env=env, capture_output=True, text=True, timeout=30)
        log = OUT / f'provider-bounds-{mode}-{depth}.log'
        log.write_text(phase.stdout + phase.stderr, encoding='utf-8')
        row['phase_exit'] = phase.returncode
        row['phase_diagnostics'] = re.findall(r'DIAGNOSTICS before_mono=(\d+)', phase.stdout)
        for match in re.finditer(r'^METADATA provider=(p\d+) bounds=(\d+) unique=(\d+) local=(\d+) foreign=(\d+)', phase.stdout, re.M):
            row['metadata'].append(dict(provider=match[1], bounds=int(match[2]),
                unique=int(match[3]), local=int(match[4]), foreign=int(match[5])))
        binary = root / f'main_{depth}.exe'
        started = time.perf_counter()
        compiled = subprocess.run([str(CLI), 'build', str(main), '-q', '-o', str(binary)],
            cwd=root, env=env, capture_output=True, text=True, timeout=30)
        row.update(compile_exit=compiled.returncode, compile_seconds=time.perf_counter()-started)
        (OUT / f'provider-bounds-{mode}-{depth}-cli.log').write_text(compiled.stdout+compiled.stderr, encoding='utf-8')
        if compiled.returncode == 0:
            row['run_exit'] = subprocess.run([str(binary)], cwd=root, env=env, timeout=10).returncode
        rows.append(row)
        (OUT / 'provider-bounds.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
        print(row, flush=True)

with tempfile.TemporaryDirectory(prefix='luna_bounds_', dir=BASE) as directory:
    base = Path(directory)
    root = base / 'source'
    root.mkdir()
    copy_sysroot(root, 'source')
    for path in FIXTURES.glob('*.ln'):
        shutil.copy2(path, root / path.name)
    measure(root, 'source', (0, 4, 8, 12))
    build_root = base / 'build'
    build_root.mkdir()
    copy_sysroot(build_root, 'artifact')
    for path in FIXTURES.glob('p*.ln'):
        shutil.copy2(path, build_root / path.name)
    env['LUNA_SYSROOT'] = str(build_root)
    for depth in range(9):
        provider = build_root / f'p{depth}.ln'
        artifact = build_root / f'p{depth}.llib'
        compiled = subprocess.run([str(CLI), 'build', str(provider), '--lib', '--emit', 'llib',
            '-o', str(artifact)], cwd=build_root, env=env, capture_output=True, text=True, timeout=30)
        (OUT / f'provider-bounds-build-{depth}.log').write_text(compiled.stdout+compiled.stderr, encoding='utf-8')
        if compiled.returncode or not artifact.is_file():
            raise RuntimeError((depth, compiled.returncode, compiled.stdout, compiled.stderr))
    root = base / 'artifact'
    root.mkdir()
    copy_sysroot(root, 'artifact')
    for path in build_root.glob('p*'):
        if path.suffix in ('.llib', '.obj', '.o'):
            shutil.copy2(path, root / path.name)
    assert not list(root.glob('p*.ln'))
    measure(root, 'artifact', (0, 4, 8))
