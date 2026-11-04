# Проверка перед публикацией, 04.11.2026

Статус: **PARTIAL**. Исходники можно публиковать как проект в разработке. Полная стабильность всех сервисов, соответствие оригиналу 1:1 и готовый релиз для трёх ОС не подтверждены.

Исходники опубликованы в [Nezeronxer/videodock](https://github.com/Nezeronxer/videodock), ветка `main`, коммит кода `43b193b5ce48bf105cad8dc7aafdadef5f7b4622`. Архив Python-прототипа опубликован отдельно. Git remote SHA сверён с локальным; README без авторизации побайтово совпал с исходником. Бинарный релиз не создавался.

Проверка выполнена на Arch Linux в отдельных каталогах. Пользовательские аккаунты, cookies и рабочая библиотека не использовались; системные маршруты не менялись. Сохранённые REA-выводы использованы для продолжения VideoDock. Новый холодный импорт оригинала в Ghidra не выполнялся.

## Исправление этого прохода

Tumblr объединял NPF-блоки и JSON-LD всех постов страницы. Рекомендация могла попасть в варианты запрошенного видео и заменить его название/длительность. Регрессия до исправления получала два видео вместо одного. Теперь NPF ограничен `idString`/числовым `id` нужного поста, а JSON-LD - его `mainEntityOfPage`. Один разбор ID используется для современных адресов с `www`, без него и адресов `blog.tumblr.com/post/...`.

Два теста проверяют рекомендации, отсутствующий пост, строковый/числовой ID, три формы адреса и чужой host. Общие HTML5-страницы и все варианты Tumblr этим не покрыты. После исправления исходный публичный Tumblr-пост скачан полностью: **231794698 байт, 575.339 с**. Файл полностью декодирован FFmpeg без ошибок.

## Локальные проверки

| Проверка | Результат |
|---|---|
| `cargo test --workspace --locked --offline` | **164 passed, 0 failed, 42 ignored** |
| Clippy workspace/all-targets, `-D warnings` | PASS |
| Свежая Linux debug-сборка desktop | PASS |
| Два реальных DASH-потока, CPU-модели обработки, сохранение кадров/AV sync, отмена модели | **5 passed**; запускаются отдельно от workspace |
| Qt + настоящий Rust Engine | PASS: темы/restart, валидация, настройки, локальная загрузка до ffprobe, история, копирование, безопасное удаление; 5 вкладок настроек и 10 диалогов |
| Qt WebEngine proxy и cookie bridge | PASS: поддерживаемые конфигурации, явный отказ для неподдерживаемых, scoped cookies и удаление |
| Независимый ffprobe + полное декодирование FFmpeg | **15 файлов, 0 ошибок**, включая повторения разных маршрутов |
| Cargo audit | **0 известных vulnerabilities**; предупреждение сопровождения `fxhash 0.2.1` остаётся |
| Semgrep точной копии Git index | **50 правил, 96 файлов, 0 findings/parse errors**; vendor и XML-переводы исключены. QML JS-каталог отдельно проверен без нераспознаваемой директивы `.pragma library`: 119 правил, 0 findings/parse errors |
| Gitleaks точной копии Git index и истории | **0 секретов** после четырёх существующих точечных исключений публичных continuation/entity fixtures. Локальные графовые кэши и исследовательские файлы в index отсутствуют |

Нативный ручной desktop-проход не выполнен: инструмента управления приложениями в текущей сессии нет. Qt/Xvfb-проверка и просмотр её рендеров не заменяют такой проход. macOS/Windows не собирались и не запускались.

## Все 42 отложенных сценария запущены явно

Их запуск **не означает успешного прохождения**. Ниже результаты отдельных полных команд; повторы и альтернативные маршруты не суммируются как независимое покрытие.

| Набор | Passed / Failed | Наблюдение |
|---|---|---|
| Полные Catalog-загрузки напрямую | **6 / 7** | Полные BitChute, Dailymotion, Flickr, Generic, Tumblr и проверка idle timeout. Сетевые отказы BiliBili, NicoVideo, Rumble, SoundCloud, TikTok, Twitch, Vimeo |
| Тот же Catalog через прежний тестовый SOCKS-маршрут | **6 / 7** | Полные BitChute, Dailymotion, Flickr, Generic, BiliBili и idle timeout. Остальные не прошли этот маршрут |
| Дополнительный SOCKS-маршрут | **4 / 9** | В том числе полные BiliBili и TikTok. Это не исправляет красные результаты других маршрутов |
| Полные YouTube через прежний SOCKS-маршрут | **2 / 3** | Zoo 240p и Sintel HD полные; три BBB-загрузки 720p/1080p обрываются HTTP 403 |
| Catalog metadata + ограниченный media probe | **3 / 7** | Этот набор не доказывает полное скачивание |
| Коллекции | **1 / 1** | SoundCloud NASA: 1581 запись. Twitch не прошёл |
| YouTube adapter | **2 / 1** | HTTP transport и полная playlist pagination прошли; общий video/captions/channel/search сценарий не прошёл |
| YouTube очередь и durable history через публичный сервис | **0 / 1** | Сетевая загрузка завершилась ошибкой; локальная очередь/history отдельно прошла |
| YouTube через локальные HTTP/SOCKS auth-proxy | **0 / 2** | Публичная загрузка не прошла. Локальные proxy fixtures с настоящими запросами прошли в workspace |
| YouTube prepare/transfer timings напрямую | **0 / 1** | Разрыв соединения на передаче |
| Реальные DASH/обработка/отмена | **5 / 0** | Полные локальные интеграции |

Проверка одинаковых публичных страниц через HTTP/2 и HTTP/1.1 не подтвердила HTTP/2 как общую причину ошибок Catalog: SoundCloud/Twitch/NicoVideo вернули HTTP 200 обоими способами, BiliBili API имел connect error обоими. Изменение транспорта без доказанной причины не добавлялось.

Большие YouTube-файлы, актуальный Naver Clip/Blog, Likee/profile, авторизованный Facebook, защищённый Vimeo, Xvideos/Redtube, все подтипы/коллекции и межплатформенная проверка остаются открытыми. Подробная исходная матрица: [stability-2026-11-04.md](stability-2026-11-04.md).

## Воспроизведение

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$HOME/.cache/videodock-rust-target"
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline --no-deps -- -D warnings
cargo build --locked --offline -p videodock-desktop
cargo test --workspace --locked --offline --no-fail-fast -- --ignored --nocapture --test-threads=2
sh crates/desktop/tests/check-appearance.sh
sh crates/desktop/tests/check-browser-proxy.sh
sh crates/desktop/tests/check-cookie-bridge.sh
```

Последняя Cargo-команда намеренно запускает и красные публичные сценарии. Для маршрута сетевых full-download harness задаётся `VIDEODOCK_TEST_PROXY`; пользовательские credentials в команды и отчёты не добавлять. Исходные журналы, файлы и `files-proof.json`: `~/.local/share/videodock/research/publish-2026-11-04/`, вне публичного репозитория.
