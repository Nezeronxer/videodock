# Проверка VideoDock0.2.2

Изменение: постоянный векторный значок ссылки20px на кнопке Paste Link. Логотипы сервисов у записей сохранены. Буфер больше не читается при активации окна. Runtime source b59043d, producer43254753785.

Обязательные проверки Rust, nativeQt EN/RU, настоящего Engine, packaged worker/clean PATH, упаковки и rollback прошли на Windows, Linux, macOS Intel и Apple Silicon. Дополнительные публичные сетевые тесты Windows/macOS завершились отказом; они не включены в PASS обязательных gates. Сетевые ограничения0.2.1 сохраняются.

Один Setup проверен на настоящих тестовых Windows10LTSC19044 и Windows11Enterprise26300: по два обновления поверх0.2.1, SQLite/WAL/SHM byte-identical до запуска, отзывчивое окно с чистым PATH. Setup SHA256 `c55a4a697fd7ecef64ece5d35c4fa107029c505ebde9fa991dc84035a6be81b3`; EXE `3bc44edbedac16cf1dc55026408c9129c7e5e959105ff33423aae69a2377fd2e`. Встроенный Rust manifest совпал с исходным.

Независимая установка готового Flatpak без SDK: verifier43272098606 SUCCESS, worker/proxy/startup и visible GUI прошли. Пакет связан с source/digest producer-артефакта; publisher проверяет эту связь перед выпуском.

Полные прежние исправления загрузок описаны в отчёте0.2.1. Для косметической правки на локальных Win10/11 повторялись установка/запуск/профиль; весь прежний набор полных интернет-загрузок повторно не заявляется.

Android создаётся отдельным портом в feature/android и не входит в готовые десктопные пакеты0.2.2.
