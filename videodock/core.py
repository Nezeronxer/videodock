import json
import os
import sqlite3
import sys
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
from pathlib import Path
from urllib.parse import urlsplit

QUALITIES = (0, 2160, 1440, 1080, 720, 480)
STATES = {'queued', 'downloading', 'processing', 'done', 'cancelled', 'error', 'interrupted'}


def validate_url(value: str) -> str:
    if not isinstance(value, str) or any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError('В ссылке есть недопустимые символы.')
    value = value.strip()
    try:
        parts = urlsplit(value)
        port = parts.port
        valid = parts.scheme in ('http', 'https') and parts.hostname and not parts.username and not parts.password
        if not valid or (port is not None and port == 0) or len(value) > 8192 or ' ' in value:
            raise ValueError
    except ValueError:
        raise ValueError('Нужна ссылка http:// или https:// без логина и пароля.') from None
    return value


def validate_proxy(value: str) -> str:
    if not isinstance(value, str) or any(c.isspace() or ord(c) < 32 for c in value):
        raise ValueError('Проверь адрес прокси.')
    if value == '':
        return ''
    try:
        parts = urlsplit(value)
        valid = parts.scheme in ('http', 'https', 'socks5', 'socks5h') and parts.hostname and parts.port
        if not valid or parts.username or parts.password or parts.query or parts.fragment or parts.path not in ('', '/'):
            raise ValueError
    except ValueError:
        raise ValueError('Прокси: socks5://127.0.0.1:1080 или http://host:port, без логина и пароля.') from None
    return value


@dataclass
class Settings:
    folder: str = str(Path.home() / ('Movies' if sys.platform == 'darwin' else 'Videos') / 'VideoDock')
    mode: str = 'video'
    quality: int = 1080
    subtitles: bool = False
    playlist: bool = False
    proxy: str = ''

    def validate(self):
        if self.mode not in ('video', 'audio') or type(self.quality) is not int or self.quality not in QUALITIES:
            raise ValueError('Неизвестный формат или качество.')
        if not isinstance(self.folder, str) or not Path(self.folder).is_absolute() or '\x00' in self.folder:
            raise ValueError('Выбери абсолютный путь к папке загрузок.')
        if type(self.subtitles) is not bool or type(self.playlist) is not bool:
            raise ValueError('Неверные настройки плейлиста или субтитров.')
        validate_proxy(self.proxy)
        return self


def download_options(settings: Settings) -> dict:
    settings.validate()
    fmt = 'bv*+ba/b'
    if settings.quality:
        fmt = f'bv*[height<=?{settings.quality}]+ba/b[height<=?{settings.quality}]'
    return {
        'format': 'bestaudio/best' if settings.mode == 'audio' else fmt,
        'merge_output_format': 'mp4',
        'paths': {'home': settings.folder},
        'outtmpl': {'default': '%(title).150B [%(id)s].%(ext)s'},
        'windowsfilenames': True,
        'overwrites': False,
        'continuedl': True,
        'noplaylist': not settings.playlist,
        'playlist_items': None if settings.playlist else '1',
        'writesubtitles': settings.subtitles,
        'writeautomaticsub': settings.subtitles,
        'subtitleslangs': ['ru', 'en'],
        'proxy': settings.proxy,
        'socket_timeout': 20,
        'retries': 3,
        'fragment_retries': 3,
        'quiet': True,
        'noprogress': True,
        'js_runtimes': {'node': {}},
        'postprocessors': ([{'key': 'FFmpegExtractAudio', 'preferredcodec': 'mp3', 'preferredquality': '192'}]
                           if settings.mode == 'audio' else []),
    }


class Store:
    def __init__(self, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(path)
        self.db.row_factory = sqlite3.Row
        self.db.executescript('''
            CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY, url TEXT NOT NULL, options TEXT NOT NULL,
                title TEXT NOT NULL, state TEXT NOT NULL, percent REAL NOT NULL DEFAULT 0,
                detail TEXT NOT NULL DEFAULT '', filepath TEXT NOT NULL DEFAULT '',
                created TEXT NOT NULL
            );
            UPDATE jobs SET state='interrupted', detail='Прервано при закрытии. Нажми «Повторить».'
                WHERE state IN ('downloading', 'processing', 'queued');
        ''')
        self.db.commit()
        if os.name != 'nt':
            path.chmod(0o600)

    def settings(self) -> Settings:
        row = self.db.execute('SELECT data FROM settings WHERE id=1').fetchone()
        return Settings(**json.loads(row['data'])).validate() if row else Settings()

    def save_settings(self, settings: Settings):
        settings.validate()
        with self.db:
            self.db.execute('INSERT OR REPLACE INTO settings VALUES (1, ?)', (json.dumps(asdict(settings)),))

    def add(self, url: str, settings: Settings) -> dict:
        url = validate_url(url)
        settings.validate()
        with self.db:
            cursor = self.db.execute('INSERT INTO jobs (url,options,title,state,created) VALUES (?,?,?,?,?)',
                                     (url, json.dumps(asdict(settings)), urlsplit(url).hostname, 'queued',
                                      datetime.now(UTC).isoformat()))
        return self.job(cursor.lastrowid)

    def job(self, job_id: int) -> dict:
        row = self.db.execute('SELECT * FROM jobs WHERE id=?', (job_id,)).fetchone()
        if row is None:
            raise ValueError('Задание не найдено.')
        result = dict(row)
        result['options'] = json.loads(result['options'])
        return result

    def jobs(self) -> list[dict]:
        return [self.job(row['id']) for row in self.db.execute('SELECT id FROM jobs ORDER BY id DESC')]

    def update(self, job_id: int, **values):
        if not values or not values.keys() <= {'title', 'state', 'percent', 'detail', 'filepath'}:
            raise ValueError('Неизвестное поле задания.')
        if 'state' in values and values['state'] not in STATES:
            raise ValueError('Неизвестное состояние.')
        with self.db:
            self.db.execute(f"UPDATE jobs SET {', '.join(k + '=?' for k in values)} WHERE id=?",
                            (*values.values(), job_id))

    def remove(self, job_id: int):
        with self.db:
            self.db.execute('DELETE FROM jobs WHERE id=?', (job_id,))

    def close(self):
        self.db.close()
