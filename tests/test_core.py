import sqlite3
from pathlib import Path

import pytest

from videodock.core import Settings, Store, download_options, validate_proxy, validate_url


@pytest.mark.parametrize('value', ['file:///etc/passwd', 'javascript:alert(1)', 'https://a:b@example.com/v', 'http://example.com/\nfoo', '--exec=bad', ''])
def test_rejects_unsafe_input(value):
    with pytest.raises(ValueError):
        validate_url(value)


def test_preserves_video_query_and_rejects_proxy_credentials():
    assert validate_url(' https://example.com/watch?v=a&list=b ') == 'https://example.com/watch?v=a&list=b'
    assert validate_proxy('socks5://127.0.0.1:1080') == 'socks5://127.0.0.1:1080'
    with pytest.raises(ValueError):
        validate_proxy('http://user:password@proxy:8080')


def test_history_survives_restart_and_running_becomes_interrupted(tmp_path):
    store = Store(tmp_path / 'state.sqlite')
    settings = Settings(folder=str(tmp_path / 'Видео'), quality=1080)
    store.save_settings(settings)
    job = store.add('https://example.com/a', settings)
    store.update(job['id'], state='downloading', title='Название')
    store.close()
    store = Store(tmp_path / 'state.sqlite')
    assert store.settings() == settings
    assert store.jobs()[0]['state'] == 'interrupted'
    assert store.jobs()[0]['title'] == 'Название'
    store.close()


def test_sqlite_bound_values_and_history_deletion_preserves_files(tmp_path):
    store = Store(tmp_path / 'state.sqlite')
    file = tmp_path / 'video.mp4'
    file.write_bytes(b'video')
    job = store.add("https://example.com/a?x=';DROP%20TABLE%20jobs;--", Settings(folder=str(tmp_path)))
    store.update(job['id'], state='done', filepath=str(file))
    store.remove(job['id'])
    assert store.jobs() == []
    assert file.read_bytes() == b'video'
    assert sqlite3.connect(tmp_path / 'state.sqlite').execute('select count(*) from jobs').fetchone() == (0,)
    store.close()


def test_options_do_not_exceed_quality_or_use_shell(tmp_path):
    opts = download_options(Settings(folder=str(tmp_path), quality=1080))
    import yt_dlp
    formats = [dict(url=f'https://example.com/{height}.mp4', height=height,
                    format_id=str(height), ext='mp4', vcodec='avc1', acodec='aac') for height in (720, 2160)]
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.process_ie_result({'id': 'sample', 'title': 'sample', 'formats': formats,
                                      'extractor': 'generic', 'webpage_url': 'https://example.com'}, download=False)
    assert info['height'] == 720
    assert opts['proxy'] == ''
    assert opts['noplaylist'] is True
    assert opts['overwrites'] is False
    assert opts['paths']['home'] == str(tmp_path)
    assert 'exec_cmd' not in opts
    assert Path(opts['paths']['home']).is_absolute()


def test_audio_and_subtitle_options(tmp_path):
    opts = download_options(Settings(folder=str(tmp_path), mode='audio', subtitles=True, playlist=True))
    assert opts['format'] == 'bestaudio/best'
    assert opts['postprocessors'][0]['preferredcodec'] == 'mp3'
    assert opts['writesubtitles'] is True
    assert opts['noplaylist'] is False


def test_invalid_settings_fail_before_download(tmp_path):
    with pytest.raises(ValueError):
        download_options(Settings(folder=str(tmp_path), quality=-1))
    with pytest.raises(ValueError):
        download_options(Settings(folder=str(tmp_path), mode='exec'))


def test_sql_column_name_is_not_accepted_from_input(tmp_path):
    store = Store(tmp_path / 'state.sqlite')
    job = store.add('https://example.com/video', Settings(folder=str(tmp_path)))
    with pytest.raises(ValueError):
        store.update(job['id'], **{'title = NULL; DROP TABLE jobs; --': 'bad'})
    assert store.job(job['id'])['state'] == 'queued'
    store.close()
