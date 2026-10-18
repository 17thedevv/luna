"""Verify byte-identical acquired evidence and every preserved original manifest row.

Read-only; does not invoke CLI/Cargo or depend on original acquisition directories.
"""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parent
def load(p): return json.loads(p.read_text(encoding='utf-8'))
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
manifest=load(ROOT/'packaging-manifest.json')
by_relative={x['relative_path']:x for x in manifest['files']}
by_original={x['original_path'].replace('\\','/'):x for x in manifest['files']}
copied=excluded=0
for item in manifest['files']:
 if item['disposition']=='copied-byte-identical':
  p=ROOT/item['relative_path'];assert p.is_file(),str(p)
  assert p.stat().st_size==item['bytes'] and sha(p)==item['sha256'],str(p)
  copied+=1
 else:
  assert not (ROOT/item['relative_path']).exists(),item['relative_path']
  excluded+=1
replay_root='replay/runs/exact-pin-20261014/'
replay=load(ROOT/replay_root/'evidence-sha256.json')
replay_copied=replay_excluded=0
for entry in replay:
 item=by_relative[replay_root+entry['path'].replace('\\','/')]
 assert item['sha256']==entry['sha256']
 assert item['bytes']==entry['bytes']
 if item['disposition']=='copied-byte-identical': replay_copied+=1
 else: replay_excluded+=1
audit=load(ROOT/'evidence-sha256.json')
for entry in audit:
 item=by_original[entry['path'].replace('\\','/')]
 assert item['sha256']==entry['sha256'] and item['bytes']==entry['bytes']
assert len(replay)==329 and len(audit)==16
assert (ROOT/'.gitattributes').read_bytes()==b'* -text\n'
print(json.dumps({'status':'PASS','copied_acquired_files_verified':copied,'excluded_files_documented':excluded,
 'replay_manifest_entries_verified':len(replay),'replay_files_byte_verified':replay_copied,
 'replay_binary_hash_only_entries':replay_excluded,'audit_manifest_entries_verified':len(audit),
 'original_manifests_rewritten':False},indent=2))
