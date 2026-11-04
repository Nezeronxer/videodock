# Catalog - проверенное поведение, 03.11.2026

Свежие полные загрузки и ограничения 04.11: [матрица стабильности](../../../../docs/stability-2026-11-04.md). Прежние 64 KiB проверки ниже сохраняются как история; они не заменяют полный pipeline.

`Catalog` получает данные и медиапотоки через Rust HTTP. В продукте нет yt-dlp/Python, конвертационных API или subprocess для извлечения ссылок. FFmpeg не участвует в этом модуле. Структурные fixtures содержат вымышленные названия, адреса `media.example` и явные placeholder tokens; реальные signed URLs, app identifiers и cookie в них не сохранены.

## Live single-media проверки

В одном полном прогоне `catalog_online` прошли 10/10: metadata + HTTP GET выбранного потока максимум64KiB. Для HTTP проверяется сигнатура медиаконтейнера, для HLS/DASH - действительный манифест. Это доказательство одного публичного sample и одного потока, а не всех форматов/аккаунтов/регионов сервиса.

| Сервис | Наблюдаемый результат | Использованный первоисточник |
|---|---|---|
| Vimeo | 2 HLS/DASH master formats,4 subtitles; HLS2685bytes | [Официальный player config](https://player.vimeo.com/video/76979871/config) |
| Dailymotion | HLS;1503bytes | [Официальный player metadata](https://www.dailymotion.com/player/metadata/video/x84sh87) |
| SoundCloud | 2 audio HLS formats;19443bytes | [Публичная track page](https://soundcloud.com/forss/flickermood), её hydration/apiClient и актуальные `a-v2.sndcdn.com/assets/*.js`; APIv2 resolve/transcoding |
| Twitch | VOD HLS;2050bytes | [Официальный frontend](https://www.twitch.tv/), его public client identifier; `gql.twitch.tv/gql` и `usher.ttvnw.net` |
| BiliBili | 6 video+2audio formats; MP4 sample65536bytes | [Официальный view API](https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD), x/player/playurl; предоставленные API backup URLs |
| Rumble | 3 progressive formats; MP4 sample65536bytes | [Официальный embed](https://rumble.com/embed/v5hzoa5/), embedJS/u3 public video API |
| NicoVideo | HLS;2949bytes | [Официальный watch](https://www.nicovideo.jp/watch/sm9), current `$watchV4.data`, bundled `resource.video.nimg.jp/...nvpc_next/assets/*.js`, `/v1/watch/{id}/access-rights/hls` |
| Flickr | 3 formats(720p/360p/HLS); MP4 sample65536bytes | [Официальная app page](https://www.flickr.com/explore), public site_key; photos.getInfo/video.getStreamInfo с публичным photo.secret |
| BitChute | Progressive MP4;65536bytes | [Официальный app bundle](https://www.bitchute.com/js/app.d1922799.js), `/api/beta9/video` и `/api/beta/video/media` |
| TikTok | 7 progressive variants; MP4 sample65536bytes | [Официальный публичный sample](https://www.tiktok.com/@tiktok/video/7106594312292453675), current `__UNIVERSAL_DATA_FOR_REHYDRATION__`, анонимная tt_chain_token lease |

Vimeo default Akamai endpoint в некоторых запросах не отвечал: модуль проверяет master manifest и использует предоставленный config альтернативный CDN. BiliBili использует только доступные адреса из baseUrl/backupUrl; проверка ограничена1байтом и4одновременными запросами. Эта проверка не гарантирует, что CDN останется доступен к моменту будущей загрузки.

TikTok показал важную регрессию парсера: HTML-сериализация script contents превращает `&signature` в `&amp;signature`. Используется raw script text; fixture URL с query проверяет отсутствие этой порчи. HTTP/2 и явный Accept нужны для совместимости с наблюдаемыми API/CDN; forceIPv4, системные DNS/прокси и TLS-хуки не добавлялись.

## Коллекции

| Область | Статус |
|---|---|
| YouTube video/playlist/channel/search | Делегировано отдельному проверенному Rust YouTube adapter. Его tests находятся в `youtube_adapter.rs`. |
| SoundCloud public channel | Live NASA organizational channel:1581entries, pagination завершена. |
| Twitch public channel VODs | Первая страница100entries доступна; продолжение возвращает `failed integrity check`. Модуль возвращает явную ошибку вместо усечённого успешного списка. Полная выгрузка требует отдельной реализации client-integrity. |
| Dailymotion playlist/user | Реализован public paginated API; полный непустой batch не проверен live. |
| SoundCloud playlist | Реализованы tracks и разрешение частичных track stubs; полный playlist не проверен live. |
| BiliBili multipart/bangumi | Реализованы parts/season/episode API; live проверен обычный одночастный clip, не Premium/bangumi. |
| Flickr photoset/photostream | Реализован public REST pagination и отбор video; live проверен отдельный video, не batch. |
| Rumble channel/user channel | Парсер реальных video links и next page; homepage/channel HTTP403 в исследовании, full batch не проверен. |
| Tumblr video/blog | HTML5/NPF + public legacy read API; legacy API ответил403. Полная современная blog pagination не доказана. |

Для списков действует предел10000entries, повторяющиеся continuation/cursors завершаются ошибкой; незавершённый список не возвращается как успешный. Отмена - drop/abort async future, фоновые polling-задачи не создаются.

## Fixture-only / неподтверждённое

- **Facebook**: current native URL fields и inline DASH parsing покрыты fixtures; sampled anonymous watch page показывала login без native streams. User-account flow не проверялся. Только отдельные video/reel/shared-video links, не выгрузка профиля; unrelated recommended videos фильтруются по requested id.
- **NaverTV**: legacy videoId/inKey + RMCNVM formats/captions покрыты fixtures. Sampled current pages не отдали playback ticket; новая Naver Clip-схема и Naver Blog playlist не реализованы.
- **Likee**: getVideoInfo/postIds и videoUrl parser покрыты fixtures; текущий API подтвердил форму postIds validation, но успешный публичный post sample не найден. Profile pagination не реализована.
- **Tumblr**: NPF video blocks и HTML5 fixtures проходят; serialized JS initial state и все современные blog layouts не гарантируются.
- **Vine**: явный unsupported для закрытого исходного сервиса.
- **Generic**: direct HTTP media, binary sniff для octet-stream, HTML5 video/audio/source/track, JSON-LD VideoObject/AudioObject/ItemList и recognized embedded players. OpenGraph title/image не считается поддержкой скачивания. JavaScript runtime/network interception не эмулируются.
- Xvideos/Redtube присутствуют в исходной ParserGraph-карте, но dedicated handlers не входят в этот перечисленный этап и не реализованы.

Ключи Facebook `browser_native_{hd,sd}_url`/`playable_url{,_quality_hd}`, Likee `postIds`/getVideoInfo и Flickr video.getStreamInfo независимо найдены в установленном original App EXE (без Data): SHA25650e3ea9303549d1700d2908af6025f8c2ce5512c8f5a0682ccc1dedbde2cebbc, offsets0x9bb5078..0x9bb50d0,0x9bb7c65/0x9bb7c92,0x9bb6697. Наличие этих строк подтверждает схему оригинального parser, но не live работоспособность сайта.

## Credentials и ограничения доказательства

Client всегда `no_proxy`, пока proxy явно не передан. Пользовательский Auth.cookie_header посылается только исходному origin и не включается в Format.headers. На redirects он не переходит на другой origin. Для TikTok/NicoVideo отдельные анонимные server-issued delivery leases хранятся как ScopedCookie с Domain/Path/Secure/TTL; Core должен применять их заново к каждому CDN/segment target. Domain и root Path проверяются, Max-Age имеет приоритет над Expires. Cookie/header значения, signed URLs и server error bodies не выводятся в ошибки/тестовые логи.

Не проверялись пользовательские аккаунты, private/Premium/geo-restricted content, исчерпывающий набор codecs/bitrates/URI forms, subtitle delivery всех сервисов, macOS/Windows и native UI. BiliBili subtitles возвращаются в исходном JSON формате и требуют отдельного локального преобразования при экспорте в SRT/VTT. HLS/DASH master variants выбирает download engine, а не этот metadata catalog.

## Проверки

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/nezeronxer/.cache/videodock-rust-target cargo test -p videodock-media
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/nezeronxer/.cache/videodock-rust-target cargo test -p videodock-media --test catalog_online -- --ignored --nocapture --test-threads=2
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/nezeronxer/.cache/videodock-rust-target cargo test -p videodock-media --test catalog_collections -- --ignored --nocapture --test-threads=2
```

Live sample availability и коллекции меняются. Свежий результат нужно проверять перед release; fixture-only строка не становится online-pass после unit test.
