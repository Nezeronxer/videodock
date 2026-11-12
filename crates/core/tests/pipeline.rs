use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};
use tokio_util::sync::CancellationToken;
use videodock_core::{
    pipeline::{self, Options},
    transfer::Limiter,
};
use videodock_media::{Format, Media, Protocol, Selection};

#[tokio::test]
async fn legacy_partial_cannot_be_resumed_as_a_newly_selected_rendition() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("sample.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=160x90:r=10",
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
        .arg(&source)
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let bytes = tokio::fs::read(source).await.unwrap();
    let size = bytes.len() as u64;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/video", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut chunk = [0; 1024];
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0 && request.len() + n <= 8192);
            request.extend_from_slice(&chunk[..n]);
        }
        if String::from_utf8(request)
            .unwrap()
            .to_ascii_lowercase()
            .contains("range:")
        {
            socket
                .write_all(
                    b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        } else {
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    let staging = temp.path().join("staging");
    tokio::fs::create_dir(&staging).await.unwrap();
    tokio::fs::write(
        staging.join("stream-0-fixture.part"),
        b"unverified legacy partial",
    )
    .await
    .unwrap();
    let output = pipeline::run(
        Media {
            id: "same-video".into(),
            url: url.clone(),
            title: "Sample".into(),
            duration: Some(1.0),
            formats: vec![Format {
                id: "fixture".into(),
                url,
                protocol: Protocol::Http,
                ext: "mp4".into(),
                width: Some(160),
                height: Some(90),
                video_codec: Some("avc1".into()),
                audio_codec: Some("mp4a".into()),
                size: Some(size),
                ..Default::default()
            }],
            ..Default::default()
        },
        reqwest::Client::builder().no_proxy().build().unwrap(),
        Options {
            selection: Selection::default(),
            output_dir: temp.path().join("output"),
            filename: None,
            embed_subtitles: false,
            tags: false,
            skip_duplicates: false,
        },
        staging,
        CancellationToken::new(),
        Arc::new(Limiter::new(0)),
        Arc::new(|_| {}),
    )
    .await
    .expect("a new rendition must start with a fresh request");
    let info = pipeline::probe(&output.path).await.unwrap();
    let streams = info["streams"].as_array().unwrap();
    assert!(streams.iter().any(|s| s["codec_type"] == "video"));
    assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
    server.await.unwrap();
}

#[tokio::test]
async fn rust_hls_download_is_muxed_from_local_segments_only() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    tokio::fs::create_dir(&source).await.unwrap();
    let status = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=320x180:rate=15",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440",
            "-t",
            "3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "15",
            "-sc_threshold",
            "0",
            "-c:a",
            "aac",
            "-hls_time",
            "1",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
        ])
        .arg(source.join("stream.m3u8"))
        .status()
        .await
        .unwrap();
    assert!(status.success());
    tokio::fs::write(source.join("master.m3u8"),"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=320x180,CODECS=\"avc1.42E01E,mp4a.40.2\"\nstream.m3u8\n").await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/entry", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let source = source.clone();
            tokio::spawn(async move {
                let mut request = [0u8; 16384];
                let n = socket.read(&mut request).await.unwrap();
                let text = String::from_utf8_lossy(&request[..n]);
                let path = text.split_whitespace().nth(1).unwrap();
                assert!(!path.contains(".."));
                if path == "/entry" {
                    socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: /cdn/master.m3u8\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                    return;
                }
                assert!(path.starts_with("/cdn/"));
                let body = tokio::fs::read(source.join(path.trim_start_matches("/cdn/")))
                    .await
                    .unwrap();
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                socket.write_all(&body).await.unwrap();
            });
        }
    });
    let media = Media {
        id: "test".into(),
        title: "Rust HLS".into(),
        url: url.clone(),
        duration: Some(3.0),
        formats: vec![Format {
            id: "hls".into(),
            url,
            protocol: Protocol::Hls,
            headers: BTreeMap::new(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = pipeline::run(
        media,
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
        Options {
            selection: Selection::default(),
            output_dir: temp.path().join("output"),
            filename: None,
            embed_subtitles: false,
            tags: true,
            skip_duplicates: false,
        },
        temp.path().join("staging"),
        CancellationToken::new(),
        Arc::new(Limiter::new(0)),
        Arc::new(|_| {}),
    )
    .await;
    server.abort();
    let result = result.unwrap();
    assert!(result.path.is_file());
    let info = pipeline::probe(&result.path).await.unwrap();
    assert!(
        info["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["codec_name"] == "h264")
    );
    assert!(
        info["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["codec_name"] == "aac")
    );
    assert!(
        info["format"]["duration"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
            > 2.5
    );
}
