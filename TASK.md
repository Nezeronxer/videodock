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
