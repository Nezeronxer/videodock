//! Opt-in end-to-end checks against public service samples, without user credentials.
use serde_json::json;
use std::time::{Duration, Instant};
use videodock_core::{Engine, pipeline};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "downloads a public YouTube sample using Rust and requires FFmpeg/ffprobe"]
async fn youtube_queue_downloads_video_and_audio_and_retains_history() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let engine = Engine::new(data.clone()).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{
            "path":temp.path().join("output"),"smartMode":true,
            "quality":480,"container":"mp4","subtitles":[],"tagsEnabled":false
        }}))
        .unwrap();
    engine
        .dispatch(json!({"action":"parse","url":"https://www.youtube.com/watch?v=jNQXAC9IVRw"}))
        .unwrap();
    let start = Instant::now();
    let completed = loop {
        let state = engine.snapshot();
        assert!(
            state["notice"].as_str().unwrap_or("").is_empty(),
            "metadata extraction failed: {}",
            state["notice"]
        );
        if let Some(job) = state["jobs"].as_array().and_then(|jobs| jobs.first()) {
            assert_ne!(
                job["state"], "error",
                "queue download failed (details retained locally)"
            );
            if job["state"] == "complete" {
                break job.clone();
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(180),
            "YouTube queue deadline exceeded"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(completed["title"], "Me at the zoo");
    assert_eq!(completed["progress"], 100.0);
    let output = std::path::Path::new(completed["path"].as_str().unwrap());
    let media = pipeline::probe(output).await.unwrap();
    let streams = media["streams"].as_array().unwrap();
    assert!(streams.iter().any(|stream| stream["codec_type"] == "video"));
    assert!(streams.iter().any(|stream| stream["codec_type"] == "audio"));
    let duration: f64 = media["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((18.0..21.0).contains(&duration));
    drop(engine);
    let engine = Engine::new(data).unwrap();
    let state = engine.snapshot();
    assert_eq!(state["jobs"][0]["id"], completed["id"]);
    assert_eq!(state["jobs"][0]["state"], "complete");
    assert!(output.is_file());
}
