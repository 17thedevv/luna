"""Build the diagnostic helper against this checkout, without production edits."""
import os
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[3]
BASE = Path('D:/tmp/luna-audit-2026-10-02')
PROJECT = BASE / 'phase-probe'
HERE = Path(__file__).parent
(PROJECT / 'src').mkdir(parents=True, exist_ok=True)
shutil.copy2(HERE / 'compiler_phase_probe.rs', PROJECT / 'src/main.rs')
dependencies = ('driver', 'ast', 'common', 'lexer', 'parser', 'semantic', 'mvir')
manifest = '[package]\nname = "audit-phase-probe"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\n'
for dependency in dependencies:
    path = (REPO / f'luna-rs/crates/luna-{dependency}').as_posix()
    manifest += f'luna-{dependency} = {{ path = "{path}" }}\n'
(PROJECT / 'Cargo.toml').write_text(manifest, encoding='utf-8')
if not (PROJECT / 'Cargo.lock').exists():
    shutil.copy2(REPO / 'luna-rs/Cargo.lock', PROJECT / 'Cargo.lock')
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(BASE / 'cargo-target'), CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_INCREMENTAL='0', LLVM_SYS_180_PREFIX='D:/Programs/LLVM-DEV')
env['PATH'] = 'C:/Users/84387/.cargo/bin;D:/Programs/LLVM-DEV/bin;C:/mingw64/bin;' + env['PATH']
with (HERE / 'evidence/phase-probe-build.log').open('w', encoding='utf-8') as log:
    subprocess.run(['cargo', 'build', '--offline', '--manifest-path', str(PROJECT / 'Cargo.toml')],
                   cwd=REPO, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
print(BASE / 'cargo-target/debug/audit-phase-probe.exe')
