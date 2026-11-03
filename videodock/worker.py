import json
import os
import re
import shutil
import sys
import time
from pathlib import Path

from .core import Settings, download_options, validate_url


def emit(event, **values):
    print(json.dumps({'event': event, **values}, ensure_ascii=True), flush=True)


def safe_message(value):
    value = re.sub(r'\x1b\[[0-9;]*m', '', str(value))
    value = re.sub(r'https?://\S+', '[ссылка]', value)
    return value[-1500:]


class Logger:
    def debug(self, message):
        pass

    def info(self, message):
        pass

    def warning(self, message):
        emit('warning', message=safe_message(message))

    def error(self, message):
        pass  # DownloadError is delivered once by run().


def run():
    try:
        if os.name == 'posix':
            os.setsid()
        request = json.loads(sys.stdin.buffer.read(65537))
        url = validate_url(request['url'])
        settings = Settings(**request['settings']).validate()
        opts = download_options(settings)
        if not shutil.which('ffmpeg'):
            raise ValueError('FFmpeg не найден. Установи его и перезапусти VideoDock.')
        Path(settings.folder).mkdir(parents=True, exist_ok=True)
        emit('ready')
        import yt_dlp

        last_progress = 0.0

        def progress(data):
            nonlocal last_progress
            info = data.get('info_dict') or {}
            if data['status'] == 'finished':
                emit('processing', title=str(info.get('title') or 'Обработка медиа'))
                return
            now = time.monotonic()
            if now - last_progress < 0.15:
                return
            last_progress = now
            total = data.get('total_bytes') or data.get('total_bytes_estimate') or 0
            downloaded = data.get('downloaded_bytes') or 0
            emit('progress', title=str(info.get('title') or 'Загрузка'),
                 percent=min(100, downloaded / total * 100) if total else 0,
                 speed=data.get('speed') or 0, eta=data.get('eta'),
                 playlist_index=info.get('playlist_index'), playlist_count=info.get('n_entries'))

        files = []

        def finished(filename):
            path = Path(filename).resolve()
            if not path.is_relative_to(Path(settings.folder).resolve()):
                raise ValueError('Движок вернул путь за пределами папки загрузок.')
            if path.is_file():
                files.append(str(path))
                emit('file', path=str(path))

        opts.update(logger=Logger(), progress_hooks=[progress], post_hooks=[finished],
                    postprocessor_hooks=[lambda data: emit('processing')])
        with yt_dlp.YoutubeDL(opts) as ydl:
            ydl.extract_info(url, download=True)
        if not files:
            raise ValueError('Загрузка завершилась без готового файла.')
        emit('done', count=len(files))
        return 0
    except Exception as exc:
        emit('error', message=safe_message(exc))
        return 1
