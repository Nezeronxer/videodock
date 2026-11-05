use serde_json::{Value, json};
use tempfile::TempDir;
use videodock_core::Engine;

fn save(directory: &std::path::Path, state: &Value) {
    let db = rusqlite::Connection::open(directory.join("state.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    db.execute("INSERT INTO state VALUES(1,?1)", [state.to_string()])
        .unwrap();
}
fn stored(directory: &std::path::Path) -> Value {
    let db = rusqlite::Connection::open(directory.join("state.sqlite")).unwrap();
    let text: String = db
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn legacy_outputs_and_download_history_survive_retirement_and_restart() {
    let data = TempDir::new().unwrap();
    let output = data.path().join("previous-output.wav");
    std::fs::write(&output, b"existing user output").unwrap();
    let legacy = json!({"id":"old-effect","title":"Saved local output","state":"complete","effect":"extract_voice","path":output,"files":[output],"progress":100,"_input":data.path().join("original.mp4")});
    let history = json!({"id":"old-download","title":"Existing history","state":"complete","path":data.path().join("download.mp4")});
    save(
        data.path(),
        &json!({"settings":{"quality":720,"theme":"light","path":data.path().join("uncreated-library"),"aiProcessingAllowed":true,"processingNotificationsEnabled":true},"jobs":[history.clone()],"subscriptions":[],"processing":[legacy.clone()]}),
    );
    let engine = Engine::new(data.path().to_owned()).unwrap();
    let snapshot = engine.snapshot();
    assert!(snapshot.get("processing").is_none());
    assert!(snapshot.get("_legacy_processing").is_none());
    assert!(snapshot["capabilities"].get("processing").is_none());
    assert!(snapshot["settings"].get("aiProcessingAllowed").is_none());
    assert!(
        snapshot["settings"]
            .get("processingNotificationsEnabled")
            .is_none()
    );
    assert_eq!(snapshot["jobs"][0], history);
    assert_eq!(snapshot["settings"]["quality"], 720);
    assert_eq!(snapshot["settings"]["theme"], "light");
    let persisted = stored(data.path());
    assert_eq!(persisted["_legacy_processing"], json!([legacy]));
    assert!(persisted.get("processing").is_none());
    assert_eq!(std::fs::read(&output).unwrap(), b"existing user output");
    let error = engine
        .dispatch(json!({"action":"process"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("удалены"));
    assert!(
        engine
            .dispatch(
                json!({"action":"settings","values":{"aiProcessingAllowed":false,"quality":480}})
            )
            .is_err()
    );
    assert_eq!(engine.snapshot()["settings"]["quality"], 720);
    drop(engine);
    let reopened = Engine::new(data.path().to_owned()).unwrap();
    assert_eq!(reopened.snapshot()["jobs"][0], history);
    assert_eq!(
        stored(data.path())["_legacy_processing"],
        persisted["_legacy_processing"]
    );
    assert_eq!(std::fs::read(output).unwrap(), b"existing user output");
}

#[test]
fn a_profile_reopened_by_an_old_version_does_not_overwrite_archived_outputs() {
    let data = TempDir::new().unwrap();
    let old = json!({"id":"same-id","state":"complete","path":data.path().join("first.wav")});
    let newer = json!({"id":"same-id","state":"complete","path":data.path().join("second.wav")});
    save(
        data.path(),
        &json!({"settings":{"path":data.path().join("uncreated-library")},"jobs":[],"subscriptions":[],"_legacy_processing":[old.clone()],"processing":[newer.clone()]}),
    );
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert!(engine.snapshot().get("processing").is_none());
    assert_eq!(
        stored(data.path())["_legacy_processing"],
        json!([old, newer])
    );
}

#[test]
fn new_profiles_have_no_processing_queue_capability_or_settings() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    let snapshot = engine.snapshot();
    assert!(snapshot.get("processing").is_none());
    assert!(snapshot["capabilities"].get("processing").is_none());
    assert!(snapshot["settings"].get("aiProcessingAllowed").is_none());
    assert!(
        engine
            .dispatch(json!({"action":"process","effect":"remove_vocals","ids":[]}))
            .unwrap_err()
            .to_string()
            .contains("удалены")
    );
}

#[test]
fn pending_legacy_work_is_archived_instead_of_becoming_a_resumable_download() {
    let data = TempDir::new().unwrap();
    let pending = json!({"id":"pending-effect","state":"queued","effect":"extract_voice","_input":data.path().join("input.wav")});
    save(
        data.path(),
        &json!({"settings":{"path":data.path().join("uncreated-library")},"jobs":[],"subscriptions":[],"processing":[pending.clone()]}),
    );
    let engine = Engine::new(data.path().to_owned()).unwrap();
    assert!(engine.snapshot()["jobs"].as_array().unwrap().is_empty());
    assert!(
        engine
            .dispatch(json!({"action":"jobs","command":"resume","ids":["pending-effect"]}))
            .is_err()
    );
    assert_eq!(stored(data.path())["_legacy_processing"], json!([pending]));
}
