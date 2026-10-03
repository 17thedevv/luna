"""Validate maintained documentation without asserting compiler conformance."""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import unquote
import ast
import json
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
errors = []
checked_links = 0


def check_link(source, url):
    global checked_links
    if re.match(r'^[a-zA-Z][\w+.-]*:', url) or url.startswith(('#', '//')):
        return
    path = unquote(url.partition('#')[0].partition('?')[0]).strip('<>')
    if not path:
        return
    destination = (source.parent / path).resolve()
    checked_links += 1
    if not destination.exists():
        errors.append(f'{source.relative_to(ROOT)}: missing link {url}')


class PageLinks(HTMLParser):
    def __init__(self, source):
        super().__init__()
        self.source = source
    def handle_starttag(self, tag, attributes):
        for name, value in attributes:
            if name in ('href', 'src') and value:
                check_link(self.source, value)


inventory = json.loads((ROOT / 'docs/documentation-index.json').read_text(encoding='utf-8'))
assert len({row['path'] for row in inventory}) == len(inventory)
for row in inventory:
    source = ROOT / row['path']
    if not source.is_file():
        errors.append(f'Missing inventory file: {row["path"]}')
        continue
    text = source.read_text(encoding='utf-8')
    if row['role'] not in ('canonical', 'evidence') and row['path'] != 'docs/documentation-index.md':
        if f'<!-- luna-doc-role: {row["role"]} -->' not in text:
            errors.append(f'Wrong or absent role notice: {row["path"]}')
    if row['role'] == 'historical':
        # Historical bodies may contain original relative links. Only the new
        # authority notice is maintained as a live navigation link.
        text = '\n'.join(text.splitlines()[:5])
    text = re.sub(r'```.*?```', '', text, flags=re.S)
    for target in re.findall(r'\[[^\]]+\]\(([^)\s]+)\)', text):
        check_link(source, target)
    if source.name == 'SKILL.md':
        if not text.startswith('---\n') or text.find('\n---', 4) == -1:
            errors.append(f'Invalid skill front matter: {row["path"]}')

pages = list((ROOT / 'docs/site').rglob('*.html'))
for page in pages:
    text = page.read_text(encoding='utf-8')
    PageLinks(page).feed(text)
    if 'BLOCKED / NOT VERIFIED' not in text:
        errors.append(f'Missing readiness boundary: {page.relative_to(ROOT)}')

for source in (ROOT / 'docs/tools').glob('*.py'):
    ast.parse(source.read_text(encoding='utf-8'), filename=str(source))
manifest = tomllib.loads((ROOT / 'luna-rs/libs/external/sysroot.toml').read_text(encoding='utf-8'))
table = (ROOT / 'docs/spec/0.1/stdlib-inventory.md').read_text(encoding='utf-8')
assert f"**{len(manifest['provider'])} providers**" in table
original = tomllib.loads((ROOT / 'docs/spec/0.1/evidence/sysroot-audit-baseline.toml').read_text(encoding='utf-8'))
assert len(original['provider']) == 32
for provider in manifest['provider']:
    assert f"`{provider['name']}`" in table
    assert (ROOT / 'luna-rs/libs/external' / (provider['path'] + '.ln')).is_file()
hooks = (ROOT / 'luna-rs/crates/luna-driver/src/lang_contracts.rs').read_text(encoding='utf-8')
assert len(re.findall(r'LanguageItemPath \{ lang_item:', hooks)) == 16
assert '**16** manifest hook identities' in (ROOT / 'docs/core-language-contract.md').read_text(encoding='utf-8')
results_path = ROOT / 'docs/spec/0.1/evidence/cli-examples.json'
if results_path.exists():
    results = json.loads(results_path.read_text(encoding='utf-8'))
    assert len(results) == 16
    for case in ('basic_reference', 'macro_name_independent', 'public_field_default', 'reject_private_field'):
        selected = [row for row in results if row['fixture'] == case]
        assert len(selected) == 2 and all(row['contract_met'] for row in selected)
    # Known gaps remain failures in the historical evidence; do not rewrite historical expectations.
    for case in ('reject_semicolon_fields', 'foreach_parenthesized_contract', 'receiver_shorthand_contract'):
        assert all(not row['contract_met'] for row in results if row['fixture'] == case)

repair_results_path = ROOT / 'docs/audits/repair-0.1/evidence/cli-examples.json'
if repair_results_path.exists():
    repair_results = json.loads(repair_results_path.read_text(encoding='utf-8'))
    assert isinstance(repair_results, list)

summary = dict(documents=len(inventory), site_pages=len(pages), checked_local_links=checked_links,
               errors=errors)
print(json.dumps(summary, indent=2, ensure_ascii=True))
sys.exit(1 if errors else 0)
