//! Real local fMP4 fixture: FFmpeg generates media, Rust alone downloads it.
use std::{collections::BTreeMap, process::Command, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use videodock_core::{dash, transfer::Limiter};
use videodock_media::{Format, Media, Protocol};

#[tokio::test]
#[ignore = "requires the app's FFmpeg and ffprobe runtime"]
async fn real_fmp4_video_audio_segments_are_downloaded_by_rust_and_probe_correctly() {
    run_fixture(false).await;
}

#[tokio::test]
#[ignore = "requires the app's FFmpeg and ffprobe runtime"]
async fn compatible_two_period_vod_finishes_the_actual_mux_pipeline() {
    run_fixture(true).await;
}

async fn run_fixture(two_periods: bool) {
    let generated = tempfile::tempdir().unwrap();
    let mpd = generated.path().join("manifest.mpd");
    let result = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x96:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
            "-t",
            "2",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-g",
            "10",
            "-c:a",
            "aac",
            "-b:a",
            "64k",
            "-f",
            "dash",
            "-seg_duration",
            "1",
            "-use_template",
            "1",
            "-use_timeline",
            "1",
            "-adaptation_sets",
            "id=0,streams=v id=1,streams=a",
        ])
        .arg(&mpd)
        .output()
        .expect("ffmpeg required");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    if two_periods {
        let original = std::fs::read_to_string(&mpd).unwrap();
        let doc = roxmltree::Document::parse(&original).unwrap();
        let root = doc.root_element();
        let period = root.children().find(|n| n.has_tag_name("Period")).unwrap();
        let range = period.range();
        let first = original[range.clone()].replacen("<Period ", "<Period duration=\"PT2S\" ", 1);
        let start = period.attribute("start").unwrap();
        let second = first.replacen("id=\"0\"", "id=\"1\"", 1).replacen(
            &format!("start=\"{start}\""),
            "start=\"PT2S\"",
            1,
        );
        let xml = format!(
            "{}{}{}{}",
            &original[..range.start],
            first,
            second,
            &original[range.end..]
        );
        let xml = xml.replacen(
            &format!(
                "mediaPresentationDuration=\"{}\"",
                root.attribute("mediaPresentationDuration").unwrap()
            ),
            "mediaPresentationDuration=\"PT4S\"",
            1,
        );
        std::fs::write(&mpd, xml).unwrap();
    }
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(generated.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            files.insert(
                format!("/{}", entry.file_name().to_string_lossy()),
                std::fs::read(entry.path()).unwrap(),
            );
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut request = [0; 8192];
            let Ok(n) = stream.read(&mut request).await else {
                continue;
            };
            let request = String::from_utf8_lossy(&request[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let data = files
                .get(path)
                .unwrap_or_else(|| panic!("unexpected fixture request: {path}"));
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                data.len()
            );
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(data).await.unwrap();
        }
    });
    let client = reqwest::Client::new();
    let cancel = CancellationToken::new();
    let mut media = Media {
        id: "fixture".into(),
        title: "DASH fixture".into(),
        duration: Some(if two_periods { 4.0 } else { 2.0 }),
        formats: vec![Format {
            id: "local".into(),
            url: format!("{base}/manifest.mpd"),
            protocol: Protocol::Dash,
            ..Default::default()
        }],
        ..Default::default()
    };
    dash::expand(&client, &mut media, &cancel).await.unwrap();
    assert_eq!(media.formats.len(), 2);
    let staging = tempfile::tempdir().unwrap();
    for format in &media.formats {
        let output = dash::download_dash(
            &client,
            format,
            staging.path(),
            &cancel,
            &Limiter::new(0),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_packets",
                "-show_entries",
                "stream=codec_name,width,height,nb_read_packets",
                "-of",
                "json",
            ])
            .arg(output)
            .output()
            .expect("ffprobe required");
        assert!(
            probe.status.success(),
            "{}",
            String::from_utf8_lossy(&probe.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        let stream = &result["streams"][0];
        if format.has_video() {
            assert_eq!(stream["codec_name"], "h264");
            assert_eq!(stream["width"], 160);
            assert_eq!(stream["height"], 96);
        } else {
            assert_eq!(stream["codec_name"], "aac");
        }
        assert!(
            stream["nb_read_packets"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                > 0
        );
    }
    if two_periods {
        let output = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let result = videodock_core::pipeline::run(
            media,
            client,
            videodock_core::pipeline::Options {
                selection: Default::default(),
                output_dir: output.path().to_owned(),
                filename: None,
                embed_subtitles: false,
                tags: false,
                skip_duplicates: false,
            },
            staging.path().to_owned(),
            CancellationToken::new(),
            Arc::new(Limiter::new(0)),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
        let probe = videodock_core::pipeline::probe(&result.path).await.unwrap();
        let streams = probe["streams"].as_array().unwrap();
        assert_eq!(streams.len(), 2);
        assert!(streams.iter().any(|s| s["codec_name"] == "h264"));
        assert!(streams.iter().any(|s| s["codec_name"] == "aac"));
        let duration = probe["format"]["duration"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert!(
            (3.8..4.3).contains(&duration),
            "unexpected muxed duration: {duration}"
        );
    }
    server.abort();
}
