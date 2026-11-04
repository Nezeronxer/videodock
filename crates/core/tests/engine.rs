use serde_json::{Value, json};
use tempfile::TempDir;
use videodock_core::Engine;

#[test]
fn changing_library_folder_removes_old_scan_rows_but_keeps_download_history() {
    let data = TempDir::new().unwrap();
    let old = data.path().join("old");
    let saved = json!({"settings":{"path":old},"jobs":[
        {"id":"library-synthetic","title":"Old library file","state":"complete","_library":true,"path":old.join("file.mp4")},
        {"id":"download-synthetic","title":"Download history","state":"complete","path":old.join("download.mp4")}
    ],"subscriptions":[],"processing":[]});
    let store = rusqlite::Connection::open(data.path().join("state.sqlite")).unwrap();
    store
        .execute_batch("CREATE TABLE state (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        .unwrap();
    store
        .execute("INSERT INTO state VALUES(1, ?1)", [saved.to_string()])
        .unwrap();
    drop(store);
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert_eq!(engine.snapshot()["jobs"].as_array().unwrap().len(), 2);
    engine
        .dispatch(
            json!({"action":"settings","values":{"path":data.path().join("new-empty-folder")}}),
        )
        .unwrap();
    let snapshot = engine.snapshot();
    let jobs = snapshot["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1, "old folder scan is still visible");
    assert_eq!(jobs[0]["id"], "download-synthetic");
}

#[test]
fn theme_is_validated_and_survives_restart() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert_eq!(engine.snapshot()["settings"]["theme"], "dark");
    for theme in ["light", "system", "dark"] {
        engine
            .dispatch(json!({"action":"settings","values":{"theme":theme}}))
            .unwrap();
        assert_eq!(engine.snapshot()["settings"]["theme"], theme);
    }
    for invalid in [json!("sepia"), json!(true), json!(null), json!(3)] {
        assert!(
            engine
                .dispatch(json!({"action":"settings","values":{"theme":invalid,"quality":720}}))
                .is_err()
        );
        assert_eq!(engine.snapshot()["settings"]["theme"], "dark");
        assert_eq!(engine.snapshot()["settings"]["quality"], 1080);
    }
    engine
        .dispatch(json!({"action":"settings","values":{"theme":"light"}}))
        .unwrap();
    drop(engine);
    let recovered = Engine::new(data.path().to_owned()).unwrap();
    assert_eq!(recovered.snapshot()["settings"]["theme"], "light");
}

#[test]
fn settings_are_normalized_persisted_and_rejected_atomically() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"quality":"4K","container":"MKV","simultaneousDownloads":"2","subscriptionUpdatePeriod":"15 min","speedLimit":"1 Mbps"}})).unwrap();
    let settings = engine.snapshot()["settings"].clone();
    assert_eq!(settings["quality"], 2160);
    assert_eq!(settings["container"], "mkv");
    assert_eq!(settings["simultaneousDownloads"], 2);
    assert_eq!(settings["subscriptionUpdatePeriod"], 900);
    assert_eq!(settings["speedLimit"], 125000);
    assert!(
        engine
            .dispatch(json!({"action":"settings","values":{"quality":720,"proxyPort":70000}}))
            .is_err()
    );
    assert_eq!(engine.snapshot()["settings"]["quality"], 2160);
    drop(engine);
    let recovered = Engine::new(data.path().to_owned()).unwrap();
    assert_eq!(recovered.snapshot()["settings"]["quality"], 2160);
}

#[test]
fn secrets_never_enter_snapshot_sqlite_or_export() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    let secret = "synthetic-secret-not-a-real-credential";
    engine.dispatch(json!({"action":"settings","values":{"proxyHost":"127.0.0.1","proxyPort":1080,"proxyLogin":"test","proxyPassword":secret}})).unwrap();
    engine.dispatch(json!({"action":"account","command":"cookie","name":"fixture","value":secret,"domain":".example.invalid","path":"/","secure":true})).unwrap();
    assert!(!engine.snapshot().to_string().contains(secret));
    assert_eq!(engine.snapshot()["settings"]["proxyPasswordSet"], true);
    let export = data.path().join("export.json");
    engine
        .dispatch(json!({"action":"export","path":export}))
        .unwrap();
    assert!(!std::fs::read_to_string(export).unwrap().contains(secret));
    for name in ["state.sqlite", "state.sqlite-wal"] {
        if let Ok(bytes) = std::fs::read(data.path().join(name)) {
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|part| part == secret.as_bytes())
            );
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(data.path().join("secrets.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn import_validates_before_mutating_and_round_trips_original_schema() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    let source = data.path().join("source.json");
    std::fs::write(&source, serde_json::to_vec(&json!({"json_schema_version":"1.0.0","items":[{"urlDesc":{"serviceName":"youtube","type":"playlist","url":"https://www.youtube.com/playlist?list=PLfixture"},"isSubscription":true,"downloadPreferences":{"format":{"container":"MKV","video":{"resolution":"720p","codec":"H264"},"audio":{"codec":"Any","bitrate":128000}},"outputDir":data.path()}}]})).unwrap()).unwrap();
    engine.dispatch(json!({"action":"import","path":source,"preferences_from_file":true,"download_complete_playlist":false,"output_dir":data.path()})).unwrap();
    assert_eq!(
        engine.snapshot()["subscriptions"].as_array().unwrap().len(),
        1
    );
    let before = engine.snapshot()["subscriptions"].clone();
    std::fs::write(
        &source,
        br#"{"json_schema_version":"1.0.0","items":[{"urlDesc":{"url":"file:///etc/passwd"}}]}"#,
    )
    .unwrap();
    assert!(
        engine
            .dispatch(json!({"action":"import","path":source}))
            .is_err()
    );
    assert_eq!(engine.snapshot()["subscriptions"], before);
    let export = data.path().join("export.json");
    engine
        .dispatch(json!({"action":"export","path":export,"subscriptions_only":true}))
        .unwrap();
    let result: Value = serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
    assert_eq!(result["json_schema_version"], "1.0.0");
    assert_eq!(result["items"][0]["isSubscription"], true);
    assert_eq!(
        result["items"][0]["downloadPreferences"]["format"]["video"]["resolution"],
        "720p"
    );
}

#[test]
fn malformed_actions_and_unavailable_processing_are_explicit_errors() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert!(
        engine
            .dispatch(json!({"action":"jobs","command":"remove","ids":["unknown"]}))
            .is_err()
    );
    assert!(
        engine
            .dispatch(json!({"action":"settings","values":{"unknown_setting":true}}))
            .is_err()
    );
    assert!(
        engine
            .dispatch(json!({"action":"process","ids":["missing"],"effect":"remove_vocals"}))
            .is_err()
    );
    assert_eq!(engine.snapshot()["processing"], json!([]));
}

#[test]
fn one_state_directory_has_only_one_engine_owner() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert!(
        Engine::new(data.path().to_owned()).is_err(),
        "a second independent actor could overwrite the first actor's SQLite snapshot"
    );
    let clone = engine.clone();
    drop(engine);
    assert!(Engine::new(data.path().to_owned()).is_err());
    drop(clone);
    assert!(Engine::new(data.path().to_owned()).is_ok());
}
