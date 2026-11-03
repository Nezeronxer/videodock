# Соответствие 4K Video Downloader+

Цель - полное соответствие обнаруженным пользовательским функциям на Linux, macOS и Windows. Эта таблица отмечает текущее состояние, не сокращает согласованный объём.

Исследован EXE оригинала, SHA-256 `50e3ea9303549d1700d2908af6025f8c2ce5512c8f5a0682ccc1dedbde2cebbc`. Найдены 18 валидных Qt bundles, 909 payloads, включая 214 QML; native registration callsites проверялись отдельно. Исходный файл не изменялся. Наблюдения ресурсов показывают заявленный интерфейс, но не доказывают поведение запущенной программы. Исходные исследовательские материалы остаются локально в `~/.local/share/videodock/research/full-parity/`.

| Область оригинала | Реализация | Проверено | Остаток |
|---|---|---|---|
| Главный список, семь вкладок, поиск, сортировка, выбор и контекстные действия | Qt/QML + Rust snapshot/action bridge | Qt bootstrap, offline core tests | Живой проход по рабочему столу |
| Smart Mode и ручной выбор форматов | Rust selection/planner, фильтры Qt | HTTP fixture, ограничения качества/кодеков/языков/VR | Полный набор источников и настоящий 4K/360 sample |
| Разбор списков и каналов | Rust catalog/YouTube adapter, выбор элементов, пауза разбора | Сохранение прогресса при паузе, локальный список; отдельные online samples | Полные коллекции каждого сервиса |
| Загрузка и обработка дорожек | Rust HTTP/Range, HLS/AES, DASH; FFmpeg с локальными путями | Реальные localhost media -> FFmpeg -> ffprobe; compatible multi-period DASH | Прочие DASH/HLS layouts, DRM не поддерживается |
| Очередь и история | SQLite, concurrency, pause/resume/retry/force/remove | Restart/resume и свежие signed URLs; неразрушающая публикация | Недетерминированные гонки и долгие стресс-прогоны |
| Субтитры и аудиодорожки | Выбор языков/дорожек, VTT/SRT/JSON/TTML, embedding | Parser tests и локальные media fixtures | Все форматы и переводы каждого сервиса |
| Подписки | Базовый список ID, периодический поиск нового, настройки | Synthetic channel: baseline/new IDs/restart без дублей | Живые каналы всех сервисов и длительный scheduler |
| Встроенный браузер и аккаунт | Новый профиль Qt WebEngine, cookie scope/expiry, авторизация YouTube | Qt native cookie callback, localhost expiry/replay/legacy tests | Реальный вход, liked/watch-later и protected content |
| Import/export | Проверка JSON, original-schema preferences/subscriptions | Атомарное отклонение опасных URL, round trip | Все варианты оригинальных экспортов |
| Медиаинструменты | Spleeter stems, DPDFNet speech, WPE dereverb | Реальные модели/demo, сохранение кадров и синхронизации, отмена | Другие CPU/OS, длинные и разнообразные пользовательские записи |
| Настройки и системная интеграция | Пять разделов, tray, autostart, sleep guard, proxy | Нормализация, отклонение ошибок, native path/cookie tests | Tray/уведомления/автозапуск и GUI на трёх ОС |
| Обновления | Проверка явно настроенного HTTPS источника | Ошибка без источника не скрывается | Собственный подписанный release/update channel |
| Локализация | Английский и русский | 279 строк Qt | Остальные языки оригинала |
| Сборка и установка | Rust/CXX-Qt build/install scripts | Linux debug build | Проверенные пакеты Linux/macOS/Windows, лицензии поставки, живой QA |

Поддержка отдельных сайтов и различие online/fixture-only описаны в [каталоге](../crates/media/tests/catalog/SUPPORT.md). Неподтверждённые Facebook/NaverTV/Likee/Tumblr, неполные коллекции Twitch/Rumble и отсутствующие dedicated handlers остаются работой по этой задаче. Закрытые код и ресурсы оригинала не включаются в продукт; собственное открытое приложение не использует его активацию.
