#!/usr/bin/env python3
"""Generate/check shipped Rust notices from locked native desktop dependency graphs.

Build-time only, Python >=3.11 and Cargo. No network requests, compilation, or
runtime dependency. Registry archive checksums must match Cargo.lock.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tomllib

if not __debug__:
    raise SystemExit('Run without Python optimization; completeness assertions must stay enabled.')

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / 'LICENSES/rust'
TARGETS = ('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc',
           'aarch64-apple-darwin', 'x86_64-apple-darwin')
LICENSE_NAME = re.compile(r'(?i)^(?:\.?licen[cs]e|copying|notices?|copyrights?|unlicen[cs]e|authors|patents|third[-_]party[-_](?:licen[cs]e|notices?))(?:$|[._-])')
CODE_SUFFIXES = {'.rs', '.c', '.cc', '.cpp', '.h', '.hh', '.hpp', '.inc', '.S', '.s', '.asm', '.py'}
ATTRIBUTION = re.compile(r'^\s*(?:(?://|/\*+|\*+|\#|;)\s*)?(?:SPDX-FileCopyrightText:|SPDX-FileContributor:|Copyright(?:\s|\(|:)|copyright(?:\s|\(|:)).*$', re.MULTILINE)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + '\n').encode()


def closure(metadata):
    nodes = {node['id']: node for node in metadata['resolve']['nodes']}
    todo = [package['id'] for package in metadata['packages'] if package['name'] == 'videodock-desktop']
    assert len(todo) == 1, 'desktop root missing'
    visited = set()
    while todo:
        identity = todo.pop()
        if identity in visited:
            continue
        visited.add(identity)
        for edge in nodes[identity]['deps']:
            if any(kind['kind'] != 'dev' for kind in edge['dep_kinds']):
                todo.append(edge['pkg'])
    return visited


def is_license(path):
    return bool(LICENSE_NAME.match(path.name) or re.search(r'(?i)\.licen[cs]e$', path.name)) or any(part.lower() in ('licenses', 'licences') for part in path.parts[:-1])


def source_notice(data):
    text = data.decode('utf-8', errors='replace')
    notices = {match.group(0).strip() for match in ATTRIBUTION.finditer(text)}
    # Retain complete leading grants (not just names), including SQLite's public-domain blessing.
    beginning = text.lstrip('\ufeff \t\r\n')
    if beginning.startswith('/*'):
        stop = beginning.find('*/', 2)
        header = beginning[:stop + 2] if 0 <= stop < 65536 else ''
    else:
        lines = []
        for line in beginning.splitlines():
            if not line.strip() or line.lstrip().startswith(('//', '# ')):
                lines.append(line)
            else:
                break
        header = '\n'.join(lines).strip()
    if re.search(r'(?i)copyright|licen[cs]e|public\s+domain|SPDX', header):
        notices = {notice for notice in notices if notice not in header}
        notices.add(header)
    return notices


def package_sources(package, checksums):
    directory = Path(package['manifest_path']).parent
    source = package['source']
    if source:
        assert source.startswith('registry+'), 'unsupported non-registry dependency; add an explicit audited source'
        archive = directory.parents[2] / 'cache' / directory.parent.name / (package['name'] + '-' + package['version'] + '.crate')
        data = archive.read_bytes()
        expected = checksums[(package['name'], package['version'], source)]
        assert digest(data) == expected, 'registry archive checksum mismatch: ' + package['name']
        with tarfile.open(fileobj=io.BytesIO(data), mode='r|gz') as bundle:
            prefix = package['name'] + '-' + package['version']
            for member in bundle:
                parts = PurePosixPath(member.name).parts
                assert parts and parts[0] == prefix and '..' not in parts, 'unsafe archive member'
                if not member.isfile():
                    continue
                relative = PurePosixPath(*parts[1:])
                if (is_license(relative) or relative.name == '.cargo_vcs_info.json'
                        or relative.suffix in CODE_SUFFIXES and member.size <= 16 * 1024 * 1024):
                    yield relative.as_posix(), bundle.extractfile(member).read()
    else:
        directory.relative_to(ROOT)  # Never collect arbitrary local files outside this source tree.
        for file in sorted(directory.rglob('*')):
            if file.is_file() and not file.is_symlink():
                relative = PurePosixPath(file.relative_to(directory).as_posix())
                if is_license(relative) or relative.suffix in CODE_SUFFIXES:
                    yield relative.as_posix(), file.read_bytes()


def generate():
    lock_data = (ROOT / 'Cargo.lock').read_bytes()
    lock = tomllib.loads(lock_data.decode())
    checksums = {(item['name'], item['version'], item.get('source')): item.get('checksum') for item in lock['package']}
    packages, target_membership, workspace = {}, {}, set()
    for target in TARGETS:
        result = subprocess.run(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', target],
                                cwd=ROOT, capture_output=True)
        assert result.returncode == 0, 'Cargo metadata unavailable for ' + target + '; restore locked offline dependencies first'
        metadata = json.loads(result.stdout)
        workspace.update(metadata['workspace_members'])
        selected = closure(metadata)
        for package in metadata['packages']:
            if package['id'] in selected:
                packages[package['id']] = package
                target_membership.setdefault(package['id'], []).append(target)
    supplements = json.loads((OUTPUT / 'supplements.json').read_bytes())['packages']
    texts, entries, original_hashes, supplemental_references = {}, [], {}, []

    def retain(data):
        sha = digest(data)
        texts['texts/' + sha + '.txt'] = data
        return sha, 'texts/' + sha + '.txt'

    for identity, package in sorted(packages.items(), key=lambda row: (row[1]['name'], row[1]['version'])):
        if identity in workspace:
            continue
        key = package['name'] + '@' + package['version']
        assert package['license'] or package['license_file'], 'missing license declaration: ' + key
        entry = {'name': package['name'], 'version': package['version'], 'license_expression': package['license'],
                 'authors': package['authors'], 'repository': package['repository'],
                 'targets': sorted(target_membership[identity]), 'files': []}
        if package['source']:
            entry['source'] = package['source']
            entry['crate_sha256'] = checksums[(package['name'], package['version'], package['source'])]
            if package['source'] == 'registry+https://github.com/rust-lang/crates.io-index':
                entry['source_archive_url'] = 'https://static.crates.io/crates/' + package['name'] + '/' + package['name'] + '-' + package['version'] + '.crate'
        else:
            entry['source'] = Path(package['manifest_path']).parent.relative_to(ROOT).as_posix()
            entry['source_note'] = 'Modified vendored source: distribute this directory from the corresponding VideoDock release.'
        notices = {}
        for relative, data in package_sources(package, checksums):
            path = PurePosixPath(relative)
            if path.name == '.cargo_vcs_info.json':
                entry['upstream_vcs'] = json.loads(data)
            if is_license(path):
                sha, destination = retain(data)
                original_hashes[(key, relative)] = sha
                entry['files'].append({'role': 'package-license-or-notice', 'source_path': relative, 'path': destination, 'sha256': sha})
            elif path.suffix in CODE_SUFFIXES:
                for notice in source_notice(data):
                    notices.setdefault(notice, []).append(relative)
        if notices:
            data = ('Source copyright/license comments reproduced from ' + key + '.\n'
                    'This is a conservative inventory of published package sources, including build/test/native code.\n\n'
                    + '\n\n'.join('Files: ' + ', '.join(sorted(paths)) + '\n' + notice
                                   for notice, paths in sorted(notices.items())) + '\n').encode()
            sha, destination = retain(data)
            entry['files'].append({'role': 'source-copyright-and-grant-notices', 'path': destination, 'sha256': sha})
        supplement = supplements.get(key, {})
        for record in supplement.get('files', []):
            data = (OUTPUT / record['path']).read_bytes()
            assert digest(data) == record['sha256'], 'supplement checksum mismatch: ' + key
            upstream = entry.get('upstream_vcs', {}).get('git', {}).get('sha1')
            if record.get('upstream_commit') and upstream:
                assert record['upstream_commit'] == upstream, 'supplement source revision mismatch: ' + key
            if record.get('source_package'):
                assert record['standard_license'] in package['license'], 'standard text does not match the declared license: ' + key
                supplemental_references.append(record)
            if record['role'] == 'upstream-license-declaration' and record['source_path'] == 'Cargo.toml':
                declaration = tomllib.loads(data.decode())['package']
                assert declaration['name'] == package['name'] and declaration['version'] == package['version'], 'release declaration mismatch'
                assert declaration['license'] == package['license'], 'upstream license declaration mismatch'
            retain(data)
            entry['files'].append(record)
        for field in ('selected_license_option', 'note'):
            if field in supplement:
                entry[field] = supplement[field]
        # A copyright header alone is not a complete license text.
        assert any(item['role'] == 'license' or item['role'] == 'package-license-or-notice'
                   and re.match(r'(?i)^(?:licen[cs]e|copying|unlicen[cs]e)(?:$|[._-])', PurePosixPath(item.get('source_path', '')).name)
                   for item in entry['files']), 'license text unavailable: ' + key
        entry['files'].sort(key=lambda value: (value['role'], value.get('source_path', ''), value['path']))
        entries.append(entry)
    for reference in supplemental_references:
        assert original_hashes.get((reference['source_package'], reference['source_path'])) == reference['sha256'], 'standard text differs from its checksum-verified source'
    assert lock_data == (ROOT / 'Cargo.lock').read_bytes(), 'Cargo.lock changed during generation'
    manifest = {'schema_version': 1, 'cargo_lock_sha256': digest(lock_data), 'root_package': 'videodock-desktop',
                'targets': list(TARGETS), 'scope': 'Default native desktop dependency graphs, normal and build edges; dev-only edges excluded.',
                'workspace_packages_excluded': sorted(package['name'] for identity, package in packages.items() if identity in workspace),
                'packages': entries}
    lines = ['# Rust dependency license notices', '',
             f'This bundle covers **{len(entries)} third-party packages** from the locked default desktop dependency graphs for Linux x64, Windows MSVC x64, and macOS arm64/x64, including build dependencies. Dev-only edges and unused platform packages are excluded.', '',
             'License expressions and authors are reproduced from Cargo metadata without changing legacy slash notation. Original license/NOTICE/AUTHORS files, packaged native-library notices, and source copyright/grant comments are preserved. Source comments conservatively include package build/test code as well as library code.', '',
             'Where an archive omits a license file, `supplements.json` records immutable upstream sources or the exact standard text from another checksum-verified locked package. The two historical dual-licensed packages lacking standalone upstream texts use their declared Apache-2.0 option; no copyright years have been invented.', '',
             'This inventory does not replace the corresponding source distribution or the separate VideoDock, Qt, FFmpeg, SABR and BgUtils notices. RustyPipe is modified GPL-3.0 source under `vendor/rustypipe` in the matching VideoDock source release.', '',
             'Regenerate: `python3 LICENSES/generate-rust-licenses.py --write`. Verify: `python3 LICENSES/generate-rust-licenses.py --check`. Both require Cargo, Python3.11+, and the locked local registry archives; neither downloads packages or compiles code.', '',
             '| Package | Declared license | Preserved texts |', '|---|---|---|']
    for entry in entries:
        links = [f"[{item.get('source_path', item['role'])}]({item['path']})" for item in entry['files']]
        lines.append('| ' + entry['name'] + ' ' + entry['version'] + ' | ' + (entry['license_expression'] or 'license-file') + ' | ' + ', '.join(links) + ' |')
    texts['manifest.json'] = encoded(manifest)
    texts['README.md'] = ('\n'.join(lines) + '\n').encode()
    return texts, len(entries)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write', action='store_true')
    mode.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected, count = generate()
    current = {package['name'] + '@' + package['version'] for package in json.loads(expected['manifest.json'])['packages']}
    supplement_file = OUTPUT / 'supplements.json'
    supplements = json.loads(supplement_file.read_bytes())
    obsolete = {file.relative_to(OUTPUT).as_posix() for file in (OUTPUT / 'texts').glob('*.txt')
                if re.fullmatch('[0-9a-f]{64}', file.stem)} - expected.keys()
    if args.write:
        for relative, data in expected.items():
            file = OUTPUT / relative
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(data)
        for relative in obsolete:
            (OUTPUT / relative).unlink()
        supplements['packages'] = {key: value for key, value in supplements['packages'].items() if key in current}
        supplement_file.write_bytes(encoded(supplements))
    else:
        assert not obsolete, 'unreferenced generated license texts; regenerate the bundle'
        assert supplements['packages'].keys() <= current, 'retired dependency supplements; regenerate the bundle'
        for relative, data in expected.items():
            assert (OUTPUT / relative).read_bytes() == data, 'stale or altered license bundle: ' + relative
    print(('generated' if args.write else 'verified') + f': {count} Rust dependencies, {len(expected) - 2} deduplicated notice texts')


if __name__ == '__main__':
    main()
