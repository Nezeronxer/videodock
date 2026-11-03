# Desktop/core protocol v1

The frontend is Qt/QML; its QObject bridge is Rust. No HTTP, extraction, persistence, processing or fake job state is implemented in QML. All commands reach `Engine::dispatch(Value)`. `Backend.snapshot` polls `Engine::snapshot()` every 250 ms. Dispatch errors appear in the notice bar and are never treated as success.

## Snapshot

```json
{
  "settings": {}, "jobs": [], "subscriptions": [], "processing": [],
  "manual": null, "account": {"logged_in":false,"user_name":"","avatar_url":""},
  "notice":"", "notification":null, "onboarding_completed":false
}
```

Job fields: `id` string, `title`, `url`, `kind` (`video`, `audio`, `playlist`, `channel`, `search`), `state` (`queued`, `parsing`, `downloading`, `paused`, `complete`, `error`), `progress` 0..100, `thumbnail`, `path`, `format`, `quality`, `size`, `speed`, `remaining`, `error`, optional `items[]`. Subscription fields are the same plus `last_checked`, `new_count`. Processing rows use job fields plus `effect`. Missing fields display empty text, never fake completion.

`manual`: `id` optional, `url`, `title`, `kind` (`video`, `audio`, `playlist`, `channel`, `search`), `state`, `error`, `thumbnail`, `formats[]`, `items[]`, `subtitles[]`, `audio_tracks[]`, `output_dir`, `filename`, `mode` (`download`/`subscribe`). Format object: `id`, `label`, `media_type`, `container`, `quality`, `video_codec`, `audio_codec`, `fps`, `vr`, `size`. Batch item: `id`, `title`, `duration`, `size`, `error`; track: `id`, `label`, `language`, `automatic`, `default`. The core owns parsing and validation. Manual dialog edits local preferences and dispatches them only on submit.

Settings camelCase keys mirror the evidence: `language`, `smartMode`, `mediaType`, `quality`, `container`, `videoCodec`, `audioCodec`, `frameRate`, `videoVR`, `subtitles`, `audioTracks`, `path`, `systemTrayIconVisible`, `autoStart`, `autoUpdate`, `betaAccessEnabled`, `simultaneousDownloads`, `subscriptionUpdatePeriod`, `preventSleepEnabled`, `separateFolderCreationEnabled`, `numerationEnabled`, `skipDuplicates`, `m3uEnabled`, `embedSubtitles`, `tagsEnabled`, `removeCompletedDownloads`, `recommendationServiceEnabled`, `aiProcessingAllowed`, `displayAllCodecs`, `speedLimit`, `proxyEnabled`, `proxyType`, `proxyHost`, `proxyPort`, `proxyLogin`, `proxyPassword`, `notificationEnabled`, `processingNotificationsEnabled`, `remindNotificationEnabled`, `similarVideosNotificationEnabled`, `askQuit`, `askDownloadChannel`, `systemNotificationsEnabled`, `notificationSound`.

Optional `settings_options` maps a setting key to a string array, replacing the frontend's finite default choices. `capabilities` may expose `processing`, `tray`, `browser`, `auto_start`. A capability set to false disables its corresponding control where applicable; absent capability means the core must reject unsupported commands explicitly.

## Actions

| action | Payload | Meaning |
|---|---|---|
| `parse` | `url` | Parse URL or batch input, publish `manual` or queue according to smart mode |
| `download` | `url`, `kind`, `mode`, `format_id`, `format`, `selected_ids[]`, `subtitles[]`, `audio_tracks[]`, `output_dir`, `filename`, `download_existing` | Queue exact manual preferences; mode `subscribe` creates subscription |
| `manual_close` | - | Clear manual state/cancel parsing dialog |
| `settings` | `values` object | Validate/persist a settings patch, report unsupported OS settings |
| `jobs` | `command`, `ids[]`, `delete_files` | `pause`, `resume`, `force`, `redownload`, `remove`, `open`, `reveal`, `open_url`, `copy_url`; empty ids means ALL only for pause/resume/remove |
| `subscriptions` | `command`, `ids[]` | `unsubscribe`, `edit`, `refresh`; edit publishes manual |
| `process` | `ids[]`, `effect` | Four effects: `remove_vocals`, `extract_vocals`, `extract_voice`, `remove_echo` |
| `import` | `path`, `preferences_from_file`, `download_complete_playlist`, `output_dir` | Import JSON |
| `export` | `path`, `subscriptions_only` | Export JSON |
| `account` | `command` | `login`, `logout`, `liked`, `watch_later`; login response may include `url` for browser; optional `subscribe` on library commands |
| `account` | `command:cookie`, `name`, `value`, `domain`, `path`, `secure`, `expires`, `removed` | Native Qt callback directly to Rust; never visible in QML state or logs |
| `app` | `command` | `check_updates`, `open_logs`, `open_directory`, `onboarding_complete`, `clear_notice` |

`dispatch` response is any JSON object; `error` string is a failure; optional `url` navigates the in-app browser, optional `clipboard` is copied through Qt's text control. All unsupported actions must return errors.

Notification shape: `{id,title,body}`. UI shows each id once through Qt's platform tray notification when enabled. No cookies, account passwords or tokens are part of the snapshot. Proxy password is a local input only: core should return `proxyPasswordSet: true` instead of the password; an untouched password field is omitted on Save. `account.login` must not claim login until the engine actually verifies authorization. Native Qt cookieAdded/cookieRemoved callbacks deliver values directly to the Rust QObject, without QML properties/signals. Expiry is a Unix timestamp (null for a session cookie); expired values are never sent. Core does not persist browser cookies: startup restoration comes only from the Qt profile. The profile uses only the new app's `data_dir/browser` directory (0700 on Unix); callbacks connect before user interaction, load existing app-profile cookies, and logout clears this profile's cookies. No Chrome/4K/other profiles are read.

## Coverage boundaries

This is independently written UI informed by observed geometry and behavior; no extracted proprietary QML or imagery is packaged. Original license/activation flows are excluded. Four Media Tools are actionable; fourteen future/survey effects remain visibly disabled. Default options are explicit choices, not promises that a source provides that format. The engine must validate format choices against parsed formats.

OS tray requires a system tray host. Native auto-start, sleep inhibition and sound are core/platform responsibilities. Updates/help/log actions need real core implementations; errors stay visible. Qt WebEngine must be present. Authentication/captcha/protected content and supported-service reliability require real service tests. QML/offscreen startup is not a live user-flow pass.

Language support currently shipped: independently translated Russian (276 messages) and source English. The original's other locales are not packaged; the language chooser offers only these two. Rebuild Russian after edits using Qt6 `lupdate qml -no-obsolete -ts qml/translations/videodock_ru.ts` and `lrelease ... -qm qml/translations/videodock_ru.qm`.
