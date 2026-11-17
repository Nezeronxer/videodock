# YouTube audio language, 17 November 2026

The automatic audio choice now follows the viewing language: an explicit `hl` in the source link, then a live, correctly scoped YouTube `PREF` language from VideoDock's browser, then the native OS UI language. Qt supplies the native language on Windows, macOS and Linux; terminal locale variables are only the non-desktop Core fallback.

The preference is saved when a job enters Smart Mode and carried into playlist/channel children and subscriptions. Existing jobs without the new optional field resolve it when starting. Manual language, original, all tracks and explicit audio format choices retain priority. If the language is absent, the existing default-track fallback applies; an incompatible codec cannot silently change an available requested language.

The SABR metadata adapter also extracts language from audio track identifiers when `xtags` has no language. Output audio streams receive the selected ISO language tag after metadata-copy settings, including when ordinary title/artist tags are disabled.

## Evidence

Evidence directory: `~/.local/share/videodock/research/audio-language-2026-11-17/`.

- `red.log`: the former selector chooses English instead of Russian, and substitutes a language to satisfy a codec.
- `green-core-all.log`: playlist language regression fails before propagation to children.
- `red-smart-language-actual.log`: enqueue loses the language before it can be persisted.
- `red-metadata-probed.log`: independent ffprobe observes `und` without explicit output language metadata.
- `workspace-green.log`: 243 passed, 0 failed, 53 intentionally ignored network/platform cases.
- `clippy-verified.log`: workspace/all-targets Clippy with `-D warnings` exits successfully. Existing Cargo manifest warnings about unused declarations remain.
- `build-native-verified.log`: the Linux desktop binary builds successfully.
- `live-ru-test.log`: a complete 826.434-second public video, Russian `251:ru.4`, 46,823,603 output bytes, full FFmpeg decode to EOF. ffprobe reports `rus`; offline detection of its first 15 seconds reports Russian with probability 0.998739.
- `live-en-alternate-route.log`: the same complete video, English, 45,658,114 output bytes, full FFmpeg decode to EOF. Offline detection reports English with probability 0.998765.
- `installation-proof.json`: the local native candidate is installed; the personal SQLite file and launcher symlink are preserved. Binary and SQLite backups are retained beside the evidence.
- `qt-en-fixed.log` / `qt-ru-fixed.log`: both real QML/Engine runs pass, including settings/proxy validation, paste downloads, restart, ordered history, local-library discovery and safe history removal. The library fixture prepares a stable file because production scanning excludes files younger than five seconds. Screenshots were inspected; an installed native Wayland window also starts in a separate profile and its QA unit is stopped afterward.
- `verification-summary.json`: consolidated independent ffprobe/ASR/file hashes, full-decode proofs, test totals, Qt outcomes and preservation checks for the personal database and installed binary.

The first live queue experiment reopened Core without replaying its synthetic browser cookie. Core intentionally does not restore browser credentials, so that experiment exercised the English system fallback. It is not evidence that a requested Russian SABR stream contained English. The corrected explicit-language complete downloads above establish the spoken languages independently of metadata.

## Remaining boundaries and yesterday's work

Language preferences from an unrelated external browser are not read automatically. A copied link with `hl`, or the language of YouTube inside VideoDock, supplies the viewing preference. With neither present, this Linux machine's configured system locale is `en_US.UTF-8`.

The first English attempts through the already prepared SOCKS endpoint 1086 encountered Google's PO rejection during transfer. The separate prepared endpoint 11086 completed; no personal network strategy was changed. This service-side failure remains visible, and no retry was added to conceal it.

Before this change, Windows CI 43955973047 / source 370335b passed the corrected EN/RU order checks and the complete verified installer handoff (35.51 seconds). Its public YouTube checks remained rejected by Google; TikTok passed. Both macOS architectures in 43950356238 passed interface/Engine checks and failed public downloads. These earlier results do not validate the new language code on native Windows/macOS. Native Windows 10/11 final client update/download gates and publication of 0.2.3 remain open; the stable release was not replaced.
