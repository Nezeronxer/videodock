use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};
use videodock_core::{Engine, pipeline};

struct Source {
    base: String,
    page_reads: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<String>>>,
    cross_host: Arc<AtomicBool>,
    playlist_size: Arc<AtomicUsize>,
    page_delay: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Source {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn source(directory: &std::path::Path) -> Source {
    let file = directory.join("source.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x90:rate=15",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ])
        .arg(&file)
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let video = Arc::new(tokio::fs::read(file).await.unwrap());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let page_reads = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let cross_host = Arc::new(AtomicBool::new(false));
    let playlist_size = Arc::new(AtomicUsize::new(1));
    let page_delay = Arc::new(AtomicUsize::new(0));
    let delay = page_delay.clone();
    let cross = cross_host.clone();
    let playlist = playlist_size.clone();
    let other_host = format!("http://localhost:{}", listener.local_addr().unwrap().port());
    let reads = page_reads.clone();
    let request_log = requests.clone();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let video = video.clone();
            let reads = reads.clone();
            let requests = request_log.clone();
            let cross = cross.clone();
            let playlist = playlist.clone();
            let other_host = other_host.clone();
            let delay = delay.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0; 16384];
                let Ok(count) = socket.read(&mut buffer).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buffer[..count]).into_owned();
                requests.lock().unwrap().push(request.clone());
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                if path.starts_with("/private/watch") {
                    let number = reads.fetch_add(1, Ordering::SeqCst) + 1;
                    tokio::time::sleep(Duration::from_millis(delay.load(Ordering::SeqCst) as u64))
                        .await;
                    let host = if cross.load(Ordering::SeqCst) {
                        other_host.as_str()
                    } else {
                        ""
                    };
                    let body = format!(
                        "<html><head><title>Local signed stream</title></head><body><video src=\"{host}/media.mp4?signature=synthetic-signed-{number}\"></video></body></html>"
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    socket.write_all(response.as_bytes()).await.ok();
                } else if path.starts_with("/playlist") {
                    let items=(1..=playlist.load(Ordering::SeqCst)).map(|number|json!({"item":{"url":format!("/private/watch/clip-{number}"),"name":format!("Clip {number}")}})).collect::<Vec<_>>();
                    let body = format!(
                        "<html><title>Local playlist</title><script type=\"application/ld+json\">{}</script></html>",
                        json!({"@type":"ItemList","itemListElement":items})
                    );
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.ok();
                } else if path.starts_with("/media.mp4") {
                    let offset = request
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("range: bytes=")
                                .and_then(|range| range.split('-').next())
                                .and_then(|number| number.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if offset >= video.len() {
                        socket.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",video.len()).as_bytes()).await.ok();
                        return;
                    }
                    let response = if offset > 0 {
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            offset,
                            video.len() - 1,
                            video.len(),
                            video.len() - offset
                        )
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            video.len()
                        )
                    };
                    if socket.write_all(response.as_bytes()).await.is_err() {
                        return;
                    }
                    for chunk in video[offset..].chunks(1024) {
                        if socket.write_all(chunk).await.is_err() {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(4)).await;
                    }
                } else {
                    socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.ok();
                }
            });
        }
    });
    Source {
        base,
        page_reads,
        requests,
        cross_host,
        playlist_size,
        page_delay,
        server,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_batch_pause_stops_new_inspections_and_resume_keeps_progress() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    source.playlist_size.store(3, Ordering::SeqCst);
    source.page_delay.store(200, Ordering::SeqCst);
    let engine = Engine::new(temp.path().join("data")).unwrap();
    assert!(engine.dispatch(json!({"action":"manual_pause"})).is_err());
    engine
        .dispatch(json!({"action":"settings","values":{"smartMode":false}}))
        .unwrap();
    engine
        .dispatch(json!({"action":"parse","url":format!("{}/playlist",source.base)}))
        .unwrap();
    let start = Instant::now();
    while source.page_reads.load(Ordering::SeqCst) == 0 {
        assert!(start.elapsed() < Duration::from_secs(10));
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    engine.dispatch(json!({"action":"manual_pause"})).unwrap();
    let paused = wait(&engine, |state| {
        state["manual"]["state"] == "parsing_paused" && state["manual"]["parsed_count"] == 1
    })
    .await;
    assert_eq!(paused["manual"]["total_count"], 3);
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(
        source.page_reads.load(Ordering::SeqCst),
        1,
        "pause must prevent starting the next item"
    );
    engine.dispatch(json!({"action":"manual_resume"})).unwrap();
    let done = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    assert_eq!(done["manual"]["items"].as_array().unwrap().len(), 3);
    assert_eq!(
        source.page_reads.load(Ordering::SeqCst),
        3,
        "resume must not restart already inspected items"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn numbered_clipboard_filenames_never_persist_private_url_queries() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let engine = Engine::new(data.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{
            "smartMode":true,"numerationEnabled":true,"path":temp.path().join("output")
        }}))
        .unwrap();
    engine.dispatch(json!({"action":"parse","url":
        "http://127.0.0.1:9/one?signature=synthetic-private-filename-marker\nhttp://127.0.0.1:9/two?signature=synthetic-private-filename-marker"
    })).unwrap();
    wait(&engine, |state| {
        state["jobs"]
            .as_array()
            .is_some_and(|jobs| !jobs.is_empty())
    })
    .await;
    let sqlite = rusqlite::Connection::open(data.join("state.sqlite")).unwrap();
    let saved: String = sqlite
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert!(
        !saved.contains("synthetic-private-filename-marker"),
        "private query leaked into saved generated filename"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn playlist_title_cannot_leak_a_private_query_into_the_output_folder() {
    let temp = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let page = format!("http://{}/playlist", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 8192];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        let body = "<html><title>http://127.0.0.1:9/source?signature=synthetic-private-folder-marker</title><script type=\"application/ld+json\">{\"@type\":\"ItemList\",\"itemListElement\":[{\"item\":{\"url\":\"http://127.0.0.1:9/clip\",\"name\":\"Synthetic clip\"}}]}</script></html>";
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    });
    let data = temp.path().join("data");
    let engine = Engine::new(data.clone()).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":true,"separateFolderCreationEnabled":true,"path":temp.path().join("output")}})).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":page}))
        .unwrap();
    let state = wait(&engine, |state| {
        state["jobs"]
            .as_array()
            .is_some_and(|jobs| !jobs.is_empty())
    })
    .await;
    let sqlite = rusqlite::Connection::open(data.join("state.sqlite")).unwrap();
    let saved: String = sqlite
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert!(!saved.contains("synthetic-private-folder-marker"));
    assert!(
        !state
            .to_string()
            .contains("synthetic-private-folder-marker")
    );
    server.await.unwrap();
}
async fn wait(engine: &Engine, predicate: impl Fn(&Value) -> bool) -> Value {
    let start = Instant::now();
    loop {
        let state = engine.snapshot();
        if predicate(&state) {
            return state;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "Timed out; states: {:?}; notice: {:?}",
            state["jobs"].as_array().map(|jobs| jobs
                .iter()
                .map(|job| job["state"].clone())
                .collect::<Vec<_>>()),
            state["notice"]
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_download_uses_rust_http_muxes_local_stream_and_never_persists_signed_urls() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    let engine = Engine::new(temp.path().join("data")).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":false,"path":temp.path().join("output")}})).unwrap();
    engine.dispatch(json!({"action":"account","command":"cookie","name":"fixture","value":"synthetic-cookie-only-for-local-test","domain":"127.0.0.1","path":"/private","secure":false})).unwrap();
    engine.dispatch(json!({"action":"account","command":"cookie","name":"stream","value":"media-boundary-fixture","domain":"127.0.0.1","path":"/media.mp4","secure":false})).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":format!("{}/private/watch",source.base)}))
        .unwrap();
    let state = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    assert!(state["manual"]["formats"][0].get("url").is_none());
    assert!(state["manual"]["formats"][0].get("headers").is_none());
    let format = state["manual"]["formats"][0]["id"].clone();
    let url = state["manual"]["url"].clone();
    engine.dispatch(json!({"action":"download","url":url,"format_id":format,"output_dir":temp.path().join("output"),"filename":"manual clip","format":{"media_type":"video","container":"MP4","quality":"Any","video_codec":"unknown","audio_codec":"unknown","fps":"Any","vr":"2D"}})).unwrap();
    let state = wait(&engine, |state| state["jobs"][0]["state"] == "complete").await;
    assert_eq!(
        state["jobs"][0]["quality"], "90p",
        "completed quality must come from final ffprobe, never the requested maximum"
    );
    let path = std::path::PathBuf::from(state["jobs"][0]["path"].as_str().unwrap());
    assert!(path.is_file());
    assert!(
        pipeline::probe(&path).await.unwrap()["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "video")
    );
    assert!(!state.to_string().contains("synthetic-signed"));
    assert!(!state.to_string().contains("synthetic-cookie"));
    let sqlite = rusqlite::Connection::open(temp.path().join("data/state.sqlite")).unwrap();
    let saved: String = sqlite
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert!(!saved.contains("synthetic-signed"));
    assert!(!saved.contains("synthetic-cookie"));
    let requests = source.requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .filter(|request| request.contains("/private/watch"))
            .all(|request| request.contains("fixture=synthetic-cookie-only-for-local-test"))
    );
    assert!(
        requests
            .iter()
            .filter(|request| request.contains("/media.mp4"))
            .all(|request| !request.contains("synthetic-cookie"))
    );
    assert!(
        requests
            .iter()
            .filter(|request| request.contains("/media.mp4"))
            .all(|request| request.contains("stream=media-boundary-fixture")),
        "domain-scoped media cookie must retain its own Path instead of being discarded at the page Path"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_restart_resume_reextracts_fresh_source_and_reuses_partial_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    let directory = temp.path().join("data");
    let engine = Engine::new(directory.clone()).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":true,"speedLimit":1000,"path":temp.path().join("output")}})).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":format!("{}/private/watch",source.base)}))
        .unwrap();
    let state = wait(&engine, |state| {
        state["jobs"][0]["state"] == "downloading"
            && state["jobs"][0]["downloaded"].as_u64().unwrap_or(0) > 0
    })
    .await;
    let id = state["jobs"][0]["id"].clone();
    engine
        .dispatch(json!({"action":"jobs","command":"pause","ids":[id.clone()]}))
        .unwrap();
    assert_eq!(engine.snapshot()["jobs"][0]["state"], "paused");
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(engine);
    let engine = Engine::new(directory).unwrap();
    assert_eq!(engine.snapshot()["jobs"][0]["state"], "paused");
    assert_eq!(engine.snapshot()["settings"]["speedLimit"], 0);
    engine
        .dispatch(json!({"action":"jobs","command":"resume","ids":[id]}))
        .unwrap();
    let state = wait(&engine, |state| state["jobs"][0]["state"] == "complete").await;
    assert!(std::path::Path::new(state["jobs"][0]["path"].as_str().unwrap()).is_file());
    assert!(
        source.page_reads.load(Ordering::SeqCst) >= 3,
        "source must be re-extracted before run and after resume"
    );
    assert!(
        source
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|request| request.to_ascii_lowercase().contains("range: bytes=")),
        "partial transfer was not resumed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_signed_source_lives_only_in_vault_and_is_skipped_by_export() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    let engine = Engine::new(temp.path().join("data")).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":true,"path":temp.path().join("output")}})).unwrap();
    let original = format!(
        "{}/media.mp4?signature=synthetic-direct-source-token",
        source.base
    );
    engine
        .dispatch(json!({"action":"parse","url":original}))
        .unwrap();
    let state = wait(&engine, |state| state["jobs"][0]["state"] == "complete").await;
    assert!(!state.to_string().contains("synthetic-direct-source-token"));
    let sqlite = rusqlite::Connection::open(temp.path().join("data/state.sqlite")).unwrap();
    let saved: String = sqlite
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert!(!saved.contains("synthetic-direct-source-token"));
    let copied = engine
        .dispatch(json!({"action":"jobs","command":"copy_url","ids":[state["jobs"][0]["id"]]}))
        .unwrap();
    assert_eq!(copied["clipboard"], original);
    let export = temp.path().join("public.json");
    let result = engine
        .dispatch(json!({"action":"export","path":export}))
        .unwrap();
    assert_eq!(result["skipped_private"], 1);
    let document: Value = serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
    assert_eq!(document["items"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn foreign_stream_cannot_select_another_origins_browser_cookie() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    source.cross_host.store(true, Ordering::SeqCst);
    let engine = Engine::new(temp.path().join("data")).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":true,"path":temp.path().join("output")}})).unwrap();
    engine.dispatch(json!({"action":"account","command":"cookie","name":"bank","value":"synthetic-unrelated-origin-cookie","domain":"localhost","path":"/","secure":false})).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":format!("{}/private/watch",source.base)}))
        .unwrap();
    wait(&engine, |state| state["jobs"][0]["state"] == "complete").await;
    let requests = source.requests.lock().unwrap();
    let media = requests
        .iter()
        .filter(|request| request.contains("/media.mp4"))
        .collect::<Vec<_>>();
    assert!(!media.is_empty());
    assert!(
        media
            .iter()
            .any(|request| request.to_ascii_lowercase().contains("host: localhost:"))
    );
    assert!(
        media
            .iter()
            .all(|request| !request.contains("synthetic-unrelated-origin-cookie"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscription_skips_baseline_downloads_only_new_ids_and_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let source = source(temp.path()).await;
    let directory = temp.path().join("data");
    let engine = Engine::new(directory.clone()).unwrap();
    engine.dispatch(json!({"action":"settings","values":{"smartMode":false,"path":temp.path().join("output")}})).unwrap();
    engine
        .dispatch(json!({"action":"parse","url":format!("{}/playlist",source.base)}))
        .unwrap();
    let state = wait(&engine, |state| state["manual"]["state"] == "ready").await;
    engine.dispatch(json!({"action":"download","url":state["manual"]["url"],"mode":"subscribe","output_dir":temp.path().join("output"),"download_existing":false})).unwrap();
    assert_eq!(engine.snapshot()["jobs"], json!([]));
    let id = engine.snapshot()["subscriptions"][0]["id"].clone();
    source.playlist_size.store(2, Ordering::SeqCst);
    engine
        .dispatch(json!({"action":"subscriptions","command":"refresh","ids":[id.clone()]}))
        .unwrap();
    let state = wait(&engine, |state| {
        state["jobs"].as_array().is_some_and(|jobs| {
            !jobs.is_empty() && jobs.iter().all(|job| job["state"] == "complete")
        })
    })
    .await;
    let urls = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["items"].as_array().unwrap())
        .filter_map(|item| item["url"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(urls.len(), 1);
    assert!(urls[0].ends_with("/clip-2"));
    engine
        .dispatch(json!({"action":"subscriptions","command":"refresh","ids":[id.clone()]}))
        .unwrap();
    wait(&engine, |state| state["subscriptions"][0]["new_count"] == 0).await;
    assert_eq!(engine.snapshot()["jobs"].as_array().unwrap().len(), 1);
    drop(engine);
    let engine = Engine::new(directory).unwrap();
    source.playlist_size.store(3, Ordering::SeqCst);
    engine
        .dispatch(json!({"action":"subscriptions","command":"refresh","ids":[id]}))
        .unwrap();
    let state = wait(&engine, |state| {
        state["jobs"].as_array().is_some_and(|jobs| {
            jobs.len() == 2 && jobs.iter().all(|job| job["state"] == "complete")
        })
    })
    .await;
    let urls = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["items"].as_array().unwrap())
        .filter_map(|item| item["url"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(urls.len(), 2);
    assert!(urls.iter().any(|url| url.ends_with("/clip-3")));
    assert!(!urls.iter().any(|url| url.ends_with("/clip-1")));
}
