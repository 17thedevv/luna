"""Acquire a deterministic source inventory for Luna lang-item design.

This reads the explicit Rust table/manifest and literal canonical attributes.
It does not resolve exports, validate signatures, execute hooks or prove support.
Run from a clean compiler/stdlib checkout; outputs are immutable acquisitions.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def acquire():
    assert not git('diff', 'HEAD', '--name-only', '--', 'luna-rs/crates', 'luna-rs/libs/external'), 'Compiler/stdlib must match HEAD'
    sources = {}

    def read(name):
        raw = (ROOT / name).read_bytes()
        blob = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=ROOT)
        sources[name] = {'raw_sha256': digest(raw), 'git_blob_sha256': digest(blob)}
        return raw.decode('utf-8')

    registry_path = 'luna-rs/crates/luna-semantic/src/lang_item.rs'
    registry = read(registry_path)
    items = [dict(variant=variant, key=key, target=target,
                  test_only=key in {'test_trait', 'test_struct'})
             for variant, key, target in re.findall(r'^\s*\((\w+),\s*"([^"]+)",\s*(\w+)\);', registry, re.M)]
    assert items and len({i['key'] for i in items}) == len(items)
    keys = {i['key'] for i in items}
    providers = tomllib.loads(read('luna-rs/libs/external/sysroot.toml'))['provider']
    by_provider = {p['name']: p for p in providers}
    attributes = []
    for provider in providers:
        path = 'luna-rs/libs/external/' + provider['path'] + '.ln'
        text = (ROOT / path).read_text(encoding='utf-8')
        matches = list(re.finditer(r'^[ \t]*#\[lang\("([^"]+)"\)\]', text, re.M))
        if not matches:
            continue
        read(path)
        for match in matches:
            key = match[1]
            assert key in keys, (key, path)
            attributes.append(dict(key=key, provider=provider['name'], source=path,
                                   line=text[:match.start()].count('\n') + 1,
                                   expected_artifact=provider['path'] + '.llib'))
    assert len({a['key'] for a in attributes}) == len(attributes), 'Duplicate canonical source annotation'
    assert {a['key'] for a in attributes} == {i['key'] for i in items if not i['test_only']}

    manifest_path = 'luna-rs/crates/luna-driver/src/lang_contracts.rs'
    manifest = read(manifest_path)
    entries = []
    # Current manifest entries are literal Rust records; braces in their string
    # values are absent. Reject unexpected structure instead of guessing a schema.
    for start in re.finditer(r'LangContractEntry\s*\{', manifest):
        depth = 1
        end = start.end()
        while depth and end < len(manifest):
            depth += (manifest[end] == '{') - (manifest[end] == '}')
            end += 1
        assert depth == 0
        block = manifest[start.end():end - 1]
        identity = re.search(r'contract_id:\s*"([^"]+)"', block)
        if identity is None:
            continue  # Struct definition, not a value entry.
        provider = re.search(r'provider_id:\s*"([^"]+)"', block)[1]
        visibility = re.search(r'visibility_kind:\s*VisibilityKind::(\w+)', block)[1]
        paths = re.search(r'auto_visible_paths:\s*&\[(.*?)\]', block, re.S)[1]
        bindings = [dict(key=key, path=path) for key, path in re.findall(
            r'lang_item:\s*"([^"]+)",\s*canonical_path:\s*"([^"]+)"', block)]
        assert provider in by_provider
        assert all(b['key'] in keys for b in bindings)
        entries.append(dict(contract=identity[1], provider=provider, visibility_kind=visibility,
                            auto_visible_paths=re.findall(r'"([^"]+)"', paths), language_items=bindings))
    assert entries and len({e['contract'] for e in entries}) == len(entries)

    intrinsic_path = 'luna-rs/crates/luna-semantic/src/semantic_tables.rs'
    intrinsic = re.search(r'pub enum IntrinsicKind\s*\{([^}]+)\}', read(intrinsic_path))[1]
    intrinsic_kinds = re.findall(r'^\s*(\w+),', intrinsic, re.M)
    variants = {i['variant']: i for i in items}
    references = []
    for name in git('ls-files', 'luna-rs/crates').splitlines():
        if not name.endswith('.rs') or '/src/' not in name:
            continue
        text = (ROOT / name).read_text(encoding='utf-8')
        hits = [(number, line, re.findall(r'LangItem::(\w+)', line))
                for number, line in enumerate(text.splitlines(), 1) if 'LangItem::' in line]
        if not hits:
            continue
        read(name)
        for number, line, names in hits:
            for variant in names:
                if variant in variants:
                    references.append(dict(key=variants[variant]['key'], source=name,
                                           line=number, text=line.strip()))

    observations = []
    searches = [
        ('drop_short_name_fallback', 'luna-rs/crates/luna-mvir/src/generator.rs', 's.name == "Drop"'),
        ('copy_short_name_in_vm', 'luna-rs/crates/luna-mvir/src/interp.rs', 'trait_sym.name == "Copy"'),
        ('future_short_type_name', 'luna-rs/crates/luna-semantic/src/typechecker.rs', 'if let Some("Future")'),
        ('try_output_name', 'luna-rs/crates/luna-semantic/src/typechecker.rs', '"Output".to_string()'),
        ('try_residual_name', 'luna-rs/crates/luna-semantic/src/typechecker.rs', '"Residual".to_string()'),
        ('iterator_single_method_guard', 'luna-rs/crates/luna-semantic/src/typechecker.rs', 'required_methods.len() != 1'),
        ('bootstrap_raw_injection', 'luna-rs/crates/luna-driver/src/registry.rs', 'inject_raw_with_span(item, symbol_id, span)'),
        ('semantic_metadata_lang_map_stub', 'luna-rs/crates/luna-driver/src/metadata_decoder.rs', 'lang_items: HashMap::new()'),
    ]
    for kind, name, needle in searches:
        text = read(name)
        matches = [dict(source=name, line=n, text=line.strip())
                   for n, line in enumerate(text.splitlines(), 1) if needle in line]
        assert matches, (kind, name)
        observations.append(dict(kind=kind, status='STATIC_REVIEW_FINDING_NOT_REPRODUCED_DEFECT', sites=matches))

    return dict(schema=1, status='SOURCE_INVENTORY_NOT_CONFORMANCE_OR_ADOPTION',
                source_commit=git('rev-parse', 'HEAD'),
                compiler_tree=git('rev-parse', 'HEAD:luna-rs/crates'),
                stdlib_tree=git('rev-parse', 'HEAD:luna-rs/libs/external'),
                counts=dict(registry=len(items), production=sum(not i['test_only'] for i in items),
                            test_only=sum(i['test_only'] for i in items),
                            source_attributes=len(attributes), bootstrap_entries=len(entries),
                            compiler_contract_entries=sum(e['visibility_kind'] == 'CompilerContract' for e in entries),
                            bootstrap_bindings=sum(len(e['language_items']) for e in entries),
                            intrinsic_enum_variants=len(intrinsic_kinds)),
                registry=items, source_attributes=attributes, bootstrap_manifest=entries,
                intrinsic_enum_variants=intrinsic_kinds, static_lang_item_references=references,
                observations=observations, source_digests=dict(sorted(sources.items())),
                limitations=['Literal source inspection, not a Rust/Luna semantic export query.',
                             'Static references can include tests inside source modules; they do not prove executed support.',
                             'Intrinsic enum variants alone do not establish an implemented or public primitive.',
                             'Canonical source annotations are counted; transitive or derived declarations are not inferred.',
                             'Review findings require isolated reproducers before defect/qualification claims.'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit('Refusing to overwrite an acquired inventory')
    result = acquire()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result['counts']))
