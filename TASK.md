# Полная копия 4K Video Downloader+ на Rust

Постановка от03.11.2026: полная копия обнаруженных функций оригинала для Linux, macOS и Windows. Получение ссылок/форматов, скачивание и логика приложения - Rust. Интерфейс - Qt/QML. Python и yt-dlp в целевом runtime запрещены; FFmpeg допустим только для обработки уже полученных локальных данных.

Готовность: все обнаруженные экраны, настройки, сервисы и сценарии перечислены в матрице, реализованы и проверены по отдельности. Неизвестное не считается готовым; статический разбор не заменяет выполнение оригинала. Матрица: [docs/feature-matrix.md](docs/feature-matrix.md). Оригинал и его Data/аккаунты/активация не изменяются.

Ветка `feature/rust-full-parity`. Python-прототип сохранён в `archive/python-prototype` и пока остаётся в рабочем дереве; текущий launcher ещё не заменён. Rust-продукт распространяется с условиями GPL-3.0 зависимости RustyPipe.

## Сделано

- [x] REA/native исследование:18 валидных Qt bundles,909 payloads,214QML/180QM; проверены108 native registration callsites, defaults, queue enums и timer logic.
- [x] Rust workspace `crates/{media,core,desktop}`, собственный Qt/QML UI, scoped native cookie bridge.
- [x] Rust planner/экстракторы, YouTube sig/nsig, video/playlist/channel/search/captions; существующие handlers сервисов с явным разделением online/fixture-only.
- [x] SQLite и durable queue, параллельность, pause/resume/retry/force/remove, подписки, import/export и локальная библиотека.
- [x] Rust HTTP Range/Content-Range, HLS/AES, DASH и локальная FFmpeg pipeline.
- [x] Пауза/продолжение ручного разбора без повторного обхода обработанных элементов; кнопка и счётчик прогресса.
- [x] Spleeter stems/DPDFNet speech/WPE dereverb с проверкой хешей assets, CPU runtime, отменой, сохранением кадров и синхронизации.
- [x] Кроссплатформенный frontend data path через Qt AppLocalDataLocation, отдельные локальные данные вне Dropbox.
- [x] Аудит собственного кода с независимой перепроверкой: очистка приватных title до filename/output_dir, dangling symlink/retry cancellation, срок cookies и отсутствие устаревшего восстановления из secrets.json.
- [x] Обновлены wreq0.16.3/lru0.19; RustyPipe адаптирован к URI/body API. Для YouTube metadata закреплён проверенный HTTP/1.1 после воспроизводимого protocol error HTTP/2.
- [x] README и отдельные Rust verification/security docs; старые Python docs помечены архивными.

## Свежие проверки на Arch

Доказательства: `~/.local/share/videodock/research/resume-2026-11-03/`. Машина перезагрузилась17:09; более ранний `/tmp` очищен, процессы остановились. Основные проверки повторены после перезагрузки.

- [x] Общий workspace offline:88passed/0failed; ignored не считаются проверенными.
- [x] Реальная YouTube queue -> Rust HTTP -> FFmpeg -> ffprobe -> Engine restart:1passed,24.21с.
- [x] YouTube adapter/public transport/полный test playlist:3passed,26.62с.
- [x] Online metadata + ограниченный media GET десяти сервисов:10passed,37.64с. Это по одному публичному примеру, не полная поддержка каждого сервиса.
- [x] Реальный DASH single/multi-period, модели stems/speech, processing cancellation и video sync:5passed.
- [x] Qt synthetic cookieAdded/Removed + expiry/std::int64_t и native AppLocalDataLocation.
- [x] Независимый localhost cookie harness: expired absent, before-expiry positive control, after-expiry absent, no serialization, restart/legacy absent, replay present.
- [x] `cargo clippy --workspace --all-targets --offline --no-deps -- -D warnings` без замечаний.
- [x] Cargo audit:0vulnerabilities/0unsound; остаётся unmaintained fxhash0.2.1. Gitleaks:0секретов; Semgrep:51rules/53files/0findings, без partial parse.
- [x] Linux debug build после текущих изменений прошёл; журналы desktop-build-complete.log и clippy-complete.log.

## Остаток до полной копии и сдачи

- [ ] Закрыть все строки матрицы сервисов: неподтверждённые Facebook/NaverTV/Likee/Tumblr, недостающие dedicated handlers, полные коллекции и авторизованные сценарии.
- [ ] Полные4K/VR samples, разнообразные реальные manifests/subtitles/audio tracks, длительная подписка и stress/race.
- [ ] Все языки оригинала, собственный подписанный update/release channel и прочие отмеченные пробелы интерфейса.
- [ ] Реальная desktop-проверка Qt-сценариев. CUA повторно вернул apps=[]/browsers=[] и unavailable host; automated/offscreen проверки её не заменяют.
- [ ] Реальные сборки/установка/QA macOS и Windows, полный deployment Qt/FFmpeg/лицензий, Rust installer на всех ОС.
- [ ] Завершить миграцию установленного Python-прототипа и once jobs только после готовой Rust установки. Shared jobs linux/darwin/2026-11-03-videodock.sh пока явно блокируют Python установку и требуют замены.
- [ ] Финальные проверки соответствия и релиз; не объявлять полную копию по проходящим базовым тестам.

## Пути и ограничения

- Исследование: `~/.local/share/videodock/research/full-parity/{original-features.md,evidence/,native/native-findings.md,rea-static-inputs.json}`. Ghidra whole-program import не завершился за20мин, snapshot пустой; псевдокода не получено. Повторный холодный import без новой причины не запускать.
- Сборки: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$HOME/.cache/videodock-rust-target`. Нельзя хранить target/SQLite/models/browser в Dropbox.
- Cargo TLS index/archive resets обходились IPv4 загрузкой из официальных crates.io endpoints с SHA256 из Cargo.lock. `/tmp` и home находятся на разных FS: для перемещения проверенного archive нужен copy, не rename.
- Qt WebEngine/Positioning/WebChannel уже установлены. Custom pacman DB отсутствует; изолированный config только official repos с `SigLevel=Required DatabaseOptional` позволяет read-only download signed packages. Системный pacman.conf не менялся.
- Временный код HTTP diagnostics удалён. Исходные пользовательские профили и секреты не использовались. Старые статические сведения и тесты не являются доказательством запуска на macOS/Windows.
