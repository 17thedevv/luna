"""Record coverage and compact outcomes from the saved audit evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

REPO = Path(__file__).resolve().parents[3]
OUT = Path(__file__).parent / 'evidence'
inventory = []
for path in sorted((REPO / 'luna-rs/libs/external').rglob('*.ln')):
    data = path.read_bytes()
    inventory.append({'source': path.relative_to(REPO).as_posix(),
        'lines': len(data.splitlines()), 'sha256': hashlib.sha256(data).hexdigest()})
summary = {'providers': inventory, 'provider_count': len(inventory),
           'source_lines': sum(row['lines'] for row in inventory),
           'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip()}
summary['artifacts'] = [{'path': path.relative_to(REPO).as_posix(),
    'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
    for path in sorted((REPO / 'luna-rs/libs/external').rglob('*'))
    if path.is_file() and path.suffix in ('.llib', '.obj', '.o')]
for name in ('existing-suites', 'cli-regressions', 'probes', 'dispatch-repeat', 'storage-retry'):
    path = OUT / (name + '.json')
    rows = json.loads(path.read_text()) if path.exists() else []
    item = {'count': len(rows)}
    if name == 'existing-suites':
        item.update(passed=sum(row.get('exit') == 0 for row in rows),
                    timeout=sum(bool(row.get('timeout')) for row in rows),
                    failed=sum('exit' in row and row['exit'] != 0 for row in rows))
        item['individual_passed'] = 0
        for row in rows:
            log = OUT / (row['suite'] + '.log')
            if log.exists():
                match = re.search(r'test result: ok\. (\d+) passed;', log.read_text(encoding='utf-8'))
                if match:
                    item['individual_passed'] += int(match.group(1))
    elif name == 'cli-regressions':
        item.update(passed=sum(row['pass'] for row in rows),
                    positive=sum(not row['negative'] for row in rows),
                    negative=sum(row['negative'] for row in rows),
                    failed=[row for row in rows if not row['pass']])
    else:
        item['outcomes'] = [{key: value for key, value in row.items()
            if key in ('fixture', 'mode', 'attempt', 'test', 'compile_exit', 'run_exit',
                       'stdout', 'timeout', 'run_seconds', 'exit')} for row in rows]
    summary[name] = item
(OUT / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps({key: (value if not isinstance(value, dict) else
    {k: v for k, v in value.items() if k not in ('outcomes', 'failed')})
    for key, value in summary.items() if key not in ('providers', 'artifacts')}, indent=2))
for row in summary['cli-regressions']['failed']:
    print('CLI FAIL', row['mode'], row['fixture'], row.get('compile_exit'), row.get('run_exit'))
for row in summary['probes']['outcomes']:
    print('PROBE', row)
