"""Exercise the production macOS helper and signed GUI in isolated application copies.

Usage: python3 scripts/rust/check-macos-update.py /path/to/VideoDock.app [report-dir]
Requires the existing VideoDock Self-Signed CI identity, not a new certificate.
"""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import zipfile

PROJECT = Path(__file__).resolve().parents[2]
TEST = 'updater::macos::tests::native_handoff'


def run(arguments, **kwargs):
    return subprocess.run([str(value) for value in arguments], check=True, **kwargs)


def version(app):
    return plistlib.loads((app / 'Contents/Info.plist').read_bytes())['CFBundleShortVersionString']


def processes(root):
    result = subprocess.check_output(['/bin/ps', '-axo', 'pid=,command='], text=True)
    return [int(line.split(None, 1)[0]) for line in result.splitlines()
            if str(root) in line and '/Contents/' in line]


def stop_fixture(root):
    for pid in processes(root):
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    deadline = time.monotonic() + 10
    while processes(root) and time.monotonic() < deadline:
        time.sleep(0.1)
    assert not processes(root), 'Fixture GUI/helper did not exit'


def seed(profile):
    profile.mkdir()
    browser = profile / 'browser'
    browser.mkdir(mode=0o700)
    (browser / 'fixture-site-data').write_bytes(b'existing browser site data')
    (browser / 'fixture-site-data').chmod(0o600)
    (profile / 'secrets.json').write_text('{}')
    (profile / 'secrets.json').chmod(0o600)
    downloads = profile / 'downloads'
    downloads.mkdir()
    existing = downloads / 'existing.mp4'
    existing.write_bytes(b'existing download must survive update and rollback\0')
    state = {'settings': {'autoUpdate': False, 'language': 'en', 'theme': 'dark',
                          'path': str(downloads)}, '_automatic_updates_v1': True,
             'jobs': [{'id': 'update-fixture-history', 'state': 'complete',
                       'title': 'Existing completed download', 'path': str(existing), 'kind': 'video', 'media_type': 'video', 'created_at': 1,
                       'url': 'https://example.invalid/fixture', 'progress': 100}],
             'subscriptions': [], 'onboarding_completed': True}
    with sqlite3.connect(profile / 'state.sqlite') as database:
        database.execute('CREATE TABLE state (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL)')
        database.execute('INSERT INTO state VALUES(1, ?)', (json.dumps(state),))
    return existing, existing.read_bytes()


def assert_profile(profile, existing, original):
    assert existing.read_bytes() == original, 'Existing download changed'
    assert (profile / 'browser/fixture-site-data').read_bytes() == b'existing browser site data'
    assert (profile / 'secrets.json').read_text() == '{}'
    assert (profile / 'secrets.json').stat().st_mode & 0o777 == 0o600
    assert (profile / 'browser').stat().st_mode & 0o777 == 0o700
    with sqlite3.connect(profile / 'state.sqlite') as database:
        assert database.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        state = json.loads(database.execute('SELECT value FROM state WHERE id=1').fetchone()[0])
    assert state['settings']['autoUpdate'] is False, 'User update preference changed'
    assert state['settings']['language'] == 'en' and state['settings']['theme'] == 'dark'
    assert any(job['id'] == 'update-fixture-history' and job['state'] == 'complete'
               and job['path'] == str(existing) and job['kind'] == 'video'
               and job['media_type'] == 'video' and job['progress'] == 100
               for job in state['jobs']), 'Completed history disappeared'


def archive(app, path):
    run(['/usr/bin/ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', app, path],
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


def update_for(path):
    with path.open('rb') as source:
        digest = hashlib.file_digest(source, 'sha256').hexdigest()
    return {'version': '999.0.0', 'url': 'https://example.invalid/fixture.zip',
            'sha256': digest, 'bytes': path.stat().st_size}


def fixture(package, archive_path, destination, report, *, rollback=False, reject=False):
    destination.mkdir()
    current = destination / 'VideoDock.app'
    run(['/usr/bin/ditto', package, current], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    old_version = version(current)
    profile = destination / 'profile'
    existing, original = seed(profile)
    before = hashlib.sha256((profile / 'state.sqlite').read_bytes()).hexdigest()
    environment = {**os.environ, 'VIDEODOCK_FIXTURE_CURRENT': str(current),
                   'VIDEODOCK_FIXTURE_ARCHIVE': str(archive_path),
                   'VIDEODOCK_FIXTURE_PROFILE': str(profile),
                   'VIDEODOCK_FIXTURE_UPDATE': json.dumps(update_for(archive_path))}
    environment.pop('VIDEODOCK_UPDATE_STAGE', None)
    if rollback:
        # The real helper runs before Qt initialization; only its new GUI child fails.
        environment['QT_QPA_PLATFORM'] = 'videodock-fixture-invalid-platform'
    else:
        environment.pop('QT_QPA_PLATFORM', None)
    if reject:
        environment['VIDEODOCK_FIXTURE_REJECT'] = '1'
    log = report / (destination.name + '.log')
    try:
        with log.open('wb') as output:
            run(['cargo', 'test', '-p', 'videodock-core', '--lib', '--locked', TEST,
                 '--', '--ignored', '--exact', '--nocapture'], cwd=PROJECT,
                env=environment, stdout=output, stderr=subprocess.STDOUT, timeout=240)
        deadline = time.monotonic() + 150
        while time.monotonic() < deadline:
            stages = list(destination.glob('.videodock-update-*'))
            if reject:
                assert not stages, 'Rejected ZIP left a live handoff'
                assert version(current) == old_version
                assert hashlib.sha256((profile / 'state.sqlite').read_bytes()).hexdigest() == before
                break
            if rollback:
                recovered = [stage for stage in stages if (stage / 'profile/state.sqlite').is_file()
                             and not (stage / 'previous.app').exists()
                             and (stage / 'update.log').is_file()
                             and 'исходный пакет и профиль восстановлены' in
                             (stage / 'update.log').read_text(errors='replace')]
                if recovered:
                    assert version(current) == old_version, 'Rollback did not restore the old bundle'
                    assert hashlib.sha256((profile / 'state.sqlite').read_bytes()).hexdigest() == before
                    break
            elif not stages and version(current) == '999.0.0':
                assert processes(destination), 'Updated GUI did not remain alive after acknowledgment'
                break
            time.sleep(0.2)
        else:
            raise AssertionError('Native update handoff did not reach the expected result')
        assert_profile(profile, existing, original)
        run(['/usr/bin/codesign', '--verify', '--deep', '--strict', current],
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    finally:
        stop_fixture(destination)
        for stage in destination.glob('.videodock-update-*'):
            source = stage / 'update.log'
            if source.exists():
                shutil.copyfile(source, report / (destination.name + '-helper.log'))
        shutil.rmtree(destination)
    print(f'macOS update {destination.name}: passed', flush=True)


def main():
    if sys.platform != 'darwin':
        raise SystemExit('Native update fixture requires macOS')
    package = Path(sys.argv[1]).resolve(strict=True)
    report = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else Path(os.environ.get('RUNNER_TEMP', tempfile.gettempdir())) / 'macos-update-report'
    report.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='videodock-native-update-') as directory:
        root = Path(directory)
        new = root / 'candidate/VideoDock.app'
        new.parent.mkdir()
        run(['/usr/bin/ditto', package, new], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        plist_path = new / 'Contents/Info.plist'
        info = plistlib.loads(plist_path.read_bytes())
        info['CFBundleShortVersionString'] = info['CFBundleVersion'] = '999.0.0'
        plist_path.write_bytes(plistlib.dumps(info))
        run(['/usr/bin/codesign', '--force', '--sign', 'VideoDock Self-Signed', new],
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        good_zip = root / 'signed-update.zip'
        archive(new, good_zip)
        fixture(package, good_zip, root / 'successful-replacement', report)
        fixture(package, good_zip, root / 'startup-rollback', report, rollback=True)
        run(['/usr/bin/codesign', '--force', '--sign', '-', new],
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        wrong_zip = root / 'wrong-signature.zip'
        archive(new, wrong_zip)
        fixture(package, wrong_zip, root / 'signature-rejection', report, reject=True)
        traversal_zip = root / 'traversal.zip'
        with zipfile.ZipFile(traversal_zip, 'w') as zipped:
            zipped.writestr('VideoDock.app/../../outside-fixture', b'escape')
        fixture(package, traversal_zip, root / 'traversal-rejection', report, reject=True)
        assert not (root / 'outside-fixture').exists(), 'Traversal escaped staging'
    (report / 'result.json').write_text(json.dumps({'success': True, 'checks': [
        'signed replacement', 'real GUI startup acknowledgment', 'profile and history',
        'existing download', 'browser and secrets preservation', 'startup rollback', 'signature rejection', 'traversal rejection']}, indent=2))


if __name__ == '__main__':
    main()
