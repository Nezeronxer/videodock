use serde_json::json;
use videodock_core::Engine;

#[test]
fn window_size_survives_restart_and_invalid_geometry_is_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data");
    let engine = Engine::new(path.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"windowWidth":1300,"windowHeight":900}}))
        .unwrap();
    for invalid in [
        json!({"windowWidth":0}),
        json!({"windowHeight":-1}),
        json!({"windowWidth":20000}),
        json!({"windowHeight":"large"}),
    ] {
        assert!(
            engine
                .dispatch(json!({"action":"settings","values":invalid}))
                .is_err()
        );
        assert_eq!(engine.snapshot()["settings"]["windowWidth"], 1300);
        assert_eq!(engine.snapshot()["settings"]["windowHeight"], 900);
    }
    drop(engine);
    let restored = Engine::new(path).unwrap();
    assert_eq!(restored.snapshot()["settings"]["windowWidth"], 1300);
    assert_eq!(restored.snapshot()["settings"]["windowHeight"], 900);
}
