# Проверка VideoDock 0.2.0 на всех ОС - 07.11.2026

Проверены существующее приложение, обновление установки и готовые пакеты. Сборки выполнены на нативных GitHub Actions runners. На личном Mac запускался готовый ARM-пакет и лёгкие регрессии; локальные ВМ и тяжёлые сборки не использовались. Полное соответствие всем функциям оригинала не заявляется.

## Исправленные ошибки

| Ошибка | ОС | Изменение и проверка |
|---|---|---|
| Сбой подготовки портил прежнюю установку | Linux, macOS | Свежий staging, backup и rollback; семь проверок отказов и packaging contract прошли |
| Устаревшие DLL оставались в новом пакете | Windows | Новый staging перед Qt deployment; нативный PowerShell fixture проверил stale DLL и rollback |
| Аудиоязыки терялись при переносе истории | Все | Необязательное валидируемое audioTracks в экспорте/импорте; legacy-поведение сохранено, Rust-регрессии прошли |
| Субтитры не совпадали с именем повторной загрузки | Все | Sidecar получает фактический stem media, например clip (1).en.vtt; файловая регрессия прошла |
| Ошибка sidecar оставляла неучтённый готовый файл | Все | Guard откатывает только outputs текущей попытки; прежние файлы и staging сохраняются |
| Unicode-пути ошибочно считались слишком длинными | Windows | Счёт UTF-16 units вместо encoded bytes; нативная Windows-регрессия прошла |
| Open/xdg-open оставляли zombie-процессы | Linux, macOS | Неблокирующий waiter освобождает Child; RED/GREEN на фактическом helper и нативные тесты |
| Checkbox обращался к отсутствующему modelData | Все | Защита binding в ManualDialog; RED воспроизводит TypeError, GREEN восстанавливает checkbox |
| Щелчок по сфокусированному адресу не раскрывал браузер | macOS; общий QML | Passive TapHandler сохраняет выделение текста и раскрывает панель; Cocoa RED/GREEN и живой готовый пакет |
| Repair подписи менял identifier вложенной библиотеки | macOS | Вместе с requirements сохраняется identifier; небольшой Mach-O bundle RED/GREEN, затем обе полные поставки прошли deep verification |

Изменения проверялись отдельно от автора: файловые коллизии/rollback, границы импорта, Qt controls и реальные установочные сценарии. Ошибки окружения и тестов исправлены отдельно: deployment тестового Windows EXE, Qt translations/Retina/focus/hover и настройки desktop portal на headless runner. Assertions продукта и sandbox не отключались.

## Результаты готовых пакетов

| Платформа | Прогон | Фактический результат |
|---|---|---|
| Windows x64 | [42385130933](https://github.com/Nezeronxer/videodock/actions/runs/42385130933) | Workspace, staging, installer, worker с чистым PATH и UI EN/RU PASS; 130 PNG. Общий job FAIL только из-за public YouTube/Shorts |
| Arch Linux x86_64 | [42394901843](https://github.com/Nezeronxer/videodock/actions/runs/42394901843) | Native job SUCCESS: workspace, UI, worker, startup и архив |
| Flatpak x86_64 | [producer 42394901843](https://github.com/Nezeronxer/videodock/actions/runs/42394901843), [verification 42403231286](https://github.com/Nezeronxer/videodock/actions/runs/42403231286) | SDK build/package/install PASS. Тот же пакет отдельно прошёл ffmpeg/ffprobe, IPC (8 невалидных запросов), выбранный proxy, startup и собственное видимое окно |
| macOS Intel и ARM | [42395354408](https://github.com/Nezeronxer/videodock/actions/runs/42395354408) | Workspace, UI EN/RU (по 130 PNG), bundle, подпись, worker с чистым PATH и startup PASS. Общие jobs FAIL только из-за public YouTube/Shorts |
| RustSec | [42382195905](https://github.com/Nezeronxer/videodock/actions/runs/42382195905) | 0 известных vulnerabilities; 1 unmaintained warning: fxhash0.2.1, RUSTSEC-2025-0057 |

Производящие исходники: Windows61cb39e, Linux877891a, macOS427f9ac. Runtime исходники и соответствующие определения producer jobs сверены с выпуском; последующие изменения касаются проверок, документации и публикации.

Windows portable: 1942 ZIP entries, CRC без ошибок, Qt/MSVC/FFmpeg и лицензии присутствуют, пользовательских SQLite/profile нет. Setup не имеет Authenticode certificate table. Mac-архивы проверены локально по SHA256 и codesign --verify --deep --strict; Intel executable на ARM не запускался. Arch tar содержит413 записей, без выходящих путей и ссылок. Размеры и SHA256 всех скачанных Actions artifacts проверены до извлечения.

## Подтверждение Flatpak

Первичный установленный worker зависал до первого подключения к proxy. Проверки темы и offscreen/xcb не устранили отказ. Символизированный [stack 42400025216](https://github.com/Nezeronxer/videodock/actions/runs/42400025216) показал ожидание QDBusPendingCallPrivate::waitForFinished в QXdgDesktopPortalTheme constructor при создании QApplication. WebEngine ещё не запускался.

В CI настроены GTK portal backend, activation environment и ReadAll readiness preflight. Неизменённый пакет после этого прошёл proxy за прежние 15 секунд; код приложения и sandbox не менялись. Проверка GUI дополнительно связывает видимое окно с новым деревом Popen и namespace PID. Глобально несколько Flatpak instances допустимы; в итоговом подтверждении4 экземпляра, из них один относится к тестовому запуску; одно видимое окно и совпавший PID 2.

Финальный verification artifact13585988657 имеет digest sha256:8f1007ecfe1ccf0e5973bfdcb1e47d40ef85fb7d73f00c0ba207ef00696ab835. Его verification.json связывает producer42394901843/source877891a/artifact13583473781 и пакет SHA256069cfc38e43638f797525af209b345b2f1fcdf6400c13747978a97d788128d42; worker_proxy,worker_startup,gui_visible равны true. Хеш совпал с локальным файлом выпуска. Повторной компиляции успешно собранного приложения не было.

## Живая проверка Mac ARM

В отдельном QA-профиле открыты русский UI, обучение и настройки. После выбора English и явного перезапуска с тем же профилем сохранились язык и завершённое обучение. Настоящее системное File menu содержит directory/import/export. Hide Browser оставляет фокус на адресе; повторное нажатие снова раскрывает панель. Скриншот осмотрен, тестовый процесс завершён. Это не полная загрузка через GUI на личном Mac.

## Сеть и границы проверки

Windows и обе Mac-архитектуры полностью скачали и декодировали public TikTok: 6 947 649 байт / 148.886 секунды. YouTube/Shorts вернули контролируемую metadata-ошибку подтверждения доступа. Windows17.01/16.89s, Mac ARM27.70/20.48s, Intel27.85/28.30s. Это не PASS анонимного скачивания или SABR на этих IP. Ранние успешные YouTube samples на других маршрутах описаны отдельно в отчёте04.11.

Проверены конкретные CI окружения: hosted Windows x64, macOS 15 Intel/ARM, Arch container и установленный Flatpak на Ubuntu 24.04 / KDE runtime 6.11. Ручная матрица всех Windows10/11, Linux-дистрибутивов, старых Mac и оборудования не повторялась. Arch-архив требует системных зависимостей. Mac использует собственную подпись без Apple Developer ID/notarization; Windows installer без доверенной Authenticode-подписи. Остальные сервисные и REA-пробелы остаются в матрице соответствия.

## Публикация и проверяемость

Выпуск содержит6 пакетов, build-manifest.json и SHA256SUMS. Publisher проверяет runtime/job equivalence, обязательные native gates, успешный отдельный Flatpak verifier, producer/artifact identity и фактический package SHA. Единственный первичный Flatpak verification failure принимается только с новым подтверждением того же файла; прочие failures/cancel/timeout отклоняются. Из manifest видна вся цепочка producer → verification → package.

Проверены34 отказных mock-сценария и 6 прежних tag/draft сценариев. Publisher не исполняет скачанные файлы, не удаляет и не перезаписывает чужие assets; перед публикацией сверяет server-side digests, tag и состояние draft. [Файлы выпуска 0.2.0](https://github.com/Nezeronxer/videodock/releases/tag/v0.2.0). Старый непроверенный installer оставлен в отдельном private draft, в выпуск не входит.
