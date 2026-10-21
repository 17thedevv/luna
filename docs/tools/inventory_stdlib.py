"""Map current source declarations with Luna's parser before API/layout redesign.

This is a pre-expansion source inventory, not resolved API or release evidence.
The caller builds docs/tools/stdlib-surface first and supplies its executable.
"""
import argparse
from collections import Counter
import hashlib
import graphlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--parser', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit('Refusing to overwrite an acquired inventory')
    dirty = subprocess.check_output(['git', 'diff', 'HEAD', '--name-only', '--',
        'luna-rs/libs/external', 'luna-rs/crates/luna-parser', 'luna-rs/crates/luna-ast',
        'luna-rs/crates/luna-lexer'], cwd=ROOT, text=True).strip()
    assert not dirty, 'Source/parser must match the recorded commit: ' + dirty
    external = ROOT / 'luna-rs/libs/external'
    manifest = tomllib.loads((external / 'sysroot.toml').read_text(encoding='utf-8'))['provider']
    sources = [external / (entry['path'] + '.ln') for entry in manifest]
    assert set(sources) == set(external.rglob('*.ln')), 'Unmapped source file'
    bindings = {}
    for entry in manifest:
        for name in [entry['name'], *entry.get('aliases', [])]:
            assert name not in bindings, f'Duplicate provider binding: {name}'
            bindings[name] = entry['name']
    result = subprocess.run([str(args.parser.resolve()), *map(str, sources)], capture_output=True, check=True)
    parsed = json.loads(result.stdout)
    assert len(parsed) == len(manifest)
    providers = []
    for entry, source, ast in zip(manifest, sources, parsed, strict=True):
        assert Path(ast['file']) == source
        raw = source.read_bytes()
        declarations, types, expressions = ast['declarations'], ast['types'], ast['expressions']

        def spelling(span):
            return raw[span['start']:span['end']].decode('utf-8')

        def expression(index):
            value = expressions[index]
            if isinstance(value, dict) and 'Literal' in value:
                literal = value['Literal']
                if isinstance(literal, dict) and 'span' in literal:
                    return spelling(literal['span'])
            return '<expression>'

        def type_name(index):
            if index is None:
                return None
            value = types[index]
            if isinstance(value, str):
                return {'Never': '!'}.get(value, value)
            kind, data = next(iter(value.items()))
            if kind == 'Builtin':
                return {'String': 'str'}.get(data, data.lower())
            if kind == 'Named':
                name = '::'.join(map(spelling, data['segments']))
                if data['generic_args']:
                    name += '<' + ', '.join(map(type_name, data['generic_args'])) + '>'
                if data['associated_bindings']:
                    name += ' <associated bindings>'
                return name
            if kind in ['Reference', 'Pointer']:
                return ('&' if kind == 'Reference' else '*') + ('rw ' if data['is_mutable'] else '') + type_name(data['inner'])
            if kind == 'Slice':
                return '[' + type_name(data['inner']) + ']'
            if kind == 'Array':
                return '[' + type_name(data['element_type']) + '; ' + expression(data['size']) + ']'
            if kind == 'Tuple':
                return '(' + ', '.join(map(type_name, data['elements'])) + ')'
            if kind == 'Function':
                return ('unsafe ' if data['is_unsafe'] else '') + 'fn(' + ', '.join(map(type_name, data['params'])) + ') -> ' + (type_name(data['return_type']) or 'void')
            if kind == 'TraitObject':
                return 'dyn ' + type_name(data['trait_type'])
            if kind == 'Lifetime':
                return spelling(data)
            return '<' + kind + '>'

        records, imports = [], []
        visited = set()

        def visit(index, namespace=(), owner=None, external_function=False):
            assert index not in visited, 'A source declaration has multiple owners'
            visited.add(index)
            kind, data = next(iter(declarations[index].items()))
            if kind == 'Extern':
                visit(data['func'], namespace, owner, True)
                return
            if kind == 'Import':
                name = spelling(data['name']).strip('"')
                assert data['kind'] == 'External' and name in bindings, f'Unresolved source import: {name}'
                imports.append(dict(spelling=name, provider=bindings[name]))
                return
            span = data.get('name') or data.get('span')
            name = spelling(span) if span else '<impl>'
            if kind == 'Impl':
                name = type_name(data['self_type'])
                if data['trait_type'] is not None:
                    name = type_name(data['trait_type']) + ' for ' + name
            record = dict(id=entry['name'] + '#' + str(index), kind=kind, namespace='::'.join(namespace),
                owner=owner, name=name, line=raw[:span['start']].count(b'\n') + 1 if span else None,
                declared_visibility=data.get('visibility'), extern=external_function)
            if kind == 'Function':
                record['parameters'] = [dict(name=spelling(declarations[p]['Param']['name']),
                    type=type_name(declarations[p]['Param']['ty']),
                    default=declarations[p]['Param']['default'] is not None) for p in data['params']]
                record['return_type'] = type_name(data['return_type']) or 'void'
                record['unsafe'] = data['is_unsafe']
                record['lifetime_signature_ast'] = data['lifetime_signature']
            if kind == 'Struct':
                record['fields'] = [dict(name=spelling(field['name']), type=type_name(field['ty']),
                    visibility=field['visibility']) for field in data['fields']]
                record['lifetime_contract_ast'] = data['lifetime_contract']
                record['anchor_contract_ast'] = data['raw_storage_anchor_contract']
            if kind == 'Enum':
                record['variants'] = [spelling(variant['name']) for variant in data['variants']]
            if data.get('generic_params'):
                record['generic_parameters'] = [dict(name=spelling(param['name']), kind=param['kind'],
                    bounds=list(map(type_name, param['bounds']))) for param in data['generic_params']]
            if data.get('annotations'):
                record['annotations'] = [spelling(annotation['name']) for annotation in data['annotations']]
            records.append(record)
            if kind == 'Module':
                for child in data['items']:
                    visit(child, (*namespace, name))
            elif kind in ['Impl', 'Trait']:
                for child in [*data['associated_types'], *data['methods']]:
                    visit(child, namespace, record['id'])

        for item in ast['items']:
            assert 'Decl' in item, 'Unexpected top-level executable statement'
            visit(item['Decl'])
        unvisited_items = [index for index, declaration in enumerate(declarations)
            if index not in visited and next(iter(declaration)) not in ['Param', 'Var']]
        assert not unvisited_items, f'Unmapped nested library items: {source}: {unvisited_items}'
        relative = source.relative_to(ROOT).as_posix()
        providers.append(dict(id=entry['name'], source=relative, sha256=hashlib.sha256(raw).hexdigest(),
            lines=len(raw.splitlines()), physical_layer=entry['path'].split('/')[0],
            manifest_visibility=entry['visibility'], aliases=entry.get('aliases', []),
            lang_contract=entry.get('lang_contract'), declared_imports=imports,
            expected_artifacts=[entry['path'] + extension for extension in ['.llib', '.obj']],
            artifact_state='EXPECTED_PATH_ONLY_NOT_VALIDATED',
            declaration_kinds=dict(sorted(Counter(record['kind'] for record in records).items())),
            declarations=records, arena_declaration_count=len(declarations)))
    provider_names = {entry['id'] for entry in providers}
    assert all(imported['provider'] in provider_names for provider in providers for imported in provider['declared_imports'])
    explicit_order = list(graphlib.TopologicalSorter({provider['id']: sorted({item['provider']
        for item in provider['declared_imports']}) for provider in providers}).static_order())
    fixture_references = {name: [] for name in provider_names}
    fixture_count = 0
    for folder in ['tests/luna', 'luna-rs/tests/luna']:
        for fixture in sorted((ROOT / folder).rglob('*.ln')):
            fixture_count += 1
            imports = set(re.findall(r'\bimport\s*<([^>]+)>', fixture.read_text(encoding='utf-8')))
            for imported in imports:
                if imported in bindings:
                    fixture_references[bindings[imported]].append(fixture.relative_to(ROOT).as_posix())
    for provider in providers:
        provider['textual_fixture_import_references'] = sorted(set(fixture_references[provider['id']]))
    pin = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    inventory = dict(schema=1, state='SOURCE_INVENTORY_NOT_RESOLVED_API_OR_CONFORMANCE', source_commit=pin,
        stdlib_tree=subprocess.check_output(['git', 'rev-parse', 'HEAD:luna-rs/libs/external'], cwd=ROOT, text=True).strip(),
        parser_binary_sha256=hashlib.sha256(args.parser.read_bytes()).hexdigest(),
        manifest_sha256=hashlib.sha256((external / 'sysroot.toml').read_bytes()).hexdigest(),
        parser_source_tree=subprocess.check_output(['git', 'rev-parse', 'HEAD:luna-rs/crates/luna-parser'], cwd=ROOT, text=True).strip(),
        limitations=['Pre-expansion parser declarations; no derived/generated semantic items.',
            'Declared visibility is not effective export visibility.',
            'Dependency edges are explicit imports, excluding implicit bootstrap contracts.',
            'Artifact paths are expected output locations, not evidence of valid bundles.',
            'Signatures are descriptive; original source is authoritative for defaults, associated bindings and lifetime clauses.',
            'Parameters/locals are nested implementation data, not separately migrated API units.',
            'Fixture references are textual import matches, not semantic coverage or executed test results.'],
        provider_count=len(providers), declaration_count=sum(len(p['declarations']) for p in providers),
        explicit_import_edges=sum(len(p['declared_imports']) for p in providers),
        explicit_import_topological_order=explicit_order, fixture_files_indexed=fixture_count, providers=providers)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(inventory, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({key: inventory[key] for key in ['source_commit', 'provider_count', 'declaration_count', 'explicit_import_edges']}))


if __name__ == '__main__':
    main()
