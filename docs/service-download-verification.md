# Полные загрузки сервисов, 03-04.11.2026

Актуальное продолжение 04.11: [полная матрица стабильности](stability-2026-11-04.md). BiliBili подтверждён через явно заданный прокси, Generic после исправления и через прокси, и напрямую, Tumblr - напрямую; прямой Dailymotion оказался нестабилен. Ниже сохранены результаты предыдущего прохода.

Проверка проходит через настоящий Rust `Catalog -> pipeline -> HTTP/HLS/DASH -> FFmpeg -> ffprobe`, без аккаунтов, пользовательских cookies, Python/yt-dlp в приложении. Успех требует целого файла с аудио, для видео также видеопотока, и длительности, совпадающей с metadata с допуском max(3 секунды, 2%). Скачивание первых 64 KiB таким доказательством не считается.

Исходный полный прогон выявил обрезку NicoVideo до 6 секунд при исходных 320, ошибку GraphQL клипов Twitch и общий 30-секундный timeout медиаклиента. Таймауты metadata и медиапотока разделены: разбор ограничен 30 секундами, передача имеет 30 секунд бездействия и не ограничивается общей длительностью запроса. Локальный медленный HTTP response успешно принимался 35 секунд; его содержимое проверено целиком. Proxy/headers/redirect policy наследуются одинаково обоими клиентами.

Twitch clip API требует `playbackAccessToken(params:{platform:"web",playerBackend:"mediaplayer",playerType:"site"})`; раньше обязательный params отсутствовал. Официальный API воспроизводил ошибку `argument: params is required on field: playbackAccessToken but missing`. После исправления текущий публичный клип организационного канала Twitch скачан полностью.

## Измеренные полные файлы

Время pipeline включает запросы манифестов, передачу, объединение и публикацию. Это разовый замер на текущем соединении; скорость короткого файла включает постоянные накладные расходы и не измеряет предельную пропускную способность канала.

| Сервис | Разбор, с | Pipeline, с | Готовый файл, байт | Длительность ffprobe, с | Результат |
|---|---:|---:|---:|---:|---|
| BitChute | 1.064 | 8.216 | 5 722 657 | 160.840 | Полный audio+video, фильтр <=360p |
| Dailymotion | 0.866 | 2.946 | 1 536 626 | 25.540 | Полный audio+video, фильтр <=360p |
| Flickr | 2.289 | 1.179 | 129 654 | 3.500 | Полный audio+video, фильтр <=360p |
| Rumble | 0.912 | 1.898 | 2 385 439 | 30.070 | Полный audio+video, фильтр <=360p |
| SoundCloud | 4.492 | 7.264 | 5 325 937 | 213.902 | Полная аудиодорожка M4A |
| TikTok | 1.481 | 3.701 | 3 308 460 | 24.402 | Полный audio+video, фильтр <=1080p; у sample нет <=360p |
| Twitch | 1.125 | 4.180 | 35 523 746 | 60.013 | Полный audio+video, фильтр <=1080p, исправленный clip query |
| NicoVideo | 1.685 | 44.178 | 15 343 146 | 320.087 | Полный audio+video, после исправления наследования HLS key и буферизации AES |

До исправления наследования HLS encryption key NicoVideo ложно публиковала 6.006 секунд вместо 320. Полный повтор подтвердил 320.087 секунды. Буферизация AES сократила время обработки локального фрагмента с 1154 до 75 мс/MiB; полный pipeline ускорился с 58.801 до 44.178 секунды. Новый duration guard не публикует усечённый файл.

## Незакрытые строки

- **Vimeo**: прежний sample 76979871 сейчас отдаёт HLS `METHOD=SAMPLE-AES`, `KEYFORMAT=com.apple.streamingkeydelivery`; проверенные publisher `url`, `avc_url`, `fallback_url` защищены, DASH имеет `ContentProtection`. Это DRM, который программа не обходит. Альтернативные inputs 1084537/22439234/90509568 вернули HTTP403, 59777392 не предоставил потоков. Поиск через OpenCLI нашёл публичный CC-фильм 59356856: официальная watch page открылась в Chrome, но анонимный config вернул HTTP403, в том числе с публичным embed hash и Referer. Полный доступный незашифрованный Vimeo sample в этом проходе не подтверждён.
- **BiliBili**: metadata доступна; sample BV1xx411c7mD длится 2055 секунд. Для обеих выбранных video/audio дорожек контрольный запрос без Range дал HTTP567, `bytes=0-` и `bytes=0-65535` дали HTTP206. Extractor теперь передаёт Range `bytes=0-`; незавершённый open range требует последовательных ограниченных запросов transfer. Это остаётся открытым до успешного полного файла.
- **Facebook**: sampled anonymous page не предоставила native video URLs; успешная авторизованная загрузка не проверена.
- **NaverTV**: текущий публичный вход завершился транспортной ошибкой; нового playback ticket не получено. Старые fixtures не доказывают актуальную Clip-схему.
- **Likee**: homepage не предоставила публичный post id; отсутствие profile pagination остаётся явным ограничением, полного sample нет.
- **Tumblr**: public staff blog вернул HTTP403. Полный публичный video sample и modern pagination не подтверждены.
- **Vine**: исходный сервис закрыт; downloader возвращает явный unsupported.
- **Xvideos/Redtube**: dedicated handlers по-прежнему отсутствуют; наличие имён в REA ParserGraph не является реализацией.
- **Generic**: общий HTML5/direct-media extractor проверяется локальными fixture/pipeline тестами; это не обещание поддержки любого сайта с JavaScript player.

## Воспроизведение

`crates/core/tests/catalog_downloads.rs` содержит opt-in полные загрузки: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$HOME/.cache/videodock-rust-target cargo test -p videodock-core --test catalog_downloads --offline -- --ignored --nocapture --test-threads=2`. Объявленные незакрытыми samples могут завершаться ошибкой; это не маскируется успешным metadata test.

Логи: `~/.local/share/videodock/research/full-downloads-2026-11-03/`. Signed delivery URLs, cookies, ключи HLS и token values в stdout не выводятся. Аккаунты, private/Premium/geo-restricted content, исчерпывающие форматы и все коллекции, macOS/Windows этим проходом не проверены.
