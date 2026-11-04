//! Opt-in complete public downloads. Logs contain timings, sizes and codecs, never signed URLs.
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use videodock_core::{
    pipeline::{self, Options},
    transfer::Limiter,
};
use videodock_media::{Auth, Selection, youtube::YouTube};

async fn complete_sample(id: &str, quality: u32, expected_duration: f64) {
    if let Some(worker) = std::env::var_os("VIDEODOCK_TEST_PROOF_WORKER") {
        videodock_media::youtube::register_proof_worker(worker.into())
            .expect("test proof worker registration");
    }
    let temp = tempfile::tempdir().unwrap();
    let proxy = std::env::var("VIDEODOCK_TEST_PROXY").ok();
    let adapter = YouTube::new(proxy.as_deref()).await.unwrap();
    let mut http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(30));
    if let Some(proxy) = proxy {
        http = http.proxy(reqwest::Proxy::all(proxy).expect("invalid test proxy"));
    }
    let start = Instant::now();
    let media = tokio::time::timeout(
        Duration::from_secs(90),
        adapter.inspect(
            &format!("https://www.youtube.com/watch?v={id}"),
            &Auth::default(),
        ),
    )
    .await
    .expect("metadata deadline")
    .expect("metadata extraction failed");
    let metadata = start.elapsed();
    let selected = videodock_media::choose_formats(
        &media.formats,
        &Selection {
            quality,
            ..Default::default()
        },
    )
    .unwrap();
    let expected_height = selected
        .iter()
        .find(|format| format.has_video())
        .unwrap()
        .height
        .unwrap();
    let expected_duration = if expected_duration > 0.0 {
        expected_duration
    } else {
        media.duration.expect("missing advertised duration")
    };
    let advertised: u64 = selected.iter().filter_map(|format| format.size).sum();
    for format in &selected {
        println!(
            "selected {id} itag={} video={:?} audio={:?} size={:?}",
            format.id, format.video_codec, format.audio_codec, format.size
        );
    }
    assert!(
        advertised <= 500 * 1024 * 1024,
        "sample exceeds 500MiB budget"
    );
    println!(
        "youtube {id} quality={quality} metadata_s={:.3} tracks={} advertised_bytes={advertised}",
        metadata.as_secs_f64(),
        selected.len()
    );
    let run_start = Instant::now();
    let transfer = Arc::new(Mutex::new(None));
    let transfer_record = transfer.clone();
    let output = tokio::time::timeout(
        Duration::from_secs(180),
        pipeline::run(
            media,
            http.build().unwrap(),
            Options {
                selection: Selection {
                    quality,
                    ..Default::default()
                },
                output_dir: std::env::var_os("VIDEODOCK_TEST_OUTPUT_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| temp.path().join("output")),
                filename: Some(format!("{id}-{quality}")),
                embed_subtitles: false,
                tags: false,
                skip_duplicates: false,
            },
            temp.path().join("staging"),
            CancellationToken::new(),
            Arc::new(Limiter::new(0)),
            Arc::new(move |progress| {
                if progress.phase == "processing" {
                    transfer_record
                        .lock()
                        .unwrap()
                        .get_or_insert((run_start.elapsed(), progress.downloaded));
                }
            }),
        ),
    )
    .await
    .expect("download deadline")
    .expect("pipeline failed");
    let elapsed = run_start.elapsed();
    let (download_time, bytes) = transfer
        .lock()
        .unwrap()
        .expect("missing transfer measurement");
    let info = pipeline::probe(&output.path).await.unwrap();
    let streams = info["streams"].as_array().unwrap();
    assert!(streams.iter().any(|s| s["codec_type"] == "video"));
    assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
    let height = streams.iter().find(|s| s["codec_type"] == "video").unwrap()["height"]
        .as_u64()
        .unwrap();
    let duration: f64 = info["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (duration - expected_duration).abs() <= 3.0_f64.max(expected_duration * 0.02),
        "truncated/wrong duration: {duration}"
    );
    assert_eq!(
        height,
        u64::from(expected_height),
        "selected geometry was not delivered"
    );
    let decoded = tokio::time::timeout(
        Duration::from_secs(180),
        tokio::process::Command::new(videodock_core::platform::media_tool("ffmpeg"))
            .args([
                "-hide_banner",
                "-nostdin",
                "-v",
                "error",
                "-xerror",
                "-threads",
                "4",
                "-i",
            ])
            .arg(&output.path)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "0:a:0",
                "-f",
                "null",
                "-",
                "-progress",
                "pipe:1",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("full decode deadline")
    .expect("FFmpeg unavailable");
    assert!(decoded.status.success(), "full decode failed");
    let progress = String::from_utf8(decoded.stdout).unwrap();
    assert!(
        progress.lines().any(|line| line == "progress=end"),
        "decode did not reach EOF"
    );
    let actual_end = progress
        .lines()
        .filter_map(|line| line.strip_prefix("out_time_us=")?.parse::<u64>().ok())
        .next_back()
        .unwrap() as f64
        / 1_000_000.0;
    assert!(
        (actual_end - expected_duration).abs() <= 3.0_f64.max(expected_duration * 0.02),
        "decoded last timestamp is incomplete"
    );
    println!("youtube {id} decoded_end_s={actual_end:.6} full_decode=true");
    println!(
        "youtube {id} quality={quality} transfer_s={:.3} mux_probe_publish_s={:.3} downloaded_bytes={bytes} output_bytes={} bytes_per_s={:.0} duration_s={duration:.3} height={height}",
        download_time.as_secs_f64(),
        (elapsed - download_time).as_secs_f64(),
        output.size,
        bytes as f64 / download_time.as_secs_f64()
    );
}

#[tokio::test]
#[ignore = "complete public YouTube download with FFmpeg/ffprobe, up to 180 seconds"]
async fn zoo_complete_240p() {
    complete_sample("jNQXAC9IVRw", 240, 19.0).await;
}

#[tokio::test]
#[ignore = "complete public YouTube download with FFmpeg/ffprobe, up to 180 seconds"]
async fn big_buck_bunny_complete_720p() {
    complete_sample("YE7VzlLtp-4", 720, 596.0).await;
}

#[tokio::test]
#[ignore = "complete public YouTube download with FFmpeg/ffprobe, up to 180 seconds"]
async fn sintel_trailer_complete_hd() {
    complete_sample("HOfdboHvshg", 1080, 53.0).await;
}

#[tokio::test]
#[ignore = "isolates public YouTube manifest preparation from chosen stream transfers"]
async fn zoo_prepare_and_http_transfer_timings() {
    let temp = tempfile::tempdir().unwrap();
    let adapter = YouTube::new(None).await.unwrap();
    let mut media = adapter
        .inspect(
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            &Auth::default(),
        )
        .await
        .unwrap();
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let selected = videodock_media::choose_formats(
        &media.formats,
        &Selection {
            quality: 240,
            ..Default::default()
        },
    )
    .unwrap();
    let cancel = CancellationToken::new();
    let start = Instant::now();
    pipeline::prepare(&http, &mut media, &cancel).await.unwrap();
    println!(
        "zoo prepare_s={:.3} prepared_formats={}",
        start.elapsed().as_secs_f64(),
        media.formats.len()
    );
    for (index, format) in selected.into_iter().enumerate() {
        for ranged in [false, true] {
            let mut headers = format.headers.clone();
            headers.remove("Range");
            if ranged {
                headers.insert(
                    "Range".into(),
                    format!("bytes=0-{}", format.size.unwrap() - 1),
                );
            }
            let start = Instant::now();
            let output = temp.path().join(format!("{index}-{ranged}.part"));
            videodock_core::transfer::download(
                &http,
                &format.url,
                &headers,
                &output,
                &cancel,
                &Limiter::new(0),
                Arc::new(|_| {}),
            )
            .await
            .expect("transfer failed");
            println!(
                "zoo stream={index} bounded_range={ranged} bytes={} transfer_s={:.3}",
                tokio::fs::metadata(output).await.unwrap().len(),
                start.elapsed().as_secs_f64()
            );
        }
    }
}

#[tokio::test]
#[ignore = "complete public HD YouTube download with FFmpeg/ffprobe, up to 180 seconds"]
async fn big_buck_bunny_modern_complete_720p() {
    complete_sample("aqz-KE-bpKQ", 720, 0.0).await;
}

#[tokio::test]
#[ignore = "complete public Full HD YouTube download with FFmpeg/ffprobe, up to 180 seconds"]
async fn big_buck_bunny_modern_complete_1080p() {
    complete_sample("aqz-KE-bpKQ", 1080, 0.0).await;
}
