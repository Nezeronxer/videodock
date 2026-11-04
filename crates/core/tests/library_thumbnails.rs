use serde_json::json;
use std::{
    fs,
    time::{Duration, Instant, SystemTime},
};
use tempfile::TempDir;
use videodock_core::Engine;

#[test]
fn local_videos_and_saved_downloads_have_cached_previews_after_restart() {
    let tmp = TempDir::new().unwrap();
    let folder = tmp.path().join("videos");
    let profile = tmp.path().join("profile");
    fs::create_dir_all(&folder).unwrap();
    fs::create_dir(&profile).unwrap();
    let source = folder.join("local.mp4");
    assert!(
        std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=160x90:r=10",
                "-t",
                "0.5",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p"
            ])
            .arg(&source)
            .status()
            .unwrap()
            .success()
    );
    let saved = folder.join("saved.mp4");
    fs::copy(&source, &saved).unwrap();
    for path in [&source, &saved] {
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(10))
            .unwrap();
    }
    let state = json!({"settings":{"path":folder},"jobs":[{"id":"saved","title":"Saved","kind":"video","state":"complete","path":saved}],"subscriptions":[],"processing":[]});
    let db = rusqlite::Connection::open(profile.join("state.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE state (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        .unwrap();
    db.execute("INSERT INTO state VALUES(1, ?1)", [state.to_string()])
        .unwrap();
    drop(db);
    let read = |engine: &Engine| {
        let started = Instant::now();
        loop {
            let state = engine.snapshot();
            if state["jobs"].as_array().unwrap().iter().any(|j| {
                j["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("library-"))
            }) {
                break state;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "Library scan deadline: path={} jobs={} notice={}",
                state["settings"]["path"],
                state["jobs"],
                state["notice"]
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    assert_eq!(
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(videodock_core::library::scan(&folder))
            .unwrap()
            .len(),
        2,
        "Fixture scan did not find both videos"
    );
    let engine = Engine::new(profile.clone()).unwrap();
    let state = read(&engine);
    assert_eq!(state["jobs"].as_array().unwrap().len(), 2);
    let previews = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|job| {
            let image = job["thumbnail"]
                .as_str()
                .filter(|s| !s.is_empty())
                .expect("Video row has no thumbnail");
            let path = url::Url::parse(image).unwrap().to_file_path().unwrap();
            assert!(path.starts_with(profile.join("thumbnails")));
            assert!(fs::read(&path).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
            (job["id"].clone(), image.to_owned())
        })
        .collect::<Vec<_>>();
    drop(engine);
    let engine = Engine::new(profile).unwrap();
    let state = read(&engine);
    for (id, image) in previews {
        assert_eq!(
            state["jobs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|j| j["id"] == id)
                .unwrap()["thumbnail"],
            image
        );
    }
}
