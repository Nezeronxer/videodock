use std::{path::Path, sync::Arc};
use tokio_util::sync::CancellationToken;
use videodock_core::processing;

#[test]
fn capability_is_offline_and_rejects_dropbox_or_relative_model_storage() {
    assert!(!processing::available(Path::new("relative")));
    let dir = tempfile::tempdir().unwrap();
    assert!(!processing::available(&dir.path().join("Dropbox/models")));
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(
            target_os = "macos",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    assert!(processing::available(dir.path()));
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "capability detection must not install assets"
    );
    let file = dir.path().join("not-a-directory");
    std::fs::write(&file, b"unchanged").unwrap();
    assert!(!processing::available(&file));
    assert!(!processing::available(&file.join("child")));
}

#[tokio::test]
async fn unknown_effect_and_cancelled_work_do_not_touch_source_or_install_models() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.wav");
    std::fs::write(&input, b"unchanged").unwrap();
    let output = dir.path().join("out");
    let data = dir.path().join("data");
    let error = processing::process(
        input.clone(),
        "unknown-effect",
        output.clone(),
        data.clone(),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("эффект"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(
        processing::process(
            input.clone(),
            "extract_vocals",
            output,
            data.clone(),
            cancel,
            Arc::new(|_| {})
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(input).unwrap(), b"unchanged");
    assert!(!data.exists());
}
