# Полные загрузки перед релизом, 05.11.2026

Свежий прогон текущего Rust Catalog → pipeline → готовый файл → ffprobe → полный FFmpeg decode с `-xerror`. Нет пользовательских cookies и аккаунтов. Один sample подтверждает только этот сценарий, а не все форматы и коллекции площадки.

| Источник | Файл, байт | Длительность, с | Pipeline, с | Маршрут | Результат |
|---|---:|---:|---:|---|---|
| BiliBili | 88071152 | 2055.638 | 21.148 | direct | Полный файл и decode PASS |
| BitChute | 5722657 | 160.840 | 3.067 | direct | Полный файл и decode PASS |
| Dailymotion | 2773602 | 25.472 | 1.485 | direct | Полный файл и decode PASS |
| Flickr | 458246 | 3.500 | 1.041 | direct | Полный файл и decode PASS |
| Generic | 4368946 | 52.208 | 2.401 | direct | Полный файл и decode PASS |
| NicoVideo | 15343146 | 320.087 | 49.022 | direct | Полный файл и decode PASS |
| Rumble | 7763365 | 30.070 | 2.630 | direct | Полный файл и decode PASS |
| SoundCloud | 5325937 | 213.902 | 9.216 | direct | Полный файл и decode PASS |
| Tumblr | 231794698 | 575.339 | 7.206 | direct | Полный файл и decode PASS |
| Twitch | 35523746 | 60.013 | 4.579 | direct | Полный файл и decode PASS |
| TikTok | 3308460 | 24.402 | 0.455 | SOCKS | Полный файл и decode PASS |

## Незакрытые источники

| Источник | Проверка 05.11 | Результат |
|---|---|---|
| YouTube | Windows 10/11 и Linux Flatpak: настоящие загрузки, файлы и decode; Windows 11 — 1080p, 634,624 с | PASS для описанных в Windows QA сценариев; это отдельный native-прогон, не catalog_downloads |
| Vimeo | 76979871: metadata получены, pipeline отверг шифрование HLS. Дополнительный публичный 33951933 проигрывается в Chrome, но config возвращает HTTP403; UA/Referer и SOCKS также403 | FAIL полного скачивания |
| Facebook | Публичный CNN sample: HTML получен, parser не получил playback URL; независимый HTTP200 ответ не содержит известных native media fields | FAIL; вход и современная схема не подтверждены |
| Likee | Открытый профиль возвращает `This page is not available in your area`; Catalog не получает post id/список видео | FAIL профильного сценария; одиночный доступный post не подтверждён |
| NaverTV | Официальная tv.naver.com сообщает о закрытии30.09.2026 и переходе в Clip; старый sample не предоставляет playback ticket | Старый сервис закрыт; новый Clip не реализован/не проверен |
| Vine | Ранее реализован явный отказ закрытого сервиса | Файл скачать нельзя; не считать PASS |
| Xvideos / Redtube | Ранее browser extension блокировал страницы; свежей полной загрузки нет | NOT VERIFIED; прежний запрет не обходился |

TikTok напрямую завершился сетевой ошибкой; через уже существующий SOCKS получен и декодирован полный файл. Dailymotion в этом проходе прошёл напрямую, но прежние нестабильные попытки не отменены. Скорость — разовый замер, без гарантии стабильности любого соединения.

Итого свежего catalog-прогона: 10 PASS и3 FAIL из13 tests (TikTok и два Vimeo). Повтор TikTok через SOCKS:1 PASS. Дополнительные Facebook/Likee-profile/NaverTV:3 FAIL. Успешных свежих источников с decode —11, включая Generic. Матрица всех18 именованных сервисов оригинала остаётся незакрытой. Полный stable release пока не готов.

Первичные локальные доказательства: `~/.local/share/videodock/research/services-release-2026-11-05/`: direct.log, tiktok-proxy.log, extra-direct.log, files-proof.json, direct/ и proxy/.

Первоисточники: [уведомление NaverTV](https://tv.naver.com/), [публичный Vimeo sample](https://vimeo.com/33951933), [прогон Windows10/11](windows-qa-2026-11-05.md), [Linux Flatpak](linux-flatpak-qa-2026-11-05.md).
