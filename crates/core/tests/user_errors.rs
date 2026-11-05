use serde_json::json;
use videodock_core::Engine;

#[test]
fn saved_technical_errors_are_readable_in_every_user_view() {
    let directory = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open(directory.path().join("state.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE state (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        .unwrap();
    let saved = json!({"settings":{},"jobs":[{"id":"fixture","state":"error","error":"YouTube не вернул адрес SABR"}],"subscriptions":[{"id":"sub","error":"Сервер YouTube SABR вернул HTTP 403"}],"manual":{"error":"YouTube требует актуальный PO token"},"notice":"Нет конфигурации SABR"});
    db.execute("INSERT INTO state VALUES(1, ?1)", [saved.to_string()])
        .unwrap();
    drop(db);
    let engine = Engine::new(directory.path().to_owned()).unwrap();
    let snapshot = engine.snapshot();
    for error in [
        &snapshot["jobs"][0]["error"],
        &snapshot["subscriptions"][0]["error"],
    ] {
        let error = error.as_str().unwrap();
        assert!(
            !error.contains("SABR") && !error.contains("PO token"),
            "{error}"
        );
        assert!(error.contains("Повторите"), "{error}");
    }
    let persisted: String = rusqlite::Connection::open(directory.path().join("state.sqlite"))
        .unwrap()
        .query_row("SELECT value FROM state WHERE id=1", [], |r| r.get(0))
        .unwrap();
    assert!(
        persisted.contains("SABR"),
        "display conversion must preserve saved diagnostics"
    );
}
