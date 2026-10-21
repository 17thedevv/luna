"""Exercise Lunar through the public CLI with a real, installed Luna compiler."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--lunar', type=Path, required=True)
    parser.add_argument('--luna', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    lunar, luna = args.lunar.resolve(), args.luna.resolve()
    root = args.out.resolve()
    root.mkdir(parents=True, exist_ok=False)
    evidence = root / 'evidence'
    evidence.mkdir()
    work = root / 'dự án space'
    work.mkdir()
    records = []

    def run(label, command, cwd=work, expected=0):
        start = time.monotonic()
        result = subprocess.run([str(x) for x in command], cwd=cwd, capture_output=True, timeout=180)
        content = result.stdout + b'\nSTDERR\n' + result.stderr
        log = evidence / f'{len(records) + 1:02d}-{label}.log'
        log.write_bytes(content)
        records.append(dict(label=label, command=[str(x) for x in command], exit=result.returncode,
                            expected=expected, seconds=round(time.monotonic() - start, 3),
                            log=log.name, sha256=hashlib.sha256(content).hexdigest()))
        (evidence / 'summary.json').write_text(json.dumps(dict(status='RUNNING', records=records), indent=2), encoding='utf-8')
        assert result.returncode == expected, f'{label}: exit {result.returncode}, expected {expected}\n{content.decode("utf-8", errors="replace")}'
        print(f'{label}: PASS', flush=True)
        return result

    def project(name, library=False):
        run(f'new-{name}', [lunar, 'new', name] + (['--lib'] if library else []))
        return work / name

    def invoke(label, project, command, *extra, expected=0, cwd=work):
        return run(label, [lunar, '--compiler', luna, '--manifest-path', project / 'luna.toml', command, *extra], cwd, expected)

    def append(project, text):
        with (project / 'luna.toml').open('a', encoding='utf-8') as file:
            file.write(text)

    app = project('hello')
    invoke('fresh-check', app, 'check')
    invoke('fresh-build', app, 'build')
    invoke('fresh-run-native-args', app, 'run', '--', 'space argument', 'tiếng Việt')
    run('nearest-manifest-discovery', [lunar, '--compiler', luna, 'check'], cwd=app / 'src')
    # Both a library and multiple binaries can coexist in one project.
    combined = project('combined', True)
    (combined / 'src/main.ln').write_text('import <combined>;\nfn main() -> i32 { return combined::answer() - 42; }\n', encoding='utf-8')
    append(combined, "\n[[bin]]\nname='first'\npath='src/main.ln'\n[[bin]]\nname='second'\npath='src/main.ln'\n")
    invoke('library-and-selected-bin', combined, 'run', '--bin', 'second')
    invoke('all-bin-targets', combined, 'build')
    # A plain provider binding can consume source copied into a vendor folder.
    vendor = app / 'vendor'
    vendor.mkdir()
    (vendor / 'plain.ln').write_text('export module plain { export fn answer() -> i32 { return 42; } }\n', encoding='utf-8')
    append(app, "\n[providers]\nextra = 'vendor/plain'\n")
    (app / 'src/main.ln').write_text('import <extra>;\nfn main() -> i32 { return plain::answer() - 42; }\n', encoding='utf-8')
    invoke('vendored-source-provider', app, 'run')
    # A leaf imported twice through a diamond must keep one genuine provider identity.
    leaf = project('leaf', True)
    middle = project('middle', True)
    append(middle, "\n[dependencies]\nbase = { path = '../leaf' }\n")
    (middle / 'src/middle.ln').write_text('import <base>;\nexport module middle { export fn answer() -> i32 { return leaf::answer(); } }\n', encoding='utf-8')
    append(app, "\n[dependencies]\nhelper = { path = '../middle' }\nbase = { path = '../leaf' }\n")
    (app / 'src/main.ln').write_text('import <helper>;\nimport <base>;\nfn main() -> i32 { return middle::answer() + leaf::answer() - 84; }\n', encoding='utf-8')
    invoke('local-diamond-check', app, 'check')
    invoke('local-diamond-artifact-run', app, 'run', cwd=root)
    # Published bundles and their generated config remain valid after staging
    # is renamed to build/. Remove library sources to force artifact-only loads.
    for project_path, stem in [(leaf, 'leaf'), (middle, 'middle')]:
        (project_path / f'src/{stem}.ln').rename(project_path / f'src/{stem}.hidden')
    run('published-artifact-only-run', [luna, 'run', app / 'src/main.ln', '--config', app / 'target/lunar/build/luna.toml', '--quiet'], cwd=root)
    for project_path, stem in [(leaf, 'leaf'), (middle, 'middle')]:
        (project_path / f'src/{stem}.hidden').rename(project_path / f'src/{stem}.ln')
    # Confirm repeated builds consume changed dependency source, rather than stale artifacts.
    (leaf / 'src/leaf.ln').write_text('export module leaf { export fn answer() -> i32 { return 43; } }\n', encoding='utf-8')
    invoke('changed-dependency-rebuild', app, 'run', expected=2)
    tests = app / 'tests'
    tests.mkdir()
    (tests / 'smoke.ln').write_text('import <helper>;\nfn main() -> i32 { return middle::answer() - 43; }\n', encoding='utf-8')
    append(app, "\n[[test]]\nname = 'smoke'\npath = 'tests/smoke.ln'\n")
    invoke('test-pass', app, 'test')
    (tests / 'smoke.ln').write_text('fn main() -> i32 { return 7; }\n', encoding='utf-8')
    invoke('test-failure-exit', app, 'test', expected=1)
    # Invalid source must not execute the previous successful binary.
    (app / 'src/main.ln').write_text('fn main() -> i32 { return missing_name; }\n', encoding='utf-8')
    invoke('compile-error-check', app, 'check', expected=1)
    invoke('compile-error-run-no-stale-execution', app, 'run', expected=1)
    (app / 'keep.txt').write_text('user file', encoding='utf-8')
    invoke('clean-owned-output', app, 'clean')
    assert not (app / 'target/lunar').exists()
    assert (app / 'keep.txt').read_text(encoding='utf-8') == 'user file'
    assert (leaf / 'src/leaf.ln').exists()
    summary = dict(status='PASS', lunar_sha256=hashlib.sha256(lunar.read_bytes()).hexdigest(),
                   compiler=str(luna), compiler_sha256=hashlib.sha256(luna.read_bytes()).hexdigest(), records=records)
    (evidence / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(f'{len(records)} public CLI steps PASS: {evidence}')


if __name__ == '__main__':
    main()
