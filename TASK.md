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
