use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};
use tokio_util::sync::CancellationToken;
use videodock_core::{
    pipeline::{self, Options, Output},
    transfer::Limiter,
};
use videodock_media::{Format, Media, Protocol, Selection};

struct Fixture {
    root: tempfile::TempDir,
    base: String,
    server: tokio::task::JoinHandle<()>,
    delayed: Arc<AtomicBool>,
    peak: Arc<AtomicUsize>,
    progress: Arc<Mutex<Vec<pipeline::PipelineProgress>>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut files = BTreeMap::new();
        for (name, frequency, codec, video) in [
            ("muxed.mp4", 440, "aac", true),
            ("original.m4a", 660, "aac", false),
            ("russian.m4a", 880, "aac", false),
            ("russian.webm", 880, "libopus", false),
        ] {
            let mut command = Command::new("ffmpeg");
            command.args(["-nostdin", "-v", "error", "-filter_threads", "1"]);
            if video {
                command.args(["-f", "lavfi", "-i", "color=c=blue:s=160x90:r=10"]);
            }
            command.args([
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={frequency}:sample_rate=48000"),
                "-t",
                "1",
            ]);
            if video {
                command.args(["-c:v", "libx264", "-threads:v", "1", "-pix_fmt", "yuv420p"]);
            }
            command.args([
                "-c:a",
                codec,
                "-metadata:s:a:0",
                if name.starts_with("russian") {
                    "language=rus"
                } else {
                    "language=eng"
                },
            ]);
            let path = root.path().join(name);
            assert!(command.arg(&path).status().await.unwrap().success());
            files.insert(format!("/{name}"), tokio::fs::read(path).await.unwrap());
        }
        let files = Arc::new(files);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let delayed = Arc::new(AtomicBool::new(false));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let server_delayed = delayed.clone();
        let server_peak = peak.clone();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let files = files.clone();
                let delayed = server_delayed.clone();
                let active = active.clone();
                let peak = server_peak.clone();
                tokio::spawn(async move {
                    let mut request = [0; 16384];
                    let count = socket.read(&mut request).await.unwrap();
                    let text = String::from_utf8_lossy(&request[..count]);
                    let path = text.split_whitespace().nth(1).unwrap_or("");
                    let body = files.get(path).unwrap();
                    let delayed = delayed.load(Ordering::Relaxed);
                    if delayed {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(current, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
                    socket.write_all(body).await.unwrap();
                    if delayed {
                        active.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }
        });
        Self {
            root,
            base,
            server,
            delayed,
            peak,
            progress: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn format(
        &self,
        file: &str,
        id: &str,
        track: Option<&str>,
        original: Option<bool>,
        default: Option<bool>,
        video: bool,
    ) -> Format {
        Format {
            id: id.into(),
            url: format!("{}/{file}", self.base),
            protocol: Protocol::Http,
            ext: file.rsplit('.').next().unwrap().into(),
            height: video.then_some(90),
            video_codec: video.then(|| "avc1".into()),
            audio_codec: Some(
                if file.ends_with("webm") {
                    "opus"
                } else {
                    "aac"
                }
                .into(),
            ),
            audio_language: track.map(|track| {
                if track.starts_with("en") {
                    "en-US"
                } else {
                    "ru"
                }
                .into()
            }),
            audio_track_id: track.map(str::to_owned),
            audio_original: original,
            audio_default: default,
            ..Default::default()
        }
    }
    async fn run(
        &self,
        name: &str,
        formats: Vec<Format>,
        selection: Selection,
    ) -> anyhow::Result<Output> {
        pipeline::run(
            Media {
                id: "fixture".into(),
                title: name.into(),
                duration: Some(1.0),
                formats,
                ..Default::default()
            },
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            Options {
                selection,
                output_dir: self.root.path().join(name),
                filename: None,
                embed_subtitles: false,
                tags: false,
                skip_duplicates: false,
            },
            self.root.path().join(format!("staging-{name}")),
            CancellationToken::new(),
            Arc::new(Limiter::new(0)),
            {
                let progress = self.progress.clone();
                Arc::new(move |event| progress.lock().unwrap().push(event))
            },
        )
        .await
    }
}
#[tokio::test]
async fn selected_tracks_overlap_with_a_bounded_number_of_requests() {
    let fixture = Fixture::new().await;
    fixture.delayed.store(true, Ordering::Relaxed);
    let formats = (0..6)
        .map(|index| {
            fixture.format(
                "original.m4a",
                &format!("track-{index}"),
                Some(&format!("en.{index}")),
                Some(index == 0),
                Some(index == 0),
                false,
            )
        })
        .collect();
    let started = std::time::Instant::now();
    let output = fixture
        .run(
            "parallel",
            formats,
            Selection {
                media_type: "audio".into(),
                container: "m4a".into(),
                audio_bitrate: 0,
                audio_tracks: vec!["all".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(audio_streams(&output.info).len(), 6);
    println!(
        "six_tracks_ms={} peak_requests={}",
        started.elapsed().as_millis(),
        fixture.peak.load(Ordering::SeqCst)
    );
    assert_eq!(
        fixture.peak.load(Ordering::SeqCst),
        4,
        "downloads should overlap, with at most four active tracks"
    );
    let progress = fixture.progress.lock().unwrap();
    let downloading = progress
        .iter()
        .filter(|event| event.phase == "downloading")
        .collect::<Vec<_>>();
    assert!(
        downloading
            .windows(2)
            .all(|events| events[0].downloaded <= events[1].downloaded)
    );
    assert_eq!(progress.last().unwrap().percent, 100.0);
    assert_eq!(
        downloading.last().unwrap().downloaded,
        std::fs::metadata(fixture.root.path().join("original.m4a"))
            .unwrap()
            .len()
            * 6
    );
}
fn audio_streams(info: &Value) -> Vec<&Value> {
    info["streams"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stream| stream["codec_type"] == "audio")
        .collect()
}
async fn packet_hashes(path: &Path, index: usize) -> Vec<String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            &format!("a:{index}"),
            "-show_packets",
            "-show_entries",
            "packet=data_hash",
            "-show_data_hash",
            "sha256",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["packets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|packet| packet["data_hash"].as_str().unwrap().into())
        .collect()
}

#[tokio::test]
async fn video_all_preserves_embedded_english_and_separate_russian() {
    let fixture = Fixture::new().await;
    let output = fixture
        .run(
            "video-all",
            vec![
                fixture.format(
                    "muxed.mp4",
                    "video",
                    Some("en.0"),
                    Some(true),
                    Some(true),
                    true,
                ),
                fixture.format(
                    "russian.m4a",
                    "ru",
                    Some("ru.0"),
                    Some(false),
                    Some(false),
                    false,
                ),
            ],
            Selection {
                container: "mkv".into(),
                audio_tracks: vec!["all".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(audio_streams(&output.info).len(), 2);
    assert!(
        output.info["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "video")
    );
}

#[tokio::test]
async fn original_separate_audio_replaces_unknown_muxed_audio_once() {
    let fixture = Fixture::new().await;
    let output = fixture
        .run(
            "separate-original",
            vec![
                fixture.format("muxed.mp4", "video", None, None, None, true),
                fixture.format(
                    "original.m4a",
                    "en",
                    Some("en.0"),
                    Some(true),
                    Some(false),
                    false,
                ),
            ],
            Selection {
                container: "mkv".into(),
                audio_tracks: vec!["original".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(audio_streams(&output.info).len(), 1);
    assert_eq!(
        packet_hashes(&output.path, 0).await,
        packet_hashes(&fixture.root.path().join("original.m4a"), 0).await
    );
}

#[tokio::test]
async fn audio_all_m4a_preserves_two_tracks_and_checks_every_source_codec_for_highest() {
    let fixture = Fixture::new().await;
    for (name, russian, unknown) in [
        ("all-aac", "russian.m4a", false),
        ("mixed-known", "russian.webm", false),
        ("mixed-probed", "russian.webm", true),
    ] {
        let mut second =
            fixture.format(russian, "ru", Some("ru.0"), Some(false), Some(false), false);
        if unknown {
            second.audio_codec = Some("unknown".into());
        }
        let output = fixture
            .run(
                name,
                vec![
                    fixture.format(
                        "original.m4a",
                        "en",
                        Some("en.0"),
                        Some(true),
                        Some(true),
                        false,
                    ),
                    second,
                ],
                Selection {
                    media_type: "audio".into(),
                    container: "m4a".into(),
                    audio_bitrate: 0,
                    audio_tracks: vec!["all".into()],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let streams = audio_streams(&output.info);
        assert_eq!(streams.len(), 2, "{name} lost a selected track");
        assert!(
            streams.iter().all(|stream| stream["codec_name"] == "aac"),
            "{name} copied a codec unsupported by M4A"
        );
        if russian == "russian.m4a" {
            for (index, source) in ["original.m4a", "russian.m4a"].iter().enumerate() {
                assert_eq!(
                    packet_hashes(&output.path, index).await,
                    packet_hashes(&fixture.root.path().join(source), 0).await,
                    "Compatible Highest selection was unnecessarily re-encoded"
                );
            }
        }
    }
}

#[tokio::test]
async fn multiple_audio_tracks_are_explicitly_rejected_for_single_stream_containers() {
    let fixture = Fixture::new().await;
    for container in ["mp3", "wav", "flac"] {
        let output = fixture
            .run(
                container,
                vec![
                    fixture.format(
                        "original.m4a",
                        "en",
                        Some("en.0"),
                        Some(true),
                        Some(true),
                        false,
                    ),
                    fixture.format(
                        "russian.m4a",
                        "ru",
                        Some("ru.0"),
                        Some(false),
                        Some(false),
                        false,
                    ),
                ],
                Selection {
                    media_type: "audio".into(),
                    container: container.into(),
                    audio_tracks: vec!["all".into()],
                    ..Default::default()
                },
            )
            .await;
        assert!(
            output.is_err(),
            "{container} silently dropped all but the first track"
        );
        let directory = fixture.root.path().join(container);
        assert!(
            !directory.exists() || std::fs::read_dir(directory).unwrap().next().is_none(),
            "Rejected output was published"
        );
    }
}

#[tokio::test]
async fn multiple_audio_tracks_are_preserved_in_the_other_multi_stream_containers() {
    let fixture = Fixture::new().await;
    for container in ["mp4", "mkv", "ogg", "webm"] {
        let output = fixture
            .run(
                container,
                vec![
                    fixture.format(
                        "original.m4a",
                        "en",
                        Some("en.0"),
                        Some(true),
                        Some(true),
                        false,
                    ),
                    fixture.format(
                        "russian.webm",
                        "ru",
                        Some("ru.0"),
                        Some(false),
                        Some(false),
                        false,
                    ),
                ],
                Selection {
                    media_type: "audio".into(),
                    container: container.into(),
                    audio_bitrate: 0,
                    audio_tracks: vec!["all".into()],
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|error| panic!("{container}: {error}"));
        let streams = audio_streams(&output.info);
        assert_eq!(streams.len(), 2, "{container} lost a track");
        let expected: &[&str] = match container {
            "mp4" => &["aac", "aac"],
            "mkv" => &["aac", "opus"],
            "ogg" => &["vorbis", "vorbis"],
            "webm" => &["opus", "opus"],
            _ => unreachable!(),
        };
        assert_eq!(
            streams
                .iter()
                .map(|stream| stream["codec_name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[tokio::test]
async fn truncated_media_is_not_published_as_complete() {
    let fixture = Fixture::new().await;
    let result = pipeline::run(
        Media {
            id: "truncated".into(),
            title: "truncated".into(),
            duration: Some(60.0),
            formats: vec![fixture.format("muxed.mp4", "muxed", None, None, None, true)],
            ..Default::default()
        },
        reqwest::Client::builder().no_proxy().build().unwrap(),
        Options {
            selection: Selection::default(),
            output_dir: fixture.root.path().join("output"),
            filename: None,
            embed_subtitles: false,
            tags: false,
            skip_duplicates: false,
        },
        fixture.root.path().join("staging-truncated"),
        CancellationToken::new(),
        Arc::new(Limiter::new(0)),
        Arc::new(|_| {}),
    )
    .await;
    assert!(
        result.is_err(),
        "one second of a sixty-second clip must not be reported complete"
    );
    assert!(!fixture.root.path().join("output/truncated.mp4").exists());
}
