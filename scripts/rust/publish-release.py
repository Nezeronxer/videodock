"""Publish validated Actions packages without granting a browser local-file access."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def gh(*arguments):
    result = subprocess.run(['gh', *arguments], capture_output=True, text=True)
    require(result.returncode == 0, 'GitHub operation failed: ' + arguments[0])
    return result.stdout


def api(path):
    return json.loads(gh('api', path))


def nonruntime(path):
    return (path in {'README.md', 'TASK.md', 'crates/desktop/TASK.md',
                     'scripts/rust/publish-release.py', 'scripts/rust/check-macos-frameworks.py'}
            or path == '.github/workflows/native-builds.yml'
            or path.startswith(('docs/', 'crates/core/tests/',
                                'crates/media/tests/', 'crates/desktop/tests/',
                                'scripts/rust/tests/')))


def producer_sources_match(paths, kind):
    return all(nonruntime(path) or (kind != 'macos' and path == 'scripts/rust/repair-macos-frameworks.py')
               for path in paths)


def succeeded(job, names):
    return all(len(matches := [step for step in job['steps'] if step['name'] == name]) == 1
               and matches[0]['conclusion'] == 'success' for name in names)


def tag_commit(reference, loader):
    seen = set()
    while reference['type'] == 'tag':
        sha = reference['sha']
        require(re.fullmatch(r'[0-9a-f]{40}', sha) and sha not in seen and len(seen) < 8,
                'Invalid or cyclic annotated tag')
        seen.add(sha)
        reference = loader(sha)['object']
    require(reference['type'] == 'commit' and re.fullmatch(r'[0-9a-f]{40}', reference['sha']),
            'Tag does not point to a commit')
    return reference['sha']


def check_tag(repo, tag, source):
    matches = [ref for ref in api(f'repos/{repo}/git/matching-refs/tags/{tag}')
               if ref['ref'] == 'refs/tags/' + tag]
    require(len(matches) <= 1, 'Duplicate tag references')
    if matches:
        commit = tag_commit(matches[0]['object'], lambda sha: api(f'repos/{repo}/git/tags/{sha}'))
        require(commit == source, 'Existing tag points to different release source')


def section(text, header):
    lines = text.splitlines()
    starts = [index for index, line in enumerate(lines)
              if re.fullmatch(re.escape(header) + r'\s*(?:#.*)?', line)]
    require(len(starts) <= 1, 'Duplicate workflow section')
    if not starts:
        require(not any(line.startswith(header) for line in lines), 'Unsupported inline workflow section')
        return ''
    start, indent = starts[0], len(header) - len(header.lstrip())
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if line.strip() and not line.lstrip().startswith('#') and len(line) - len(line.lstrip()) <= indent:
            end = index
            break
    return '\n'.join(lines[start:end]).rstrip()


def build_inputs_match(producer, source, jobs):
    headers = ['env:', 'permissions:', 'defaults:'] + ['  ' + name + ':' for name in jobs]
    return all(section(producer, header) == section(source, header) for header in headers)


def validate_producer(repo, number, source, kind):
    require(re.fullmatch(r'[1-9][0-9]*', number), 'Invalid producer run ID')
    run = api(f'repos/{repo}/actions/runs/{number}')
    require(run['repository']['full_name'].lower() == repo.lower(), 'Producer repository mismatch')
    require(run['status'] == 'completed', 'Producer is not complete')
    head = run['head_sha']
    require(re.fullmatch(r'[0-9a-f]{40}', head), 'Invalid producer SHA')
    diff = subprocess.run(['git', 'diff', '--name-only', head, source], capture_output=True, text=True)
    require(diff.returncode == 0, 'Producer history unavailable')
    require(producer_sources_match(diff.stdout.splitlines(), kind), 'Runtime source differs from producer')
    jobs = api(f'repos/{repo}/actions/runs/{number}/jobs?filter=latest&per_page=100')['jobs']
    workflow = subprocess.run(['git', 'show', head + ':.github/workflows/native-builds.yml'],
                              capture_output=True, text=True)
    require(workflow.returncode == 0, 'Producer workflow unavailable')
    source_workflow = Path('.github/workflows/native-builds.yml').read_text()
    builders = ['linux', 'linux-flatpak'] if kind == 'linux' else [kind]
    require(build_inputs_match(workflow.stdout, source_workflow, builders), 'Producer build inputs differ')
    if kind == 'linux':
        require(all(any(job['name'] == name and job['conclusion'] == 'success' for job in jobs)
                    for name in ['linux', 'linux-flatpak']), 'Linux package gates did not pass')
        require(workflow.returncode == 0 and 'flatpak-worker" --proxy --startup' in workflow.stdout,
                'Installed Flatpak startup/proxy coverage missing')
    elif kind == 'windows':
        needed = ['Verify Windows package staging rollback', 'Test and build Windows package',
                  'Verify installed worker with clean PATH', 'Build Windows installer',
                  'Verify native Windows interface and real Engine in English and Russian']
        require(any(job['name'] == 'windows' and succeeded(job, needed) for job in jobs),
                'Windows package gates did not pass')
    else:
        needed = ['Verify native macOS interface and real Engine in English and Russian',
                  'Build and verify application bundle']
        for arch in ['x64', 'arm64']:
            require(any(job['name'].startswith('macos (') and '(' + arch + ',' in job['name']
                        and succeeded(job, needed) for job in jobs), 'macOS ' + arch + ' gates did not pass')
    return {'run_id': int(number), 'head_sha': head}


def collect(repo, run, artifact, directory, expected):
    destination = directory / artifact
    destination.mkdir()
    gh('run', 'download', run, '--repo', repo, '--name', artifact, '--dir', str(destination))
    files = [path for path in destination.rglob('*') if path.is_file()]
    require(not any(path.is_symlink() for path in destination.rglob('*')), 'Unexpected artifact symlink')
    require({path.name for path in files if path.name != 'SHA256SUMS'} == set(expected),
            'Unexpected package contents: ' + artifact)
    require(all(path.parent == destination and 0 < path.stat().st_size < 2 * 1024**3 for path in files),
            'Unexpected package layout or size: ' + artifact)
    return {path.name: path for path in files if path.name != 'SHA256SUMS'}


def main():
    if sys.argv[1:] == ['--self-test']:
        assert nonruntime('docs/releases/0.2.0.md') and nonruntime('.github/workflows/native-builds.yml')
        assert nonruntime('crates/desktop/tests/appearance.cpp')
        for path in ['crates/desktop/qml/Main.qml', 'crates/core/src/pipeline.rs',
                     'Cargo.lock', 'scripts/rust/install.sh', 'LICENSES/GPL-3.0.txt']:
            assert not nonruntime(path), path
        assert succeeded({'steps': [{'name': 'UI', 'conclusion': 'success'}]}, ['UI'])
        assert not succeeded({'steps': [{'name': 'UI', 'conclusion': 'failure'}]}, ['UI'])
        assert not succeeded({'steps': []}, ['UI'])
        assert not succeeded({'steps': [{'name': 'UI', 'conclusion': 'success'},
                                         {'name': 'UI', 'conclusion': 'failure'}]}, ['UI'])
        source = 'a' * 40
        assert tag_commit({'type': 'commit', 'sha': source}, lambda _: None) == source
        assert tag_commit({'type': 'tag', 'sha': 'b' * 40},
                          lambda _: {'object': {'type': 'commit', 'sha': source}}) == source
        try:
            tag_commit({'type': 'tag', 'sha': 'b' * 40},
                       lambda _: {'object': {'type': 'tag', 'sha': 'b' * 40}})
        except RuntimeError:
            pass
        else:
            raise AssertionError('Cyclic tag accepted')
        before = 'env:\n  A: one\njobs:\n  windows:\n    run: build\n  macos:\n    run: build\n'
        assert build_inputs_match(before, before, ['windows'])
        assert not build_inputs_match(before, before.replace('A: one', 'A: two'), ['windows'])
        assert not build_inputs_match(before, before.replace('run: build', 'run: other'), ['windows'])
        assert not nonruntime('.github/scripts/change-package.sh')
        assert producer_sources_match(['scripts/rust/repair-macos-frameworks.py'], 'windows')
        assert producer_sources_match(['scripts/rust/repair-macos-frameworks.py'], 'linux')
        assert not producer_sources_match(['scripts/rust/repair-macos-frameworks.py'], 'macos')
        print('Release gate self-check passed')
        return
    repo, source = os.environ['GITHUB_REPOSITORY'], os.environ['GITHUB_SHA']
    require(re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repo), 'Invalid repository')
    require(re.fullmatch(r'[0-9a-f]{40}', source), 'Invalid release source')
    require(os.environ['GITHUB_ACTOR'].lower() == repo.split('/')[0].lower(), 'Only the owner may publish')
    version = tomllib.loads(Path('Cargo.toml').read_text())['workspace']['package']['version']
    tag = os.environ['RELEASE_TAG']
    require(tag == 'v' + version and re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+', tag), 'Tag/version mismatch')
    notes = Path('docs/releases') / (version + '.md')
    require(notes.is_file(), 'Release notes missing')
    check_tag(repo, tag, source)
    windows, linux, macos = (os.environ[key] for key in ['WINDOWS_RUN', 'LINUX_RUN', 'MACOS_RUN'])
    producers = {kind: validate_producer(repo, number, source, kind)
                 for kind, number in [('windows', windows), ('linux', linux), ('macos', macos)]}
    releases = api(f'repos/{repo}/releases?per_page=100')
    matching = [release for release in releases if release['tag_name'] == tag]
    require(len(matching) == 1 and matching[0]['draft'] and not matching[0]['prerelease'],
            'Expected one unpublished stable release draft')
    release = matching[0]
    with tempfile.TemporaryDirectory(prefix='videodock-release-') as temporary:
        directory = Path(temporary)
        packages = {}
        specs = [
            (windows, 'VideoDock-windows-x64', {'VideoDock-windows-x64.zip': f'VideoDock-{version}-windows-x64.zip',
                 f'VideoDock-{version}-windows-x64-Setup.exe': f'VideoDock-{version}-windows-x64-Setup.exe'}),
            (linux, 'VideoDock-linux-arch-x86_64', {'VideoDock-linux-arch-x86_64.tar.gz': f'VideoDock-{version}-linux-arch-x86_64.tar.gz'}),
            (linux, 'VideoDock-linux-flatpak-x86_64', {f'VideoDock-{version}-linux-x86_64.flatpak': f'VideoDock-{version}-linux-x86_64.flatpak'}),
            (macos, 'VideoDock-macos-x64', {'VideoDock-macos-x64.zip': f'VideoDock-{version}-macos-x64.zip'}),
            (macos, 'VideoDock-macos-arm64', {'VideoDock-macos-arm64.zip': f'VideoDock-{version}-macos-arm64.zip'}),
        ]
        output = directory / 'release'
        output.mkdir()
        for run, artifact, mapping in specs:
            for name, path in collect(repo, run, artifact, directory, mapping).items():
                destination = output / mapping[name]
                shutil.copyfile(path, destination)
                with destination.open('rb') as file:
                    digest = hashlib.file_digest(file, 'sha256').hexdigest()
                packages[destination.name] = {'sha256': digest, 'bytes': destination.stat().st_size}
        manifest = {'version': version, 'source_sha': source, 'producers': producers, 'packages': packages}
        (output / 'build-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        with (output / 'build-manifest.json').open('rb') as file:
            digest = hashlib.file_digest(file, 'sha256').hexdigest()
        sums = [info['sha256'] + '  ' + name for name, info in sorted(packages.items())]
        (output / 'SHA256SUMS').write_text('\n'.join(sums + [digest + '  build-manifest.json']) + '\n')
        existing = {asset['name']: asset.get('digest') for asset in release['assets']}
        require(set(existing) <= {path.name for path in output.iterdir()}, 'Draft contains unexpected assets')
        fresh = api(f'repos/{repo}/releases/{release["id"]}')
        require(fresh['draft'] and not fresh['prerelease'] and fresh['tag_name'] == tag,
                'Release state changed before update')
        require(fresh['updated_at'] == release['updated_at'], 'Draft changed while packages were downloaded')
        gh('release', 'edit', tag, '--repo', repo, '--target', source,
           '--title', 'VideoDock ' + version, '--notes-file', str(notes))
        for path in sorted(output.iterdir()):
            if path.name in existing:
                with path.open('rb') as file:
                    expected = 'sha256:' + hashlib.file_digest(file, 'sha256').hexdigest()
                require(existing[path.name] == expected, 'Existing draft asset differs; refusing to delete it')
            else:
                gh('release', 'upload', tag, '--repo', repo, str(path))
        for attempt in range(15):
            current = api(f'repos/{repo}/releases/{release["id"]}')
            if all(asset.get('digest') for asset in current['assets']):
                break
            time.sleep(2)
        require(current['draft'] and not current['prerelease'], 'Release state changed during upload')
        require({asset['name'] for asset in current['assets']} == {path.name for path in output.iterdir()},
                'Uploaded asset set mismatch')
        for asset in current['assets']:
            with (output / asset['name']).open('rb') as file:
                expected = 'sha256:' + hashlib.file_digest(file, 'sha256').hexdigest()
            require(asset.get('digest') == expected, 'Uploaded asset checksum mismatch')
        check_tag(repo, tag, source)
        gh('release', 'edit', tag, '--repo', repo, '--draft=false', '--prerelease=false', '--latest')
        print(f'Published {repo} {tag}: six packages, manifest and verified checksums')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print('Release stopped: ' + str(error), file=sys.stderr)
        raise SystemExit(1)
