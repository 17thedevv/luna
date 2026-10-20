"""Exercise the actual installer on a disposable Windows CI user profile only."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import threading
import time
import winreg

STATE = r'Software\Luna\Installer'
UNINSTALL = r'Software\Microsoft\Windows\CurrentVersion\Uninstall\Luna.Toolchain_is1'


def registry(key, name):
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key) as opened:
            return winreg.QueryValueEx(opened, name)
    except FileNotFoundError:
        return None


def write_path(value):
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER, 'Environment') as opened:
        if value is None:
            try:
                winreg.DeleteValue(opened, 'Path')
            except FileNotFoundError:
                pass
        else:
            winreg.SetValueEx(opened, 'Path', 0, value[1], value[0])


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--installer', type=Path, required=True)
    p.add_argument('--iscc', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    assert os.environ.get('GITHUB_ACTIONS') == 'true', 'Registry tests require a disposable hosted CI profile'
    assert registry(UNINSTALL, 'Inno Setup: App Path') is None, 'Profile already has Luna installed'
    assert registry(STATE, 'OwnPath') is None, 'Profile already has Luna PATH ownership'
    a.out = a.out.resolve()
    a.out.mkdir(parents=True, exist_ok=False)
    source = Path(__file__).resolve().parent
    lock = json.loads((source / 'installer-lock.json').read_text())
    install = a.out / 'Luna cài đặt space'
    work = a.out / 'consumer outside checkout'
    work.mkdir()
    evidence = a.out / 'evidence'
    evidence.mkdir()
    report = dict(status='FAIL', installer_sha256=sha(a.installer), workflow_run=os.environ.get('GITHUB_RUN_ID'),
                  lock_sha256=sha(source / 'installer-lock.json'), steps=[])
    before = registry('Environment', 'Path')
    system = Path(os.environ['SystemRoot'])
    # No host LLVM/GNU lookup, and intentionally conflicting development overrides.
    child_env = dict(os.environ, PATH=str(system / 'System32') + ';' + str(system),
                     LLVM_HOME='X:\\stale llvm', CC='X:\\stale cc.exe', LUNA_SYSROOT='X:\\stale sysroot',
                     LUNA_RUNTIME_LIB='X:\\stale runtime.lib', LUNA_HOME='X:\\stale home')

    def run(label, command, expect=0, env=None):
        invocation = command if isinstance(command, str) else list(map(str, command))
        result = subprocess.run(invocation, cwd=work, env=env, capture_output=True, timeout=900)
        (evidence / (label + '.stdout')).write_bytes(result.stdout)
        (evidence / (label + '.stderr')).write_bytes(result.stderr)
        report['steps'].append(dict(label=label, exit=result.returncode, expected=expect))
        print(f'{label}: exit {result.returncode}', flush=True)
        if expect is None:
            assert result.returncode != 0, label + ' unexpectedly succeeded'
        else:
            assert result.returncode == expect, label + ': ' + result.stderr.decode(errors='replace')[-3000:]
        return result

    def setup(label, exe, directory, tasks='addtopath', expect=0):
        return run(label, [exe, '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS',
                           '/DIR=' + str(directory), '/TASKS=' + tasks, '/LOG=' + str(evidence / (label + '.log'))], expect)

    def uninstall(label):
        log = evidence / (label + '.log')
        run(label, [install / 'unins000.exe', '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/LOG=' + str(log)])
        # Inno's original unins000.exe can exit while the relocated second
        # phase is still finishing its script and removing temporary files.
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if log.exists() and 'Uninstallation process succeeded.' in log.read_text(encoding='utf-8-sig'):
                return
            time.sleep(0.2)
        raise AssertionError(label + ': uninstaller second phase did not complete')

    try:
        # The real downloader must reject bytes whose digest differs from the lock
        # before creating the installation, uninstall key, or changing the PATH.
        fixture = a.out / 'tampered-download'
        fixture.mkdir()
        (fixture / lock['luna']['name']).write_bytes(b'intentionally not the pinned Luna archive')
        handler = lambda *args, **kwargs: http.server.SimpleHTTPRequestHandler(*args, directory=fixture, **kwargs)
        with http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            negative = a.out / 'negative-build'
            run('build-tamper-test', [shutil.which('python') or os.sys.executable, source / 'build_installer.py',
                                    '--iscc', a.iscc, '--out', negative,
                                    '--test-base-url', f'http://127.0.0.1:{server.server_port}'])
            rejected = a.out / 'rejected install'
            setup('reject-tampered-payload', negative / 'lunainstaller.exe', rejected, expect=None)
            server.shutdown()
        rejected_log = (evidence / 'reject-tampered-payload.log').read_text(encoding='utf-8-sig')
        assert 'File hash is incorrect' in rejected_log, 'Negative control must fail specifically on SHA-256 verification'
        assert not (rejected / 'bin/luna.exe').exists()
        assert registry('Environment', 'Path') == before
        assert registry(UNINSTALL, 'Inno Setup: App Path') is None

        setup('install', a.installer.resolve(), install)
        cli = install / 'bin/luna.exe'
        entry = str(install / 'bin')
        added = registry('Environment', 'Path')
        assert added and sum(x.lower() == entry.lower() for x in added[0].split(';')) == 1
        assert registry(STATE, 'OwnPath')[0] == 1
        package = install / 'packages' / lock['luna']['root']
        manifest = json.loads((package / 'manifest.json').read_text())
        for name, digest in manifest['files'].items():
            assert sha(package / name) == digest, name
        for name, digest in lock['llvm']['files'].items():
            assert sha(install / name) == digest, name
        assert len(list((package / 'libs/external').rglob('*.llib'))) == 49
        version = run('version-clean-path-stale-env', [cli, '--version'], env=child_env)
        assert version.stdout.strip() == b'luna 0.1.0'
        shortcut_script = work / 'read-shortcut.ps1'
        shortcut_script.write_text('param([string]$ShortcutPath)\n'
                                  '$s = (New-Object -ComObject WScript.Shell).CreateShortcut($ShortcutPath)\n'
                                  '$s | Select-Object TargetPath, Arguments | ConvertTo-Json\n')
        shortcut_path = Path(os.environ['APPDATA']) / 'Microsoft/Windows/Start Menu/Programs/Luna/Luna Command Prompt.lnk'
        shortcut = json.loads(run('read-start-menu-shortcut', [shutil.which('pwsh'), '-NoProfile', '-File',
                                                             shortcut_script, shortcut_path]).stdout)
        assert Path(shortcut['TargetPath']).name.lower() == 'cmd.exe'
        assert shortcut['Arguments'] == '/K ""' + str(install / 'bin/luna-shell.cmd') + '""'
        prompt = run('start-menu-command-prompt', '"' + shortcut['TargetPath'] + '" /D ' +
                     shortcut['Arguments'].replace('/K ', '/C ', 1), env=child_env)
        assert b'luna 0.1.0' in prompt.stdout
        run('llvm-version', [install / 'toolchains/llvm/bin/llc.exe', '--version'], env=child_env)
        for name in ('quickstart', 'stdlib'):
            fixture_source = work / (name + '.ln')
            shutil.copyfile(package / 'examples' / fixture_source.name, fixture_source)
            native = work / (name + '.exe')
            run(name + '-build', [cli, 'build', fixture_source, '--quiet', '--no-config', '-o', native], env=child_env)
            run(name + '-native', [native], env=child_env)
        # Argument quoting and the public run command must work with spaces too.
        run('quickstart-run', [cli, 'run', work / 'quickstart.ln', '--quiet', '--no-config'], env=child_env)
        missing = run('missing-source-exit', [cli, 'check', work / 'does not exist.ln', '--no-config'], expect=1, env=child_env)
        assert b'E' in missing.stderr + missing.stdout
        setup('reinstall', a.installer.resolve(), install)
        assert registry('Environment', 'Path') == added, 'Reinstall duplicated or changed PATH'
        # Preserve files the user created, and unrelated PATH updates after setup.
        note = install / 'my-project-notes.txt'
        note.write_text('user file must survive uninstall')
        sentinel = str(a.out / 'unrelated user tool')
        write_path((added[0] + ';' + sentinel, added[1]))
        uninstall('uninstall')
        current = registry('Environment', 'Path')
        report['post_uninstall_path'] = current
        report['post_uninstall_ownership'] = registry(STATE, 'OwnPath')
        assert current and sentinel in current[0].split(';') and entry not in current[0].split(';'), 'Uninstall PATH preservation/removal failed'
        assert note.read_text() == 'user file must survive uninstall'
        assert not cli.exists() and not (package / 'bin/luna.exe').exists()
        assert registry(STATE, 'OwnPath') is None and registry(UNINSTALL, 'Inno Setup: App Path') is None
        # A pre-existing entry is never owned or removed by Luna Setup.
        preexisting = ((before[0] + ';' if before and before[0] else '') + entry, winreg.REG_EXPAND_SZ)
        write_path(preexisting)
        setup('install-preexisting-path', a.installer.resolve(), install)
        assert registry('Environment', 'Path') == preexisting
        assert registry(STATE, 'OwnPath') is None
        uninstall('uninstall-preexisting-path')
        assert registry('Environment', 'Path') == preexisting
        report.update(status='PASS', providers=49, payload_hashes=len(manifest['files']),
                      unicode_space_path=True, host_tools_required=False, tampered_payload_rejected=True,
                      reinstall_path_unique=True, preexisting_path_preserved=True, user_file_preserved=True)
    except Exception as error:
        report['failure'] = str(error)
        raise
    finally:
        write_path(before)
        (evidence / 'installer-smoke-summary.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
