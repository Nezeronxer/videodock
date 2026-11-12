# Бесплатный VideoDock с полным соответствием 4K Video Downloader+

Постановка от03.11.2026: полная копия обнаруженных функций оригинала для Linux, macOS и Windows. Получение ссылок/форматов, скачивание и логика приложения - Rust. Интерфейс - Qt/QML. Python и yt-dlp в целевом runtime запрещены; FFmpeg допустим только для обработки уже полученных локальных данных.

Все функции нашей программы полностью бесплатны: Pro-тарифов, ключей активации, платных блокировок и покупок нет. Уточнение пользователя исключает лицензионные/платёжные экраны оригинала из цели копирования; ограничения оригинала учитываются только как границы исследования.

Готовность: полное совпадение1:1 интерфейса и наблюдаемого поведения, включая геометрию, шрифты, графику, анимации, состояния, все настройки и сервисы. Все обнаруженные экраны и сценарии должны быть перечислены в матрице, реализованы и проверены по отдельности. Неизвестное не считается готовым; статический разбор не заменяет выполнение оригинала. Матрица: [docs/feature-matrix.md](docs/feature-matrix.md). Оригинал и его Data/аккаунты/активация не изменяются.

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

### Продолжение 03.11: точность интерфейса и тёмная тема

Результат: сверить UI с ресурсами оригинала, воспроизвести размеры и расположение собственными компонентами; завершить тёмную тему на всех поверхностях. Объём - Qt frontend и сохраняемая настройка оформления. Проверка - Rust persistence/validation tests, Qt runtime с реальными QML, скриншоты dark/light и узкого окна, сборка приложения. Живой desktop-проход требует доступного CUA.

- [x] Восстановить контекст, текущее дерево и REA evidence; повторный холодный Ghidra import не нужен.
- [x] Сверить исходные размеры toolbar/settings и dark palette; CUA вернул пустой список приложений и unavailable host.
- [x] Общая палитра, сохранение выбора темы и применение без перезапуска ко всем компонентам.
- [x] Геометрия toolbar/Smart Mode/list header/settings и собственные векторные иконки.
- [x] Qt regression harness: темы, меню, все диалоги, ограниченная ширина и рендеры; также проверены выбор 720p до dispatch, Ctrl+click и режимы браузера.
- [x] Rust tests:89 passed/0 failed; clippy/build и изолированный запуск бинаря со встроенными QML/QRC прошли. Подробности и ограничения: [docs/ui-parity.md](docs/ui-parity.md).
- [ ] Парное живое сравнение оригинала и VideoDock на рабочем столе; пиксельное совпадение всей программы пока не доказано. Установленный Python-прототип этим проходом не заменяется.

### Дополнительная проверка по запросу «проверяй»

Результат - проверить реальные действия и устранить воспроизводимые дефекты, оставив отдельным непроверенным пунктом живой desktop. Объём - Qt-сценарии с настоящим Rust Engine в изолированном профиле, без пользовательских аккаунтов. Проверка - ошибки/настройки/restart, локальная media-загрузка до ffprobe, действия меню и фильтров.

- [x] Повторно проверить CUA: apps=[]/browsers=[], browser access host unavailable.
- [x] Проверить фильтры форматов и совпадение команд QML/core: воспроизведены и исправлены регистр MP4/mp4, неверное отображение лимита скорости и двойное подтверждение удаления с пустыми ID. Опасное удаление перепроверено свежим независимым агентом, exit0.
- [x] Подключить тестовый Qt frontend к настоящему Engine; проверены настройки, ошибки host/port и сохранение темы после перезапуска.
- [x] Локальная UI-загрузка -> Rust HTTP -> готовый MP4 -> ffprobe: video+audio; история пережила restart. Контекстное копирование ссылки и безопасное удаление истории/отмена прошли.
- [x] Повторены Qt-сценарии с реальным Engine, 6 Rust persistence/validation tests, clippy и сборка; доказательства в `~/.local/share/videodock/research/ui-verification-2026-11-03/`. Живой desktop по-прежнему недоступен, полное 1:1 не доказано.

- [ ] Закрыть все строки матрицы сервисов: неподтверждённые Facebook/NaverTV/Likee/Tumblr, недостающие dedicated handlers, полные коллекции и авторизованные сценарии.
- [ ] Полные4K/VR samples, разнообразные реальные manifests/subtitles/audio tracks, длительная подписка и stress/race.
- [ ] Все языки оригинала, собственный подписанный update/release channel и прочие отмеченные пробелы интерфейса.
- [ ] Полная desktop-проверка Qt-сценариев. CUA восстановлен отдельной задачей: на физическом рабочем столе через штатный Linux Sky проверены AT-SPI, открытие/закрытие Preferences, снимок окна и Quit на отдельном xcb-процессе VideoDock. Остальным сценариям и парному сравнению с оригиналом ещё нужен живой проход; старый Codex TUI подхватит новый MCP после перезапуска.
- [ ] Реальные сборки/установка/QA macOS и Windows, полный deployment Qt/FFmpeg/лицензий, Rust installer на всех ОС.
- [ ] Завершить миграцию установленного Python-прототипа и once jobs только после готовой Rust установки. Shared jobs linux/darwin/2026-11-03-videodock.sh пока явно блокируют Python установку и требуют замены.
- [ ] Финальные проверки соответствия и релиз; не объявлять полную копию по проходящим базовым тестам.

## Продолжение REA: живой desktop (03.11, текущий проход)

Результат - закрыть доступные живые UI-сценарии Rust-приложения и сопоставить видимые экраны с оригиналом. Объём - отдельный профиль, desktop CUA; без аккаунтов и пользовательских загрузок. Проверка - настоящие AX/screenshot, действия и состояние после перезапуска, затем профильные regression checks для найденных дефектов.

- [x] Восстановлены TASK, прошлый диалог и ограничения; CUA видит X11 приложения, REA отвечает.
- [x] Отдельный Rust-профиль: пять шагов onboarding, настройки, light/dark, меню и сохранение темы после штатного restart.
- [x] Оригинал запущен в отдельном Wine prefix; screenshot обучения сопоставлен с ресурсами. CUA видит изображение, но Wine не реагирует на его фоновые клики/клавиши; полное парное сравнение остаётся открытым.
- [x] Обучение сделано полноразмерным экраном с карточкой611px и кнопками100×40; исправлена потеря зелёного accent в Basic style. Локализованы значения Smart Mode/настроек и тип/статус в сведениях; protocol values сохранены. Red/green воспроизводят старые дефекты.
- [x] Финальная сборка проверена CUA: dark onboarding, translated manual filters, browser→manual→Cancel, сведения, сохранённая история. Встроенный профиль изолирован, Qt runtime без предупреждений.
- [x] Свежие проверки:89 Rust tests/0failed (21 ignored), RU/EN Qt+real Engine, desktop build, Clippy -D warnings, git diff --check. Loopback download: H.264+AAC по ffprobe. Доказательства `~/.local/share/videodock/research/live-ui-2026-11-03/`; закрыты только перечисленные сценарии, остальные строки матрицы открыты.

## Пути и ограничения

- Исследование: `~/.local/share/videodock/research/full-parity/{original-features.md,evidence/,native/native-findings.md,rea-static-inputs.json}`. Ghidra whole-program import не завершился за20мин, snapshot пустой; псевдокода не получено. Повторный холодный import без новой причины не запускать.
- Сборки: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$HOME/.cache/videodock-rust-target`. Нельзя хранить target/SQLite/models/browser в Dropbox.
- Cargo TLS index/archive resets обходились IPv4 загрузкой из официальных crates.io endpoints с SHA256 из Cargo.lock. `/tmp` и home находятся на разных FS: для перемещения проверенного archive нужен copy, не rename.
- Qt WebEngine/Positioning/WebChannel уже установлены. Custom pacman DB отсутствует; изолированный config только official repos с `SigLevel=Required DatabaseOptional` позволяет read-only download signed packages. Системный pacman.conf не менялся.
- Временный код HTTP diagnostics удалён. Исходные пользовательские профили и секреты не использовались. Старые статические сведения и тесты не являются доказательством запуска на macOS/Windows.

## Полное1:1 - продолжение по уточнённому критерию

Результат - устранить все установленные расхождения с оригиналом, а не заменять точность общим прохождением тестов. Объём - оригинал26.3.5.0396 и Rust-приложение, отдельные профили. Проверка - пара экранов одинакового размера/темы/языка, измерения цветов/шрифтов/размеров, реальные действия и сохранённое состояние.

- [x] Native Linux original26.3.5.0396 извлечён из cachedofficialtar; SHA256 dd430691749453b660659b30a5070592062ff18225da50b200b7ea83f707ed61 совпал сPKGBUILD. ОтдельныеXDG config/data/cache; CUAкликиработают.
- [ ] Зафиксировать reference-экраны и список состояний; статически сверить toolbar/manual/list и settings/tools/transfer.
- [ ] Исправлять по reference, проверять каждое расхождение, вести точную матрицу.
- [ ] Пройти все доступные пользовательские сценарии; не объявлять1:1 без сравнения.

Текущая интеграция строгогоUI: SmartModeBar.qml (one-line controls, nested menus, OS aliases, path history, language catalog), SettingsDialog.qml (sidebar/top-close/ListSlider/proxy editor), ManualDialog.qml (128/88px header/footer, table/language popups/shared output preview), JobList.qml (thumbnail controls/continuous selection/doubleclickplay). Core:Never/0+manualrefresh,10nativeOSmappings,video/audio containers,MRUpaths,sharedpreview,defaulttitleбезID. Это ещё не готовое1:1.

Native reference: `~/.local/share/videodock/research/exact-ui-2026-11-03/reference/`;originallightmanualдляjNQXAC9IVRwснят,разборпрошёлбезаккаунтов/скачивания. Локальныйappearance.rcc483изображенияизисследуемойкопии: `~/.local/share/videodock/appearance.rcc`;manifest+SHAвexact-ui,пакетнеявляетсяоткрытымкодомпроекта. RustQtзагружаеттолькокартинкипоbasename.

Проверки перед продолжением: roottoolbar-red воспроизвёлstackedcontrols; integration-2 ENдоasset/previewпрошёлQt+realEngine. Свежийwith-assetsпрогонупалнастарассертеAny:containerтеперьправильноинициализируетсясохранённымMP4;assertисправленявнымвыборомAny. BindingloopimplicitHeightименюбезшириныпойманыреальнымиpointertests,исправлены;нуженповтор. Nativeбелаяподложка#f6f6f6,menubar30pxизмерены. Сейчасlupdate/RUtranslations,previewfrontendиобщаярегрессияещёвинтеграции.

Продолжение20:xx: sourceпакет483PNG/GIF/SVGреальнопрочитанRustQtчерезglobalGenericData/videodock/appearance.rcc,sourceкодбриджаиграфиканеисполняютQMLизпакета. Scopedsecurityreviewреальнымиprobe:0выходовизprefix,0SVGexternal/scriptrefs,slash/backslashreject,регистрациянеподменяетранееcompiledresources. НативныйRustпроцессstrict-profileзапущенсEN/light,скринstrict-desktop-light-empty.png;живойпрогонвыявилtruncatedchoicevaluesприширокомокне—увеличенточныйtitle/value/indicatorlayoutдо6+4+20px. Down/Spaceпоменяныпосourceнараскрытиеменью.

BrowserPaneисправляетсяпоживому/source:убранReload,header16/8/8margins и28pxnavbuttons+24pxgaps,исходныелоготипырисуютсявnaturalSVGразмерах(ониразные,40pxломалNaver/YT). Download210×56floatingbottom16,hide36×15topcenter,минимизированныйplaceholderиSearch/Go;кнопкавышеполяудалена. Qt5Compat.GraphicalEffectsужеестьлокально,reuseDropShadowпараметровsource. Нужнособрать/повторитьQt+CUAпослеэтойревизии.

Originalread-onlyruntime-модели:стараяREAFridaошибкаmissingbindingпочиненавустановленнойdependencyбезапдейтаверсий;живойREAserverдержитcachedfailure. Nativeattachзапрещёнptrace_scope1(неизменён);agentcoreпробуетownedspawnновогоизолированногоoriginal. OriginalCUAэксклюзивзаagentcoreдоегоуведомления. NativeязыковаяменюreferenceпоказываетNone,Default,первые5избранных,затемnativeNames;ISOкаталог183ещёнеточноесоответствиеruntimeпорядку.

## Уточнение пользователя: полностью бесплатно

- В20:xx пользовательподтвердил:Pro-версияненужна,всянаша программабесплатна;следующее«прожлди»означаетпродолжитьэтужецель. Не переноситьoriginalUpgrade/Activation/Prolocks.
- ВчистомoriginalFile→ImportDownloadsпоказалProgate (original-import-license-gate.png); этоНЕпричинатребоватьProдлянашейпрограммы,атолькопределдинамическойверификации. AgentsettingsнашёлнеstrippedELFсимволы importMain/importItemsOld/DownloadsSerializer::csvDeserialize/Base::CsvSerializer;продолжаетreadonlyasmбезизмененияoriginalactivation.
- NativeoptionsреальносчитаныFrida:71subtitles/72audioTracks;FPSHighest,240,120,60,50,30,25;qualityBest/UHD8K/HD4K/HQ2K/.../144p;VRAny/2D360/3D180/3D360/None;битрейтHighest/320/256/128/64. RootLanguageOptions.jsужесодержитточные71/72,неISO183. RootSmartBarlabelmapping/enumchoicesещёподключаются.
- CoreagentпишетtypedVR сlegacyboolcompat;Agenttoolbarпишетpipelinemulti-audiomap/compatibilityреальнымиFFmpegtests;Agentsettingsпассивныеlegacyimportparsers. Этоактивныепараллельныеизменения,неперезаписыватьихфайлыбезкоординации.

## Возобновление REA 03.11, после 21:40

Результат - интегрировать накопленные изменения UI, VR, аудиодорожек и импорта; устранить подтверждённые расхождения. Объём - текущая ветка, отдельные тестовые профили, без изменений рабочего стола и его блюра. Проверка - workspace, Qt с настоящим Engine на RU/EN, реальные desktop-действия и сопоставление с сохранёнными reference.

Уточнение пользователя: тестировать на обычном отдельном пустом рабочем столе, где нет его приложений. `special` не использовать. Предыдущие тестовые окна закрыты; для следующих запусков выделен свободный стол 90, без переключения экрана пользователя. GUI-регрессии продолжаются в изолированном Xvfb.

- [x] Восстановлены последний диалог, TASK и рабочее дерево; прежние дочерние агенты завершены, активных нет.
- [x] Повторить Rust workspace и Qt integration после всех накопленных изменений: 136 passed/0 failed/21 ignored; RU с pack и EN без pack, настоящий Engine/ffprobe/history.
- [x] Восстановлен доступный выбор темы через View → Appearance; дополнены 26 переводов, всего 319. Исправлены размещение пустого заголовка, навигационные иконки/поиск и измеренные dark-цвета настроек. Red/green воспроизводят дефекты; тестовый runner исправлен для прокрутки виртуализированного меню, resize и отдельной папки state вместо HOME.
- [x] Исправлена совместимость JSON-экспорта: Flat сохраняется как `2D` оригинала, внутренний/UI `None` остаётся прежним; round-trip и legacy regressions прошли.
- [x] Собран встроенный Qt frontend; реальный CUA: выбор dark, штатный Quit/restart, сохранение темы, парные снимки пустого экрана и General. Только отдельный обычный стол 90, запуск сразу через `workspace="90 silent"`.
- [x] Обновлены матрица и документация. Полное пиксельное совпадение, оставшиеся сервисы/аккаунты и три ОС не объявляются завершёнными.

Доказательства этого продолжения: `~/.local/share/videodock/research/continued-2026-11-03/`. Сравнение регионов главного экрана сохраняет ненулевые различия, поэтому равенство 1:1 пока не подтверждено. Работа на обычном столе 90 проверена через список клиентов и активный монитор; special-стол не использовать.

## Проверка платформ и дальнейшего 1:1

Результат - получить реальные сборочные и runtime-доказательства на доступных ОС, устранять подтверждённые расхождения. Объём - VideoDock Rust/Qt и отдельные профили; Windows только в отдельной копии образа, Linux UI только обычный пустой стол 90. Проверка - нативный build, отсутствие недостающих библиотек, запуск, локальная загрузка/ffprobe и сохранение состояния; для 1:1 - пары одинаковых экранов.

- [x] Инвентаризация: Git remote/CI отсутствует; Tailscale на Arch не авторизован, прямого Mac SSH нет; есть выключенные libvirt Windows и физический образ win10.qcow2. Старые пути TPL-Win10 больше не существуют.
- Mac включён, но пользователь сейчас не может открыть новую сессию. Готовая проверка `claude-shared/once/darwin/2026-11-03-videodock-native-verify.sh` подхватится существующим SessionStart; отчёт ожидается в `.verification/darwin.json`. Синтаксис и запрет устаревшего source digest проверены локально. Это пока очередь, не подтверждение работы на Mac; больше действий от пользователя в этом проходе не запрашивать.
- [ ] Подготовить отдельное Windows-окружение без изменений учебного образа и выяснить доступные инструменты сборки.
- [x] Подготовлена копия Windows 10 LTSC 19044, домен `videodock-platform-qa`, 4 vCPU/6 GiB; исходная win10 остаётся выключенной и не изменялась. Консоль только на столе 90. Старый агент 0.12.1 не имел exec/file RPC; заменён в копии на официальный 110.2.3 с проверкой SHA-256. Проверены guest-ping, process exec, передача архива и выход в сеть.
- [ ] Нативная Windows-проверка: исходники переданы с SHA-256; bootstrap проверен собственным PowerShell Parser. Установка изолированных MSVC/Rust/Qt/FFmpeg и release build выполняются внутри QA-копии; не считать запуск самой Windows проверкой VideoDock.
- В 22:41:08 MSK хост перезагрузился (`uptime -s`, новый boot ID); в 22:44:44 поднялась служба libvirt. Windows-проверка остановилась на Visual Studio; постоянный диск и артефакты сохранились. Возобновлять тот же QA-домен и проверять кэш установщиков, не создавать новую копию и не трогать исходную win10. После возобновления время QA-гостя синхронизировано штатным guest-set-time.
- [ ] Исправить и проверить установку/упаковку Qt, FFmpeg и ресурсов на macOS/Windows.
- [ ] Получить доступные нативные build/runtime результаты; недоступный Mac обозначить отдельным требованием доступа, не заменять Linux-тестом.
- [ ] Продолжить парное сравнение состояний и записать оставшиеся конкретные расхождения.

Текущие подтверждённые изменения: окно сохраняет размер в Rust settings и восстанавливает его один раз при старте. На настоящем окне изменены 1420×960, выполнен Quit, размер проверен в SQLite и новом окне через CUA (`geometry-proof.json`). Workspace: 137 passed/0 failed/21 ignored; Qt+Engine EN и Clippy прошли. Новый тест соседних FFmpeg/ffprobe предназначен только для macOS/Windows и на Linux не считается пройденным.

macOS installer теперь собирает текущие исходники вместо старого существующего binary, умеет staging через `VIDEODOCK_MAC_APP`, включает FFmpeg/ffprobe и проверяет ссылки otool. Linux mock-contract воспроизвёл старую ошибку и прошёл после исправления; это не нативная проверка Mac. Queued Mac job дополнен сборкой `.app` в cache и запуском её media tools с системным PATH.

Windows: MSVC 17.14.44, Rust 1.99.1, Qt 6.8.3 установлены в отдельной QA-копии. При первых реальных Cargo build обнаружены недостающие Git и NASM; это подготовка build environment, не успешная проверка приложения. Актуальный архив исходников v2 SHA-256: `ae3eef6f9bb121afa099187afea624571b781f1259a5aefdc23c1d02e0ee6644`.

CUA для сверки UI работает в отдельном Xvfb :99 через тот же настроенный public CUA MCP и скрипт `codex/verification/cua_live.py` (локальная копия меняет только каталог артефактов). Физический монитор не используется. Экспериментальный headless output VIDEODOCK-QA удалён, физические workspace/геометрия проверены (`output-cleanup.json`).

## Реальные загрузки, скорость и прокси - продолжение 03.11

Результат - проверить реальные публичные ролики через Rust до готового файла, измерить разбор/передачу и устранить подтверждённые задержки; проверить HTTP/SOCKS proxy и все заявленные сервисы. Объём - текущие adapters, queue, transfer и UI на отдельном профиле, без пользовательских аккаунтов и изменения системной сети. Проверка - opt-in online integration, ffprobe, proxy request counters, повторные замеры и desktop CUA.

- [x] Восстановлен актуальный статус и dirty tree; прежний catalog smoke проверял только 64KiB, это не полная загрузка.
- [ ] YouTube: реальные короткий/длинный/HD samples, раздельные замеры metadata/download/mux и повтор после исправлений.
- [ ] Полные публичные загрузки каталогов и честная матрица неподтверждённых сервисов.
- [ ] HTTP CONNECT/SOCKS5, credentials, fail-closed и сквозное применение proxy.
- [ ] Живой Qt download на отдельном обычном workspace90 silent; настройки proxy/ошибки/готовый файл.
- [ ] Профильные regression checks, сборка и итоговые результаты с ограничениями.

Промежуточные воспроизведения (ночь 03-04.11): SOCKS отсутствовал в reqwest features; включён и подтверждён локальным proxy/auth/fail-closed плюс полным YouTube через HTTP CONNECT/SOCKS5. YouTube known-size streams требуют bounded Range для скорости: тот же742369B, baseline18.767с transfer, послеRange+parallel0.175/0.259с, ffprobe19.064с. Шесть локальных дорожек1299→504мс, активных запросов максимум4. NicoVideo раньше давал ложный успех6.006/320с: m3u8-rs сбрасывает KEY после каждого URI, добавлено наследование до METHOD=NONE/rotation. Полный Nico15.34MB/320.087с прошёл; буферизацияAES1154→75мс/MiB, полныйповтор58.801→44.178с. Добавлена проверка длительности до публикации. Доказательства: `~/.local/share/videodock/research/downloads-2026-11-03/`, профильные отчёты в `docs/` дополняются. Это промежуточные результаты, HD/direct и часть сервисов ещё проверяются.

## Продолжение 04.11: устойчивость загрузок на всех заявленных площадках

Результат - свежая матрица всех площадок REA/каталога с полными загрузками доступных публичных samples, исправлениями воспроизведённых ошибок и явно отмеченными границами. Объём - существующий Rust runtime, без Python/yt-dlp и изменений аккаунтов/системной сети. Проверка - online pipeline до файла, ffprobe длительности/потоков, regression offline, workspace tests/Clippy, Qt build и доступный UI.

- [x] Восстановить TASK, сохранённые REA evidence, прошлые журналы и dirty tree.
- [x] Выполнены полные прогоны каталога и YouTube short/long/HD; ошибки не скрыты: по 8 pass/1 timeout Dailymotion в двух direct-повторах, у YouTube 2 pass/3 HTTP403.
- [x] Сверены 53 ParserGraph: матрица 18 именованных сервисов и Generic; публичные варианты и причины отказов записаны.
- [x] Исправлены Generic User-Agent 403, начальный предел Range аудио-CDN, bounded transient retries/cancellation и stale library rows; регрессии сначала воспроизвели ошибки.
- [x] Range/resume/truncation/cancel, max retry/403, HTTP/SOCKS5 × auth/no-auth, fail-closed и HLS/subtitles/probe прошли.
- [x] Workspace 162 passed/0 failed/42 ignored, Clippy, Linux build, Qt appearance+Engine и independent full decode; отчёт [docs/stability-2026-11-04.md](docs/stability-2026-11-04.md).

Остаток не закрыт: три длинных YouTube sample получают HTTP403 после первых диапазонов; Vimeo DRM/access, Facebook anonymous URLs, новая Naver Clip/Blog-схема, Likee/profile и Xvideos/Redtube остаются неподтверждёнными/не реализованными. Direct Dailymotion нестабилен, два full proxy повтора прошли. BiliBili дал полный файл через proxy; Generic после исправления прошёл proxy и direct, Tumblr - напрямую. Полная загрузка получена у 12 типов источников, это не полная поддержка всех сценариев. Нативный CUA видит окно, но UI input не подтверждён; вместо ручного pass явно отмечен пробел. macOS/Windows в этом проходе не запускались.

Доказательства: `~/.local/share/videodock/research/stability-2026-11-04/`. Новые зависимости/системные конфиги/аккаунты не менялись; временные сетевые diagnostics перенесены из tests в evidence. Основной пользовательский профиль не открывался.

Открытые пункты этого продолжения:
- [ ] Получить целые длинные YouTube файлы вместо позднего HTTP403 и подтвердить повтором.
- [ ] Закрыть неподтверждённые площадки/подтипы по матрице, а не считать единичный sample полным покрытием.
- [ ] Подтвердить ручной нативный GUI-сценарий; macOS/Windows отдельно.

## Продолжение 04.11: полная проверка и публикация GitHub

Результат - проверить накопленную Rust/Qt-ветку и опубликовать исходники с фактической матрицей. Объём - текущий VideoDock, изолированные fixtures и публичные media samples, без пользовательских аккаунтов и изменения сетевых настроек. Проверка - workspace/Clippy/build, полные pipelines с ffprobe, Qt/native CUA, секреты/лицензии перед push, remote SHA.

- [x] Восстановить предыдущий диалог, TASK, REA evidence и dirty tree.
- [x] Свежая workspace-регрессия:164 passed/0failed/42ignored; Clippy, сборка и Qt EN/RU. Реальные DASH/processing:5passed;15 файлов независимо полностью декодированы.
- [x] Все42 ignored-сценария запущены явно: сетевые отказы сохранены, три длинных YouTube по-прежнему HTTP403. Исправлена подтверждённая подмена Tumblr-поста рекомендацией: red/green,2регрессии, полный реальный файл.
- [x] Публичный состав: только исходники/тесты/docs; Gitleaks index-копии и всей истории:0секретов, Semgrep export:50rules/96files/0findings/0parseerrors; сохранены лицензии/атрибуция, закрытый appearance.rcc не публикуется.
- [x] Коммит исходников43b193b от Nezeronxer опубликован в [Nezeronxer/videodock](https://github.com/Nezeronxer/videodock), основная веткаmain; отдельно archive/python-prototype. Remote SHA совпал; анонимный README побайтово совпал с локальным.
- [x] Отчёт фактических проверок и пробелов: [docs/publish-verification-2026-11-04.md](docs/publish-verification-2026-11-04.md). Статус PARTIAL, стабильный релиз/1:1 не заявлены.

## Первый публичный релиз - 04.11.2026

Результат: продолжить инженерную работу, проверить приложение от лица пользователя и опубликовать первый GitHub Release с проверенным установочным артефактом. Объём: существующий Rust/Qt продукт; без обещания неподтверждённых платформ и сервисов. Проверка: свежие workspace/Qt/security проверки, живой CUA-проход, полная загрузка и ffprobe, установка пакета в чистый каталог, проверка публичного релиза и checksum.

- [x] Уточнить проект: VideoDock; восстановить TASK, исходники и результаты предыдущего прохода.
- [ ] Повторить workspace, Qt и security проверки; проверить основные публичные загрузки.
- [ ] Пройти пользовательские сценарии через CUA в отдельном профиле; устранить подтверждённые дефекты.
- [ ] Собрать и проверить релизный пакет, зависимости и лицензии.
- [ ] Подготовить release notes с фактической поддержкой и ограничениями.
- [ ] Опубликовать первый релиз на GitHub и проверить скачивание/хеши без авторизации.

Решение пользователя: только стабильный релиз, сначала устранить все блокеры; prerelease не публиковать.

- [x] Свежие workspace: 165 passed / 0 failed / 42 ignored; Clippy и три Qt harness PASS. Cargo audit: 0 vulnerabilities, unmaintained fxhash; Gitleaks истории: 0 leaks.
- [x] Native Sky/Xvfb: собственные ресурсы без appearance.rcc, пять onboarding шагов и пять разделов настроек; браузер → ручная загрузка в отдельную папку → ffprobe (видео+аудио, 5.000 с, 51506 B), выбор light.
- [x] Исправлен flaky profile restart: close удерживал flock при дубликате дескриптора; Owner::drop явно unlock после последнего владельца. Детерминированный red/green и свежий независимый std::fs self-check подтверждают фикс и эксклюзивность живого Engine.
- [x] Убраны Windows-only unused warnings в platform.rs через cfg(unix).
- [x] Подготовлен native-builds.yml для Linux/Windows/macOS; actionlint PASS, публикации в нём нет. Облачный прогон ещё не выполнен.
- [ ] Закрыть публичные загрузки: direct YouTube 0/6, SOCKS 2/5, длинные BBB всё ещё HTTP403.
- [ ] Windows: прежний build возобновлён jobs=1; текущий архив 69a479bb6725581e6df2abf0fef8c12d8699acbc0dd5508107da66a9a918ae4e передан в отдельный QA-домен.
- [ ] Источник подписанных обновлений, стабильный YouTube и остаток сервисов, macOS runtime, финальная упаковка/лицензии и полный пользовательский проход - блокеры stable. Релиз и тег не создавать до их закрытия.

Linux release после lock fix пересобран: exit0, 2м13с. Через Sky выполнен реальный restart release-бинаря: light и complete-запись восстановлены, штатное закрытие проверено. Windows предыдущего архива: release build exit0 за 15м49с; текущий архив обрабатывает guest-exec PID7316. Push release/first-stable дважды отвергнут удалённым GitHub (Internal Server Error; fatal error in commit_refs); native workflow пока не запускался.

Windows packaging прежнего архива выявил недостающий Qt6ShaderTools.dll: qtshadertools добавлен в native CI; установка в QA-госте guest-exec PID7952, текущая сборка PID7316 ещё выполняется. Новые Windows-unused импорты Command/Stdio также ограничены cfg(unix), повторный Linux Clippy exit0. Исследовательские диагностики и native screenshots сохранены вне репозитория.

Qt ShaderTools в Windows QA установлен: aqt exit0, Qt6ShaderTools.dll присутствует. Native текущего source archive ещё в работе, не считать сборку/установку подтверждённой. Финальный Gitleaks текущей истории: 0 leaks.

Текущий Windows source archive завершился ошибкой линковки PDB LNK1318 FILE_SYSTEM. Причина окружения подтверждена Get-PSDrive: 52,441,088 B свободно на QA C:. Запущен cargo clean --profile dev только в C:\VideoDockQA\target; исходники, release package и профиль не удаляются. Последний Linux release повторно exit0 за 2м13с.

Причина Windows PDB подтверждена и устранена: cargo clean --profile dev освободил 22.5 GiB, свободно 22,497,370,112 B. Для повторной проверки отключены только dev/test debug symbols (также в Windows CI). Новый архив source-final.zip: SHA256 5057cfe0ad3341b83e1e9464df77c60956f42c32d927217f6c836b72b6d6b466; script diagnostics/videodock-windows-final.ps1, это текущий повтор, а не успешный результат.

## Возобновление первого stable после перезагрузки хоста

Результат: закрывать блокеры stable, без prerelease. Объём: текущая ветка release/first-stable, прежний изолированный Windows QA-диск, доступные cloud native jobs, отдельная диагностика YouTube. Проверка: реальный результат Windows build/package, GitHub jobs и remote SHA, полные длинные загрузки с ffprobe, затем targeted regressions для подтверждённых исправлений.

- [x] Восстановить TASK, отчёт и git; host boot 04.11 13:09 остановил QA-домен.
- [x] GitHub снова принял ветку; native workflow run 41812660820 поставлен в очередь.
- [x] Запустить сохранённый videodock-platform-qa без создания новых образов.
- [ ] Получить сохранённый результат предыдущей сборки и завершить Windows package/runtime.
- [ ] Проверить cloud native jobs, исправить подтверждённые отказы.
- [ ] Диагностировать актуальную выдачу YouTube отдельно от CDN; исправления принимать только после полного файла.
- [ ] Завершить остаток матрицы/пользовательского прохода и выполнить стабильный релиз после всех gates.

Первый реальный cloud macOS-прогон 41812660820: библиотечные тесты прошли, audio_track_pipeline 6/7; OGG multi-track упал на Unknown encoder libvorbis. Первопричина подтверждена официальными Homebrew Formula: обычный ffmpeg не включает libvorbis, ffmpeg-full включает и установлен keg-only. Native workflow исправлен на ffmpeg-full с его bin в PATH; тест остаётся обязательным.

Cloud Linux первого прогона: workspace и release build завершились, Qt appearance остановился при запуске Chromium от root в одноразовом контейнере (Running as root without --no-sandbox is not supported). Только Linux CI job получил QTWEBENGINE_DISABLE_SANDBOX=1 для localhost fixtures; production launcher и приложение не менялись.

Windows после удаления повреждённого кэша пакета и возобновления: 162 passed / 0 failed / 42 ignored, 34 suites; актуальный release build exit0 за 6м09с, windeployqt дошёл до translations. Итоговый код упаковки и native runtime проверяются отдельно.

## Продолжение 04.11: Windows GUI и публикация проверенного кода

Результат: завершить оставленные Windows packaging/GUI исправления и отправить проверенные исходники на GitHub. Объём: текущая инженерная ветка, существующий QA-гость и native workflow; stable и prerelease остаются запрещены до закрытия прежних gates. Проверка: Windows PE subsystem и запуск пакета без build PATH, workspace/Clippy/Qt/actionlint, secret scan, remote SHA и фактические native jobs.

- [x] Восстановить последнюю рабочую сессию и сохранить незавершённые изменения.
- [x] Windows: GUI subsystem 2 подтверждён на настоящем PE; production installer в пользовательской сессии завершён и создал ярлык, собственное окно видно на снимке; финальный smoke новой копии проверяется отдельно.
- [x] Workspace165/0/42ignored, Clippy,3Qt harness, actionlint, Linux release PASS; диагностики сохранены вне репозитория;330 Windows source files побайтово совпали.
- [x] Код602f03a и отчётb382f15 отправлены в main; remote SHA совпал, публичный raw-отчёт без авторизации побайтово совпал. Native run41843213464 выполняет release-сборки после прошедших тестов Linux/macOS.
- [x] Обновлён docs/first-stable-release.md: stable не выпускается, YouTube/остаток сервисов/полный GUI/macOS runtime/updater остаются открытыми.

Свежий локальный прогон возобновления: workspace 165/0/42ignored, Clippy PASS; Qt appearance с настоящим Engine PASS. Gitleaks history:0. В export исходников детектор отметил 4 публичные строки RustyPipe fixtures (continuation tokens), классификация выполняется отдельно. Повтор полного длинного YouTube напрямую снова завершился ошибкой; stable остаётся заблокирован.

Независимое ревью diff019b160..602f03a:critical/important нет, обе lease-регрессии повторены. Источник Gitleaks export-предупреждений проверен:2 публичных continuation-token assert и2animationActivationEntityKey fixture, учётных данных нет. Generic повторён полным файлом52.208с/4368946B и полностью декодирован FFmpeg. Новая попытка длинного YouTube напрямую и через SOCKS не прошла metadata. Linux/macOS native CI прошёл тестовые шаги и выполняет release-сборки; Windows job ещё в работе.

Windows после возобновления: свежий повтор162passed/0failed/42ignored,34suites,exit0. Production installer в пользовательской сессии PASS с ярлыком; freshQA app PATH толькоWindows/System32: ffmpeg/ffprobePASS, PEsubsystem2, idle/Responding/window=true, ScheduledTaskResult0. Снимок собственного onboarding подтверждает живое окно. Это startup smoke, не полный Windows user-flow.

## macOS bundle: подтверждённый отказ native CI

Run41843213464: Linux полностью PASS (165/0/42ignored, release,3Qt). macOS core/media тесты и release-сборка прошли, bundle упал на bundled ffmpeg exit134: transitive @rpath libwebp/libjxl_cms/libsharpyuv/libggml-base ищутся через Frameworks/../lib; macdeployqt также не разрешил split Qt frameworks и сообщил invalid signature libjxl_cms. Объём исправления: общий Darwin installer и его contract regression. Проверка: воспроизведение из native лога, локальный red/green contract для libpath/RPATH/signing, затем новый настоящий macOS job; Windows текущего run не отменять.

- [x] Скачать и разобрать native macOS лог; причина отделена от успешных Rust tests/build.
- [x] Кандидат исправления: split Qt libpaths, @loader_path для flat dylibs, повторная подпись и deep/strict verify; bundled tools обязательно запускаются. Нативный успех пока не подтверждён.
- [x] Contract red/green, shell syntax, actionlint/diff PASS; независимое review59004d5..8f85766:critical/important нет. Нативный успех не заявлен.
- [ ] Повторить настоящий macOS job и записать фактический результат.

Первый native run41843213464 завершён:LinuxSUCCESS,WindowsSUCCESS(162/0/42ignored,release25м06с,production installer exit0),macOSFAIL(bundle ffmpeg134 после прошедших tests/build). Red/green contract воспроизвёл принятие broken executable старым installer и отказ исправленного. Для ручного macOS-повтора добавлен platform input workflow; обычный push по-прежнему проверяет все ОС.

Кандидат macOS8f85766 опубликован в fix/macos-bundle; targeted run41850414274 (platform=macos) поставлен в очередь. Main59004d5 не содержит неподтверждённого Mac-кандидата.

Состояние handoff: targeted macOS run41850414274 создан2026-11-04T14:43:29Z, в14:50UTC всё ещёqueued, steps=[]; повтор Linux/Windows штатноskipped. Зависимый шаг: дождаться фактического native Mac-результата для8f85766, при отказе скачать job log и исправить подтверждённый дефект; до PASS не считать bundle исправленным и не переносить кандидат в main. Это очередь внешнего runner, не результат тестирования.

## YouTube и реальная пользовательская проверка перед выпуском

Результат: исправить подтверждённый YouTube media-CDN HTTP403, проверить целые короткие/длинные/HD файлы и пользовательские сценарии настоящим computer use; завершить платформенные блокеры, затем stable release. Объём: Rust extractor/transfer, собственный Qt UI и установочные пакеты; без yt-dlp/Python в runtime, без изменения оригинала/сетевых настроек и пользовательских аккаунтов. Проверка: red/green регрессия root cause, полные файлы+ffprobe+decode/restart, native CUA на физическом рабочем столе, native Linux/Windows/macOS пакеты, secret/license gates и анонимное скачивание релиза/checksum.

- [ ] Восстановить native CUA через существующую регистрацию и получить свежий экран приложения.
- [ ] Установить причину YouTube403 отдельно по client/signature/PO token/CDN transport; не добавлять неподтверждённые обходы.
- [ ] Исправить shared Rust код и доказать полными файлами на доступных маршрутах.
- [ ] Пройти пользовательские сценарии через computer use, исправить найденное.
- [ ] Получить фактический macOS bundle PASS и закрыть прочие release gates.
- [ ] Собрать, опубликовать stable и проверить артефакты без авторизации после всех gates.

Дополнительно по живому пользовательскому замечанию: у импортированных локальных видео нет thumbnail в snapshot, thumbnailAction не задаёт pointing cursor. Веткаfix/youtube-delivery: регрессия Engine на локальный clip+сохранённую историю воспроизвела missing thumbnail (red). Добавлены bounded local FFmpeg PNG preview/cache, backfill library/history, fallback завершённой загрузки, HoverHandler на play; Qt harness проверяет реальный Image.Ready и pointing cursor. Нужен green и повтор на физическом desktop.
CUA: старый MCP entry ссылается на GUIChatGPT; exposed Node REPL не имеет trusted sky service. Документированный standalone Linux Sky работает на DISPLAY=:1; собранный VideoDock запущен в отдельном QA-профиле на физическом рабочем столе,5шагов onboarding пройдены реальными кликами.
YouTube PO: debug helper не работал из-за отсутствующего /tmp/.../js/98_global_scope_shared.js; это packaging debug-ресурсов, не отказ PO. Сохранившийся release helper0.1.2 токен генерирует, но Desktop player через SSH при одинаковом proxy для helper/метаданных даёт TryAgain. В продукт helper пока не подключён.

Живые исправления preview/play подтверждены: release build exit0(2м30с), Sky на DISPLAY1 получил полный AT-SPI после process-local QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1. В реальном окне локального ролика появился кадр; native XFixes cursor capture над play — изображение руки(hotspot10,1,hashdaf22df359ad). Qt harness с настоящим Engine прошёл Image.Ready для завершённой загрузки и все прежние сценарии. Регрессия local library+saved history+restart green. Снимки и cursor PNG в research/youtube-release-userqa-2026-11-04/evidence. Для устойчивости между сообщениями QA-процесс запущен transient user unit videodock-userqa; глобальные настройки не изменены.

Уточнения пользователя19:05: Windows-проверка с его зелёным zapret; приложение автоматически использует системный маршрут zapret/TUN, обнаруживает активный VPN proxy и выбирает его даже при proxy-only, без изменения конфигурации VPN/zapret. У ссылки/строки — значок соответствующей платформы(YouTube/TikTok/другие), как у4K. Вставка из системного буфера должна автоматически направлять URL в правильный handler и сразу показывать принятый источник.
Наблюдение paste до правок: настоящий wl-copy с publicYouTube sample -> CUA click Paste; скрытое clipboardField содержит правильный URL, но Smart Mode перед metadata не создаёт ни строки, ни manual dialog(state.manual=null). Гипотеза«буфер вовсе не читается» не подтверждена; нужно немедленное отображение разбора и отдельная проверка внешнего clipboard/протоколов.
Авто-триггер zaebal L1 относился к ненаправленному слову оTikTok — false trigger; точный injected dismiss command выполнен1раз, AppImage попыталсяGUI безDISPLAY и завершился1, счётчик не сброшен. Аудит не запрашивался; новые требования не объявлены повторным провалом уже завершённого исправления.

Параллельные направления (dispatching-parallel-agents): paste_platforms — immediate Smart queue + brand icons; auto_network_windows — auto proxy discovery + Windows QA; youtube_transport — native Rust YouTube delivery. Root — интеграция/CUA/релиз. У агентов нет git/branch/commit права.
Paste/brands: первая гипотеза«нечитаетбуфер» опровергнута; исправлена невидимая metadata-фаза SmartMode, быстрые независимые paste не теряются, duplicate inspect убран. 15 собственных brandSVG(включаяFlickr/BitChute), безproprietaryRCC. Qt frontend17cases(домены/short/uppercase/spoof)+реальнаяClipboardButton/CtrlV3sources PASS. Нужен финальныйcombinedCore/QML+CUA.
Autonet: explicit manual overrides; else env/staticOS proxy/known ownedVPNlistener+SOCKS handshake;5s cache, credentials не сериализуются. Auto-only Catalog.new_automatic per-request bypass localhost/127/8/::1, manual loopback proxy сохраняется. Core/network Linux5tests+proxy8tests(HLS/captions/auth/ffprobe) PASS. Windows nativeWinHTTP+ownedlistener6tests PASS; полныйcore network5/proxy8 PASS послеизолированнойзамены повреждённогоVS CMake(нулевыеmodules) наофициальныйportableCMake3.31.6 +DEV/TEST_DEBUG0. Это ещё не final app exactsource.
Flowseal по явномуURL пользователя: официальный1.10.4 ZIP проверенsha256291af673bd0238506f9548e0e94ca0abf244000f4c9a06960544fb01c5f97b34; rollback VM snapshotbefore-flowseal-20261104 +overlay; временныйpinnedGoogle443winws/ValidWinDivert driver, безhosts/autoUpdate/autostart. Watch page200неполныйvideo. NativeoldEngineSHA7695006... metadata timeoutZoo/BBB: guestNAT forwarded черезhostsplittun2022, поэтому ISP-direct не доказан. Агенту разрешён temporaryQA macvtap наwiredNIC +downтолькоNATlink, rollbackXML/команды, безизмененияhostIP/routes/TUN/zapret/firewall/DNS/другихVM.
YouTube: проблема подтвержденакакSABR(Desktop/iOS playerOK,format URLs0). Existing Rust sabr-rs MIT reference сначалапропускал format_id.itag; теперь реальныеOpussegments~1MB/60s, ноSPS2attestationнепринимается/полныйфайлнеполучен. ReleaseBotguard0.1.2 игнорировалSOCKSenv(егоreqwestбезsocks), mismatchroute. Новыйupstreamhelperсsocks/http1иpagecontext во внешнейпробе; никакихнеподтверждённыхruntimefallbackещёвпродукте. Все raw токены/подписанныеURLневыводитьинекидатьвgit.
Текущий UI release собранпослеpaste/icons/autonet(2м14с), но старый QAпроцесс пользовательзакрыл/нормальноexit0. Надо запустить заново черезtransientunitvideodock-userqa и проверитьCua. NativeSky helper/tmp/videodock-live-sky.mjs (документированный Linux standalone API), DISPLAY1, DBus/run/user1000, process-local QT_LINUX_ACCESSIBILITY_ALWAYS_ON1 даётполныйAT-SPI. Глобальныйcua_repl указываетнаGUIChatGPT, неMCP; не запускатькакstdio.

## 04.11 - выравнивание карточек при изменении окна

Пользователь прислал скрин с первой карточкой по центру, остальными слева. Это запрос на вёрстку, не на исправление YouTube. Существующие незакоммиченные изменения сети/attestation/icons сохранить.

- [x] Локализовать: ListView управляет x корневых делегатов, конфликт с центрированием Rectangle.x.
- [x] Регрессионный Qt-тест: до правки FAIL (x=0 вместо17 при ширине1000), после PASS на5ширинах с обновлением прогресса.
- [x] Центрирование карточки внутри отдельного полноширинного делегата; весь прежнийfrontendнабор PASS (темы/диалоги/меню/иконки/узкое окно/контраст). Скроллбар, выбор и обработчики сохранены.
- [x] Release build --offline PASS (3м49с). Реальный Sky/AT-SPI: после перезапуска3прежнихзаписи сохранены, карточки визуально выровнены при1100/2200/2538px. Окно оставлено открытым, возвращено2538x1336 на втором мониторе; плавающий режим выключен. Профиль прежний, youtube-release-userqa-2026-11-04/profile.

Данные проверки: ~/.local/share/videodock/research/layout-2026-11-04/. Исходный JobList.qml сохранён там с текущими изменениями icons/parsing.

## Текущее продолжение после явного решения пользователя

Пользователь выбрал «Да, Qt для проверки Google (рекомендуется)» и затем «ПРОДОЛЖИ». Допустима только анонимная Google-attestation в уже поставляемом QtWebEngine, без внешнего браузера/аккаунта; metadata, ссылки, подписи и медиатранспорт остаются Rust. Не спрашивать это повторно.

- [x] Внешний native прототип дважды полностью получил BBB635s720p60: audio10258925B/video150524867B, contiguous seq64/123 и terminal coverage634601/634567ms, mux161019956B, полныйdecode634.594s/38074video/31730audio, повторныйSHA совпал. Это доказательство подхода, не productPASS.
- [x] Независимый review выявил нужные исправления: per-track contiguous/atomic retry, redirects доPO, boundedUMP+finish, неправ.UMPencoder не переносить, privateIPC/childlifecycle, exactproxy snapshot, normalUI не умирать отautoproxyerror.
- [ ] youtube_transport реализует production Rust proof/SABR/model+pipeline, attestation_review реализует desktop same-exe --youtube-proof доEngine/DB/QML. Согласованversion1/UUID/videoId/proxy,32KiB pipes,85/90sdeadline; opaquePO/visitor/UA, безсохранения/argv/envсекретов. Worker реальноPASS вsameRustexe6.74s, но текущиеhardening/HTTPauthещёдорабатываются.
- [ ] Финальный integrated queue/pipeline полныйкороткий+635sHD, decoderlastPTS и sequence/bytes, cancellation/retry/privacy, freshreview.
- [ ] Финальный Windowsexactarchive+SHA -> auto_network_windows собирает/устанавливает; RootделаетCua черезlocalviewer.
- [ ] MacOSnewworker/build/package: oldnativeMacRusttests/buildPASS, installerlibpath/sign candidatefix/macos-bundle8f85766 неслит; повтор41850414274 дваждыcancelled доrunner/steps. Нужна новаяреальнаяпроверка, не считатьlegacyCIуспехом.
- [ ] StableGitHubrelease после реальныхgates, безprerelease/ложной1:1готовности.

CUA текущиефакты уточнены: nativeSky EWMHactivate не переключал HyprlandLua workspace; фоновое окно не получалоWaylandclipboard. Фактический compositor черезтекущий /run/user1000/hypr/.socket, неstaleChromeenv. Правильный временныйfocus: hyprctl eval hl.dispatch(hl.dsp.focus({monitor=...})); hl.dispatch(hl.dsp.focus({workspace=...})), затемпроверитьexactPID active+другоймониторнеизменён; legacydispatch focuswindow возвращает7. Послеактивации реальныйwl-copyTikTok→CUAbuttonPaste и YouTube→CtrlV сразу показалистроки+правильныеиконки. TikTokреальноcomplete3308605B,H264576x1024+AAC24.401995s иfulldecode到end. Снимки /tmp/videodock-cua-tiktok-active-accepted.jpg иyoutube-shortcut-accepted.jpg; copyвresearch/evidence. РанееbackgroundpasteDialogпустой неошибочноназыватьдефектомclipboardproduct. Физическийcursor лучше перепроверитьвактивномокне; исходныйнативныйXFixeshand быллогическимXwaylandсостоянием.

Windows/DNS: auto_network_windows state /tmp/videodock-auto-network-state.md —последниесекцииавторитетнееoldpartial. NativeDoH7localtests/6Windows+bothtraitrealpublicAпроверкаPASS. ПослеplaintextNXDoHлечитDNS, носпровереннымиGoogleIPv4 наphysicalRU нетTCP; 2YT A и1Googlefrontend curltimeconnect0/HTTP000, этонеCDN403. Никакихновыхстратегий/hostсетииIpv6настроек. Всёmacvtap/NAT/DNSвQAточнооткачено, Flowseal1.10.4временноactive; старые/nativeDNSuiEngineнефинальноеapp. VMwlms.exe cleanhourlyshutdown отистёкшейevalлицензии; лицензирование/защитунеменять, собиратьза<1h. Финальныйbuild portableCMake3.31.6,DEV/TEST_DEBUG0,jobs1, всеNEW/untrackedфайлывarchive.

Выравнивание — PASS. Живые снимки: research/layout-2026-11-04/live-proof-1100.png, live-proof-2200.png, live-final.png в ~/.local/share/videodock/. Test RED: Download alignment1000, x0 вместо17; GREEN: весь frontend-набор тем, вкладок, диалогов, меню, иконок, narrow/contrast. Новые прогоны скачиваний не требуются для этой правки вёрстки; Windows/macOS не запускались. Незавершённая работа транспорта сохранена без правок.

Продолжение: integrated production Zoo worker→Rust metadata→SABR→publish PASS:742369receivedB,744360muxB,decode19.063583s. Long BBB пока неPASS: выявлен negotiation defaultAAC258(5.1) противstereo capabilities; transport agent исправляет и повторяет failing case. Desktop IPC/URL/auth hardening frozen: HTTPauth positive+boundedwrongpassword PASS. Независимый final_transport_review идёт по текущему diff.
Mac packaging candidate перенесён точечнымpatch, контракт PASS; workflow теперь standardmacos-15-intel из актуальногоofficialrunner-images, cleanPATHproofchecks+ZIPartifacts. actionlint1.7.7 не знает новуюметку; остальныепроверкиPASS приузкомignoreтолькоэтойпровереннойметки. НативногоMacPASSещёнет.

FinalproductionSOURCEFREEZE: Zoo240p742369B/19.063583s;BBB720p60181292478B/634.624s;BBB1080p60288387264B/634.624s, lasttransfer27.695s. НезависимыйtransportreviewCritical/Important0. Rootfreshworkspace204pass0fail42ignored,all-targetClippy-Dwarnings,release2m41s,Qtappearance/browser/cookie/proofboundaries/worker8invalid+proxyrefusal+badstartup —PASS.
Packagingreviewнашёл2реальныхImportant: validworkerподrootCIсsandboxenvсброшенным(productionguardверный), CIrootтеперь8invalidIPCтолько, nativeMac/Winполныеworkerchecks; отсутствовалипоставляемыеlicense. ДобавленыstagingGPL3+LGPL+Bg/SABRMIT+runtimeversion/config, Rustdependencylicensecollectorидёт. NoticesкопируютсяДОMacsign, contractassertslicensefilesPASS.
NativeCUAновогоrelease: invalidautomaticproxyнеубиваетUI; Settings→Connection→manualSOCKS127.0.0.1:1080→Save реальноработаетприоставленномinvalidenv. CtrlVсвнешнегобуферасразу4ястрока+YouTubeicon; BBB1080p60complete275.4MiB10:34, thumbnailвиден. НезависимыйdecodeэтогоGUIфайлаидёт. CUAделатьсоfreshAXIDs(ониперенумеровываются),set-e иboundedclipboardreadiness; неисполнятьpasteпослеfailedprecondition. Физическийhoverнеподтверждён: Waylandcursorдвигалсявовремяпробы; QtcursorregressionPASS.
WindowsnativeберётsourceZIP5589f4adfa416ae8c22a778b8d81469364f2c9413c1932718e01d0dd10caad50(413files), compileunchanged; missingprostofflinecacheбудетlockedfetch. Packaginglicensepatchпередатьдосборкипакета. ПользовательповторноявнопопросилWindowsVMтест; этоактивныйreleasegate.

## Актуальные требования перед полным релизом

Пользователь подтвердил удаление всех ИИ-функций обработки медиа вместе с кнопками и моделями. Дополнительно требует обычный Windows .exe installer, полноценный проход Windows 10 и Windows 11 на ВМ до публикации полного GitHub-релиза, универсальную Linux-поставку. Старые видео, результаты и история сохраняются.

- [x] Core: удалить MediaTools module/API/scheduler/settings, снять лишние зависимости; legacy processing rows сохранить скрытым SQL archive. Migration 4, engine 7, local downloads 9, library 1 и core Clippy PASS.
- [x] Desktop: удалить весь раздел, кнопки, Processed-tab, настройки, onboarding-шаг и переводы. EN/RU appearance PASS, 302 RU translations / 0 unfinished.
- [x] Курсор: исходный handler исправен; прежняя проверка делалась до готовности кадра. Новая frame-sync regression проверяет native Arrow→Hand→Arrow, негативный контроль без handler падает. Product QML handler сохранён.
- [x] Контекстное меню по скриншоту: два скрытых subscription MenuItem занимали высоту. Red/green на All/Subscriptions, height 0 для скрытых пунктов, смежность видимых строк и положение Delete PASS.
- [x] Удалить оставшиеся AI notices, пересчитать license bundle: 366 Rust dependencies, 393 texts. Dedicated model caches 317 MB перемещены в Trash; медиафайлы не удалялись.
- [ ] Общий post-AI workspace/Clippy/release и native установленный Linux GUI.
- [ ] Готовый per-user Windows installer: Inno recipe/wrapper и CI добавлены, ждут actual compiler/install/uninstall/upgrade checks. В пакет включать официальный app-local CRT и qoffscreen; профиль не удалять.
- [ ] Windows 10: pre-AI native 200/0/42ignored, release и clean-PATH worker 8-invalid/proxy/startup PASS; повторить затронутые границы и GUI на версии без ИИ.
- [ ] Windows 11: готового образа нет, нужна отдельная официальная Eval VM с TPM2/SecureBoot; последовательный boot с Win10 из-за RAM. Без обхода требований/лицензирования.
- [ ] Universal Linux: Flatpak с KDE6.11/WebEngine BaseApp native nested sandbox; SDK/runtime/FFmpeg codecs проверены. Probe и сборку ведёт youtube_transport. Actual distro matrix остаётся обязательной; не обещать все версии ОС без проверок.
- [ ] Полный GitHub release после всех пользовательских gates. Пока опубликован только dev source branch fix/youtube-delivery.

Mac diagnostic run 41897879185 фактически получил Intel runner и идёт; это pre-AI source fa76501, не финальная версия интерфейса. Соответствующий исходный код с vendored registry подготовлен, offline metadata с пустым Cargo home PASS; перед публикацией создать свежий post-AI source archive, чтобы не оставить удалённые старые файлы в staging.

## Пользовательская доводка и выпуск, 05.11.2026

Результат: понятные ошибки без SABR/PO jargon, корректное меню, готовые Windows 10/11 установщики с живой проверкой; публикация GitHub после gates. Объём: завершить текущие изменения и исправить подтверждённые дефекты, модель/effort прежние. Проверка: regression red/green, workspace/Clippy/Qt, полные загрузки+ffprobe, реальный Linux GUI, установка/обновление/удаление и GUI обеих Windows VM.

- [x] Восстановить предыдущие требования и незакоммиченный diff; baseline cargo test --workspace --offline PASS.
- [x] Исправить проверку отказа YouTube до чтения адреса потока; regression и исходная ссылка.
- [x] Проверить тексты, меню и ошибки в доступных Linux-сценариях.
- [x] Собрать и пройти установленный Linux GUI с сохранением текущего профиля.
- [ ] ОТЛОЖЕНО ПО ПРОСЬБЕ ПОЛЬЗОВАТЕЛЯ: завершить Windows 11 Eval VM без обхода TPM/SecureBoot/лицензии.
- [ ] ОТЛОЖЕНО ПО ПРОСЬБЕ ПОЛЬЗОВАТЕЛЯ: Native Windows 10/11 installer + upgrade/uninstall/profile + GUI/download QA.
- [ ] Финальные проверки, упаковка, GitHub release и anonymous checksum.

05.11 09:33: новые regression RED→GREEN: player refusal до URL (LOGIN/UNPLAYABLE/ERROR/missing URL); readable persisted job/sub errors без изменения исходных диагностик. Core transient notice/manual тоже covered. EN/RU appearance до следующих UI-правок PASS. Live Sky через standalone @oai/sky на DISPLAY=:1 с DBUS/XDG получил реальное окно и AX; без DBUS видел только корень (не баг Qt). Изолированный QA unit videodock-polish-qa. Fresh screenshot More-menu обнаружил привязку к глобальному курсору вместо кнопки; новый Qt regression RED, исправлены popup(row,eventPoint.position) и popup(moreButton,0,height). Пользователь прислал зелёный switch без thumb: software backend теряет весь layer из-за DropShadow; pixel regression RED, shadow удалён, весь software EN appearance GREEN. Проверены все 302 RU строки, устранены неверные формулировки/регистр. Новый final build/установленный GUI ещё нужны.

Исходная ссылка w1Pr_IXdW2A: anonymous player ERROR/UNPLAYABLE, Chrome с пользовательским входом тоже player ERROR «Видео недоступно» (проверено OpenCLI, QA tab закрыта). Это не успешно скачанный файл. Из headless shell proof worker без QPA не стартовал; QT_QPA_PLATFORM=offscreen даёт valid worker response. Public Zoo: direct CDN connection fails; socks1080 получает полный короткий audio, но protection=2 и current fail-closed final PO gate отказывает. Нельзя ослаблять gate/объявлять транспорт рабочим; диагностировать acceptance или другой существующий маршрут. Тест zoo-happ-proxy.log проверяет socks10808.

Windows11: настоящий официальный ISO установлен на ранее пустой отдельный диск (MS checksum совпадает), SecureBoot/TPM2 включены. Seed включает подписанный vioser driver из Win10 и прежний qemu-ga MSI; firstlogon устанавливает агент и shutdown. Приватный локальный пароль/Autounattend/seed ISO остаются только research, не читать/не публиковать. Сейчас стадия installing 42% после первого рестарта; Win10 videodock-platform-qa выключена штатно для RAM. Seed/состояние: ~/.local/share/videodock/research/windows11-qa.

Случайный cargo fmt --all переформатировал clean vendor/media tests; восстановлено reverse patches, бэкапы /tmp/videodock-{vendor-formatting,formatting-only}-backup.patch. Не запускать fmt --all; только rustfmt конкретных изменённых файлов с skip_children. В git по-прежнему чужие исходные изменения удаления processing/installer (сохранять).

Win11 setup отказ подтверждён Panther: SMI unexpected tag ns0:component, 0x8022000b. Причина QA provisioning: повторная сериализация ElementTree без register_namespace поменяла default namespace на ns0, который Setup отвергает. Seed v2 исправлен с register_namespace, assertion на отсутствие ns0 и наличие default namespace PASS. Исходный неудачный диск сохранять, установку повторить на новом пустом QA-диске. Это QA provisioning defect, не дефект приложения.

Root transport fix 05.11: short tracks may finish with protection=2 (pending) before final acceptance; прежний immediate ensure(attested) давал ложный отказ. Regression complete_media_waits_for_pending_attestation_before_publication RED→GREEN: ждать при полном файле+pending; только accepted+полное coverage/bytes даёт готовность, accepted+truncated всё равно error. Перенесён final-ready check до stall counter, сохранились max3 no-progress cycles и retries. Реальный Zoo240 после правки PASS 742369B transfer / 744360 mux / 19.063583s full decode. BBB720 непосредственно до правки PASS 181292478B / 634.624s full decode. BBB1080 повтор после правки запущен. Не считать предыдущий direct CDN failure или unavailable исходную ссылку успехом.

05.11 09:44 пользователь сообщил о переполнении RAM и выбрал «До 2 ГБ - Windows отложить». Прекратить все VM и параллельные builds/tests; дальнейшие локальные проверки по одной, под MemoryMax=1G/MemorySwapMax=0 (запас для основного процесса агента). Win11 штатно managedsave, /var/lib/libvirt/qemu/save/videodock-win11-qa.save 5.9GB; domain shut off, Win10 тоже off. Rust compiler orphan 135986 завершён, cargo127911/130144 и youtube-test134739 остановлены. Не возобновлять Windows без новой просьбы. На момент остановки доступно10GiB из31, swap4GiB занят; чужой Discord renderer21240≈10GiB не трогали. Heavy сценарии остаются требованием полного stable release, а не якобы пройденными. Текущая сборка release-after-attestation прервана; последняя успешная release-final имеет UI fixes, но не последнюю transport change. Завершённый BBB1080 результат проверить по логу (процесс остановлен, если ещё шёл). Core-after-attestation и clippy-after-attestation сверить по tail, не объявлять прерванное PASS.

Финал Linux 05.11: release-lowmem PASS 3m09s, scope MemoryMax1G/MemorySwapMax0, наблюдаемый peak807403520B. Установлен SHA256 c7d8db5a208e648b33ff329033a6e27c3921d74a4ee2331c5e0e84872aa26417; rollback binary research/user-polish-2026-11-05/installed-before-polish. Native Sky установленного бинаря: окно и AX доступны, белый thumb виден в software backend; screenshot installed-final.png, QA service остановлен. Последовательный lowmem-final-qa PASS: EN/RU appearance (thumb оба состояния во всех темах, меню, real Engine local download/restart/remove), proof8-invalid + selected-proxy refusal + bad-proxy startup, license366packages/393texts. Publication-only export gitleaks PASS0; точечные reviewed exclusions для публичного BgUtils client ID и Hello World AES примеров aws-lc-rs в notices. Ignored graphify cache не публикуется. Windows10/11 и 1080p остаются отложены; обе ВМ off. Stable release НЕ публиковать до этих gates.

Пользователь возобновил Windows VM QA. Одна ВМ за раз, без параллельных host builds; перед стартом14GiB available. Win11 восстановить из managedsave и завершить setup, затем штатно выключить и собрать актуальный Windows installer на Win10. Источник текущей версии commit53304e24519894c827702eeb5de44285c33be48c. Проверить installation/upgrade/uninstall/profile + GUI/download обеих ОС, затем release gates.

Возобновление VM: Win11v2 завершила этап установки, но OOBE «Why did my PC restart?» просит update, Next повторяет цикл. Через ShiftF10 штатно установлен подписанный vioser (подтверждён Red Hat диалог Install) и qemu-ga из seed E:, скрипт штатно shutdown. После рестарта ещё проверить QGA и диагностировать OOBE (не обходить setup/лицензирование). virt-cat auto inspect больше не находит OS, не считать это доказательством порчи/шифрования. Win11 сейчас off. Win10 config currentMemory временно4GiB (backup win10-domain-before-memory.xml), единственная running VM. Source archive HEAD53304e2 SHA256139a5e33a2a8fb6774471fde1cd8af9807d3aa3c02d06d9a9e0dc650dfce5cc5, загружен C:\VideoDockQA\source-53304e2.zip. Build script /tmp/videodock-build-windows-current.ps1 скопирован туда же и запущен QGA pid7140. Свежая src-53304e2, preserved package-before-53304e2, jobs1, debug0, portable cmake3.31.6; result/tests/build/installer logs *-53304e2. Progress helper /tmp/videodock-windows-progress.ps1. Тяжёлых host builds нет.

Windows10 native core+media 53304e2 tests PASS; copied full log research/user-polish-2026-11-05/windows10-tests-53304e2.log. Release/installer build is running inside single4GiB Win10 VM, no host build.

Windows10 runtime53304e2:200testsPASS/0fail/39ignored; release+Inno compile PASS, installer165778073B SHA25690596180cc0792821cb4d0672a89a777580820f9bde9775830f632fb64eb3882 copied+guest hash MATCH. Limited interactive user install2/upgrade/uninstall/profile sentinel/reinstall/cleanPATH GUI PASS; installed exe6FA9FF9EE30C731719B233ACD9574966309FE347250DD3776D1ECA3C93DB8FE9. QMP menu no gap, switch visible. ZooGUI direct failed Соединение SABR прервано; temporary guest-only HTTP CONNECT→existing1080 SOCKS succeeded:MP4744478B/H264320x240+AAC/19.063583s, bundled ffprobe/full FFmpeg progress=end PASS. Direct routing is NOT PASS. Bridge /tmp/videodock-qa-route.py auto240s, no global network changes. Harness waits corrected (StartProcess-Wait orphan handling; bounded MainWindowHandle readiness), actual package worked. QGA status responses consumed after first read; cannot re-fetch, use observed summary JSON and screenshots. Win10 shutdown requested, Win11 still needs OOBE diagnosis and appQA.

Win11 preparation diagnosis: native DNS resolves download.windowsupdate.com, direct curl Microsoft times out. Existing host socks1080 returns HTTP200; temporary guest-only Microsoft HTTP/CONNECT bridge /tmp/videodock-win11-update-route.py bound192.168.122.1:47375, auto900s. In-guest WinHTTP config backed up C:\VideoDockQA\winhttp-before.txt, set proxy onlyinsideQA VM; curl explicitproxyMicrosoftHTTP200 confirmed viafreshQMP. Rollback netsh -f C:\VideoDockQA\winhttp-before.txt (before bridge shutdown). Retried normal OOBE Next after connectivity fixed. QGA service originally -d -p retry-path looked invalid; backedup service registry C:\VideoDockQA\qga-service-before.reg and tried explicit virtio path, but1053. Foreground qemu-ga-v proves missing channel device \\.\Global\org.qemu.guest_agent.0; need diagnose virtio driver/ports, not claim fixed. User cannot be asked to bypass OOBE/requirements. Helper /tmp/videodock-vmtype.py now validates all chars before typing, supportsbraces; a partial curl command was cancelledbefore rerun.

Win11 OOBE resolved normally after temporary WinHTTP Microsoft-route fixed network: reached normal lockscreen/login/desktop; no setup or license bypass. Eval watermark valid90days. User password submitted through stdin, never shown. Installer transferred via guest-only temporary HTTP server /tmp/videodock-win11-file-transfer.py, port43103, whitelist setup.exe/validate.ps1/resultPOST onlyIP248, auto900s. SHA256verified inuser-script. Windows11 ordinaryuser install2/upgrade/uninstall/profile-sentinel/reinstall/cleanPATH GUI PASS; result JSON research/user-polish-2026-11-05/windows11-user-result.json, same installed exe6FA9FF9EE30C731719B233ACD9574966309FE347250DD3776D1ECA3C93DB8FE9. Source Win11 test script preserved /tmp/videodock-windows11-user-validation.ps1. Google test route /tmp/videodock-win11-google-route.py auto600s, port metadata /tmp/videodock-win11-google-route.json. App restarted normally viauser PS script with process-only HTTPproxy and sameQAprofile; clipboard BBB aqz-KE-bpKQ, CtrlV submitted1080p. Wait complete+full decode, menu screenshot, rollback guestWinHTTP netsh -f C:\VideoDockQA\winhttp-before.txt (needsAdmin), restore QGA config backup ifleaving, shutdownVM. Host no builds.

Windows11 full1080p PASS: same installer+exeSHA, native user GUI download BBB complete288738862B, H2641920x1080+AAC,634.624000s, bundled ffprobe+FFmpeg full decode end634624000us/progress=end. Persistent report windows11-media-result.json, installer report windows11-installer-result.json. Native menu/switch screenshots saved windows11-context.png/windows11-complete1080.png. Guest WinHTTP restored via backedupnetshscript, showproxyDirectaccess observed; QGAserviceregbackup restored (operationcompleted), QGAdeviceunavailable remainsQAintegrationissue, not app. Temporary guest-only transfer/update/Google bridges stopped; Win11 shutdown requested, Win10 off. Next: verify bothoff/freemem, update docs with Winmatrix and networklimitation, finishLinux universal packaging gate + GitHub release. Do not claim directnetworkPASS; proxy tests are actualcompletefiles.

Both Windows10/11 app QA complete. Before fullrelease: LinuxFlatpak cachedprobe is notproduction. Hostbinary GLIBC2.44(sinh/cosh) butSDK/runtime2.42, so copyinghostexe isinvalid. SDKqmake6.11.3 matchesBaseApp, WebEngine mkspec /app/mkspecs/modules; QML location investigate. Preparing cached officialRust1.99.1 extraction (~minimal rustc/std/cargo), clone hostrelease1.8GiB viareflink to ~/.cache/videodock-flatpak/sdk-target. Build insideSDK, offline vendor-registry730MiB from corresponding-source; caphostbuild1GiB/noSwap/jobs1. AllVMoff/nootherbuild. No release published yet.

Flatpak SDK build isrunning. Importantresource boundary: Flatpak relocates cargo into own app-flatpak scope, escaping outer systemd-run scope. Verified /proc/cargo/cgroup and imposed MemoryHigh768M/MemoryMax1G/SwapMax0 directlyon actual app-flatpak-io.github.Nezeronxer.VideoDock-915830148.scope; verify actualscope for everyFlatpakrun. Cached Rust toolchainSHAverified against official static.rust-lang.org checksum. SourceSDKsubset deps compileofflinefromvendor. Driver /tmp/videodock-build-flatpak-sdk.sh, SDKsource/sdk-target/sdk-cargo/sdk-rust under~/.cache/videodock-flatpak. HostArch release notcompatibleSDKglibc; no binarysymbolpatches.

SDK buildfixes: officialRust checksumPASS, missinglibclang solved signedFlathub llvm22//25.08 (directTLSfailed, childhttps_proxy=socks5h1080 installPASS), builtinstddef.h required BINDGEN_EXTRA_CLANG_ARGS=-isystem /usr/lib/sdk/llvm22/lib/clang/22/include. Currentrun /tmp/videodock-run-sdk-lowmem.py -> /tmp/videodock-build-flatpak-sdk.sh, logs flatpak-sdk-build-clang-headers.log, session42748. Controller gates cargo until own actualapp-flatpak scope memory.max1GiB/swap0 verified; outer systemd-run is insufficientbecauseFlatpakmovesCG. CurrentlycompilingRustyPipe, no otherbuild, bothVMoff.
New packaging metadata/script inrepo: packaging/linux/flatpak-metadata, scripts/rust/package-flatpak.sh (UNTESTED untilSDKbinaryready). docs/windows-qa-2026-11-05.md recordsactualWinresults/networklimits. sandbox_settings.rs is prepared regression (NOT RUN/implemented yet): native Flatpak cannot registerhostautostart, runtime lacks systemd-inhibit. Need rejectunsupportedflags withoutsavingotherpreferences, capabilitiesauto_start/prevent_sleep=false and matchingSettingsToggle disabled, normalizeoldtrue flagsfalse forFlatpak. Run RED onlyafterSDKbuildends (oneheavyprocess rule), implementminimalLinux-onlyguard/test GREEN, then copychangedCore/QML intoSDKsource and rebuildcached. Windowsbinary33304e2 remainsunchanged/tested; Linux-onlypackagingchanges musthaveaccurate sourceprovenance; don't claimallplatformscompiledfromnewcommit. Or removeunimplementedtest ifscope changes; do notpush failingtest.

## Возобновление полного выпуска, 05.11 18:10

- [x] Сверены свежие журналы: Windows 10/11 installer/media PASS; Linux Flatpak production release build и final sandbox tests/Clippy/appearance PASS (журналы 13:23–14:45). Прежний текст об ожидающей SDK-сборке устарел.
- [x] Повторно вычислены SHA256: Windows `90596180cc0792821cb4d0672a89a777580820f9bde9775830f632fb64eb3882`, Flatpak `c1e0750bf3fcbc40835e288a29797f4fc9b03f7d47ae7fca1afa92fa70ecd8fd`.
- [ ] macOS: run 41897879185 FAIL на packaged proof worker, stdout пуст; FFmpeg/FFprobe и codesign до него прошли. В harness обработан socket.timeout совместимо с системным Python 3.9, добавлена проверка exit code до JSON и сохранение native crash reports. Нужен новый native run текущего кода.
- [ ] Свежая проверка установленного Flatpak, комплект соответствующих исходников и точная матрица поддержанных ОС.
- [ ] Публикация только после закрытия native macOS gate; существующий v0.2.0 остаётся Draft.

Пользователь 05.11 прислал скриншот контекстного меню: обрезаны длинные русские подписи, недостаточный горизонтальный padding и избыточные вертикальные интервалы. До выпуска нужны отдельные geometry regressions и проверка свежего меню. Дополнительно требуется повторный полный download/decode каждого сервиса и подтипа из матрицы 4K; старые metadata/ограниченные GET не засчитываются. Обновить матрицу PASS/FAIL/BLOCKED по фактическим файлам.

Русские подписи интенсивности загрузок: новое geometry assertion воспроизвело наложение (RED); размер шрифта12 даёт раздельные подписи с зазором6+px. Полный Qt fixture EN/RU GREEN, screenshot в /tmp/videodock-layout-qa/green-{en,ru}; эти изменения пока не включены в прежние бинарные пакеты.

Меню: regression RED `Context action lacks horizontal padding`; общий ContextAction теперь задаёт12px по бокам, компактную высоту32+px, меню вычисляет ширину по самому длинному видимому пункту, separators9px. Полные Qt fixture EN/RU GREEN, длинные подписи и оба вида меню проверены. Снимок `research/user-polish-2026-11-05/menu-spacing-fixed-ru.png`.

Проверка первоисточников сервисов 05.11: официальная tv.naver.com показывает уведомление о закрытии NaverTV30.09.2026 и переходе в Clip; старый parser ticket не подтверждает поддержку Clip. Likee public profile возвращает `This page is not available in your area`. Новый публичный Vimeo33951933 доступен в Chrome с video и длительностью1:00; добавлен отдельный полный тест, успех пока НЕ подтверждён. Идёт последовательная lowmem сборка catalog_downloads в ~/.cache/videodock-service-check, journal /tmp/videodock-service-build.log; новые full_sample теперь требуют FFmpeg decode до успешного PASS. Не прерывать только из-за незавершённости; сетевые отказы фиксировать отдельно.


05.11 18:48: свежие core/media202uniquePASS +1nested-childPASS/0FAIL/43ignored; sandbox_settings PASS, DNS7PASS. DNS nativeMac run42052359410 отказал на fixture: таймер ожидания1с запускался до подготовки HTTP-клиента. Подготовка клиента перенесена до запуска fixture-thread, production DNS не менялся; локальная проверка7PASS, nativeMac ещё нужен. Mac cache restore/save разнесены, чтобы сохранять сборочные файлы и после позднего test/package FAIL; pinned save/action.yml прочитан через GitHub MCP. Нельзя выдавать сбой unitfixture за исправленный macOS runtime.

Свежие полные сервисы: direct10PASS/3FAIL (TikTok network, Vimeo DRM, Vimeo-public403), TikTok SOCKS1PASS; additional Facebook/Likee-profile/NaverTV3FAIL. files-proof.json содержит11 файлов с full decode. docs/services-release-2026-11-05.md фиксирует всю матрицу и ограничения; fullstable не готов. Только metadata/HTML200 не означает успех.

Кэши предыдущей SDK-сборки отсутствовали. Восстановлены официальные rustc/rust-std/cargo1.99.1 с проверкой SHA256 каждого архива; подготовка ~/.cache/videodock-flatpak/baseapp с KDE SDK6.11 и llvm22. Далее SDK build текущего QML, пакет/живой GUI, fresh source archive и nativeMac. Windows предыдущий проверенный installer53304e2 сохраняется; новые QML изменения требуют нового бинаря.


05.11 19:11: Facebook исправлен: официальный public embed отдаёт videoData со sd_src/hd_src. Parse target строго video_id; regressionRED→GREEN (wrong-ID/recommendations отклонены, geometry и query-ampersands сохранены). Анонимный fallback Auth::default(), credentials не пересылаются. Полный прямой файл12474213B /44.330667s /VP91080×1080+AAC /FFmpegdecodePASS. files-proof.json обновлён до12 свежих источников. Core/media после изменения203uniquePASS +1nested-childPASS /0FAIL. Отдельный Likee public post тоже отказал; publicAPI code50003/system error. Vimeo player33951933 даёт401verification, другой sampleзащищён; обходов DRM/доступа нет.

SDK current release PASS20m52s; проверка freshness после всех production changesPASS0.16s. Новая упаковка candidate-20261105-menu-fb выполняется, journal /tmp/videodock-menu-fb-package.log. ActualSDK build scope1GiB/swap0 verified. Native cloud run42058290877: LinuxSUCCESS, macOStestsPASS и bundle в работе, Windowsbuild в работе. Cloudcommit2cd5b15 ещё не включает новый Facebook fix; не выдавать его будущие артефакты за новую FBверсию. Перед финальным выпуском нужен следующий native run Facebook commit с reuse caches.


Live nativeSDK GUI: настоящая вставка URL через диалог → completeFacebook1080p44s+AAC; full FFmpegdecode PASS, контекстное меню целиком, сохранён Zoo. Найден metadata regression новогоembedfallback: вместо названия ID. Отдельный liveassert REDдо скачивания → перенос og:title/og:image исходной watchpage → GREENполный файл12474213B44.331s. ВМ/основной профиль не затронуты, temporaryclipboard восстановлен. SDKtitleincremental пересборка session99475, log/tmp/videodock-sdk-title-build.log. Candidate4f637f1d... успешно установлен Flatpak, но ЕЩЁбез titlefix; финальный пакет пересобрать послеtitlebuild. NativeQAunit штатноstop. Clippycore/media начат log/tmp/videodock-clippy-facebook.log. Linux/Windows cloud2cd5b15SUCCESS, macOSbundleвработе. Новые FBизменения пока uncommitted.


TitlefixSDK releasePASS2m24s. Live nativeGUI повтор: реальное описательное названиеCNN иобложка, complete1080p44s; freshfile полный decodePASS. BinarySHA256c901cf4a4fe1fd155f1ff1c4787ce6f5800cfd0c008d26d9645f5e22bee63ddb атомарно установлен в target ~/.local/bin/videodock (launcher symlink сохранён), старыйбинарь research/.../installed-before-menu-HHMMSS; точный rollbackpath вnative-menu-install-proof.txt. История/профиль/аккаунты не заменялись. InstalledQAunitvideodock-menu-installed-qa проверяется на отдельномпрофиле. FinaltitleFlatpak packaging log/tmp/videodock-final-title-package.log; предыдущийcandidate4f... НЕ финальный. Clippycore/media all-targets warningsdeniedPASSпослеtitlefix.

05.11 20:00 Windows source8face run42065864692 FAIL на томже DNSfixture (47unit,46PASS/1FAIL), артефактов нет. Кэш другойqa-ветки не наследуется; следующийWindows run вести наfix/youtube-delivery, где есть успешныйcache2cd. Причинаfixture воспроизведена отдельным задержанным стартом клиента1.1сек RED. Accept/read окна тестового HTTPfixture теперь6/5сек вместо1/1 при productionDOH5сек. DNS8testsGREEN6.02сек, productionграницы безизменений. Workflow concurrencyразделенапоplatforminput; Windowscache тожеrestore/savealways, чтобыне терятьbuildпосле позднегоFAIL. Это исправлениестенда, не заменаapplicationQA.

macOSe1d1run42067796897 выполняется (nativeCocoa вместоoffscreen); предшествующийrun42058290877 завершилсяSIGABRT-6 selected-proxyworker, noJSON/no crashreports; cacheMac1.5GB сохранён. Windows10QA VM videodock-platform-qa запущена4194304KiB, Win11off, interactiveuserесть. QGAhelper/tmp/videodock-qga.py работает; старый validation533PSсохранёнвгосте. HosttemporaryHTTP+CONNECT bridge/tmp/videodock-win-qa-http.py session53852, port/tmp/videodock-win-qa-port, lifetime1200сек. ACLguest99+hostgateway, CONNECTтолькоFacebook/fbcdn443, serverhealth/traversal/private-targetchecksPASS. Installerещёне получен. Не считатьпрошедшийWindows2cdinstallerверсиейсFBfix.

Уточнение пользователя: проверять ТОЛЬКО общедоступныевидео БЕЗвхода. Не открыватьaccount/private gatedсценарии; доступчерезloginне являетсяцельюпроверки. FBанонимныйPASSпо-прежнемуактуален.

Windows6b run42070142733 сноваFAILanonymous DNSfixture47PASS/1FAIL; задержанныйстартGREEN, поэтому оставшийсяотказнеобъяснёнтаймером. В9ecca13 добавленаONLYcfg(test) диагностикаsend/body/parse(stage,bool/len) ичистыйсчётчикfixtureзапросов, безURL/ответов/секретов; workflow послеfailureповторитbootstrapunitотдельновготовомtestexe. Productionошибкиосталисьредактированными. Нужен результатследующегоWindowsrun, не объявлятьrootfixedбезнего.

Macowncertificate подготовлен по сохранённомуправилу VideoDock Self-Signed, постоянныйP12+парольвзащищённойпапкевнеDropbox/репозитория; GitHubActions2signingsecrets переданычерезstdin, значениянигденевыводились. Importhelper используетэфемерныйkeychainпослеRustreleasebuild, rawP12удаляется,tempkeychaincleanupalways. Packagingcontract RED→GREEN проверяетnamedidentityдляQt ипереподписиbundle; missing-secretsfailclosedtestPASS. Это self-signed, неAppleDeveloperID/нотариат; nativeподписаннаясборкаещёнезапускалась. ТекущийMacrun42067796897 пока unsignedCocoaA/B, новыйnamedworkflowещёнезапущен, чтобысначалаподтвердитьoffscreenпричину.

Win10QA штатноshutdownпокасборкаготовится; Evalводянойзнакexpiredнаблюдался, параметрылицензии/времени/активациинеизменялись. HTTPqa-bridgeTTL20min, кфактическомутестусерверперезапуститьпослеготовогоinstaller, старыйportнеиспользовать. ВсеготовыеWindowsрезультаты533/2cdпокаНЕявляютсяFBфиксверсией.

Windows9ecca13 run42074387590: parallelbootstrapFAIL, безопасныйtrace `send failure timeout=false connect=false`, fixture requests=1. Тотже готовый exe/test отдельноPASS0.01sec; причинаосталасьвtransportстендаподнагрузкой, неclientdeadline. Новыйfixture пишетheaders+bodyоднимwrite, shutdown(Write) иожидаетpeercloseпристаромboundedreadtimeout; Linux8DNSGREEN. СледующийWindowsnativeparallelрезультатнужендопреждеподтвержденияпричины. Нельзябесконечноподниматьтаймаут/отключатьпараллельностьдлязелёногостатуса.

MacCocoa e1d1run42067796897: SIGABRTисчез, workerвозвращаетвалидныйJSON, CONNECTкзаданномупроксизамечен. Однакоstageнеhomepage, LLDBsyntheticclosedproxyпоказал stage=internal exit0. Новаядиагностикаprintтолькобезопасныеok/stage; LLDBonlyдляфиксированногоlocalhost:9requestпропускаеттолькологгинг-init, secureChromiumflagsпереданывenv, sandboxосталсявключён. Цель—увидетьпочемуQt renderer/helperпадает; такойdebuggerrunне считаетсяproductionPASS. macos-crash-diagnosticsпредыдущегоrunсохранён/tmp/videodock-macos-cocoa-diagnostics, полезногоbacktraceпока нет.

Windowscb3run42076780731: responsegracefulshutdownнеустранилparallelbootstrapFAIL (requests=1/send failure), отдельнототжеготовыйtestexePASS. ЗначитнельзяобъявлятьTCP-closeпричиной. ДобавленредактированныйDebugsourceтолькоcfg(test), безURL; нужноегоnativeсообщение. WindowsCIтеперьсохраняетQAкандидатдажеприunitFAIL (общийjobостаётсяFAIL, stablepublicationзапрещена), чтобыбылавозможнареальнаяVMпроверкабезожиданияремонтастенда. Не путатьcandidateartifactсрелизом.

НАЙДЕНАportablefixtureпричина: listener.set_nonblocking(true), acceptedSocketнепереключалсявblocking. RuststdWindowsacceptпростооборачиваетWinsockacceptбезсбросаFIONBIO; Microsoftacceptдокподтверждаетinheritproperties. Поэтомуподнагрузкойserverread_exactвиделWouldBlockдоHTTPheaders→закрывалsocket→client senderror requests=1; изолированноheadersуспевалиприйти. НаLinuxсмоделированWindowsрежимaccepted.set_nonblocking(true), новыйrawTCPclientзадерживаетheaders30мспослеподтвержденногоaccept: RED0.05s. Исправленaccepted.set_nonblocking(false)сwhycomment, новоеdelayed-headersиостальныеDNSтестыGREEN. ВременныйacceptedtrueНЕкоммитить. НуженещёнативныйparallelWindowsPASS.

Mac758run42080205939 остановилсянаunitдоsigning: тотжепараллельныйDoH senderror, теперьточныйsanitizedsource `hyper::Error(Io, Os{code:54,kind:ConnectionReset,"Connection reset by peer"})`, fixture requests=1. Это согласуетсяснеблокирующимacceptedсокетомнаBSD. Импортсертификатаэтимrunещёнепроверялся, поэтомунеписатьsignedPASS. ПослеfcfявногоblockingнуженновыйMacrun.

Win10fcfactualinstallerPASS: install2/upgrade/uninstall/profile/reinstall/cleanPATH, installedexe7f03abd0...; currentFBGUIчерезограниченныйQAproxy12474451B44.330667sVP9 1080x1080+AAC, bundledFFprobe/fullFFmpegprogress=endPASS. Данныевresearch/windows-release-fcf0427/windows10-{result,media-result,decode-progress}. QAдискосвобождёнcargo clean --profile dev ONLYC:\VideoDockQA\target\debug (12.1GiB), профили/видеосохранены. QMPHIDнужныотдельныеinput-send-event дляbuttondownиup, иначеодинUSBsyncсхлопываетclick. SetClipboardвhostvalidationбылошибочнопослеthrowвcatch, исправленhostскрипт; отдельныйuserSessiontaskVideoDockQA-LinkподалпубличныйURL, реальныйGUIinputпрошёл. НативноезначениеfilepathC:\Users\Nrzeronxer\Videos\VideoDock, неprofile/downloads.

Macfcf42083105499: unitPASS, signidentityimportPASS, bundle+FFmpeg+codesignPASS, workerпо-прежнемуokFalse/internal, releaseнепубликовать. LLDBthread-returndiagnosticнепринёсQtлогов (deadline), неявляетсяуспешнойпроверкой. Следующийdiagnosehelperсобираетactualbundlemanifest/helperlibraries/entitlements/systemerrors+zip.app (никакихuserprofiles/siblingkeychain); нужендляреальногоразбораrenderer. СобственныйMacвTailscaleoffline, hardwareGUIтестнедоступен.

LinuxinstalledcurrentYouTubezooPASSсactualc901worker, realGoogleproof/SABR742369transfer/744360output19.063583sfull decode. Это13-йсвежийисточник(включаяGeneric); Vimeoвсе6advertisedHLSurl/avc/fallbackдвухCDNпрямоподтвержденыSAMPLE-AES KEYFORMATcom.apple.streamingkeydelivery, незашифрованныйпотокненайден; аккаунтынеприменять.


## Возобновление REA 05.11, macOS renderer

- [x] Восстановлены свежие TASK/git/CI: HEAD fc77f65, Windows native fcf0427 PASS; macOS fc77f65 run42093840131 FAIL на selected-proxy stage=internal.
- [ ] Прочитать сохранённые helper libraries/signatures/system errors и диагностический app; установить причину renderer.
- [ ] Минимальное исправление с regression check и нативным подтверждением.
- [ ] Свежая упаковка/матрица перед публикацией; Draft не выпускать до macOS gate.

В дереве уже есть незакоммиченные YouTube/QML/appearance изменения; этот проход их не перезаписывает. Диагностика fc77 загружается в /tmp/videodock-macos-fc77-diagnostics, failed log /tmp/videodock-macos-fc77-failed.log.

macOS причина подтверждена actual system log: dyld не находит QtWebChannel по @executable_path/../Frameworks внутри nested QtWebEngineProcess. Helper также сохраняет absolute Homebrew load commands. Реальные 4 MachO из fc77 archive: LLVM переписал23команды, все38bundledlinks разрешаются; codesign/runtime наLinux НЕ проверялись. Скрипт repair-macos-frameworks переводит load dependencies в @loader_path относительно каждого бинаря, сохраняет helper entitlements при переподписи и пересобирает nested seals. Полный offline scanner дополнительно выявил13отсутствующихQt frameworks у QMLплагинов; установщик добавляет Qt rpaths и второй macdeployqt pass для графа уже обнаруженных плагинов. Regression installer сначалаRED(skipped repair), затемGREEN; framework dependency checks/packagecontract/sh-n/diff-check PASS. Нужен nativeMacrun, release всёещё Draft. Доказательство ~/.local/share/videodock/research/macos-frameworks-2026-11-05/offline-links.json.


## Скриншот пользователя: TikTok audio, 05.11

- [x] Найдена ошибочная job553056b1; ffprobe входного stream-0-bitrate-0: HEVC720x800148.866667s БЕЗaudio.
- [x] Публичная hydration того же ролика: bitrateInfo Format=dash video-only, отдельный bitrateAudioInfo с AACMP4 источниками. Парсер ранее помечал всеvideo какmuxed и игнорировалaudioсписок.
- [x] RED→GREEN parser/selection regression: DASHvideo без audio; bitrateAudioInfo становится отдельными audioformats.
- [ ] Полная загрузка именно этого ролика/ffprobe/full decode, актуальная сборка и проверка приложения. Профиль/история/готовые файлы сохраняются.


## Расширенный аудит по просьбе пользователя, 05.11

- [ ] Выбор форматов/раздельный звук/контейнеры/субтитры: regressions+реальные файлы.
- [ ] Сеть/таймауты/HTTPиSOCKSproxy/ошибки/неполные файлы/cancel+resume.
- [ ] Очередь/профиль/перезапуск/некорректные настройки/пути и конфликты имён.
- [ ] UIгеометрия/ошибки/повторнаязагрузка на актуальном Linux.
- [ ] Native Windows unit/build/installer/runtime и проблемный ролик.
- [ ] Native macOS unit/build/codesign/worker/runtime; текущийrun42104482583.
- [ ] Security/dependency checks и итоговая матрица PASS/FAIL/BLOCKED; не обещать отсутствие всех возможных ошибок.

06.11 возобновление: TikTok userreported полный файл6947649B/148.886349s/HEVC720x800+AAC; full pipeline+decode PASS05.11, freshffprobe06.11PASS. Файл research/tiktok-audio-2026-11-05/TikTok-adaptive-audio.mp4. Исправление НЕ установлено в desktop. macOSrun42104482583FAIL доупаковки: add_rpath исчерпал MachOheader. Вdesktop/build.rs добавленmacOS-only -Wl,-headerpad_max_install_names; нуженfreshnativebuild. Core/media полный regression запущен с1GiB/noSwap/jobs1, постоянныйлог research/audit-2026-11-06/core-media-tests.log. macOSfailedlog пока/tmp/videodock-macos-dc33548-failed.log; diagnostics сохраненыresearch/macos-frameworks-2026-11-05/dc33548-diagnostics. Продолжить: закончитьlocaltests+Clippy+RU/ENQt, собратьSDKLinux/установитьатомарно сrollback ипроверитьTikTokGUI; commit/pushcurrentchanges иnativeMac/Windows, реальныеinstaller/runtimeQA, расширенныйаудит. StableDraft не выпускатьдозакрытияgates.

06.11 localaudit: core/media209PASS0FAIL47ignored; Clippycore/mediaall-targets-DwarningsPASSпослеread_exactвDNSfixture (старыйClippyREDсохранён); DNS9PASS. QtappearanceEN/RU+realEnginePASS (темы,5tabs,9dialogs,menus,narrowwindow,localdownload,ffprobe,restart,history,safe removal,newjoborder). Cargo-audit0vulnerabilities, fxhash0.2.1unmaintained; gitleakscurrentdiff0. Независимыйrequesting-code-review агент: подтверждённыхважныхдефектовнет, обаPythonpackagingchecksPASS; nativeMacruntimeостаётсянепроверен. SDKLinuxbuildзапущен scripts/rust/build-flatpak.py сactualcgroup1GiB/swap0/jobs1, log research/audit-2026-11-06/linux-sdk-build.log. HostVMoff. CUA SkyработаетпослеDISPLAY=:1+userDBusenv, VideoDockwindowsсейчаснет. ДалееnativeMac/Windowscurrentcommit.

Source3cc194cf704c73993a290df770bde66294bf0d8c pushed. NativeMacrun42174843408 иNativeWindowsrun42174846909 запущены; watchlogs сохраненыaudit-2026-11-06/{macos,windows}-watch.log. ЭтиjobsвключаютTikTok,Shorts,queueorderingиheaderpadding; прежние533/fcfinstallerнеподменяютновыйбинарь.

06.11 installedLinux3cc194c GUI PASS: binaryccab00cd10091decfdf9006c7be76a4d6544e0d4176993bc9b719cdb58b03752, rollbackaudit-2026-11-06/installed-before-audit. РеальнаякнопкаPasteLink→publicTikTok→complete9d4a475f, MP46947936B148.886sHEVC720x800+AAC/full decodePASS; native-tiktok-proof.json иscreenshot. File→Выйти штатно закрылокно; restartсохранилcompleteифайл. QAпрофильотдельный. NativeCuaтребуетDISPLAY=:1,userDBus,QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1. Обучение3Next+FinishPASS. Filechooserнаскрытомworkspaceнеудержался, егоnativeflowНЕPASS; outputpathзаданONLYвQAfixture. ClipboardфоновыйVoxFlowпереписываетmarker, wl-copyдочернийdaemonдержалstdoutpipe; временныйQtX11providerустранилпомехудлянастоящегоPasteLink; исходныйclipboardвозвращёнбезвыводаданных.
Windows3cc194c run42174846909SUCCESS; artifactdownloadedSHArecorded. Windows10VMvideodock-platform-qa4GiB: ordinaryuserinstall2/upgrade/uninstall/profile/reinstall/cleanPATHGUI/hashMATCHPASS. QGAhelperиtransferрезультатыaudit-2026-11-06. QXLчёрныйscreendumpпочиненподключениемvirt-viewerнаworkspace90silent+обычнымредроу; OSactiveconsole,непроблемаVideoDock. ActualWinGUIонбордингSkipPASS, CtrlVсpublicTikTokвыполнен; дождатьсяcompleteиFFprobe/full decode. Transferserver39737, TTL1800с, whitelistACL99/gateway; остановитьпослеQA. Macrun42174843408tests/build/importPASS,packagingвработе.
Новыеаудиткандидатыдляследующейпачки: confirmedinvalidURLпоказываетrawEnglishrelativeURLerror; hiddenclipboardFieldпоказываетегоValueвAX (наsyntheticmarker). Исправитьнапонятнуювалидацию+очистку/Accessible.ignored, regressionдоизменений. Portraitqualityпокаstaticinference: selectionсравниваеттолькоheight; воспроизвестина1080x1920vs720x1280преждеисправления. Не считатьэтигипотезыготовымиfix. HostbuildsнепараллелитьсQA VM; новыеcore/desktopфиксысобиратьпослееёштатногоshutdown.

06.11 source3cc194c nativegates: Macrun42174843408SUCCESS (tests/build/selfsignedcodesign/FFmpeg/8IPC/proxyrefusal/badproxyUIstartup), Maczip195MiB сохранёнaudit-2026-11-06/macos-artifact. ПолныйdownloadнаMacещёНЕпроверен. Windows10installerреальныйGUIpublicTikTokdirectPASS:6947936B148.886349sHEVC720x800+AAC,bundledFFprobe+FFmpegполныйdecode0stderrbytes; файлwindows10-tiktok-proof.json. NativeClose+restartсохранилmedia, File→ВыйтиPASS; QMPmouse требуетhold75ms(безнегоpress/releaseможетсхлопнуться). WindowshelperstdoutпереведёнвUTF8послеREDискаженияCyrillicpath; приложениеUnicodeфайлсоздалоуспешно. VMштатноoff,scheduledQA taskremoved,transferTTLистёк,viewerзакрылся.

СледующаяаудитпачкаподтвержденаRED→GREEN: malformedURLguidance/inputwithoutEcho; portrait1080x1920приlimit1080былошибочноотброшен; manual/finalquality720x1280был1280p. ОбщийFormat.video_quality()используеткороткуюсторону,реальныедимензиисохранены. FileMenugeometryRED→AppMenuадаптивнаяширинаGREEN; invalidclipboardRED→ручнойввод,clearhiddenbufferиAccessible.ignoredGREEN; RUplural1/2/5/21RED→numerusTS+QM GREEN. Core/media212PASS0FAIL47ignored,Clippyalltargets-DwarningsPASS,EN/RUfullQt+realEngineGREEN. Reviewagentновыйboundedpassno importantfindings, обеofflineMacpackagechecks,YAML+11Bashblocks parsePASS;currentdiffgitleaks0.
Новыйjobmacos-downloadsпослеMacpackagingпроверяетappнаfreshnativeIntelrunnerБЕЗQt:готовыйbundle+RustcoretestsTikTokиYouTubeполныефайлы/probe/decode,rc/logs/filesartifact;jobнеподменятьстатическимPASS. Пинdownload-artifactv8.0.2проверенпоofficialGitHubrelease/ref/action.yml,node24. НужныfreshnativebuildsисвежаяLinuxустановкаэтойпачкидоотчётаоcurrentOSPASS.

06.11 дополнительнаяYouTuberootcause: installed699nativeGUIanonymousZooLOGIN_REQUIRED, но тотжеworker+standalonedirectadapter ZooPASS17.05smetadata/742369transfer/744360file19.063583sfull decode. AppавтодетекторвыбиралнеVPN,аслужебныйsing-box (cgroupanydesk-quattro.service1081;ai-proxy.service7899тожеошибочныйкандидат). Вmemorysplit-tunподтверждёнзапретиспользовать7899внеAI; hostконфигиНЕменялись. Удаленаэвристикапоименампроцессов/слушающимSOCKSсокетамцеликом(200+LOC); OS/env/manualproxyпрежние,explicitfailclosed/authнеослаблены,TUN/zapretчерезOSdirect. Удалены3bootstrapтестатолькоустранённойфичи; core36PASS+proxy8PASS2ignored,ClippyalltargetsPASS,gitleaksdiff0,independentreviewno importantregressions.
Flatpakorg.freedesktop.Flatpaktalkудалёнвworkingtree:единственныйнашhostspawnбылss. Реальныйinstalledworkerс--no-talk-name=org.freedesktop.Flatpak→selectedlocalproxyrefusalPASSexit0/homepage,renderer+QtSandboxсохраняются;flatpak-no-host-bus-proof.json. ДоcommitнужныactualdefaultGUIretryпослесборкииобновлённаяупаковка. Native699Windowsrun42186105032SUCCESS;Mac42186101264вработе(проверитьliveAPI,watchможетустареть). Nativepackaging/публичныйMacdownloadjob699ещёнеобъявлятьPASS.

06.11 routefixLinuxSDKreleasePASS2m27s (workingtreeonlynetworkfallback+metadata+docs). ПродолжитьcloseQA699→installnextbinary→retryfailedZooUIdefaultбезизмененияproxysettings;actualHostprivatehelpersостаютсяrunning. Core36+proxy8/0fail,ClippyPASS;reviewno importantregression. StandaloneFlatpak--no-talkFlatpakbus negativeproxyPASS(Homepage),отсутствиeQtзависимостиотhostbusподтверждено. НужноcommitroutefixпослеGUIpass, затемfreshnativeOSgates,packagewithoutHostExecbusиfullFlatpakflow. Старые3bootstrapтестыудаленывместеспортовоюэвристикой,неexplicitproxy/auth/failclosedпроверки.

ВАЖНОЕУТОЧНЕНИЕ06.11посленативногоповтора:текущийGUIYouTubeотказНЕдоказанкакследствиеAnyDesk/AIэвристики. GNOMEproxy mode=manual,HTTPS/HTTP127.0.0.1:10809;этотявносистемныйHappимеетприоритетраньшеавтопоискапроцессов. Удалениеэвристикибылоотдельнымправильнымhardening,нонеустранилотекущийотказиз-зауваженияOSproxy. UsercardreplyпрямоавторизовалVideoDockdefault«ПрямоймаршрутОС/zapret». ТеперьproxyEnabled=false→None/strictdirect,включённыйручнойproxyнеизменён. Никакиеhost/proxy/TUN/zapretнастройкиНЕменялись. Удаленыnetwork.rsиwindows.rs,неиспользуемыйCacheиtupleproxy_policy;политикапроксиводнойобщейфункцииsecrets::proxy. ПодписьSettingsзамененанаDirectconnection(OSroute),TS/QMобновлены.
DirectdefaulttestREDнастаромповеденииSomevsNone;GREEN8proxychecks,вт.ч.directHLS+captions+duration/probeигнорируетambientHTTPиSOCKS/auth,explicitmanualLibraryAutoOptInсохранены,failedexplicitневыходитdirect. Полныйcore/media207PASS0FAIL47ignored(5retiredautodiscoveryunitcasesудаленывместеудалённойфичей),ClippyPASS. ПередфинальнымcommitнужныactualGUIrestart/retryиобновлённыйFlatpak/crossOSbuild.
Mac699run42186101264:macospackagejobSUCCESS;newstandalonejob(nativeбезQt)TikTokPASS6947649B148.886s+full decode;YouTubeFAIL101LOGIN_REQUIREDafter63.33s. ЭтоНЕкрашбиблиотек/подписи,ноYouTubeнаMacнеPASS. Artifacts macos-public-699/{result.json,tiktok.log,youtube.log,files};freshMacTailscaleoffline06.11 13:41. Windows699native42186105032SUCCESS,currentinstallerещёненативнотестированвVM(3ccinstallerGUIпрошёл).

06.11 usercardexplicit: defaultVideoDock«ПрямоймаршрутОС/zapret»,otherappsettingsне менять;Macпоканедоступен(непроситьповторно). DefaultproxyEnabled=falseтеперьстрогоNone/NoProxy. УдалёнвесьOS/env/processCacheNetworkmodule+Windowsbackend(неиспользуемаяветка),сохраненыmanualproxyиexplicitopt-inCatalog.new_automaticAPI. No-newoptions/schema/migrations. DirectdefaultREDoldSomevsNone→GREEN207core/media/0FAIL/47ignored;proxy8PASSinclcompleteHLS+captionsdirectunderambientHTTP/SOCKS,manualauth/failclosedсохранены;ClippyPASS.
LinuxSDKrelease3m14sPASS,installedbinary1b3fb457748ef4fbff5074a391e761dd43bfd3e4ace3ea685793ef4556e70907;backupinstalled-before-direct-default. ACTUALSAMEfailedZooGUIresume→complete100%,742369transfer/744478MP4/H264320x240+AAC/19.063583s/full decode0stderr,proxysettingFalse,globalHapp/gsettingsstillmanual10809unchanged. native-direct-recovered-youtube.{json,png} savesproof. Thisclosesactualdirect-modeappfailure;oldinferenceAnyDeskcausetextaboveissupersededbythiscorrection.
Pendingfinal:commitcurrentdirectpolicy,updatedSDKFlatpakwithoutHostbus+install/runtime;freshWin/Macnativecurrentcode;Win11latestinstallerflowifinteractiveaccessavailable. Mac699build/QtMintworked/TikTokfullPASS;MacYouTubeLOGIN_REQUIREDnotPASS,ownMacofflineuserconfirmed. Stablev0.2.0DraftdoNOTpublishascompleteparity.

## Контрольная точка 06.11: исходник 3171688

- [x] Прямой маршрут ОС по умолчанию выбран пользователем и установлен; глобальные Happ/TUN/zapret не менялись.
- [x] Полный core/media: 207 PASS, 0 FAIL, 47 ignored; Clippy all-targets -D warnings PASS. Доказательства direct-final-{core-media,clippy}.log.
- [x] Чистый Linux executable 1b3fb457748ef4fbff5074a391e761dd43bfd3e4ace3ea685793ef4556e70907 установлен атомарно; резервная копия сохранена.
- [x] Flatpak без org.freedesktop.Flatpak talk собран и установлен. Обычный GUI завершил TikTok (HEVC+AAC, 148.910998с, 6947928Б) и YouTube Zoo (H264+AAC,19.063583с,744478Б); полное декодирование внутри контейнера exit0/stderr0. Proof JSON в audit-2026-11-06.
- [x] Flatpak с обычным Wayland backend открылся, сохранил три completed jobs. Скрытый workspace давал старый растянутый buffer; существующий hires helper вызвал настоящий rerender. Временный монитор удалён; геометрия/рабочие столы восстановлены. Полную загрузку именно через Wayland этим не подменять.
- [x] Windows CI 42207833852 SUCCESS. Latest installer/exe SHA записаны в windows-expected-3171688.json; installer165392286Б, exeB1872D4FFF0344E05DBA4D103CDCCACF8C6AD68FA53DE1482D2C0DE00FEFBD81.
- [x] Windows10 ordinary-user installer2/upgrade/uninstall/profile sentinel/reinstall/cleanPATH GUI PASS. TikTok через GUI complete6947936Б/148.886349с HEVC720x800+AAC; bundledFFmpeg full decode exit0/stderr0. Restart той же версии сохранил4completed jobs; proxyEnabled=false.
- [x] Mac317 native bundle/codesign/worker PASS (job macos run42207830501). Fresh runner безQt: TikTok full download/decode PASS; YouTube FAIL101 с запросом подтверждения доступа. Evidence macos-public-3171688/result.json иlogs. Это не PASS всегоworkflow и не проверка личногоMac.
- [ ] Свежая Windows11 installer/user-flow: при восстановлении QA VM чёрный framebuffer и отсутствует QGA. Штатный ACPI shutdown завершён; прежний installer53304e2 PASS не заменяет317.
- [ ] Личный Mac/YouTube на его маршруте: пользователь ответил «Mac пока недоступен», повторно не спрашивать.
- [ ] Остальная сервисная матрица/длинные ролики/cancel+resume: прежние результаты учитывать с их исходником и границами; не объявлять исчерпывающим аудитом всех ошибок.

Текущий отчёт: docs/audit-2026-11-06.md. Релиз остаётся Draft. Диагностический SDK executable с логированием транспортных классов НЕ установлен/НЕ упакован; исходник восстановлен, production hashes выше чистые. При следующей SDK сборке требуется обычный rebuild восстановленного исходника, нельзя брать диагностический target как release.

Дополнение той же проверки: Windows10 latest317 YouTube Zoo черезGUI direct завершён после restart:744478Б/H264320x240+AAC/19.063583с, bundledFFmpeg full decode0/stderr0; windows10-youtube-3171688-proof.json. File→Выйти закрыл именноQAпроцессы, scheduledtask удалён, Windows10 штатноoff; временный transfer39647 остановлен. НативноеLinuxиFlatpakQAокна закрыты штатным compositorclose, чужиеокна не трогались.
Windows11 повторный ОБЫЧНЫЙboot после завершённогоACPIshutdown показал lockscreen вместоblackbuffer. Это supersedes предположение о постоянномblackscreen: для дальнейшегоQAнуженлокальныйвход. Плашка отправлена, парольнепросили/нечитали. QGAканал остаётсянедоступен, понадобится прежний ограниченныйлокальныйtransfer после входа. Не сбрасыватьпароль/учётку/лицензию. ДоответаVMоставленанаlockscreen.


## Проверка всех ОС и выпуск 0.2.0 - 07.11.2026

Что сделать: исправить воспроизводимые ошибки на всех ОС и опубликовать проверенные пакеты. Объём: существующий продукт, нативные CI runners; без тяжёлых сборок и ВМ на личном Mac. Проверка: workspace/Qt/installer/package tests, реальный worker, видимое окно, контрольные суммы и публичный релиз.

- [x] Исправлены десять ошибок: install rollback, stale Windows DLL, audioTracks export/import, subtitle collisions, rollback outputs, Windows Unicode paths, Unix child reaping, ManualDialog modelData, browser address focus, Mac nested signing identifier.
- [x] Регрессии, RED/GREEN и независимое ревью исправлений; архивы закрытых ресурсов оригинала в поставку не включены.
- [x] Windows x64 42385130933: workspace/staging/installer/clean PATH worker/UI EN+RU прошли,130 PNG. Public TikTok PASS; YouTube/Shorts controlled metadata confirmation.
- [x] macOS Intel/ARM 42395354408: workspace/UI EN+RU/bundle/signature/clean PATH worker/startup прошли, по130 PNG. Public TikTok PASS; YouTube/Shorts controlled metadata confirmation.
- [x] Готовые Mac-архивы проверены локально по SHA256 и strict deep signature. На ARM вживую проверены обучение, настройки, сохранение языка, настоящее меню и повторное раскрытие браузера при сохранённом фокусе.
- [x] Linux native 42394901843: workspace/UI/worker/startup и архив прошли.
- [x] Flatpak того же прогона собран внутри KDE SDK6.11; package/install/ffmpeg/ffprobe/IPC прошли. Первичный proxy test остановился в Qt portal Settings при инициализации runner.
- [x] Причина Flatpak CI подтверждена стеком; на неизменном пакете с настроенным GTK portal proxy/startup PASS (42400855612),15s и sandbox сохранены.
- [x] Подтвердить видимое окно именно запущенного Flatpak instance и verification.json: финальный run42403231286 SUCCESS, artifact13585988657, SHA пакета069cfc38e43638f797525af209b345b2f1fcdf6400c13747978a97d788128d42. Прежнее окно было видно, но harness ошибочно требовал один global instance; проверка теперь связана с деревом Popen и NSpid.
- [x] Шесть файлов в dist/0.2.0, sizes/SHA256 сверены с Actions artifacts; старые Linux-кандидаты сохранены отдельно.
- [x] Publisher связывает producer/verifier/job definitions/artifact ID+digest/package SHA; проверены34 rejection cases и прежние6 tag/draft mocks. Никакого исполнения скачанных файлов или delete/clobber.
- [x] На реальных GitHub metadata проверены native Windows/Mac/Linux gates и эквивалентность runtime исходников.
- [x] Завершены release notes, отчёт всех ОС и README; документы опубликованы вместе с исходниками.
- [x] Опубликован стабильный v0.2.0 (publish run42404154016 SUCCESS): Windowsrun42385130933, Linuxrun42394901843, Macrun42395354408 и новый успешный Linuxverificationrun.
- [x] Проверены публичный tag/source SHA cc9f392440684acd57ca2c62c9bc46ccb3defe52, все 8 assets и server-side digests; manifest/SHA256SUMS скачаны без авторизации и сохранены рядом с 6 пакетами. Все 7 строк SHA256SUMS прошли проверку.

Ограничения: YouTube доступ из CI не считается успешным скачиванием; собственная Mac подпись без Apple notarization, Windows installer без Authenticode. Полная ручная матрица всех дистрибутивов/железа не заявляется. Подробности - docs/cross-platform-audit-2026-11-07.md.

Публичный выпуск: https://github.com/Nezeronxer/videodock/releases/tag/v0.2.0 . Он отмечен Latest, draft=false/prerelease=false. Локальные файлы: dist/0.2.0; build-manifest SHA2565cecc7c4dd49e2ba748fda67903b8af380f5737e7fec7a3fd9ed0cf1e6d8eb50.


## Исправления интерфейса Mac после установки - 07.11.2026

Что сделать: иконка Dock, правильные пропорции изображений и значок платформы у Paste Link по буферу при активации. Объём: точечные изменения существующего UI/упаковки, без тяжёлых сборок и ВМ на личном Mac. Проверка: failing/passing packaging contract, Qt UI regression, сборка в CI и проверка установленного окна с сохранением пользовательского профиля.

- [x] Подтверждено отсутствие CFBundleIconFile/ICNS и постоянный icon.source кнопки.
- [x] По снимку уточнены логотипы сервисов стартового экрана; Qt regression воспроизвёл Service logo is stretched: browserServiceIcon-youtube. Причина: сохранение implicitWidth/implicitHeight меняло sourceSize между чтениями, Image.Stretch искажал результат при масштабе >1.
- [x] Packaging RED→GREEN: ICNS + CFBundleIconFile. Clipboard RED→GREEN: общий platformIcon, чтение только при активации/старте, hidden buffer очищается, логотип без перекрашивания. Логотипы помещаются в квадрат с PreserveAspectFit и ограничением только sourceSize.width.
- [x] Локальные проверки и повторный CI42409767775: ARM/Intel UI PASS; ARM bundle/signature/clean-PATH worker/startup PASS, artifact13590316545 получен.
- [x] Обновлён /Applications/VideoDock.app, проверены пользовательский профиль/история, логотипы и YouTube у Paste Link в настоящем Cocoa окне.

Локальные regression: packaging missing Dock icon RED → PASS. Qt logo aspect RED на YouTube → PASS при том же appearance.rcc. Полный Cocoa harness нельзя совмещать с управлением Chrome: desktop focus влияет на Cmd+V. Проба размера1700x1000 на физическом Mac вывела окно за экран и сломала pixel-check onboarding; новую проверку оставили при реальном исходном размере.

Исходники/регрессии сохранены в GitHub fix/macos-interface, commit2f30847cb70ad7f8c8f763d9ac95aac800dad061. git push недоступен: credential helper ссылается на отсутствующий ~/.local/bin/gh; применён штатный GitHub MCP create_blob/tree/commit/branch. Локальный полный UI прошёл все UI-only разделы; optional Engine section остановился из-за отсутствующего sample argument, не из-за продукта. Для повтора создан локальный sample.mp4.

Локальный финальный Qt/Cocoa EN+RU: PASS, оба с настоящим ui_engine/sample MP4, сохранение истории, повторная вставка, restart, ffprobe. EN transient second-paste failure не воспроизвёлся при одиночном прогоне; desktop automation не совмещать с Cocoa harness. Packaging contract PASS;7 install staging tests PASS; manual checkbox and shell/diff checks PASS. Логи/снимки ~/.local/share/videodock/research/interface-fix-2026-11-07. CI run42406213413, Mac ARM job128504361811, Intel128504361958, source2f30847.

CI ARM cache restore сообщил Cache not found для native-macOS-ARM64-1.99.1-fb5c65d9d22aa814340cfdde2757d33587cb0dca7472db2d8dc083e4520514bc; полная холодная компиляция. На13мин общего времени cargo step7м, последние строки Compiling cxx-qt-lib/videodock-desktop. Не запускать вторую параллельную сборку без причины.

CI ARM workspace tests PASS, началась Build native release before importing signing identity; Intel ещё workspace. Все исходники2f30847 неизменны после локальных EN/RU PASS.

ARM2f CI workspace+release PASS, UI FAIL: Service logo did not render в обоих языках; подпись/упаковка SKIPPED. Причина локально воспроизведена БЕЗ appearance.rcc: новый QImageReader тест трактовал относительный icons/service-*.svg как qrc. При rcc источник абсолютный, поэтому локальные EN/RU PASS это не покрыли. Тест исправлен через QQmlEngine::contextForObject(image)->resolvedUrl. Продуктовые исходники не изменены. Нужен новый CIcommit/run с прогретым ARMкэшем.

Тест URLfix commit8ed02c3f82daf91d27417492aff1b5fe2c9e2772 в fix/macos-interface. Локальный БЕЗrcc полный UI-only EN PASS; сrcc ранее EN/RU+Engine PASS. Повторный workflow пока НЕ запущен: concurrency native-branch-platform/cancel-in-progress=true отменил бы незавершённую холодную Intel сборку до сохранения её кэша. ARM cache уже saved. Дождаться Intel cache save в run42406213413, затем dispatch macos на8ed02c3. В Chrome retryTab форма открыта с выбраннойветкой; финальнаякнопкаещёне нажата.

Intel холодный release тоже PASS, перешёл к UI step. Полный локальный UI безrcc после resolvedUrl fix: PASS (en); лог interface-fallback-en.log сохранён рядом с остальными доказательствами.

Первыйrun42406213413 завершёнfailure из-за неверного пути в тесте; ARM и Intel cache save SUCCESS. Повторныйdispatchmacos на8ed02c3 отправлен после сохранения обоихкэшей. Продуктовый исходник тотже2f30847.

Повторныйrun42409767775 запущен23:10MSK07.11 на8ed02c3. Браузер послеdispatchдваждыдавалPage.getFrameTreetimeout;reload того же boundtab открывает уже правильныйrun. Не повторятьdispatch из-заэтоготаймаута. ТекущийretryTab показываетrun42409767775.

Повторныйrun42409767775: ARM+Intel workspace/release/UI EN+RU PASS. Идёт step15 Build and verify application bundle. ARM journal macdeployQt содержит Cannot resolve rpath QtStateMachine(Qml), пока нет результата. Сравнена предыдущая Mac42395354408/ARMjob128471438861: step15 SUCCESS, duration20m42s (весьjob failure толькоиз-заpublicdownloads step21). Долгая упаковка сама по себе не означает зависание; не ломать проверку из-за этих сообщений до её результата. В Chrome retryTab открыттекущийARMjob, historyTabпредыдущий.

Финальная установка: artifact13590316545 SHA256397384251b1b8169b647155243f3f8dca789f39b5e905a4726bbd2ca4843aa7c совпал. Archive boundaries, ARM64, strict/deep codesign, CFBundleIconFile/ICNS (SHA256e0edf35036d15548c42e2f84925ef83ea17d4faeecce500e13b1d6f44895b293) PASS. Пакет установлен через stage+rename+rollback, LaunchServices refreshed. Старый app и SQLite backup: ~/.local/share/videodock/research/interface-fix-2026-11-07/backup. SQLite bytes не менялись при замене.
Native CUA: после обновления сохранились1completed job, RU/dark, smartMode,1080p,MP4/Movies. В раскрытом стартовом браузере все логотипы без вытягивания. Через AX setValue→freshAX→CmdA/C скопирована публичная YouTube URL (без перехода/скачивания); полный quit/reopen показал красный YouTube у Paste Link. Простая batch typeText при одновременном раскрытии поля теряла символы, для QA использован проверенный AX setValue.
Предел проверки Dock: файл ICNS/CFBundleIconFile, подпись и регистрация проверены; доступ к системному Dock через CUA (bundle ID и app path) оба раза timeoutReached, сам Dock визуально не подтверждён.
Ограничения CI: ARM UI+bundle PASS, весьjob failure на последующем Complete public macOS downloads (как и у предыдущего42395354408); этот исход не выдавать за полныйCI PASS. Транспортные исходники в этой правке не менялись. На личном Mac публичные загрузки не повторялись. Intel UI PASS, упаковка ещё выполнялась при установке ARM. Stable assets0.2.0 не перезаписывались: установлен исправленный Mac build8ed02c3, исходники fix/macos-interface.


## Исправления по скриншоту - 09.11.2026 (Arch)

Результат: устранить YouTube SABR error и TikTok missing audio, доставить общие функции (включая значок платформы буфера) на Linux/Windows/macOS. Объём: общий Rust/Qt код и поставка. Проверка: регрессии, полные проблемные загрузки с ffprobe/decode, Qt EN/RU, сборка и нативный Windows QA.

- [x] Восстановлены источники, ошибки реального профиля и доступные Windows QA VM.
- [ ] Воспроизвести TikTok и YouTube на проблемных публичных ссылках.
- [ ] Исправить подтверждённые причины и добавить регрессии.
- [ ] Проверить общий UI буфера и обновить Linux с резервной копией.
- [ ] Собрать и проверить Windows; отдельно отметить фактические версии ОС и ограничения.
- [ ] Сохранить доказательства и итоговый отчёт.

09.11 текущая диагностика: TikTok user-reported full download PASS 6947649 B / 148.886 s / video+audio / full decode. YouTube monitor RINzuNBpPpk и контроль Zoo воспроизводят remote code 2 на первом SABR request до сегментов. Ослабление media_capabilities не помогло и отменено; audio_track_id=69 тоже не устранил отказ, пока эксперимент, не готовый fix. Нужна общая причина отказа SABR, не править повторными retry. Доказательства: ~/.local/share/videodock/research/fixes-2026-11-09/current.

09.11 после диагностики: monitor360p дважды полностью загружен и декодирован на исходной заявке, эталон googlevideo5.0.1 также получил оба потока. Изменения capabilities/audioTrackId отменены. Точная причина временного отказа внешнего сервера не установлена; подтверждён дефект обработки известного server code2/no_available_formats как окончательной ошибки. Точечный fix переводит только эту пару type/code в существующий ограниченный retry (до2повторов), с прежними rollback/cancel/проверками полного файла. Новый regression RED→GREEN; core/media215PASS0FAIL48ignored; Clippy alltargets-DwarningsPASS; gitleaks diff0. Финальные live3YouTube, Linuxrelease/UI и nativeWindows/Mac ещё выполняются/ожидаются. Диагностический дамп с временной proof/session удалён; секретов в diff нет.

09.11 расширенная проверка: monitor360 PASS; shorts1080 и watch1080 FAIL даже с bounded retries, поэтому не закрывать. В googlevideo5.0.1 src/core/SabrStream.ts/selectFormats подтверждено: SABR больше не поддерживает video-only; наш single-track transport отправлял enabled_track_types=2, пустой selected_audio и audio capabilities. Сейчас workingtree передаёт выбранную audio Delivery в video request, mode0+обе capabilities; audio-only mode1 сохранён, companion session проверяется Arc::ptr_eq. Delivery/ABR добавлены audio_track_id(69),drc(46),voice_boost(76). Live short paired test session24818 ещё идёт, не считатьPASS. Локальная release build session32773 остановленаSIGINT с сохранениемcache. Ветка fix/download-recovery commit3f04a2e запушена, но CI ещё НЕ запущен, текущие paired изменения НЕ committed. Временный /tmp/videodock-sabr-oracle/shorts-input.json содержит ephemeral proof: удалить до завершения, в чат/репо не выводить.

09.11 сеть: paired Shorts через явный SOCKS1080 полный PASS24130123B/52.594s/1080x1200+Opus/full decode. Direct paired+CDNfallback получил audio888120B и video8122498B/18s, затем connectfailure после106.65s: этоPARTIAL, неготовыйфайл. Core/media217PASS0FAIL48ignored, включая новые CDN/foreign-host/counter tests и parser flags. Проверяется последний ограниченный вариантHTTP1 дляSABR (как у имеющегосяmetadata transport), source youtube.rs+1; еслинепоможет, отменить. ВременныеoracleJSON/binудалены.

09.11 directHTTP1Shorts PASS:24130123B/52.594s/1080x1200+Opus/full decode, transfer24.896s, metadata21.239s. Цепочка: парнаяSABRзаявка+проверенныйfallbackпоmn/fvip+HTTP1SABRtransport; прямойHTTP2послеfallbackобрывалсяпосле18s. РезультатнеподменятьproxyPASS. Запущеныfinalcore/media217suite(session1506)+ui_engine, final3reportedYouTube(session28548), Linuxreleasefinal(session49528). CIнадоdispatchпослеcommitэтойпачки, непредыдущего3f04a2e.

09.11 повторнаяверификация: finaldirect3reported ->1PASS(monitor)/2FAIL(shorts,watch1080) из-заconnectfailureпослесегментов; одиночныйShortsHTTP1PASSнеозначаетстабильность. В docs/download-recovery-2026-11-09.md статусPARTIAL. Userpendingcard: сохранитьпрямойилиявныйсуществующийSOCKS1080; безответапроксинастройкиНЕменять. Диагностика zapret-diagnose ошибочнонеувидела /opt/zapret; actualzapret.serviceactive. YouTubeвsplit-tun ->zapret-native(local1086)->zapret-native-egressbind_interfaceenp3s0; этофизическийdirect, AI7899НЕиспользуется, системныеконфигиНЕменялись. QtRUполныйUI+EnginePASS; ENпервыйпрогонhoverfailed, одиночныйповторsession14650ещёидёт; Xvfbservers191/192должныбытьразными, outputdirsпредварительносозданы. Proxyfinal3testssession84124получилwatch381850693video+12129597audio, ожидаетсяdecode.

09.11 finalcore/media217PASS0FAIL48ignored; finalClippyPASS; gitleaksdiff0; QtRU/EN+EnginePASS (ENодиночныйповторпослеhoverflake; Xvfb :191/:192). ЧерезявныйSOCKS1080 все3reportedYouTubePASS, включаяwatch394501566B/826.554s/1080+Opus/full decode,transfer18.770s. Безproxyfinal1/3PASS: не заявлятьполнуюстабильностьdirect. Выборproxyнастройкиостаётсяpending, не менятьего безответа. Linuxrelease49528нафинальнойкомпиляции.


## Выпуск исправлений 0.2.1 - 09.11.2026

Что сделать: завершить исправления загрузок и проверить Windows 10/11, выпустить новые установщики. Объём: существующий Rust/Qt продукт; настройки и история сохраняются. Проверка: regression/workspace/Qt, native CI, установка и обновление в QA ВМ, полные файлы/ffprobe/decode, опубликованные SHA256.

- [x] Восстановлены текущий код 129f57b, предыдущие результаты и два уже запущенных native CI; незакоммиченные examples сохранены.
- [x] Обновить согласованную версию всех пакетов и устранить hardcoded macOS version; 09.11 повторно PASS: version agreement, macOS staging contract, 7 install-staging tests, bash syntax.
- [ ] Проверить Windows 10 и 11: установка/обновление, старт, профиль, worker, UI.
- [ ] Проверить сборки всех поставляемых платформ и обязательные release gates.
- [ ] Опубликовать 0.2.1 и сверить публичные assets/checksums.

## Windows 10/11: повторная проверка по запросу 09.11.2026

Что сделать: запустить обе существующие QA ВМ и проверить баги прошлой сессии в настоящем установленном VideoDock. Объём: текущие исправления YouTube SABR/Shorts, TikTok audio, clipboard/platform UI, обновление и сохранение профиля. Проверка: нативные окна, установщик, полный файл с ffprobe/decode, UI/история после перезапуска; реальные результаты отдельно для каждой ОС.

- [x] Восстановлены TASK и отчёты, сохранены существующие незакоммиченные изменения версии 0.2.1.
- [x] Запущены videodock-platform-qa и videodock-win11-qa (по 4 GiB RAM).
- [x] Обе гостевые сессии готовы: Win10 LTSC 19044, Win11 Enterprise Evaluation 26300. Win11 штатно откатила неудачное обновление и загрузилась; исходные данные сохранены. На обеих ОС старое приложение запущено с копией профиля regression-20261109; profile-before сохранён. Win10 SQLite integrity ok, 5 jobs, RU/dark/1080p/MP4/direct.
- [ ] Получить Windows candidate текущего 129f57b из уже выполняющегося CI 42599688958; не подменять его прежним установщиком.
- [ ] Проверить установку/обновление/профиль, UI EN/RU и проблемные полные загрузки на обеих ОС.
- [ ] Исправить воспроизведённые дефекты, повторить затронутые сценарии и сохранить отчёт.

09.11 Windows native current checkpoint: candidate built inside Win10 from immutable 129f57b3e59121ad0906631cfc29d001b44fbb59 (git archive SHA256 f3bf5e9a4ec145e46726142df6e99e7a8ea141e365c47229167bb0e504149ed7). Installer 165769096 B / de678c267d9522d0fa143869d4c8e32010b891582dd9c6450f8d6518656b6643; installed exe 199dad775c8031512d2ef10b32eb8f935baf86574152760ef056f88b448fa906. Both Win10 19044 and Win11 26300: 2 upgrades PASS, SHA matching, profile files byte-unchanged, clean PATH GUI responds, clipboard YouTube icon now visible. Evidence/scripts: ~/.local/share/videodock/research/windows-regression-2026-11-09/. Existing dirty 0.2.1 version files untouched.
Current blocker: actual GUI Shorts 4wayF-jA2Os on Win10 direct FAILED with raw persisted error YouTube SABR не передаёт новые сегменты; повторите проверку Google. Win11 same source stalled around 60.7813%, final status pending. Do not claim all bugs fixed. Win10 candidate was closed gracefully to collect win10-direct-short/state.sqlite; Win11 candidate still open. Need diagnose stall/protection/companion protocol, then repeat native tests on fixed code.
Runtime QA: libvirt domains videodock-platform-qa (Win10, guest agent works) and videodock-win11-qa (QGA disconnected, use local HTTP + transient user driver). Bridge-only HTTP service videodock-qa-transfer-20261109 on 192.168.122.1:39709 (env user bus /run/user/1000). Root serve-files.json maps explicit public QA assets only. Win11 driver.ps1 polls next.json, executes step.ps1, acknowledges win11-ack.json; last sequence 5 requests AX; next action must use 6+. Scripts in evidence root. Win10 run-guest.py runs PS scripts via guest agent; upload with platform-parity-2026-11-03/guest.py then run-action.ps1 schedules user action-20261109.ps1.
Build driver C:\VideoDockQA\build-129f57b.cmd completed; native release PASS 15m22s, installer successful 156.5s. build-harness.ps1 (guest-exec pid808) now building core ui_engine release plus Qt appearance runner (cmd2152, cargo6844); output C:\VideoDockQA\regression-build-129f57b\qa-tools.zip pending. Current source C:\VideoDockQA\src-129f57b, cache C:\VideoDockQA\target. CI42599688958 current code workspace tests PASS, release candidate still building at last read.
Transfer note: guest-file-read 1 MiB times out on QGA; use 32 KiB for small profiles. Installer transferred by bounded HTTP POST /setup-upload from Win10, full SHA verified server-side (setup-upload-proof.json). Failed QGA partial saved separately; do not use it. Profiles: both guests %LOCALAPPDATA%\VideoDockQA\regression-20261109, old backup profile-before and app-before-129f57b, upgrade backup profile-after-install. Original validation-3171688 folders not changed.

09.11 new correction after real Windows reproduction: preserve verified companion audio init/segment state across SABR video responses and acknowledge both initialized formats and ranges. Changed core/pipeline.rs and core/sabr.rs, plus regression for duplicate/truncated audio/rollback. Host core/media218PASS0FAIL48ignored; Clippy PASS. All3reportedYouTube direct application-route full PASS with decode: Shorts24130123B/52.594s, monitor17343119B/603.034s, watch394501566B/826.554s. Logs companion-short-direct-v3.log and reported_*companion.log in windows-regression evidence. Host proof worker requires QT_QPA_PLATFORM=offscreen, XDG_RUNTIME_DIR=/run/user/1000 and uses ~/.local/lib/videodock/videodock; earlier setup errors are not media results.
0.2.1 native Windows candidate: base129f57b + patch6fb5f24a79cb00e2c227440bd764c516ae1e94f185ccd43a4d36ac5b30f13760 (Cargo version/lock and 2core files); compilation PASS11m26s. Win10 auto-powered off at roughly1h during packaging, restarted and resumed packaging ONLY. Setup165760441B SHA25683e8c94fe21e863baae58258793e3d22a47e9afac62c1b8b024d50efa2a7028c; exe200ce4529a053d1ab2436f7c19a25664cf764fc6448a2793672b3b9245c0ed16. Host file VideoDock-0.2.1-companion-windows-x64-Setup.exe verified by bounded HTTP push. companion-candidate.json is authoritative. Win11 driver seq13 now installing0.2.1 (install-companion.ps1); Win10 still needs0.2.1 install after RU UI run.
Native hover QA: original QTest-only harness consistently failed at64PNGs/locale with hovered0/cursorArrow, whereas actual OS input on installed product gives Hand. Win11 QEMU HID Tablet absolute mode index3 leaves hover stale; temporarily selecting PS/2 mouse index2 and sending relative move gives Hand atsamecontrol; restored index3. Win10 actual QMP input also Hand. New test-only appearance.cpp injects Win32 SendInput MOVE_NOCOALESCE before unchanged hover/cursor assertions. Win10 EN full harness nowPASS66PNGs incl realEngine/history/delete safety/smartpaste; RU queued same new harness. No production QML hover change made. Native helper244224B SHA2564032f4dcec495dbcf0b5352765646aabc684c15a80be783be9978abb3d253d35 at evidence/videodock-appearance-native.exe; Win11 still needs downloading this helper and repeating EN/RU with ui.ps1. Original qa-tools.zip containsOLDharness; copy new appearance.exe over after verifying hash.
Helper gotchas resolved: portable app QML must be in app-qml, since runtime qml/QML is an existing Qt module; ui.ps1 uses cached process.Handle + Refresh to get real ExitCode instead of null on PS5. User scheduled run-action.ps1 now uses -WindowStyle Hidden so diagnostic shell does not cover the tested window. Win11 cursor class guarded for reuse in persistent driver.


## Возобновление 09.11 после смены сессии

- [x] Сохранено dirty tree; восстановлены обе работающие Windows QA ВМ и кандидат 0.2.1. Свежий Win10 executable SHA256 совпал с companion-candidate.json; процесс отвечает.
- [x] Свежие QA SQLite snapshots обеих ОС: integrity_check=ok; доказательства windows-regression-2026-11-09/{win10,win11}-resume-state и *-resume-proof.json. Это снимки текущих результатов, НЕ новый успешный download.
- [ ] Shorts остаётся FAIL на обеих ОС: 18,318,427 из 24,110,609 bytes; сохранённые progress 60.7813% и ошибка «Соединение SABR прервано». Причина не установлена; host direct PASS не закрывает Windows gate.
- [ ] Win11 EN с проверенным native helper повторно FAIL после 64 PNG на native hover play (hovered=0/cursor=Arrow, nativeDistance=0, SendInput delivered=1). Повтор с QEMU PS/2 вместо Tablet также FAIL; значит одной смены устройства недостаточно. RU ранее PASS; Win10 EN/RU ранее PASS. Проверки hover не ослаблены, production QML не менялся.
- [x] Повторные проверки версии 0.2.1, macOS packaging contract, 7 install staging tests, bash -n и git diff --check PASS. Это проверка staging contract на Linux, не новый macOS native runtime QA.
- [ ] Для продолжения: диагностировать точную причину сетевого обрыва SABR в Windows и native hover в Win11; после исправлений заново full download+decode и EN/RU. Выпуск 0.2.1 не готов к публикации.


## Исправление двух Windows сбоев - явный запрос пользователя 09.11

Результат: исправить Shorts/SABR и сбой hover Win11 EN, не ослабляя проверки. Объём: общий transport и тестовый нативный ввод; только QA-профили, без изменения пользовательского proxy/аккаунтов. Проверка: воспроизводящий RED, минимальная правка, native Windows10/11 full download+ffprobe/decode и EN/RU UI, затронутые offline regressions.

- [x] Зафиксированы исходные FAIL и сохранены SQLite/proof/logs; незакоммиченные изменения сохранены.
- [ ] Получить безопасную native диагностику transport (категория/OS-код) и hover (WindowFromPoint/native cursor/foreground).
- [ ] Исправить установленные причины; regression RED -> GREEN.
- [ ] Собрать исправленный Windows candidate; проверить полные проблемные загрузки и EN/RU на обеих ОС.
- [ ] Сохранить итоги и точные ограничения, удалить диагностические изменения из финального продукта.

Диагностические исходники пока изолированы в research/windows-regression-2026-11-09/diagnostic-20261109/sources.zip и QA checkout C:\VideoDockQA\src-129f57b; production tree не менялся. Диагностический build нельзя выпускать как финальный.


Checkpoint исправления (09.11, активная работа):
- На Win10 нативный pipeline сначала один раз PASS, затем 3/3 FAIL. Свежий GUI job также FAIL до байтов. Пакеты на vnet3: TCP SYN/SYN-ACK проходит, TLS ClientHello 285B на CDN остаётся без ServerHello до 20s connect timeout; qa-connect-vnet-headers.json сохраняет только заголовки/длины, не payload. Это подтверждённая граница отказа, не проблема mux/диска.
- Production patch: SABR Session теперь wreq::Client (тот же имеющийся TLS transport, что metadata); core использует http-body-util::BodyExt::frame вместо reqwest::chunk. Redirect allowlist/no_proxy/custom DNS/timeouts/cancellation/limits сохранены. Новых registry packages нет, только 2 прямые зависимости core из существующего lock. Host core/media218PASS0FAIL48ignored. Wreq host full: monitor и Shorts PASS/full decode; watch сначала protection=2/pending и stall, повтор со свежим proof идёт до конца с protection=1 (итог смотреть wreq-host-watch-refresh.log). Не объявлять universal stability.
- Win10 native clean patch wreq-patch.zip SHA25609a861a0ec6c20c7f2e3ec50a890ac441c72b294975cc7a4f0fa6363659640b9. build-wreq.ps1 проверяет хеш и touch-ит файлы после Expand-Archive из-за различия host/guest clocks. Native release test youtube_downloads-b94a8178aa93ad50.exe собран; C:\VideoDockQA\wreq-test-ready.txt есть; clean desktop ещё строится, маркер wreq-desktop-ready.txt. Исходники C:\VideoDockQA\src-129f57b. Диагностические runtime вставки этим zip удалены. Native tests/youtube_downloads.rs пока имеет диагностический Auth::default вместо старых fixture cookies, перед финальным reproducible snapshot вернуть файл из repo.
- Win10 eval выключила ВМ по истечении часа, она запущена снова (libvirt id4/vnet3), native build возобновлён из cache. Win11 id2/vnet1 работает. Следующее автоотключение примерно через час после запуска, не отключать лицензионное ограничение.
- Старый диагностический exe 9E711F5F... в отдельном appflow-diagnostic-20261109/profile, не установлен в production, сейчас открыт на Win10. Его TLS failures подтверждены app-diag-tail.ps1. Данные пользователя не используются.
- Обнаружено ограничение QA orchestration: Start-ScheduledTask при уже Running по умолчанию игнорирует новый старт. Для wreq repeat используется отдельный task VideoDock-Wreq-Repeat-20261109 + C:\VideoDockQA\wreq-repeat-current.ps1, он проверяет предыдущий task; JSON должен иметь phase=wreq-shorts-repeat и имя/хеш фактического test binary. Не считать прежний result.json новым запуском.
- Hover: SendInput absolute и ожидание первого rendered frame отдельно не исправили FAIL; обе гипотезы откатили. Текущая проверяемая минимальная правка appearance.cpp: QGuiApplication::sync после native move до QTest::mouseMove. Проверки hovered/PointingHand/Arrow и таймаут2s не ослаблены. Helper appearance-native-sync.exe SHA2561c783ea9f4473f6efd5ac8d2a7a9d4b22b2ec4ac7b2688c4ae6fb6b29140c67a, Win11 driver seq35 EN/RU выполняется, свежая output folder appearance-native-sync.
- Input probes (не production): событие MouseMove приходит в верный HWND, foregroundOwn/windowOwn=1, hoverEnabled=1, manualVisible/opened=0, grab=none. Один полный EN probe случайно PASS; повторяемость не доказана. scene childAt не дошёл до кнопки. Диагностический appearance-probe.cpp в evidence содержит target ancestor/device/live-target trace, но последний полный успешный probe не дошёл до failure trace. Успешный probe нельзя считать фиксом. Packet/header and UI logs в windows-regression-2026-11-09.
- Host clippy и gitleaks текущего diff запущены (wreq-clippy.log/wreq-gitleaks.log); дождаться и проверить. Релиз не публиковался.

Контроль гипотез: wreq TLS transport не исправил Windows (новый executable youtube_downloads-b94a8178aa93ad50.exe, 3/3 FAIL). Изменения core/Cargo.toml, media sabr/youtube/youtube_sabr и lock возвращены к исходному companion patch; диагностические wreq артефакты не выпускать. Прямой physical QA route с ip rule88(from99)/89(from248) также дал3/3 FAIL исходного native core, то есть одного обхода TUN недостаточно. Временные rules пока активны, rollback research/windows-regression-2026-11-09/rollback-qa-direct.sh обязателен. Планируется контроль через существующий локальный zapret-native1086 (physical enp3s0, без внешнего VPN) в отдельном QA, не менять реальные пользовательские proxy preferences.
UI: frame-ready и sync+freshEnter не дали стабильный EN/RU PASS, не объявлять их фиксом. Исходный device-probe повторно запущен Win11 seq39 в ui-target-inspection для полной цепочки ancestor/live-target. После него нужен seq40 на чтение лога.

Новый подтверждённый результат: исходный companion core через локальный zapret-native ingress, bind_interface=enp3s0, на Win10 3/3 Shorts PASS с полным decode. Existing local SOCKS1086 открыт только двум QA IP через временный relay192.168.122.1:39711 (systemd user videodock-qa-native-relay-20261109, RuntimeMaxSec1200, создан15:57MSK). Реальный установленный companion exe200CE4529A... на Win10 и Win11: свежий QA профиль -> UI Paste Link ->24130123B/52.594s/video+audio/full decode -> Close/restart/history PASS. Proof win10-installed-native-egress-pass.json и win11-installed-native-egress-pass.json. Старая ошибка в пользовательском direct QA snapshot не означает новый protocol failure; обычный egress ВМ блокируется на TLS. Настройки настоящих профилей не меняли; QA proxy только в native-egress-gui-20261109/profile. Relay и ip rules88/89 убрать в конце.
UI input sync/outside/frame hypotheses reverted. Target-chain evidence подтвердило live-target same=1 (не stale delegate), geometry valid. Scene hit instead stops on a generic QQuickItem at xy0,158 wh1150x64 z0 over the delegate. Win11 seq44 currently tests ONLY explicit slot z:1 in temporary copied JobList.qml, original strict helper4032F4..., fresh EN/RU directory ui-delegate-z-one-20261109; script finally restores original QML bytes. Do not call this fixed before results.

Подтверждена и исправлена причина hover: Qt6.8 hover trace показывает moreButton после появления на row.hovered в geometry0,0 40x40, перекрывающей thumbnailAction; после grabWindow/layout polish он переходит в1008,12. Поэтому ни pointer warps, ни highlight/z, ни ожидание картинки не исправляли первопричину. Production JobList.qml держит обе action кнопки в RowLayout всегда, скрывая opacity0 + enabledfalse + Accessible.ignored; геометрия стабильна и скрытые действия не доступны для фокуса/AX. Исходный строгий helper4032F4... и неизменённые assertions: Win11 EN66/RU66 PNG, оба Appearance+RealEngine PASS (seq52, win11-ui-stable-actions-pass.json). Win10 первый финальный UI проход прерван часовым выключением evaluation OS; ВМ снова запущена16:33MSK, повтор в ui-stable-actions-after-reboot-20261109; не считать его PASS без нового JSON.
Linux current-source final checks запущены в research/windows-regression-2026-11-09/final-validation-k1kwbmqt (core/media, real Engine+Qt EN/RU). Windows final clean source patch stable-actions-final-patch.zip SHA256787f61569122e12540ed1252dcb2a0328d859303e362906791291491862c9527; полный member manifest stable-actions-final-source.json. Все диагностические Rust/media/C++ sources в guest заменяются на repo versions, включая исходный тест и CMake; source hashes проверяются до build. build-stable-actions-final.ps1 строит native release+installer C:\VideoDockQA\stable-actions-0.2.1, logs stable-actions-final-build*.log, marker stable-actions-package-ready.txt. Сборка сейчас запускается параллельно повтору Win10 UI.
Временные ip rules88/89 уже удалены с проверкой. QA SOCKS relay auto-expired, перед финальной GUI download verification его можно запустить заново только временно, затем stop/reset-failed. Исходные реальные proxy preferences и профили не изменялись; все сетевые эксперименты wreq и UI sync/entry возвращены. Production runtime delta текущего хода только JobList.qml, предшествующие companion changes сохранены.

## Продолжение 09.11 после перезагрузки хоста, финальный Windows candidate

Результат: довести stable-actions 0.2.1 до проверенного установленного приложения. Объём: упаковка уже собранного native executable, upgrade/profile и полная UI-загрузка в изолированных QA-профилях. Проверка: manifest/SHA256, installer exit code, ffprobe/full decode, restart/history.

- [x] Проверен сохранённый Win10 EN/RU result: 66 PNG на язык, exit0, Appearance+RealEngine PASS; отдельная копия win10-ui-stable-actions-pass.json.
- [x] Linux final-validation-k1kwbmqt/result.json PASS; все 16 членов финального source manifest совпадают с текущим деревом.
- [x] Обе QA ВМ запущены. Native release перед перезагрузкой собран за 6m24s; упаковка оборвалась во время compression, 1.2 MB partial installer непригоден.
- [ ] Повторить только упаковку с проверкой guest source manifest и совпадения staged/release exe; затем проверить новый installer на Windows 10/11.
- [ ] Windows11 требует обычный вход в VideoDockQA после boot. Отправлена плашка, пароль не запрашивается; до входа UI-проверки Win11 не выполняются.
- [ ] После проверок остановить временные QA transfer/relay службы. Полный релиз не публиковать до закрытия gates.

Новый финальный installer FEB2448F… (162096631 B) прошёл два upgrade с неизменным QA SQLite, но запуск завершился 0xC000012F ещё до Qt. Static/runtime trace: staged и installed MSVC DLL byte-identical, однако MZ/PE заголовки нулевые; исходные VS14.44 CRT и прежние QA-tools имеют валидный AMD64 PE. Этот installer сохранён как непригодный, не использовать. Повреждение обнаружено в staging после прерывания хоста; причина потери данных внутри VM точно не установлена.
Добавлен общий check-windows-binaries.ps1 перед публикацией stage и упаковкой installer: MZ, bounds PE offset, PE signature, AMD64, PE32+. Regression RED zeroed accepted → GREEN valid+6invalid; Windows transactional staging tests GREEN, включая binary failure без изменения предыдущей поставки. package-windows также отвергает фактические повреждённые DLL. CI включает новый check. Выполняется свежий build-windows (Rust cached15.33s), Qt+CRT заново из исходных SDK, затем recovered installer. Все guest source hashes сверяются с recovered-final-source.json. В Windows11 по ответу пользователя пока не входить, продолжать Windows10.

Recovered installer: 165761362 B, SHA25609D65A045E980742025CEC30686DF4ECF9C68D1E1EE4A8726B039DC1E77C31C6, installed exeFB23FF06CB0770784F2EB28AA20879742BF50F1E312E6DEEFAE281AB241C52BC. Win10 два upgrade PASS, profile byte-unchanged, clean-PATH responsive GUI PASS. Reproducible overlay recovered-final-source.zip содержит21member с проверенными SHA; manifest+archive hashes в final-candidate.json/recovered-final-archive.json.
Реальный установленный GUI: monitor360p PASS17343119B/603.034s/8.765s download, full decode+restart/history. Shorts первая fresh QA попытка FAIL(SABR stall, сохранён raw SQLite); Linux тот же ingress PASS22.51s full test; Windows fresh retry PASS24130123B/52.594s/7.788s download/full decode+restart/history. Это intermittent, не считать полностью исправленным. TikTok physical QA ingress FAIL(public video-detail absent), guest default egress PASS6947649B/148.886349s/2.710s/full decode+restart/history; исходные профили не меняли.
Watch: первый медленный прогон был ошибочно прерван перезапуском20min QA relay, получил connection error и не годится для выводов о стабильности. Fresh watch-stable запущен с отдельным relay RuntimeMaxSec3600, без дальнейших сетевых изменений; текущий прогресс в watch-stable-progress.jsonl, observer exec session80020. Worker C:\VideoDockQA\watch-stable-wrapper.ps1, native task VideoDock-Watch-Stable-20261109, итог C:\VideoDockQA\watch-stable-run.json и QA profile stable-final-watch-stable-20261109. После PASS собрать combined proof и остановить videodock-qa-native-relay-stable-20261109/videodock-qa-transfer-resume-20261109. Отчёт docs/windows-qa-2026-11-09.md уже создан, watch пока pending.

Финал текущего Win10 прохода: watch-stable PASS232632368B/826.554s/video+audio/full decode/restart-history, download295633ms/total331146ms; watch-stable-run native task exit0. После запуска этой попытки relay и сеть не менялись. final-windows10-summary.json объединяет четыре успешных полных образца, сохранённые первичные отказы и ограничения; общий статус PARTIAL из-за непостоянного Shorts и непройденных gates.
- [x] Recovered Windows installer и два upgrade/profile/clean-PATH GUI.
- [x] Полные monitor/Shorts retry/watch-stable/TikTok default: ffprobe+decode+restart/history.
- [x] Header guard regression, transactional staging, gitleaks scripts0findings, YAML parse и diff --check.
- [x] QA transfer и native relay остановлены, listening39709/39711 отсутствуют; постоянный1086 не менялся.
- [ ] Выяснить первый SABR stall Shorts (raw state сохранён; последующий PASS не означает исправления).
- [ ] Окончательный installer Windows11: пользователь выбрал пока продолжать Windows10; обычный login остаётся нужен.
- [ ] Остальные cross-platform/parity/release gates сохранены. Релиз не публиковался.
Отчёт: docs/windows-qa-2026-11-09.md. Текущий установленный QA candidate оставлен на completed watch profile для просмотра (show-final-watch.ps1), пользовательский основной профиль не использовался.

## Чиним непостоянный Shorts по прямому запросу 09.11

Что сделать: установить причину SABR stall, исправить общий transport, подтвердить несколькими полными анонимными Windows10 загрузками и регрессией. Объём: текущий Rust SABR, без смены архитектуры, пользовательских профилей и сетевых настроек. Проверка: воспроизведение с counters-only trace, RED→GREEN по установленному сценарию, core/media+Clippy, native installed GUI/full decode/restart.

- [x] Проверены все callers State::progress и критический receive loop. Stall считает только байты выбранной дорожки, отбрасывая companion progress; пока это гипотеза, а не установленная причина первого отказа.
- [x] Публичный integration test больше не подставляет фиктивные cookies для Shorts; используется Auth::default(), как в анонимном пользовательском пути.
- [ ] Native trace build: guest core содержит временные счётчики itag/sequence/bytes/companion/protection/backoff/elapsed/frame kinds/context type IDs; никаких URL/token/context values. Эти вставки только в diagnostic overlay, в repo их нет и в installer они не пойдут. Источник shorts-trace-source.zip/json; сборка C:\VideoDockQA\build-shorts-trace.cmd, stderr shorts-trace-build-error.log, Cargo artifact JSON shorts-trace-build.jsonl. После сборки start-shorts-trace.ps1 запускает до8fresh native Shorts и останавливается на первом отказе.
- [ ] Проверить причину и сделать минимальный fix+regression; вернуть диагностические guest sources перед финальной сборкой.
- [ ] Перед финалом остановить QA transfer-shorts и native-relay-shorts. Оба временные, relay60min, никаких перезапусков во время загрузки.

Установлена причина в настоящем GUI: unfixed diagnostic GUI после обычного reboot первой fresh попыткой воспроизвёл отказ. Аудио251 ужеполучено888120B, protection=2/attested=false; трипустыхопроса длились5+3+15ms, после чего общийpipeline оборвался. Native CLI доэтого8/8PASS (protection=1), поэтомустарыеCLIпроверкинепокрывалиданныйGUIслучай. Первичные доказательства shorts-original-gui-result.json и shorts-original-gui-pending-trace.json.
Регрессии RED→GREEN: (1) companion audio иновоеattestation не считаютсязависаниемvideo; (2) готоваядорожка приpending не публикуется ивыдерживает>3пустыхответа, междуопросами>=250ms, по30sбюджетуошибка; (3) обычноеистинноезависаниепо-прежнемуFAILпосле3циклов. Проверены12SABR tests, полныйcore/media219PASS/48ignored, finalworkspaceClippyPASS (shorts-final-host-checks.json). Исходники fixed snapshot21files вshorts-fixed-source.zip/json, диагностическийQA SABRtraceвнихотсутствует. Nativefixedreleaseсобран3m56s, упаковкавshorts-fixed-0.2.1 идёт. Нужноmanifest/push-shorts-fixed/install-shorts-fixed +start-shorts-fixed-gui (до8fullGUIclips). Исходныйinstallerостаетсянетронутым, новыйбудетVideoDock-0.2.1-shorts-fixed-windows-x64-Setup.exe.

### Дополнение: оба этапа подтверждения Google

- Unfixed GUI trace воспроизвёл protection=2 с полными аудиобайтами; три пустых poll за 23 ms ошибочно обрывали pipeline. После bounded wait + SABR refresh первая установленная GUI загрузка прошла (37576 ms, full decode, restart/history, одна строка).
- Во втором свежем профиле actual SQLite зафиксировал `Google не подтвердил анонимную проверку; повторите загрузку`, state=error, progress=0, `_sabr_refreshed` отсутствовал: это отказ worker до SABR, прежний budget его не охватывал. Harness остановлен, первичный raw snapshot `shorts-refreshed-second-state/` сохранён. Первый parse FAIL был ошибкой UTF8 без BOM в PS5, не результатом приложения; QA PS scripts теперь UTF8-BOM, ParseFile=0errors.
- Добавлен media::AttestationRetryable для challenge/interpreter/snapshot/integrity/mint/timeout. Validation/network/internal/повреждённый IPC остаются обычными ошибками. Core queue использует общий однократный бюджет с PendingAttestation, сохраняет тот же job id, очищает только внутренний stage. Pause/remove и явный Google reject/HTTP403 не получают автоматического повтора; ручной resume/redownload сбрасывает budget.
- Runtime/DXC гипотеза проверена контролями. Один и тот же worker в одной и той же папке дал protection=1 и полный PASS с обеими штатными версиями DXC; без DXC получил protection=2. Не менять поставку DXC по этой гипотезе, `--no-system-dxc-compiler` НЕ добавлен. Crossover copies только в C:\VideoDockQA\worker-crossover, оригинальные директории не менялись.
- Текущий окончательный source snapshot: `shorts-recovery-source.zip/json`, 24 SHA-verified members, source id `129f57b+complete-attestation-recovery`. Native build `build-shorts-recovery.ps1`, logs `shorts-recovery-build*.log`, marker `C:\VideoDockQA\shorts-recovery-ready.txt`, output `C:\VideoDockQA\shorts-recovery-0.2.1`. В source и installer нет counters-only QA SABR instrumentation.
- После ready: `shorts-recovery-manifest.ps1` -> сохранить JSON в `shorts-fixed-candidate.json`; `push-shorts-recovery.ps1` -> host `VideoDock-0.2.1-shorts-fixed-windows-x64-Setup.exe` (bounded upload по размеру/SHA); затем перед запуском обновить source id в install script и использовать новый prefix профилей `shorts-recovery-1..8-20261109`.
- QA scripts уже готовы и загружены: install-shorts-recovery.ps1, gui-shorts-recovery-1..8.ps1, run-shorts-recovery-gui.ps1, start-shorts-recovery-gui.ps1. Они UTF8-BOM, ловят видимые окончательные Google/YouTube ошибки, каждая PASS включает полный MP4, video+audio, FFmpeg full decode и restart/history. Серия останавливается на первой ошибке.
- Win10 был штатно перезапущен перед последней GUI серией около19:00 MSK; не допускать evaluation shutdown во время новой сборки/загрузки. Win11 выключен штатно (пользователь пока выбрал Win10).
- Сейчас идут host full core/media + Clippy: exec session90230, result shorts-recovery-host-checks.json, logs shorts-recovery-{core-media,clippy}.log. Предыдущий вариант221PASS; новый добавляет1media test, ожидается222, но утверждать только после чтения результата.
- Активные временные user services: videodock-qa-transfer-shorts-20261109 (39709, до2ч от17:50) и videodock-qa-native-relay-recovery-20261109 (39711, до1ч от19:24). Продлевать только МЕЖДУ попытками без активной загрузки. После финальных доказательств обе остановить. Не менять постоянный1086/настоящие proxy prefs, релиз не публиковать до Win11/gates.

### Текущий checkpoint: bounded recovery, 20:09 MSK

Предыдущая GUI серия (shorts-verified) дала3PASS, затем реальный FAIL4: SQLite `shorts-verified-fourth-state/` показывает pending timeout после ужеиспользованного `_sabr_refreshed=true`. Один повтор недостаточен; введён конечный общий budget4refreshes (=5попыток), по нормальной ограниченной политике reference SABR. Новый счётчик `_attestation_refreshes`, legacy boolean учитывается как1, ручнойresume/force/redownload сбрасываетоба; negative deny/pause/remove gates сохранены. RED→GREEN проверяет4разрешённыхповтора, запрет5-го, тотжеjobid и неизменный exhausted state. Host222tests+ClippyPASS дляэтогоsource: shorts-bounded-host-checks.json.

Окончательный source сейчас `shorts-bounded-source.zip/json` (24 members), source id `129f57b+bounded-attestation-recovery`. Native build `build-shorts-bounded.ps1`, logs/marker/output `C:\VideoDockQA\shorts-bounded-*`, статус `shorts-bounded-build-status.ps1`. Пока сборка/упаковка, неиспользоватьпредыдущийinstaller. Послеready: shorts-bounded-manifest.ps1 ->shorts-fixed-candidate.json; push-shorts-bounded.ps1 ->VideoDock-0.2.1-shorts-fixed-windows-x64-Setup.exe; затем start-shorts-bounded-gui.ps1 и observe-shorts-bounded.py (8 fresh prefixes shorts-bounded-1..8-20261109).

QA harness поправлен: UTF8-BOM, nativeParseFile=0; именаAXмогутбытьnull, теперьпропускаютсятолькобезымянные/offscreen/stale элементы, настоящиеошибкиприложенияловятся. Предыдущий shorts-recovery FAILnull былошибкойharness, неapp: actual SQLite100%complete. Не считатьегоSABRрегрессией. Текущийобычныйscopeclipнеизменён: fullfile+video/audio+decode+restart/history, неослаблятьпроверки.

Win10 evaluation сама выключилась около20:00; штатно запущена снова20:06, guestagent+autologinготовы. АктивныеQAunit: videodock-qa-transfer-verified-20261109 (39709, до21:3x) и videodock-qa-native-relay-bounded-20261109 (39711, до21:06). Не менять/перезапускать во время загрузок. Все исходные пользовательские настройки и постоянный1086 сохранены.

Если потребуется продолжение из compaction: закончить упаковку, проверитьhash/source/отсутствиеQAtrace, установить и выполнить8GUIпопыток. Сохранить snapshots каждогоprofile и `_attestation_refreshes` (доказательство auto recovery без duplicate rows). Затем обновитькороткийотчёт, положитькопиюготовогоQAinstallerвignored dist/qa (git check-ignore ужеPASS), остановитьобеQAслужбы. Win11 всёещёотложенпользователем, полныйрелизнепубликовать.

## Исправление Shorts завершено: финальная проверка Windows10

- [x] Native bounded candidate собран, упакован и передан с проверкой полного SHA256; обе установки/обновления прошли с сохранением профиля.
- [x] Восемь последовательных GUI загрузок проблемного Shorts: 8PASS/0FAIL. В каждой полный файл video+audio, ffprobe, полный FFmpeg decode, штатное закрытие/restart/history.
- [x] Восемь SQLite snapshots: integrity ok, ровно1complete job. Реальное auto recovery подтверждено: attempt5=2refreshes, attempt7=1refresh, остальные0. Дубликатов нет.
- [x] Финальный host source:222testsPASS/48ignored, Clippyworkspace-DwarningsPASS, gitleaks core/media0findings. Все24currentrepo source hashes совпали с собранным shorts-bounded-source.json.
- [x] Готовый проверенный installer скопирован с SHA в ignored dist/qa/VideoDock-0.2.1-shorts-fixed-Setup.exe.
- [x] Временные QA relay-bounded/transfer-verified остановлены,39709/39711 не слушают. Постоянный1086 не менялся.
- [x] Итоговый отчёт docs/shorts-recovery-2026-11-09.md, полное доказательство research/windows-regression-2026-11-09/shorts-final-summary.json.
- [ ] Окончательный кандидат Windows11 и прочие project release gates остаются открыты; полный релиз не публиковался.

## Проверка окончательного кандидата на Windows11, 09.11

Что сделать: проверить тот же установщик bounded recovery на Windows11, включая upgrade/profile, реальные полные файлы, повторные Shorts и restart/history. Объём: существующая QA VM, отдельные QA profiles, без публикации релиза. Проверка: SHA256 installed candidate, полные ffprobe/FFmpeg decode, SQLite integrity и отсутствие дублей, актуальный GUI.

- [x] Восстановлены текущий installer/source manifest и доказательства Win10; Win10 проверки без новых изменений не повторяются.
- [x] Windows11 VM запущена; найден существующий локальный QA credential file. Пароль не выводить/не копировать в контекст, использовать только для обычного входа через проверенный экран.
- [x] Войти с сохранённым QA credential, установить итоговый кандидат и пройти Windows11 QA.
- [x] Сохранены доказательства и закрыт Windows11 runtime gate; временные transfer-win11-final и native-relay-win11-final остановлены.


## Windows11 окончательный кандидат: PASS, 09.11

- [x] Тот же installer SHA2561C1B9081498DAED38B2F0263FD5284DE5BBD03A8AC3E12D06B4798D8EF8979DF / exe2DD378F375D9D1D08639FCCB86B319B359FE2ABB727C58EF2E7A1F21F8069408, два upgrade, профиль byte-unchanged, clean-PATH startup.
- [x] EN66/RU66 PNG, exit0/0, Appearance+RealEngine sections PASS. Current QML stable actions checked by portable harness; installed current EXE checked separately by all real downloads.
- [x] 8/8 Shorts complete MP4 video+audio/full decode/restart/history; 8SQLite integrity ok, однаcompleteстрока и одинMP4каждый. Actual refresh counts [1,0,0,0,0,0,1,1].
- [x] Monitor17343119B/603.034s, watch232632368B/826.554s, TikTok6947649B/148.886349s: полныйфайл+decode+restart/history.
- [x] Actual installed first-run: четыреonboardingшага, readyPasteLink, restart безповторногообучения.
- [x] Uninstall/Reinstall QAcopy: профили и восемьготовыхвидео сохранены поSHA, новыйEXEточнототже, профильоткрылся.
- [x] Installed Play action открыл штатныйWindowsMediaPlayer; видеокадрменялся, время0:51/0:52, decoder errorнебыло. Audio проверенfullFFmpegdecode, физическаяслышимостьвнеVMнеизмерялась.
- [x] Driverseq7 stop=true, ackPASS; QA39709/39711остановлены. Постоянный1086не менялся.

Доказательства: research/windows-regression-2026-11-09/win11-final-summary.json, raw8DB win11-final-state/, screenshotwin11-final-playback-a.png. Отчёт docs/windows11-final-qa-2026-11-09.md. Все24currentrepo source hashes ещёсовпадаютссобраннымoverlay; исходникиприложенияэтимпроверочнымпроходомнеменялись.

Windows10/11 functional installer gate закрыт. Остальные cross-platform/package/parity gates не закрывать этим результатом; полногоGitHubрелизанебыло.

## Доведение релиза, 12.11.2026

Что сделать: разобрать текущие незавершённые правки, исправить воспроизведённые сбои, проверить итоговые Linux/Windows поставки и опубликовать проверенный релиз. Объём: минимальные полные исправления, приоритет YouTube, без смены модели и без субагентов. Проверка: соответствие source manifest, core/media и Qt integration, реальные полные загрузки с ffprobe/decode, package/install/restart и сохранение профиля.

- [x] Восстановлены TASK и отчёты Windows 10/11; существующее dirty tree сохранено.
- [x] Все24sourceSHA совпали с Windows bounded candidate; SHA installer1c1b9081 подтверждён.
- [x] Linux:222PASS/48ignored, ClippyworkspacePASS, releasebuildPASS, QtEN/RUPASS, worker/proxy/startupPASS, gitleakscrates0. Три полных образца через существующий1086+XCB/настоящийDISPLAY прошлиffprobe/fullDecode. Direct3/3FAILmetadata confirmation; offscreen не засчитывать как рабочую графическую среду.
- [ ] Проверить и обновить Linux поставку; закрыть подтверждённые баги.
- [ ] Проверить финальные поставки, обновить отчёт и опубликовать релиз после закрытия обязательных gates.
- [x] Самостоятельно просмотрены ошибки истории; добавлены и проверены баги Proxy Save и порядка новых загрузок.

12.11 checkpoint: исправлен stale LICENSES/rust/manifest.json (только cargo_lock_sha256); generate--write затем--check366dependencies/393textsPASS. Windows10VM запущена для перепаковки того же EXE с актуальным license manifest. FlatpakSDK кэш восстановлен (официальные rustc/std/cargo1.99.1 с SHA), productionbuild в собственной cgroup1GiB/noSwap/jobs1. Штатный CUA/Sky недоступен: Trusted RPC service is not configured: sky; ручной LinuxGUI этим ходом не засчитывается. Логи иcheckpoint: ~/.local/share/videodock/research/release-2026-11-12/.

### Все ошибки пользовательской истории, 12.11

Пользователь уточнил: самостоятельно просмотреть все ошибки приложения, а не ждать список. Read-only snapshot найден в настоящем state.sqlite:25jobs,14errorдоисправленияскриншота. Установленный старыйbinary4ddd0dda заменён актуальным0ab19688, backup+SQLitebackup сохранены. Все25records/settingsсначалаbyte-equivalent; после установлена только явная appSOCKS127.0.0.1:1086 (существующий physicalzapret), без изменения системногоHapp. Более свежий сетевойTASKот11.11 подтверждает пользовательский выбор Happ, старая памятьdirect-onlyнеактуальна. Обычный no-proxyпутьпрограммы всёравно шёл черезHapp; прежние логи сlabeldirect не доказывают физическийdirectFAIL. Исходныйjob e4819a42 послеresume сталcomplete на томжеid,25recordsсохранены; исходныйроликa9oJKJeHVeY полученв4K2160x3840сaudio/23.615s/fullDecode.

- [ ] Оставшиеся13error:YouTube3, TikTok4(3unique),4kfilm6(1unique).
- [ ] Воспроизвести каждую из7уникальныхссылок, исправить общие причины, проверить все исходные записи.
- [ ] После новыхruntimeизменений обновить Linux/Windowsкандидаты и выполнить необходимые регрессии.

12.11 всеошибки: обычныйTikTokHTTP+анонимныйQtвернули10204; официальныйembed40302. ЧерезсуществующийHapp10808 публичныйSSRstatus0; выбранпробныйлокальныйsing-boxrouter127.0.0.1:11086 (exec43444, НЕостанавливатьпокаестьзагрузки). YouTube+Googleproofhosts→существующийnative1086, остальное→Happ10808. Глобальныеконфигинесменялись. TikTokadaptiveaudioчерезrouterполныйPASS6947649B/148.886s/fullDecode. Реальнаяочередьчерезhelperexec5386: фильмjob6b077460скачивается(уже~2GB/52%), одинTikTokdec0c582complete, TikTok553056b1старыйpartial416, дваkitashura403/metadatafailure. HelperсамвернётUIчерезtransientsystemdunit послефинала.
Новыеruntimeправки: queueпринимаетисчерпанныеSABRRetryableвтотжеобщийbudget4refreshes (RED→GREEN), Retryablepub(crate); реальныедваYouTubejob5bf0086c/96d37aacпослеправкиcompleteвтомжеid. catalogподдерживаетcustomvideo-player→публичныйVideoHubAPI(толькооткрытоеmovie, voiceпервыйдубляж), numericqueryvalidation+MP4formats. ПервыйdeliveryHTTP400воспроизведёнтестом: APIвыписывалURLдляAPP_UA, mediaUA=Chrome132. dataAPIпереведённаsource_get(тотжеUA)→Range206/1024ftypGREEN.
Ещёвыявленstale-resume416: coretransferпослестарогоpartialнеперезапрашивалцелыйфайл. НовыйреальныйTCPregressionRED; добавленоднократныйfreshGETбезRange/IfRangeтолькодляinitialstaleoffset. Тествключает200/403/416ипроверяетсохранениестарогоpartialприотказе; GREENexecвпроцессе. НовыекандидатыWindows/Linuxпослеэтихruntimeизмененийнужнопересобрать(старые09.11и12.11ранниепакетыНЕфинальные).

Checkpoint перед финальным partialidentity: persistent videodock-routing.service активен/enabledна127.0.0.1:11086; прототипexec43444остановленпослеidle. Sharedassets/videodock-routing/install.py иonce/linux+darwin/2026-11-12-videodock-routing.sh созданы. --checkPASS, повторныйреальныйinstallнеизменилPID(идемпотентностьPASS); macOSприменитсятолькоприэквивалентныхсуществующихлокальныхupstream(иначеno-op), живойMacнепроверен. Все3свежиеTikTokвключаяoriginalqualityчерезrouterполныйPASS. Исходныеkitashura2job3329bbf9/e93ef74dиshintarodec0c582вочередиcomplete; осталсяan1mu553056b1сlegacybitrate-0partial5671332байт.
Фильм6b077460complete3042828376B/7710.119s; fullDecodeдоEOFподтверждён, proof6b077460-full-file-proof.json. ПятьисторическихдублейэтогоURLнеудалятьинескачиватьпятькопий. Вverificationбылоошибочносопоставленодвеожидаемыхdurationкjobid: правильныесвязи6cb5d156→w1Pr_lXdW2A826.554s,96d37aac→RINzuNBpPpk603.034s. Этоошибкаharness, неapp; исправленнаяпроверкапоsourcevideoidexec20343.
Новыйрегрессионныйpipelineтестlegacy_partial_cannot_be_resumed_as_a_newly_selected_renditionвпроцессеRED. Планправильногофикса: fingerprintmedia+formatдляHTTPpartial, старые/изменившиесяpartialнепереиспользовать, matchingfingerprintсохранять; толькоstaging, готовыевидеонетрогать. ВместоэтогонеослаблятьHTTP403guard. ВременныйStalePartialtypedleaseподход(addedcoretransfer+queue)планируетсяубратькакненужныйпослеidentityfix. ДополнитьTikTokbitrate.sizeизPlayAddr.DataSize. Послеэтогоhelperпересобрать/повторитьтолькоan1mu, новыеnativeWindows/Linuxpackagesобязательны.

## Дополнительный баг Save proxy, пользователь12.11

Факт: реальныйQt+Engine regression упал `Unsaved proxy protocol was reset by a settings refresh`. Root: ComboBox.currentIndexбылпривязанксохранённомуsettingsобъекту; фоновыеrefreshсбрасывалиdraft. Saveнауспехетолькоскрывалinlineeditor, неSettingsDialog; validationnoticeбылзаmodalокном. Исправлениевпроцессе: explicitdraftprotocolinitonOpened/cancel, inlineproxyErrorLabel, успешныйSaveзакрываетroot. ТестрасширенпроверкойHost/Port/HTTP/Login/PasswordSetпослеactualEngineRestartиочисткиpasswordредактора. GREEN ENexecвпроцессе; нуженRUиnativeWin10/11. ЭтоизменилоQML/C++testпослеall-errors-source.zip; ОБЯЗАТЕЛЬНОобновитьархив/manifestпередновымWindowsbuild.
WindowsbuildраньшеостановилсянаExecutionPolicy: добавить-ExecutionPolicyBypassтолькодовереннымscript-вызовамэтогопроцесса, системнуюpolicyнетрогать. FreeC2.5GiB; cargo clean--profiledevзапущенqgaPID6276(толькоreproducibledebug/testcache, release/package/profileсохранены). QAWin10перезапущенпослеevaluationauto-shutdown; обеимVMofflineдобавленвторойgraphicsVNCтолько127.0.0.1, SPICE/QXLнесменялись. ИсходныеXMLбэкапывresearchrelease-2026-11-12/*-before-vnc.xml; послеQAвернутьконфигиoffline. Win10VNCport5902, localwebconsolehttp127.0.0.1:6082, Websockifyexec21201 withExpectOrigin(negative403/positive101проверены), ChromeownTabwin10Tab/1760272770. NativeSkyRPCнеподключён; используемофициальныйChromepluginчерезлокальнуюконсоль. x11vncустановленизofficialExtraчерезвременныйpacmanconfigбезнеисправногоopenai-chatgptInclude(глобальныйpacman.confнетронут). websockify0.13.0uvtoolустановлен; officialnoVNC1.7.0NPMarchiveSHA512проверен.

12.11 checkpoint: все 9 исходных восстановленных записей прошли полный ffprobe/FFmpeg decode; история25 записей сохранена (20complete,5старыхдублейфильмаerror). Proxy Save: Qt+actualEngine EN/RU GREEN, draft не сбрасывается, inlineошибка видна, Save закрывает окно, настройки переживают restart. b6c035ad pushed, CI43199194885.
Dependency gate FAILED: RUSTSEC-2026-0459 в bundled libsqlite3-sys0.30.1; первичный RustSec advisory подтверждает затронутуюверсию. Scope defensive audit: собственный Cargo.lock/core Store, локальные копииDB/QA; разрешены SCA, обновление, unit/integration, packaging. Исключены чужие системы/аккаунты и изменение реальнойDB тестовымиSQL. PRIMARY supply-chain-security; источники read-only HTTPS, динамика offline/localhost. Обновлены rusqlite0.41.1/libsqlite3-sys0.39.1(SQLite3.54.1), лицензии363packages/387texts, vendor cache. Полный core/media прогон в процессе. Финальные пакеты b6c НЕ публиковать; нужны новые сборки после dependency fix.

Дополнение пользователя: новые вставленные загрузки должны сразу быть первыми, без появления в середине. Текущая Main.qml уже сортирует date descending и для одинаковой секунды использует обратный порядок вставки; parse сбрасывает предыдущие filter/name-sort, JobList возвращает viewport к новой первойстроке. Проверки fixture это покрывают. Добавлен actualEngine regression: две последовательные PasteLink поверх существующей истории, первыйряд сразу после вставки, после full download и после Engine restart. EN/RU в процессе. Не добавлять второй конкурирующий механизм сортировки или менять очередность скачивания.
Cargo-audit0.22.2 с актуальной первичной RustSecDB:0vulnerabilities, только unmaintained fxhash; SQLitefinding закрыта. SQLitecompat25recordsPASS. Новыйsource5188519push, CI43205359791. Linuxnative пересобран/установлен, пользовательскийпрофиль20complete/5historicduplicateerrorсохранён. НастоящийвидимыйGUI:Saveзакрылsettings, YouTubeиTikTokчерезPasteLinkcomplete/full decode, restartPASS. ОшибочнаясклееннаяссылкавQAprofile создана harnessиз-заappendвtoolbar; корректноотклонена приложением, к пользовательскойистории не относится.

Порядок: actualEngine EN/RU PASS включая две вставки, completion и restart, отдельный fixture same-second/scroll/filter PASS; commit1099a73 (tests/docs only, runtime5188519). Win10 native послеSQLite: core/media testsPASS, releasePASS6min, installerPASS139.922s. Запущен native EN/RU appearance/order/proxy. PowerShell5 трактовал stderr qInfo как terminatingNativeCommandError при *>; исправлен shared tests/check-appearance.ps1 на Start-Process с раздельными stderr/stdout и настоящим ExitCode. Это test runner, не ошибка приложения. Проверка нового runner в процессе.
SDKLinux новыйbinaryсобран14:13; packagingforegroundпрерван143дофинальногоbundle, НЕсчитатьстарыйdist/qa*.flatpakфинальным. Повтор упаковки в transient userunit videodock-qa-sqlite-flatpak-package-20261112 (RuntimeMax3600), неforegroundexec. СборкаSDKтожеuserunit videodock-qa-sdk-sqlite-build-20261112, завершена; исходный foregroundjob97216прерван143и былвозобновлён. Cargo-auditбинарьсобранuserunitпослеforeground143, актуальнаяDBв~/.cache/videodock-audit-db, JSON/tmp/videodock-20261112-sqlite-audit.json:0vulnerabilities.

Win10 nativeEN/RU order+proxy+realEngine PASS, result en0/ru0; scripts/check-appearance.ps1 PS5 fix GREEN. Windowscandidate: installer51ad03078a220cad6855049e80dd0c1765f9217f5dbf9f5d498061bdbaa4f616,165828473B; EXEdf5e19bf4c2480c3ab9e7745d25609bc0ef368f42ac0663bc7ec8b423463f0b5; helper87fc582dc3cf811f2fc167fd6daa07a6a336a59e05dacf6f6332cdcc016bc8d3 transferredhostpublic-qa-share/ui_engine.exe41133568B withSHA.
SDKFlatpakpackagecomplete143609944B SHA4d1f785250a0230ba0d7eddcab2623661668286aaea3ead82b36bc5cc43c7b37, installedupdateinprocess69236. systemdpackagingunitfinished14:26:55. CurrentCI43205359791 dependency-auditSUCCESS; otherbuildersinprogress.

Flatpakfinalinstalled worker offline8/proxyrefusal/startupPASS в реальном userDBus(журнал/tmp/videodock-20261112-flatpak-final-worker.log). Windows10appearanceactualEngineEN0/RU0подтверждены14:27. Windows11bootednormalQAlogin; QGAнеподключёндоlogin. ШтатнаяSPICETypeTextкнопкаоткрылаJS prompt, browser-clientaccept/getJSdialogупёрлисьвCDP Emulation.setFocusEmulationEnabled timeout10s. Парольневыводился; blindretryне делать. Нужнодиагностировать/закрытьтольконашустраницуконсоли, не менятьучётные данные/защитуOS.

Browserrecovery: новыйTab1760272841черезтотжеChrome2работает; старыйtarget2840застрялпослеJS prompt. QAконсоль(толькоprivateHTML)заменилаpromptнаtype=passwordinputсочисткойпередSpiceHtml5.typeText; локальныйQAcredentialвведёнбезвывода, полеочищеноtrue. Windows11обычныйлогин, небайпас. Отдельныйverifier43209711280слишкомрано: guardInvalid producer, потомучто43205359791ещёвпроцессе. ПерезапуститьfailedjobПОСЛЕзавершениявсегопроизводителя, guardsнеослаблять.

Windows11normal login PASS: актуальныйсекретвзяттольколокальноизLocalAccount VideoDockQA в provision-seed-v2.iso (en-US); старыйqa-local-password.txtотдругогосида не подходит. Сначалаинпутслаласинхронно: privateSPICEHTMLтеперьпоказываетText sentпослереальногоawait, очищаетполедопередачи; tempcredentialудалён. Защиту/паролиOSнеменяли. ТекущаяWin11EnterpriseEvalbuild26100(24H2),83днялицензии,IP192.168.122.248; QGAнеподключёнкакраньше(памятьvoxflow-windows-qa), нуженстарыйHTTPdriver/обычныйScheduledTaskGUI. Win10QAвыключенпослеEN/RU0, архив7.34MBс66+скриншотамивresearchwindows10-order-proxy-evidence.zip SHAad81e986..., исходныйXMLдоVNCвозвращёнoffline; тестовыйTдискотсоединён, cacheQCOWоставлен.

Win11 loginподтверждёнобычнымвходомсактуальнымLocalAccountизseed-v2; экранDesktopвиден. TaskSchedulerrootсодержиттолькоMS/OneDrive, нетQADriverзадачи. FileExplorerоткрытC:\VideoDockQA; естьстарыйdriver.ps1(09.11,1KB), готовкштатномуRunwithPowerShell. PrivateHTTPcontrollerперенесёнис09Novserve.py, ненужныеinstallerPOSTудалены; fixedGETmapиallowlistIPs99/248/host1, health/unknown/traversalchecksPASS. unitvideodock-qa-control-20261112 :39709; nextsequence1/read-onlyOSmetadata,stopfalse,pending. Proxybridge :39711→127.0.0.1:11086 scopeonlylibvirtguestsubnet.

Win11HTTPdriverreadyseq1PASS, actualGetCimInstancebuild26300 (суперседитскриншотbranding26100). FileExplorerdriver858Bсовпадаетshipped09Novhostscript858B; RunwithPowerShellнеподхватилпараметры, обычныйRunсфиксированнымexe-NoProfile-ExecutionPolicyBypass-FileC:\VideoDockQA\driver.ps1запустилбезсменыполитикиOS. seq2profileinitFAILиз-заошибкитестовогоscript(systemNotificationsвместосуществующегоsystemNotificationsEnabled); atomicsettingsнепринялинеизвестныйключ, seq3исправлениеPASS/0jobs/proxy_savedtrue. РодительскийдрайвервтестовойWindowsPowerShell, втерминалкодневводился; исходныйscriptзапущенчерезштатныйRun. CIWindowsinstaller/interfaceготовятсякзавершению; artifactVideoDock-windows-x64id13943056577/403499801Bдоступен, fetchвuserunitvideodock-qa-fetch-windows-ci-20261112 черезфактический~/.local/bin/gh (не/usr/bin).

CIinstallerdownloadedverified: Setup6195c1be8ed63481581c8fc41b1e1d81c75e2061d02fb96353aa2760300263d3/165476107B; EXE852e173051f5a95b6d16c79a04473ec226a189b9f05bd3a2ea0d355e6be3d345; shippedRustmanifestE95952...equalscurrenthost. Manifestresearchci-windows-candidate.json. NativeWin11seq4перепроверяетименноэтотустановщик:дваupgradepass, SHAстарогопрофиля, cleanPATH/отзывчивыйHWND. НоваякомбинированнаяQAпрограммабылаотклоненаdcg(heredoc.shell:launcher-unverified, dynamic &condition). Вместоallowlist/bypassиспользованстарыйпроверенныйлинейныйinstallerfixture; следующиесценарииписатьбездинамическогозапускаscriptblock.

16:35MSK: обнаруженыистёкшиевременныеQAконтроллер/мост(90min) - этообъясняетпоздниесетевыесрывы. Восстановленыкакvideodock-qa-control-final-20261112/.bridge-final RuntimeMax10800, bindтольколибвирт; персональный11086неменялся. Win10exactCIproxysaveчерезnativeUIAтеперьPASS/windowclosed/restartHTTP; передповторомнужнобылозакрытьоставшийсяпослеfailednetworkQAэкземпляр, профильleaseзащищалданные. Win11exactCIproxysaveHTTPиSOCKS5обапрошли. НеподтверждённыегипотезыTikTokRange/AcceptEncodingнеприменялисьвproduction: свежийdirectcontrolбезRange/сidentityдал200ифайл5450708B/20.25s audio+video; другиевыдачидали403,причинавариативностипокаuncertain. ИстёкшиймостпроверяетсявосстановленнымнаNativeUIretry.

16:38 nativeWin11: послевосстановленияQAсетиобычныйcontextMenu→Resumeтого же2326b723показалcomplete52s/23MB/1080p: новаястрокасверху, новыйjobне создавался. NativeWin10exactCIProxySaveтеперьPASS (windowclosed/restartHTTP/history2); прежнийNoWindowбылтестовымвторымэкземпляром/старымрезультатом, послеclose-existing/живогоконтроллерапрошёл. ВоWin10запущенещёодинреальныйShortsbчерезHTTP, воWin11обычныйResumeTikTokcb08черезSOCKS5. sourceproductionне менялся. TemporaryRustprobeперемещёнвresearchqa-tiktok-leases.rs,непубликовать.

Реальныйкрайнийслучайновоготребования:послекоррекциичасовсозданныевбудущемстарыеjobsперекрываютновые. REDподтверждёнCoretestadded_order_survives...иQtCorrecting the clock put a new download below older history. Добавленpersistedadded_order/private_last_added_order: единаяstampпослеизмененийstate, миграциябезизмененияid/даты/файла, library0, очередьможетменятьpriorityмассиванепереставляяdisplayorder; groupreplacementсохраняетorderоригинала. QMLdate-sortиспользуетordinal,legacyfixturefallbackdate. GREEN227/0/52ignored,Clippyworkspace/all-targets-DwarningsPASS,QtEN/RUactualEnginePASSвключаяfutureclockregression. retired_media_toolsfixtureусиленточныможиданиемстарогозаписа+новогопорядкового поля.
QAclock: Win10PacificUTCбыл+11h,Win11timezoneUTCбыл+3hиз-заlibvirtlocaltimeотhostEurope/Moscow. Officiallibvirtformatdomain(Timekeeping)подтверждаетoffsettimezone/utc. СделаныXMLбэкапы*-before-clock.xml,rollbackclock-rollback.md; Win10RTCtimezoneAmerica/Los_Angeles,Win11RTCUTC. Win10послебута2026-11-12T14:10:39ZсовпадаетсреальнымUTC. Никакихизмененийактивации/лицензии/искусственнойдаты,толькореальноевремяQA. NativeWin11послеперезагрузкинуженобычныйвход+driver,console6084→SPICE5901(5903послеперезагрузкинеактуален),next23/clockcheckstopfalse.
НовоеизменениеruntimeordinalобязательнотребуетновыхCIпакетов; 43205359791/6195CINativeбольшеНЕфинальные(всеихпроверкисохранены). Flatpakverifier43209711280успешенпослереруна,строгиеguardsнеослаблялись.

17:28MSK: послеRTCисправленияWin10UTC14:28совпалсреальнымUTC; Win11lockscreenпоказал14:20UTCиобычныйвходвVideoDockQAуспешен. Повторнаяошибкапаролябыласрывомввода12msподнагрузкой: privateSPICEclientkeyhold/shiftsettle80msиgap100ms,emptyfieldverified,Textsentпередsubmit,секретвtemp600удалён. Никакихсменыпароля/authbypass. BrowserноваяsessionсохраняетChrome2binding;старыеtabhandlesнепереподключать,новыйWin11tab1760272893. AXidsмогутstaleмеждувызовами: свежийget(state,disableDiffingtrue)иnumericlookupвидимыхlabel+buttonвтомжеnodecallрешили.
Linuxnativef8a5f25built/installedPASS,2GBpeak,3m49s; actualinstalledonboardingstartupпрожил8s(ожидаемыйtimeout),profilecopy25ids+всеcreated_at/pathсохраненыиorderдобавлен. НоваяCI43235498448f8a5f25.

17:53MSK final f8 installer PASS Windows10/11: two upgrades preserve profile, clean PATH startup, real UI Paste YouTube19s+TikTok32s full decode/newest-first/restart, native proxy Save closes+persists on both. Flatpak CI1a325836 installed and proof_worker --proxy --startup exit0. Evidence win{10,11}-ordinal-{install,zoo,tiktok,proxy}-proof.json. macOS producer still packaging; independent verifier must wait entire producer completed.
