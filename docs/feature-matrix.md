# Соответствие 4K Video Downloader+

Цель - соответствие обнаруженным пользовательским функциям на Linux, macOS и Windows с согласованными изменениями. По явному решению пользователя ИИ-медиаинструменты удалены целиком. Эта таблица отмечает текущее состояние и оставшиеся проверки.

Исследован EXE оригинала, SHA-256 `50e3ea9303549d1700d2908af6025f8c2ce5512c8f5a0682ccc1dedbde2cebbc`. Найдены 18 валидных Qt bundles, 909 payloads, включая 214 QML; native registration callsites проверялись отдельно. Исходный файл не изменялся. Наблюдения ресурсов показывают заявленный интерфейс, но не доказывают поведение запущенной программы. Исходные исследовательские материалы остаются локально в `~/.local/share/videodock/research/full-parity/`.

| Область оригинала | Реализация | Проверено | Остаток |
|---|---|---|---|
| Главный список, шесть вкладок, поиск, сортировка, выбор и контекстные действия | Qt/QML + Rust snapshot/action bridge | Qt bootstrap, offline core tests; живые File export/import, список и сведения; меню без пропусков, native pointing cursor | Прочие пользовательские состояния и три ОС |
| Smart Mode и ручной выбор форматов | Rust selection/planner, фильтры Qt; раздельные 2D/360°, 3D/180°, 3D/360° и обычное видео | Точный каталог 71/72 языков из runtime оригинала; сохранение VR и запрет подмены проекции | Полный набор источников и настоящий 4K/360 sample |
| Разбор списков и каналов | Rust catalog/YouTube adapter, выбор элементов, пауза разбора | Сохранение прогресса при паузе, локальный список; отдельные online samples | Полные коллекции каждого сервиса |
| Загрузка и обработка дорожек | Rust HTTP/Range, HLS/AES, DASH; FFmpeg с локальными путями | Реальные localhost media -> FFmpeg -> ffprobe; compatible multi-period DASH | Прочие DASH/HLS layouts, DRM не поддерживается |
| Очередь и история | SQLite, concurrency, pause/resume/retry/force/remove | Restart/resume и свежие signed URLs; неразрушающая публикация | Недетерминированные гонки и долгие стресс-прогоны |
| Субтитры и аудиодорожки | Выбор языков/дорожек, VTT/SRT/JSON/TTML, embedding; все выбранные аудиодорожки в совместимых контейнерах | 16 тестов выбора и 5 реальных localhost/FFmpeg/ffprobe сценариев нескольких дорожек, original/default/all и контейнеров | Все форматы и переводы каждого сервиса |
| Подписки | Базовый список ID, периодический поиск нового, настройки | Synthetic channel: baseline/new IDs/restart без дублей | Живые каналы всех сервисов и длительный scheduler |
| Встроенный браузер и аккаунт | Новый профиль Qt WebEngine, cookie scope/expiry, авторизация YouTube | Qt native cookie callback, localhost expiry/replay/legacy tests | Реальный вход, liked/watch-later и protected content |
| Import/export | JSON original-schema; legacy Downloads Version 1 CSV через CSV/TXT/4KV; экспорт обычного видео как dimension=2D | 11 legacy-тестов, UTF-8/16/32 BOM, кавычки/разделители, атомарное отклонение; VR round trip | Динамический импорт оригинала закрыт его Pro gate; старые подписки и прочие форматы не подтверждены |
| Настройки и системная интеграция | Пять разделов, tray, autostart, sleep guard, proxy | Нормализация, отклонение ошибок, native path/cookie tests | Tray/уведомления/автозапуск и GUI на трёх ОС |
| Точность UI и тёмная тема | Общая палитра; Вид → Оформление; иконки; размеры toolbar/списка/настроек и режимы браузера | Живые парные dark-снимки Linux 1703×1336: пустой экран и General; Qt RU/EN и вариант без локального appearance pack; [подробности](ui-parity.md) | Полный парный проход всех состояний и пиксельное совпадение остаются открытыми |
| Обновления | Проверка явно настроенного HTTPS источника | Ошибка без источника не скрывается | Собственный подписанный release/update channel |
| Локализация | Английский и русский | 302 завершённых перевода Qt, EN/RU appearance после удаления ИИ | Остальные языки оригинала |
| Сборка и установка | Rust/CXX-Qt build/install scripts | Linux debug build | Проверенные пакеты Linux/macOS/Windows, лицензии поставки, живой QA |

Поддержка отдельных сайтов и различие online/fixture-only описаны в [каталоге](../crates/media/tests/catalog/SUPPORT.md). Неподтверждённые Facebook/NaverTV/Likee, неполные коллекции Twitch/Rumble и отсутствующие dedicated handlers остаются работой по этой задаче. Закрытые код и ресурсы оригинала не включаются в продукт; собственное открытое приложение не использует его активацию.

Матрица полного скачивания и ограничений до исправления YouTube: [stability-2026-11-04.md](stability-2026-11-04.md). Успешный одиночный Tumblr sample не закрывает blog pagination. После добавления анонимного proof/SABR длинные YouTube samples 720p60 и 1080p60 прошли полное декодирование; текущие результаты и оставшиеся native gates - в [проверке YouTube](youtube-delivery-2026-11-04.md).

Свежий проход05.11: [полные загрузки и decode по всем проверенным сервисам](services-release-2026-11-05.md), [Windows10/11](windows-qa-2026-11-05.md), [Linux Flatpak](linux-flatpak-qa-2026-11-05.md). Vimeo/Facebook/Likee остаются незакрытыми, NaverTV завершил работу и перешёл в новый Clip. Полный stable release пока заблокирован.
