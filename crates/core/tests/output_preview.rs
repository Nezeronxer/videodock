use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};
use videodock_core::{Engine, pipeline};

async fn wait(engine: &Engine, predicate: impl Fn(&Value) -> bool) -> Value {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state = engine.snapshot();
            if predicate(&state) {
                return state;
            }
            assert!(
                state["jobs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|job| job["state"] != "error"),
                "Job failed: {}",
                state["jobs"]
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Engine did not reach the expected state")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_preview_is_pure_and_matches_the_real_changed_format_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.mp4");
    assert!(
        Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=160x90:r=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100",
                "-t",
                "1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac"
            ])
            .arg(&source)
            .status()
            .await
            .unwrap()
            .success()
    );
    let source = Arc::new(tokio::fs::read(source).await.unwrap());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let source_url = format!("{base}/watch?signature=synthetic-source-token");
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let source = source.clone();
            let base = base.clone();
            let requests = observed.clone();
            tokio::spawn(async move {
                let mut buffer = [0; 16384];
                let count = socket.read(&mut buffer).await.unwrap();
                requests.fetch_add(1, Ordering::SeqCst);
                let request = String::from_utf8_lossy(&buffer[..count]);
                let path = request.split_whitespace().nth(1).unwrap();
                let (content_type, body) = if path.starts_with("/watch") {
                    ("text/html", format!("<title>{base}/fallback?signature=synthetic-title-token</title><video src=\"/media.mp4?signature=synthetic-stream-token\"></video>").into_bytes())
                } else {
                    ("video/mp4", source.to_vec())
                };
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            });
        }
    });
    let output = temp.path().join("output");
    let engine = Engine::new(temp.path().join("data")).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"smartMode":false,"path":output}}))
        .unwrap();
    engine
        .dispatch(json!({"action":"parse","url":source_url}))
        .unwrap();
    let state = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    let initial = state["manual"]["download_path"]
        .as_str()
        .expect("Manual snapshot lacks the real default download path");
    assert!(initial.ends_with(".mp4"));
    assert!(!initial.contains("synthetic-title-token"));
    let before_requests = requests.load(Ordering::SeqCst);
    let before = engine.snapshot();
    let default_preview = engine
        .dispatch(json!({"action":"preview_output","url":state["manual"]["url"]}))
        .unwrap();
    assert_eq!(default_preview["download_path"], initial);
    let mut action = json!({"action":"preview_output","url":state["manual"]["url"],"format_id":state["manual"]["formats"][0]["id"],"format":{"media_type":"video","container":"MKV","quality":"Any","video_codec":"Any","audio_codec":"Any","fps":"Any","vr":"2D"},"filename":"custom file.mkv","output_dir":output});
    let preview = engine.dispatch(action.clone()).unwrap();
    assert_eq!(
        preview["download_path"],
        output.join("custom file.mkv").to_str().unwrap()
    );
    assert!(!output.exists(), "Preview created the output directory");
    assert_eq!(
        requests.load(Ordering::SeqCst),
        before_requests,
        "Preview made a network request"
    );
    assert_eq!(engine.snapshot(), before, "Preview mutated engine state");
    let mut invalid = action.clone();
    invalid["filename"] = json!("../escape.mkv");
    assert!(engine.dispatch(invalid).is_err());
    assert!(!output.exists());
    action["action"] = json!("download");
    engine.dispatch(action).unwrap();
    let completed = wait(&engine, |state| state["jobs"][0]["state"] == "complete").await;
    assert_eq!(completed["jobs"][0]["path"], preview["download_path"]);
    let path = std::path::Path::new(completed["jobs"][0]["path"].as_str().unwrap());
    let probe = pipeline::probe(path).await.unwrap();
    assert!(
        probe["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "video")
    );
    assert!(
        probe["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "audio")
    );
    assert!(!completed.to_string().contains("synthetic-title-token"));
    engine
        .dispatch(json!({"action":"parse","url":source_url}))
        .unwrap();
    let state = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    let default_path = state["manual"]["download_path"].clone();
    assert_eq!(default_path, initial);
    engine
        .dispatch(json!({"action":"download","url":state["manual"]["url"]}))
        .unwrap();
    let completed = wait(&engine, |state| {
        state["jobs"].as_array().unwrap().len() == 2 && state["jobs"][1]["state"] == "complete"
    })
    .await;
    assert_eq!(completed["jobs"][1]["path"], default_path);
    assert!(!completed.to_string().contains("synthetic-title-token"));
    let first_path = std::path::PathBuf::from(default_path.as_str().unwrap());
    let first_bytes = std::fs::read(&first_path).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":source_url}))
        .unwrap();
    let state = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    let repeated_path = state["manual"]["download_path"].clone();
    assert!(repeated_path.as_str().unwrap().ends_with(" (1).mp4"));
    engine
        .dispatch(json!({"action":"download","url":state["manual"]["url"]}))
        .unwrap();
    let completed = wait(&engine, |state| {
        state["jobs"].as_array().unwrap().len() == 3 && state["jobs"][2]["state"] == "complete"
    })
    .await;
    assert_eq!(completed["jobs"][2]["path"], repeated_path);
    assert_eq!(
        std::fs::read(first_path).unwrap(),
        first_bytes,
        "A repeated title replaced the first complete file"
    );
    assert!(!completed.to_string().contains("synthetic-title-token"));
    let probe = pipeline::probe(std::path::Path::new(repeated_path.as_str().unwrap()))
        .await
        .unwrap();
    assert!(
        probe["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "video")
    );
    server.abort();
}

#[test]
fn default_names_match_the_original_title_without_source_ids() {
    let folder = tempfile::tempdir().unwrap();
    let media = videodock_media::Media {
        id: "jNQXAC9IVRw".into(),
        title: "Me at the zoo".into(),
        ..Default::default()
    };
    let first = pipeline::preview_output_path(&media, folder.path(), None, "MKV").unwrap();
    assert_eq!(first, folder.path().join("Me at the zoo.mkv"));
    std::fs::write(&first, b"first complete file").unwrap();
    let another = videodock_media::Media {
        id: "different-id".into(),
        ..media
    };
    let second = pipeline::preview_output_path(&another, folder.path(), None, "MKV").unwrap();
    assert_eq!(second, folder.path().join("Me at the zoo (1).mkv"));
    assert_eq!(std::fs::read(first).unwrap(), b"first complete file");
    assert!(!second.exists());
}

#[cfg(unix)]
#[test]
fn public_preview_preserves_dangling_symlink_collision_and_safe_stems() {
    let folder = tempfile::tempdir().unwrap();
    let occupied = folder.path().join("chosen.mp4");
    std::os::unix::fs::symlink("missing", &occupied).unwrap();
    let media = videodock_media::Media {
        id: "id".into(),
        title: "../NUL: clip".into(),
        ..Default::default()
    };
    let path =
        pipeline::preview_output_path(&media, folder.path(), Some("chosen.mp4"), "MP4").unwrap();
    assert_eq!(path, folder.path().join("chosen (1).mp4"));
    assert!(occupied.is_symlink());
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 1);
    let safe = pipeline::preview_output_path(&media, folder.path(), None, "MKV").unwrap();
    assert_eq!(safe.parent(), Some(folder.path()));
    assert_eq!(safe.extension().unwrap(), "mkv");
    assert!(
        !safe
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(['/', '\\', ':'])
    );
    assert!(!safe.exists());
}
