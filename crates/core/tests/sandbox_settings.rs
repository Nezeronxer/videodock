#![cfg(target_os = "linux")]
use serde_json::json;
use std::process::Command;
use videodock_core::Engine;

#[test]
fn sandbox_settings_fail_before_changing_saved_preferences() {
    if std::env::var_os("VIDEODOCK_TEST_SANDBOX_CHILD").is_none() {
        let home = tempfile::tempdir().unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sandbox_settings_fail_before_changing_saved_preferences",
            ])
            .env("VIDEODOCK_TEST_SANDBOX_CHILD", "1")
            .env("FLATPAK_ID", "io.github.Nezeronxer.VideoDock")
            .env("HOME", home.path())
            .status()
            .unwrap();
        assert!(result.success());
        assert!(
            !home
                .path()
                .join(".config/autostart/videodock.desktop")
                .exists()
        );
        return;
    }
    let data = tempfile::tempdir().unwrap();
    let engine = Engine::new(data.path().to_owned()).unwrap();
    let snapshot = engine.snapshot();
    assert_eq!(snapshot["capabilities"]["auto_start"], false);
    assert_eq!(snapshot["capabilities"]["prevent_sleep"], false);
    for key in ["autoStart", "preventSleepEnabled"] {
        let mut values = json!({"quality":720});
        values[key] = json!(true);
        let error = engine
            .dispatch(json!({"action":"settings","values":values}))
            .unwrap_err();
        assert!(error.to_string().contains("пакете"));
        assert_eq!(engine.snapshot()["settings"], snapshot["settings"]);
    }
}
