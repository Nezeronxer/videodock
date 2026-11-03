# План реализации VideoDock

Цель: реализовать `docs/design.md` в этом сеансе. Изолированный новый каталог, работа последовательно. Новых фоновых служб и серверов нет.

1. `tests/test_core.py`: отклонение локальных схем/credentials/control characters, сохранение настроек и истории, восстановление прерванной загрузки, ограничение качества и MP3 options. Проверить падение до реализации. Создать `videodock/core.py`, повторить `python -m pytest tests/test_core.py`.
2. `tests/test_worker.py`: создать тестовое видео через FFmpeg, отдать через локальный HTTP fixture, запустить `python -m videodock --worker`, проверить JSON, выход 0 и ffprobe выходного файла. Реализовать `worker.py` и `__main__.py`, проверить MP3 и ошибочный URL.
3. `tests/test_ui.py`: Qt offscreen, временный каталог данных, добавить fixture URL, дождаться завершения, проверить карточку и SQLite. Реализовать `ui.py`: настройки, добавление/очередь, отмена/повтор, открытие файлов, фильтры истории, блокировка второго экземпляра. Проверить отмену и перезапуск.
4. `scripts/launch.py`, `scripts/build.py`, `scripts/install_linux.py`, `pyproject.toml`, `README.md`, `LICENSE`, CI: запуск из исходников, Linux-ярлык, сборка на каждой целевой ОС. Venv и build вне Dropbox. Проверить Linux build, версию и тесты собранного worker.
5. Аудит новых исходников и зависимостей, повтор pytest/ruff, визуальная инспекция Qt-снимка. Отчёт хранит конкретные команды и ограничения. Проверить git identity Nezeronxer до первого коммита, затем `gt rig add videodock file:///...`.
