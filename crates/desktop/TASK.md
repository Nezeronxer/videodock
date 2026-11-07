# Qt frontend

- [x] Define typed snapshot/action protocol and own UI scope.
- [x] Rust QObject bridge and Qt-only C++ platform glue.
- [x] Main toolbar/tabs/list/context actions/sort/selection/details.
- [x] Manual clip/batch/subscription and all format/language filters.
- [x] Five settings tabs, import/export and onboarding.
- [x] WebEngine home/navigation/download and private native cookie bridge.
- [x] Media Tools removed by the user; regression harness asserts no dialog remains.
- [x] English/Russian translation resources; sensitive inputs stay out of snapshots.
- [x] Cargo check/build, Qt6 lint, isolated real-app bootstrap, synthetic cookie self-check.
- [ ] Final core action integration and user-display walkthrough (separate whole-project gate).

Current boundary: `COVERAGE.md` distinguishes frontend controls, backend behavior and missing validation. No commit made.
