import json
import subprocess
from dataclasses import asdict
from pathlib import Path

import pytest

from videodock.core import Settings


@pytest.mark.parametrize('mode,extension', [('video', '.mp4'), ('audio', '.mp3')])
def test_real_media_download_and_conversion(media_server, tmp_path, mode, extension, worker_command):
    settings = Settings(folder=str(tmp_path / 'Результат'), mode=mode)
    result = subprocess.run(worker_command,
                            input=json.dumps({'url': media_server, 'settings': asdict(settings)}),
                            text=True, capture_output=True, timeout=45)
    assert result.returncode == 0, result.stderr + result.stdout
    events = [json.loads(line) for line in result.stdout.splitlines()]
    assert events[-1]['event'] == 'done'
    file = Path(next(e['path'] for e in events if e['event'] == 'file'))
    assert file.is_file() and file.suffix == extension
    probe = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration',
                            '-of', 'json', str(file)], capture_output=True, text=True, check=True)
    assert float(json.loads(probe.stdout)['format']['duration']) > 1


def test_bad_url_emits_structured_error_without_traceback(tmp_path, worker_command):
    result = subprocess.run(worker_command,
                            input=json.dumps({'url': 'file:///etc/passwd',
                                              'settings': asdict(Settings(folder=str(tmp_path)))}),
                            text=True, capture_output=True, timeout=10)
    assert result.returncode == 1
    assert json.loads(result.stdout.splitlines()[-1])['event'] == 'error'
    assert 'Traceback' not in result.stderr


@pytest.mark.parametrize('playlist,count', [(False, 1), (True, 2)])
def test_playlist_requires_explicit_opt_in_and_subtitles_are_saved(media_server, tmp_path, worker_command, playlist, count):
    folder = tmp_path / 'output'
    settings = Settings(folder=str(folder), playlist=playlist, subtitles=True)
    request = {'url': media_server.replace('sample.mp4', 'playlist.html'), 'settings': asdict(settings)}
    result = subprocess.run(worker_command, input=json.dumps(request), text=True,
                            capture_output=True, timeout=45, check=False)
    assert result.returncode == 0, result.stdout + result.stderr
    events = [json.loads(line) for line in result.stdout.splitlines()]
    assert events[-1] == {'event': 'done', 'count': count}
    assert len(list(folder.glob('*.mp4'))) == count
    subtitles = list(folder.glob('*.vtt'))
    assert subtitles and 'Test subtitle' in subtitles[0].read_text()
