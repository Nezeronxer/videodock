# Frontend coverage

Implemented independently in Rust/CXX-Qt + Qt/QML; proprietary source/assets are not packaged. Each control dispatches a real action. A control is not proof that its backend action works: core/pipeline coverage must be checked separately.

| Area | Frontend status |
|---|---|
| Main toolbar, paste, smart mode, search, sort, 7 tabs | Implemented; actual clipboard uses Qt text control |
| List selection, context actions, details, deletion confirmation | Implemented; destructive file deletion requires an explicit checkbox |
| Manual clip/batch/subscription dialog | Implemented with parsed format filters, item selection, subtitle/audio multi-selection, output directory/name, download-existing |
| Settings General/Advanced/Connection/Notification/Account | All observed setting groups represented; core must validate/persist options and reject unavailable platform operations |
| Browser | Qt WebEngine home with 10 services; back/forward/home/URL/search/current-page download; no extractor runs in JS |
| Account cookies | Native Qt signals directly to Rust QObject, new private app profile only; values never enter snapshot/QML properties or logs |
| Media Tools | 4 enabled tool choices, process disabled when backend capability false; 14 original survey/future choices disabled |
| Import/export | Native file/folder dialogs and original-schema options; core handles JSON validation and storage |
| Onboarding | 5 independently illustrated steps; persisted completion is a core action |
| Tray/notification | Qt native implementation where supported; QApplication fallback enabled; actual delivery requires desktop host |
| Localization | Own English + Russian, 276 translated messages; remaining original locales are not implemented |

Verification performed:

- `cargo check -p videodock-desktop` passed with real core and Qt6.12/CXX-Qt0.10.
- `cargo build -p videodock-desktop` passed (jobs=2, target outside Dropbox).
- Qt6 `qmlformat` and `qmllint` parse all 9 components; generated Backend import requires build output import path.
- Real Rust binary booted for 5 seconds under Xvfb with a fresh temporary data directory; screenshot `/tmp/videodock-real-bootstrap.png` inspected. Rust QObject, core snapshot, translation resources and QML instantiated successfully.
- Temporary external QML test fixture exercised 5 settings tabs, audio filtering, manual dialog, media tools, transfer, details, onboarding and browser home. This was a fixture-based UI runtime check, not service behavior proof.
- `sh tests/check-cookie-bridge.sh` passed: synthetic localhost cookie addition/removal through isolated Qt profile reached QObject; removed cookie callback delivered an empty value. Generated CXX-Qt metaobject has the matching cookieEvent signature.

Remaining validation: live desktop flows on the user's actual display, account login and authorization verification, notifications/tray under their session, actual downloading/subscriptions/processing/export against the final Rust backend. The original Windows program was not run. Four AI tools remain unavailable unless actual compatible models/runtime are installed; no FFmpeg filter is presented as an AI substitute.
