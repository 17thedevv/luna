"""Generate the documentation inventory, provider table and static spec mirrors.

Only repository-authored documentation is included. Vendor packages, build
outputs and saved command logs are not maintained documentation. Historical
bodies are preserved; the inventory makes their authority explicit.
"""
from pathlib import Path
import html
import json
import os
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
SPEC = ROOT / 'docs/spec/0.1'
SITE = ROOT / 'docs/site'
ADOPTED = {
    'docs/normative-rules-p0-p1.md', 'docs/runtime/abi-v1.md',
    'docs/diagnostics/diagnostics-v1.md', 'docs/project/rename-luna-v1.md',
    'luna-rs/docs/spec/lifetime-formalism.md',
    'luna-rs/docs/spec-hardening-borrow-closure.md',
    'luna-rs/docs/spec-hardening-ffi-contracts.md',
    'luna-rs/docs/memory/raw-pointer-unsafe-semantics.md',
    'luna-rs/docs/raw_storage_anchor_v1.md',
    'luna-rs/docs/std_namespace_01_canonical_namespace.md',
    'luna-rs/docs/whole_file_io_v1.md',
    'luna-rs/docs/phase6_core_formatting_v1.md',
    '.agents/skills/luna-semantic-compliance/references/lle_v1_rfc.md',
    '.agents/skills/luna-semantic-compliance/references/visibility-02-public-field-default-rfc.md',
}
CURRENT = {
    'README.md', 'ROADMAP.md', 'Status.md', 'INCOMPLETE_FEATURES_PLAN.md',
    'CLAUDE.md', '.agents/AGENTS.md', 'docs/LanguageReference.md',
    'docs/grammar.md', 'docs/core-language-contract.md', 'docs/architecture.md',
    'docs/SemanticInvariants.md', 'docs/PROJECT_STATUS.md', 'docs/RoadMap.md',
    'docs/site/README.md',
    'docs/documentation-index.md',
}


def owner(name):
    lowered = name.lower()
    if 'capability-validation' in lowered:
        return 'conformance'
    if any(word in lowered for word in ('macro', 'attribute')):
        return 'macros'
    if any(word in lowered for word in ('runtime', 'allocator')) or re.search(r'[/_-]abi[./_-]', lowered):
        return 'runtime'
    if any(word in lowered for word in ('stdlib', 'formatting', 'whole_file', 'std_namespace')):
        return 'stdlib'
    if any(word in lowered for word in ('module', 'artifact', 'lang-contract', 'core-language')):
        return 'modules'
    if any(word in lowered for word in ('grammar', 'languagereference', 'visibility')):
        return 'syntax'
    if any(word in lowered for word in ('lifetime', 'borrow', 'pointer', 'anchor', 'semantic', 'memory', 'drop')):
        return 'semantics'
    return 'conformance'


def role(name):
    if name.startswith('docs/history/'):
        return 'historical'
    if name.startswith('docs/spec/0.1/'):
        return 'canonical'
    if name in ADOPTED:
        return 'adopted-contract'
    if name in CURRENT or name.startswith(('.agents/skills/', '.agents/rules/')):
        return 'guidance'
    if name.startswith('docs/audits/'):
        return 'evidence'
    if name.startswith('docs/integration/'):
        return 'evidence'
    return 'historical'


def maintained_paths():
    tracked = subprocess.check_output(['git', 'ls-files'], cwd=ROOT, text=True).splitlines()
    names = {name for name in tracked if name.lower().endswith('.md')}
    for folder in ('docs/spec/0.1', 'docs/history', 'docs/audits', 'docs/integration'):
        names.update(p.relative_to(ROOT).as_posix() for p in (ROOT / folder).rglob('*.md'))
    names.add('docs/documentation-index.md')
    return sorted(name for name in names if (ROOT / name).is_file()
                  and not any(piece in name.split('/') for piece in ('node_modules', 'target', '.diff-tmp')))


def notices(names):
    for name in names:
        if name.startswith(('docs/spec/0.1/', 'docs/history/', 'docs/audits/', 'docs/integration/')) or name == 'docs/documentation-index.md':
            continue
        path = ROOT / name
        text = path.read_text(encoding='utf-8-sig')
        if '<!-- luna-doc-role:' in text:
            continue
        classification = role(name)
        relative = Path(os.path.relpath(SPEC / 'README.md', path.parent)).as_posix()
        descriptions = {
            'adopted-contract': 'Retained detailed contract. Prior acceptance and freeze claims remain dated evidence; current release conformance is tracked separately.',
            'guidance': 'Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides.',
            'historical': 'Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness.',
        }
        notice = f'<!-- luna-doc-role: {classification} -->\n\n> **Luna 0.1 — {classification}.** {descriptions[classification]} See the [versioned specification]({relative}).\n\n'
        if text.startswith('---\n'):
            end = text.find('\n---', 4)
            if end != -1:
                split = end + len('\n---')
                text = text[:split] + '\n\n' + notice + text[split:].lstrip('\n')
            else:
                raise ValueError(f'Invalid front matter: {name}')
        else:
            text = notice + text
        path.write_text(text, encoding='utf-8')


def provider_inventory():
    manifest = ROOT / 'luna-rs/libs/external/sysroot.toml'
    providers = tomllib.loads(manifest.read_text(encoding='utf-8'))['provider']
    original = tomllib.loads((SPEC / 'evidence/sysroot-audit-baseline.toml').read_text(encoding='utf-8'))['provider']
    audited_names = {provider['name'] for provider in original}
    text = '# Luna 0.1 provider inventory\n\nGenerated from [sysroot.toml](../../../luna-rs/libs/external/sysroot.toml). Provider identity is not a namespace or a completeness claim.\n\n'
    text += f'Current manifest: **{len(providers)} providers**. The [original audit manifest](evidence/sysroot-audit-baseline.toml) has {len(original)}; later additions are implementation inventory, not automatic spec adoption or conformance. Public/internal visibility controls import access, separately from exported declarations.\n\n'
    text += '| Provider | Source component | Import visibility | Contract family | Evidence boundary |\n|---|---|---|---|---|\n'
    for provider in providers:
        source = provider['path'] + '.ln'
        boundary = 'original 32-provider audit' if provider['name'] in audited_names else 'later addition; unverified'
        text += f"| `{provider['name']}` | [{source}](../../../luna-rs/libs/external/{source}) | {provider['visibility']} | {provider.get('lang_contract', 'ordinary provider')} | {boundary} |\n"
    text += '\nComponent builds and API conformance are separate. See [stdlib.md](stdlib.md) and [gaps.md](gaps.md).\n'
    (SPEC / 'stdlib-inventory.md').write_text(text, encoding='utf-8')


def document_inventory(names):
    rows = [dict(path=name, role=role(name), owner=owner(name)) for name in names]
    text = '# Luna 0.1 documentation inventory\n\nUpdated 2026-10-03. [Authority](spec/0.1/README.md) · [Gaps](spec/0.1/gaps.md).\n\n'
    text += 'All repository-authored tracked Markdown and new versioned/archived/audit documents are classified below. Agent guidance is included. Vendor documentation, generated build logs and test-failure dumps are excluded. Website pages are generated from the same baseline.\n\n'
    text += 'Historical bodies are preserved, with an authority notice. Replaced entry pages have pre-0.1 snapshots. A historical FROZEN statement is not current release certification.\n\n'
    text += '| Document | Role | Owning 0.1 chapter |\n|---|---|---|\n'
    for row in rows:
        rel = Path(os.path.relpath(ROOT / row['path'], ROOT / 'docs')).as_posix()
        text += f"| [{row['path']}]({rel}) | {row['role']} | [{row['owner']}](spec/0.1/{row['owner']}.md) |\n"
    (ROOT / 'docs/documentation-index.md').write_text(text, encoding='utf-8')
    (ROOT / 'docs/documentation-index.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
    return rows


def inline(text, source, target):
    def link(match):
        label, url = match.group(1), match.group(2)
        if not re.match(r'\w+://|#', url):
            path, separator, anchor = url.partition('#')
            destination = (source.parent / path).resolve()
            if destination.parent == SPEC and destination.suffix == '.md':
                destination = SITE / 'spec' / ('index.html' if destination.stem == 'README' else destination.stem + '.html')
            url = Path(os.path.relpath(destination, target.parent)).as_posix() + (separator + anchor if separator else '')
        return f'<a href="{html.escape(url, quote=True)}">{html.escape(label)}</a>'
    parts = re.split(r'(\[[^\]]+\]\([^)]+\))', text)
    result = []
    for part in parts:
        match = re.fullmatch(r'\[([^\]]+)\]\(([^)]+)\)', part)
        if match:
            result.append(link(match))
        else:
            escaped = html.escape(part)
            escaped = re.sub(r'`([^`]+)`', r'<code>\1</code>', escaped)
            escaped = re.sub(r'\*\*([^*]+)\*\*', r'<strong>\1</strong>', escaped)
            result.append(escaped)
    return ''.join(result)


def render_markdown(source, target):
    lines = source.read_text(encoding='utf-8').splitlines()
    output, paragraph, code = [], [], None
    def flush():
        if paragraph:
            output.append('<p>' + inline(' '.join(paragraph), source, target) + '</p>')
            paragraph.clear()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith('```'):
            flush()
            if code is None:
                code = []
            else:
                output.append('<pre><code>' + html.escape('\n'.join(code)) + '</code></pre>')
                code = None
        elif code is not None:
            code.append(line)
        elif line.startswith('#'):
            flush()
            count = len(line) - len(line.lstrip('#'))
            title = line[count:].strip()
            anchor = re.sub(r'[^\w\- ]', '', title.lower()).replace(' ', '-')
            output.append(f'<h{count} id="{anchor}">{inline(title, source, target)}</h{count}>')
        elif line.startswith('|'):
            flush()
            table = []
            while i < len(lines) and lines[i].startswith('|'):
                table.append(lines[i]); i += 1
            output.append('<table>')
            for row_index, row in enumerate(table):
                if re.fullmatch(r'[| :\-]+', row):
                    continue
                tag = 'th' if row_index == 0 else 'td'
                cells = row.strip('|').split('|')
                output.append('<tr>' + ''.join(f'<{tag}>' + inline(cell.strip(), source, target) + f'</{tag}>' for cell in cells) + '</tr>')
            output.append('</table>'); continue
        elif re.match(r'^(- |\d+\. )', line):
            flush(); output.append('<p class="list-item">' + inline(line, source, target) + '</p>')
        elif not line.strip():
            flush()
        else:
            paragraph.append(line)
        i += 1
    flush()
    if code is not None:
        raise ValueError(f'Unclosed code fence in {source}')
    return '\n'.join(output)


def page(target, body, language='en', title='Luna 0.1'):
    target.parent.mkdir(parents=True, exist_ok=True)
    def relative(path):
        return Path(os.path.relpath(path, target.parent)).as_posix()
    links = [(SITE / 'en/index.html', 'EN'), (SITE / 'vi/index.html', 'VI'),
             (SITE / 'spec/index.html', 'Spec 0.1'), (SITE / 'spec/gaps.html', 'Gaps')]
    navigation = ' '.join(f'<a href="{relative(path)}">{label}</a>' for path, label in links)
    text = f'''<!DOCTYPE html>
<html lang="{language}"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title)}</title><link rel="stylesheet" href="{relative(SITE / 'assets/styles.css')}">
<style>main{{max-width:960px;margin:auto;padding:2rem}}table{{width:100%;border-collapse:collapse}}td,th{{padding:.6rem;border:1px solid #777;text-align:left}}pre{{overflow:auto;padding:1rem;background:#161b22;color:#e6edf3}}code{{font-family:monospace}}nav a{{margin-right:1rem}}.version-note{{padding:1rem;border:1px solid #777}}</style></head>
<body><header class="topbar"><div class="container nav-wrap"><strong>Luna 0.1</strong><nav>{navigation}</nav></div></header>
<main><p class="version-note">Contract baseline 0.1 · Release conformance: BLOCKED / NOT VERIFIED · Updated 2026-10-03</p>
{body}</main><footer class="footer"><div class="container">Generated from the versioned specification. Historical freeze evidence is not current release certification.</div></footer></body></html>
'''
    target.write_text(text, encoding='utf-8')


def site_pages():
    for source in SPEC.glob('*.md'):
        target = SITE / 'spec' / ('index.html' if source.stem == 'README' else source.stem + '.html')
        page(target, render_markdown(source, target), title=source.read_text(encoding='utf-8').splitlines()[0].lstrip('# '))
    originals = [p for p in SITE.rglob('*.html') if 'spec' not in p.relative_to(SITE).parts]
    for target in originals:
        name = target.relative_to(SITE).as_posix()
        vietnamese = name.startswith('vi/')
        title = 'Tài liệu Luna 0.1' if vietnamese else 'Luna 0.1 documentation'
        body = f'<h1>{title}</h1>'
        body += ('<p>Giữ các contract đã định nghĩa; implementation còn thiếu được ghi riêng trong danh sách khoảng hở. Spec và release conformance là hai trạng thái khác nhau.</p>' if vietnamese else '<p>Defined contracts are retained. Implementation failures and missing evidence are tracked separately; the baseline is not release certification.</p>')
        for chapter in ('syntax', 'semantics', 'macros', 'modules', 'stdlib', 'runtime', 'conformance', 'gaps'):
            rel = Path(os.path.relpath(SITE / f'spec/{chapter}.html', target.parent)).as_posix()
            body += f'<p><a href="{rel}">{chapter.title()}</a></p>'
        if 'language' in name or 'examples' in name:
            body += '<h2>Canonical example</h2><pre><code>struct Point { x: i32, y: i32, };\nfn main() -&gt; i32 {\n    dec p = Point { x: 10, y: 20 };\n    dec rw value = p.x + p.y;\n    value = value + 12;\n    return value - 42;\n}</code></pre>'
            body += '<p>Field default: public under Visibility-02; containing type accessibility is independent. Removed spellings and current grammar disagreements are documented in Syntax/Gaps.</p>'
        if 'compiler' in name or 'design' in name:
            body += '<p>No compiler mapping of library macro names, output prefixes, newline suffixes, container names or library-owned layouts is permitted. Generic language mechanisms and explicit adopted contracts define the boundary.</p>'
        page(target, body, language='vi' if vietnamese else 'en', title=title)
    return len(originals)


if __name__ == '__main__':
    provider_inventory()
    names = maintained_paths()
    notices(names)
    rows = document_inventory(names)
    # Include the generated index itself on the first run.
    if 'docs/documentation-index.md' not in names:
        rows = document_inventory(maintained_paths())
    sites = site_pages()
    counts = {classification: sum(row['role'] == classification for row in rows)
              for classification in sorted({row['role'] for row in rows})}
    print(json.dumps(dict(documents=len(rows), roles=counts, original_site_pages=sites), indent=2))
