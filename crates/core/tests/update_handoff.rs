//! Real public installer, isolated Windows handoff and profile preservation; never installed with the app.
#![cfg(windows)]
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;
use videodock_core::updater::{Update, download, install_on_exit};

#[test]
#[ignore = "child process of the isolated Windows installer handoff test"]
fn install_handoff_child() {
    assert_eq!(
        std::env::var("VIDEODOCK_DISPOSABLE_UPDATE_TEST").as_deref(),
        Ok("1"),
        "real installer test requires an explicitly disposable Windows environment"
    );
    let file =
        std::env::var_os("VIDEODOCK_HANDOFF_METADATA").expect("parent handoff fixture required");
    let update: Update = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let installer = PathBuf::from(std::env::var_os("VIDEODOCK_HANDOFF_INSTALLER").unwrap());
    install_on_exit(&installer, &update).expect("verified installer handoff failed");
}

#[tokio::test]
#[ignore = "downloads and installs the real public Windows package into an isolated temporary directory"]
async fn full_verified_windows_update_handoff() {
    assert_eq!(
        std::env::var("VIDEODOCK_DISPOSABLE_UPDATE_TEST").as_deref(),
        Ok("1"),
        "real installer test changes per-user app registration; use only a disposable Windows environment"
    );
    use std::process::{Command, Stdio};
    let temp = tempfile::Builder::new()
        .prefix("VideoDock update ' ")
        .tempdir()
        .unwrap();
    let root = temp.path();
    let app = root.join("installed app");
    let profile = root.join("profile");
    let output = root.join("output");
    for directory in [&app, &profile, &output] {
        std::fs::create_dir(directory).unwrap();
    }
    let existing = output.join("existing-download.bin");
    std::fs::write(&existing, b"existing download fixture").unwrap();
    let saved = serde_json::json!({"_automatic_updates_v1":true,"onboarding_completed":true,"settings":{"autoUpdate":false,"askQuit":false,"path":output},"jobs":[{"id":"existing-fixture","state":"complete","kind":"video","title":"Existing fixture","path":existing,"created_at":1}]});
    let db = rusqlite::Connection::open(profile.join("state.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE state (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        .unwrap();
    db.execute("INSERT INTO state VALUES(1,?1)", [saved.to_string()])
        .unwrap();
    drop(db);
    let client = videodock_media::catalog::Catalog::new(None)
        .unwrap()
        .client();
    let mut response = client
        .get("https://api.github.com/repos/Nezeronxer/videodock/releases/latest")
        .header("User-Agent", "VideoDock-update-QA")
        .send()
        .await
        .expect("public release metadata request failed")
        .error_for_status()
        .expect("public release metadata status failed");
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.unwrap() {
        assert!(
            bytes.len() + chunk.len() <= 1024 * 1024,
            "release metadata exceeded fixture budget"
        );
        bytes.extend_from_slice(&chunk);
    }
    let release: Value = serde_json::from_slice(&bytes).unwrap();
    let version = release["tag_name"]
        .as_str()
        .unwrap()
        .trim_start_matches('v');
    let name = format!("VideoDock-{version}-windows-x64-Setup.exe");
    let asset = release["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["name"] == name)
        .unwrap();
    let update = Update {
        version: version.into(),
        url: asset["browser_download_url"].as_str().unwrap().into(),
        sha256: asset["digest"]
            .as_str()
            .unwrap()
            .strip_prefix("sha256:")
            .unwrap()
            .into(),
        bytes: asset["size"].as_u64().unwrap(),
    };
    let installer = download(&client, &update, root, &CancellationToken::new())
        .await
        .unwrap_or_else(|_| panic!("verified stable installer download failed"));
    let metadata = root.join("metadata.json");
    std::fs::write(&metadata, serde_json::to_vec(&update).unwrap()).unwrap();
    let executable = app.join("VideoDock.exe");
    std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    let bootstrap_hash = Sha256::digest(std::fs::read(&executable).unwrap());
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
    let powershell = system.join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let quoted = format!("'{}'", executable.display().to_string().replace('\'', "''"));
    struct CloseFixture {
        shell: PathBuf,
        path: String,
    }
    impl Drop for CloseFixture {
        fn drop(&mut self) {
            let script = format!(
                "Get-Process VideoDock -ErrorAction SilentlyContinue | Where-Object {{$_.Path -eq {}}} | ForEach-Object {{$null=$_.CloseMainWindow(); if(-not $_.WaitForExit(10000)){{$_.Kill()}}}}",
                self.path
            );
            let _ = Command::new(&self.shell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
    let cleanup = CloseFixture {
        shell: powershell.clone(),
        path: quoted.clone(),
    };
    let mut child = Command::new(&executable)
        .args([
            "--exact",
            "install_handoff_child",
            "--ignored",
            "--nocapture",
        ])
        .env("VIDEODOCK_HANDOFF_METADATA", &metadata)
        .env("VIDEODOCK_HANDOFF_INSTALLER", &installer)
        .env("VIDEODOCK_DATA_DIR", &profile)
        .env("QT_QUICK_BACKEND", "software")
        .env(
            "PATH",
            format!("{};{}", system.join("System32").display(), system.display()),
        )
        .env_remove("QT_PLUGIN_PATH")
        .env_remove("QML_IMPORT_PATH")
        .env_remove("QML2_IMPORT_PATH")
        .env_remove("QTDIR")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "bootstrap did not arrange the installer handoff"
    );
    let script = format!(
        "$p=Get-Process VideoDock -ErrorAction SilentlyContinue | Where-Object {{$_.Path -eq {} -and $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -eq 'VideoDock' -and $_.Responding}}; if($p){{exit 0}}; exit 1",
        quoted
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(240);
    loop {
        let ready = Command::new(&powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        if ready {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "installer did not restart a responsive packaged VideoDock window"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert_ne!(
        Sha256::digest(std::fs::read(&executable).unwrap()),
        bootstrap_hash,
        "installer did not replace the bootstrap executable"
    );
    let lease = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(profile.join("state.lock"))
        .expect("restarted app did not open the fixture profile");
    assert!(
        lease.try_lock().is_err(),
        "restarted app is not holding the fixture profile lease"
    );
    let db = rusqlite::Connection::open(profile.join("state.sqlite")).unwrap();
    let state: String = db
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    let state: Value = serde_json::from_str(&state).unwrap();
    assert!(
        state["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| job["id"] == "existing-fixture")
    );
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        b"existing download fixture"
    );
    drop(db);
    drop(cleanup);
    println!(
        "real stable Windows update: verified download, exited bootstrap, silent installer, responsive packaged restart, preserved history/file; version={} bytes={}",
        update.version, update.bytes
    );
}
