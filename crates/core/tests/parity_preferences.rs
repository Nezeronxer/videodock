use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use videodock_core::Engine;

#[test]
fn whole_language_catalog_is_accepted_and_oversized_update_is_atomic() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().join("data")).unwrap();
    let languages = (0..183).map(|index| format!("lang-{index}")).collect::<Vec<_>>();
    engine.dispatch(json!({"action":"settings","values":{"subtitles":languages}})).unwrap();
    let before = engine.snapshot()["settings"].clone();
    let oversized = (0..257).map(|index| format!("lang-{index}")).collect::<Vec<_>>();
    assert!(engine.dispatch(json!({"action":"settings","values":{"subtitles":oversized,"quality":"4K"}})).is_err());
    assert_eq!(engine.snapshot()["settings"], before);
}

fn expire_subscription(directory: &std::path::Path) {
    let connection = rusqlite::Connection::open(directory.join("state.sqlite")).unwrap();
    let serialized: String = connection
        .query_row("SELECT value FROM state WHERE id=1", [], |row| row.get(0))
        .unwrap();
    let mut state: Value = serde_json::from_str(&serialized).unwrap();
    state["subscriptions"][0]["last_checked"] = json!(0);
    connection
        .execute("UPDATE state SET value=?1 WHERE id=1", [state.to_string()])
        .unwrap();
}

#[test]
fn never_interval_and_path_history_survive_restart_and_validate_atomically() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().join("data")).unwrap();
    let first = data.path().join("first");
    let second = data.path().join("second");
    engine
        .dispatch(
            json!({"action":"settings","values":{"subscriptionUpdatePeriod":"Never","path":first}}),
        )
        .unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"path":second}}))
        .unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"path":first}}))
        .unwrap();
    let settings = engine.snapshot()["settings"].clone();
    assert_eq!(settings["subscriptionUpdatePeriod"], 0);
    let paths = settings["paths"].as_array().unwrap();
    assert_eq!(paths.first(), Some(&json!(first)));
    assert_eq!(
        paths.iter().filter(|path| **path == json!(first)).count(),
        1
    );
    assert!(paths.contains(&json!(second)));
    assert!(engine.dispatch(json!({"action":"settings","values":{"subscriptionUpdatePeriod":-1,"path":data.path().join("rejected")}})).is_err());
    assert_eq!(engine.snapshot()["settings"], settings);
    drop(engine);
    let engine = Engine::new(data.path().join("data")).unwrap();
    assert_eq!(engine.snapshot()["settings"]["paths"], settings["paths"]);
    assert_eq!(engine.snapshot()["settings"]["subscriptionUpdatePeriod"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn never_disables_scheduler_but_manual_refresh_still_checks_the_source() {
    let data = TempDir::new().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let source = format!("http://{}/playlist", listener.local_addr().unwrap());
    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let reads = observed.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 4096];
                let _ = socket.read(&mut buffer).await;
                reads.fetch_add(1, Ordering::SeqCst);
                let body = b"Temporary fixture failure";
                let _ = socket.write_all(format!("HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await;
                let _ = socket.write_all(body).await;
            });
        }
    });
    let directory = data.path().join("data");
    let engine = Engine::new(directory.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"subscriptionUpdatePeriod":0}}))
        .unwrap();
    let import = data.path().join("subscriptions.json");
    std::fs::write(&import, serde_json::to_vec(&json!({"json_schema_version":"1.0.0","items":[{"urlDesc":{"serviceName":"html","type":"playlist","url":source},"isSubscription":true}]})).unwrap()).unwrap();
    engine
        .dispatch(json!({"action":"import","path":import,"output_dir":data.path()}))
        .unwrap();
    let id = engine.snapshot()["subscriptions"][0]["id"].clone();
    drop(engine);
    expire_subscription(&directory);
    let engine = Engine::new(directory.clone()).unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(
        reads.load(Ordering::SeqCst),
        0,
        "Never caused an automatic source request even though its interval was disabled"
    );
    engine
        .dispatch(json!({"action":"subscriptions","command":"refresh","ids":[id.clone()]}))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while reads.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Manual refresh did not reach the local source");
    tokio::time::timeout(Duration::from_secs(5), async {
        while engine.snapshot()["subscriptions"][0]["error"]
            .as_str()
            .unwrap_or("")
            .is_empty()
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Manual refresh result was not recorded");
    drop(engine);
    expire_subscription(&directory);
    let engine = Engine::new(directory).unwrap();
    let previous_reads = reads.load(Ordering::SeqCst);
    engine
        .dispatch(json!({"action":"settings","values":{"subscriptionUpdatePeriod":60}}))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while reads.load(Ordering::SeqCst) == previous_reads {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("A positive interval no longer performs automatic checks");
    drop(engine);
    server.abort();
}

#[test]
fn os_aliases_reset_codec_and_separate_media_containers_survive_restart() {
    let data = TempDir::new().unwrap();
    let engine = Engine::new(data.path().join("data")).unwrap();
    for (mode, mappings) in [
        (
            "video",
            [
                ("Windows", "mp4"),
                ("Mac OS", "mp4"),
                ("Linux", "mkv"),
                ("iOS", "mp4"),
                ("Android", "mp4"),
            ],
        ),
        (
            "audio",
            [
                ("Windows", "mp3"),
                ("Mac OS", "m4a"),
                ("Linux", "ogg"),
                ("iOS", "m4a"),
                ("Android", "mp3"),
            ],
        ),
    ] {
        engine
            .dispatch(json!({"action":"settings","values":{"mediaType":mode}}))
            .unwrap();
        for (alias, container) in mappings {
            let codec = if mode == "video" {
                "videoCodec"
            } else {
                "audioCodec"
            };
            engine.dispatch(json!({"action":"settings","values":{codec:if mode == "video" { "h264" } else { "aac" }}})).unwrap();
            engine
                .dispatch(json!({"action":"settings","values":{"containerOSAlias":alias}}))
                .unwrap();
            let settings = engine.snapshot()["settings"].clone();
            assert_eq!(settings["containerOSAlias"], alias);
            assert_eq!(settings["container"], container);
            assert_eq!(settings[codec], "any");
        }
    }
    engine
        .dispatch(json!({"action":"settings","values":{"mediaType":"video","container":"MKV"}}))
        .unwrap();
    assert_eq!(engine.snapshot()["settings"]["containerOSAlias"], "");
    engine
        .dispatch(json!({"action":"settings","values":{"mediaType":"audio","container":"M4A"}}))
        .unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"mediaType":"video"}}))
        .unwrap();
    assert_eq!(engine.snapshot()["settings"]["container"], "mkv");
    engine
        .dispatch(json!({"action":"settings","values":{"mediaType":"audio"}}))
        .unwrap();
    assert_eq!(engine.snapshot()["settings"]["container"], "m4a");
    let before = engine.snapshot()["settings"].clone();
    assert!(
        engine
            .dispatch(
                json!({"action":"settings","values":{"containerOSAlias":"UnknownOS","quality":720}})
            )
            .is_err()
    );
    assert_eq!(engine.snapshot()["settings"], before);
    assert!(engine.dispatch(json!({"action":"settings","values":{"container":"MP4","path":data.path().join("rejected")}})).is_err());
    assert_eq!(engine.snapshot()["settings"], before);
    drop(engine);
    let engine = Engine::new(data.path().join("data")).unwrap();
    assert_eq!(engine.snapshot()["settings"]["container"], "m4a");
    engine
        .dispatch(json!({"action":"settings","values":{"mediaType":"video"}}))
        .unwrap();
    assert_eq!(engine.snapshot()["settings"]["container"], "mkv");
}
