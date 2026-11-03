# Проверки Rust-версии

Дата: 03.11.2026, Arch Linux. Ветка `feature/rust-full-parity`. Исходники в `crates/`, сборки вне Dropbox: `~/.cache/videodock-rust-target`.

- Общий offline workspace test после перезагрузки17:09: **88 passed, 0 failed**. Сетевые/тяжёлые тесты запускаются отдельно; `ignored` не считается успешным прохождением.
- Сквозная очередь YouTube с новым HTTP transport: **1 passed**, 24.21с. Извлечение и загрузка Rust, FFmpeg объединяет локальные данные; ffprobe подтвердил видео+аудио и18-21с. После Engine restart запись complete и готовый файл сохранены.
- YouTube adapter: **3 passed**, 26.62с. Публичный player HTTP, media/captions/channel/search и полная тестовая playlist continuation. Это ограниченные публичные samples, не все типы контента/аккаунтов.
- Catalog online: **10 passed**, 37.64с; metadata и ограниченный GET медиапотока публичного sample каждого сервиса.
- Реальные DASH/медиаинструменты: **5 passed**, включая два совместимых периода, официальные модели, отмену и синхронизацию видео.
- Clippy workspace/all-targets без предупреждений.
- Qt native callback: synthetic cookie addition/removal, expirationDate и системный AppLocalDataLocation прошли. Сигнатура int64 приведена к тому же виду, который генерирует CXX-Qt.
- Независимый localhost cookie harness: expired absent, positive control before expiry, after expiry absent, no cookie serialization, restart/legacy absent, replay present.
- Linux debug build и clippy прошли после последних изменений (desktop-build-complete.log, clippy-complete.log).
- Проверки безопасности и ограничения: [rust-security.md](rust-security.md).

В `wreq0.16` обнаружено несовпадение поведения: простой HTTPS GET проходил, player POST по HTTP/2 получал protocol stream error. Контрольный тот же POST через curl и HTTP/1.1 проходил. YouTube adapter использует HTTP/1.1 с явным комментарием о временном ограничении. Системные DNS/VPN/zapret/прокси не менялись; другие экстракторы и браузер этот выбор не наследуют.

Актуальные журналы: `~/.local/share/videodock/research/resume-2026-11-03/`. Отчёты, относящиеся к Python-прототипу, сохранены в `docs/verification.md` и не доказывают работу Rust-версии.

Computer use повторно вернул пустые apps/browsers и ошибку browser host unavailable. Живой проход кнопок на рабочем столе, системные диалоги, плеер, tray, реальный вход и macOS/Windows **не проверены**. Автоматические Qt/core проверки не заменяют этот проход. Матрица незавершённого соответствия: [feature-matrix.md](feature-matrix.md).
