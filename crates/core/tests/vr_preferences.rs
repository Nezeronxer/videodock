use serde_json::{Value, json};
use videodock_core::Engine;

#[test]
fn exact_vr_modes_persist_and_original_schema_round_trips_without_collapsing() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let mut engine = Engine::new(data.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"subscriptionUpdatePeriod":0}}))
        .unwrap();
    for (index, mode) in ["Any", "2D/360°", "3D/180°", "3D/360°", "None"]
        .into_iter()
        .enumerate()
    {
        engine
            .dispatch(json!({"action":"settings","values":{"videoVR":mode}}))
            .unwrap();
        assert_eq!(engine.snapshot()["settings"]["videoVR"], mode);
        let native_dimension = if mode == "None" { "2D" } else { mode };
        let input = temp.path().join(format!("input-{index}.json"));
        std::fs::write(&input,serde_json::to_vec(&json!({"json_schema_version":"1.0.0","items":[{"urlDesc":{"serviceName":"html","type":"playlist","url":format!("http://127.0.0.1:9/fixture-{index}")},"isSubscription":true,"downloadPreferences":{"format":{"container":"MKV","video":{"dimension":native_dimension}}}}]})).unwrap()).unwrap();
        engine
            .dispatch(json!({"action":"import","path":input,"output_dir":temp.path()}))
            .unwrap();
        drop(engine);
        engine = Engine::new(data.clone()).unwrap();
        assert_eq!(engine.snapshot()["settings"]["videoVR"], mode);
        let export = temp.path().join(format!("export-{index}.json"));
        engine
            .dispatch(json!({"action":"export","path":export,"subscriptions_only":true}))
            .unwrap();
        let exported: Value = serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
        assert_eq!(
            exported["items"][index]["downloadPreferences"]["format"]["video"]["dimension"],
            native_dimension
        );
    }
    engine
        .dispatch(json!({"action":"settings","values":{"videoVR":true}}))
        .unwrap();
    drop(engine);
    let engine = Engine::new(data).unwrap();
    assert_eq!(engine.snapshot()["settings"]["videoVR"], true);
}

#[test]
fn old_database_selection_true_remains_any_vr_instead_of_becoming_3d() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let engine = Engine::new(data.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"subscriptionUpdatePeriod":0}}))
        .unwrap();
    let input = temp.path().join("legacy.json");
    std::fs::write(&input,serde_json::to_vec(&json!({"json_schema_version":"1.0.0","items":[{"urlDesc":{"serviceName":"html","type":"playlist","url":"http://127.0.0.1:9/legacy"},"isSubscription":true,"downloadPreferences":{"format":{"container":"MKV","video":{"dimension":"360"}}}}]})).unwrap()).unwrap();
    engine
        .dispatch(json!({"action":"import","path":input,"output_dir":temp.path()}))
        .unwrap();
    drop(engine);
    let connection = rusqlite::Connection::open(data.join("state.sqlite")).unwrap();
    let raw: String = connection
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    let mut old: Value = serde_json::from_str(&raw).unwrap();
    old["subscriptions"][0]["_spec"]["selection"]
        .as_object_mut()
        .unwrap()
        .remove("vr_mode");
    assert_eq!(old["subscriptions"][0]["_spec"]["selection"]["vr"], true);
    connection
        .execute("UPDATE state SET value=?1 WHERE id=1", [old.to_string()])
        .unwrap();
    drop(connection);
    let engine = Engine::new(data).unwrap();
    let output = temp.path().join("legacy-export.json");
    engine
        .dispatch(json!({"action":"export","path":output,"subscriptions_only":true}))
        .unwrap();
    let exported: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(
        exported["items"][0]["downloadPreferences"]["format"]["video"]["dimension"],
        "VR"
    );
}
