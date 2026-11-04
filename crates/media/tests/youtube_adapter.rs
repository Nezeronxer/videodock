use videodock_media::{Auth, Format, MediaKind, Protocol, youtube::YouTube};

#[tokio::test]
#[ignore = "checks TLS compatibility with a public service endpoint, without credentials"]
async fn public_youtube_http_transport() {
    let http = wreq::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap();
    let response = http.get("https://www.youtube.com/").send().await.unwrap();
    assert!(response.status().is_success());
    let pipe = rustypipe::client::RustyPipe::builder()
        .strict()
        .no_storage()
        .no_reporter()
        .no_botguard()
        .n_http_retries(1)
        .build_with_client(wreq::Client::builder().no_proxy().http1_only())
        .unwrap();
    let result = pipe.query().unauthenticated().player("jNQXAC9IVRw").await;
    assert!(result.is_ok(), "public player request failed");
}

#[tokio::test]
#[ignore = "public YouTube network and media requests"]
async fn live_youtube_video_media_captions_channel_and_search() {
    let adapter = YouTube::new(None).await.unwrap();
    let media = adapter
        .inspect(
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            &Auth::default(),
        )
        .await
        .unwrap();
    assert_eq!(media.id, "jNQXAC9IVRw");
    assert_eq!(media.title, "Me at the zoo");
    assert!(!media.channel_id.is_empty() && !media.thumbnail.is_empty());
    assert!(media.formats.iter().any(Format::has_audio));
    let stream = media
        .formats
        .iter()
        .find(|format| format.has_video() && format.protocol == Protocol::Http)
        .unwrap();
    assert!(
        !stream
            .headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("cookie"))
    );
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    let mut request = http.get(&stream.url).header("Range", "bytes=0-65535");
    for (name, value) in &stream.headers {
        if !name.eq_ignore_ascii_case("range") {
            request = request.header(name, value);
        }
    }
    let mut response = request
        .send()
        .await
        .unwrap_or_else(|_| panic!("media HTTP transport failed"))
        .error_for_status()
        .unwrap_or_else(|_| panic!("media HTTP status failed"));
    let mut sample = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .unwrap_or_else(|_| panic!("media body failed"))
    {
        let remaining = 65536usize.saturating_sub(sample.len());
        sample.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if sample.len() >= 65536 {
            break;
        }
    }
    assert!(
        sample.windows(4).any(|bytes| bytes == b"ftyp"),
        "downloaded body is not MP4"
    );
    let sample_path =
        std::env::temp_dir().join(format!("videodock-youtube-live-{}.mp4", std::process::id()));
    std::fs::write(&sample_path, &sample).unwrap();
    let subtitle = media
        .subtitles
        .iter()
        .find(|subtitle| subtitle.language == "en")
        .unwrap();
    let body = http
        .get(&subtitle.url)
        .send()
        .await
        .unwrap_or_else(|_| panic!("caption HTTP transport failed"))
        .error_for_status()
        .unwrap_or_else(|_| panic!("caption HTTP status failed"))
        .text()
        .await
        .unwrap_or_else(|_| panic!("caption body failed"));
    assert!(body.starts_with("WEBVTT"));
    println!(
        "video={} formats={} captions={} local_sample={}",
        media.id,
        media.formats.len(),
        media.subtitles.len(),
        sample_path.display()
    );
    let channel = adapter
        .inspect(
            &format!("https://www.youtube.com/channel/{}", media.channel_id),
            &Auth::default(),
        )
        .await
        .unwrap();
    assert!(channel.entries.iter().any(|entry| entry.id == media.id));
    println!("channel entries={}", channel.entries.len());
    let search = adapter
        .search("jNQXAC9IVRw", &Auth::default())
        .await
        .unwrap();
    assert!(search.entries.iter().any(|entry| entry.id == media.id));
    println!("search entries={}", search.entries.len());
}

#[tokio::test]
#[ignore = "public YouTube playlist continuation requests"]
async fn live_youtube_complete_playlist() {
    let media = YouTube::new(None)
        .await
        .unwrap()
        .inspect(
            "https://www.youtube.com/playlist?list=PL2_OBreMn7FrsiSW0VDZjdq0xqUKkZYHT",
            &Auth::default(),
        )
        .await
        .unwrap();
    assert_eq!(media.kind, MediaKind::Playlist);
    assert_eq!(media.title, "Homelab");
    assert_eq!(media.entries.len(), 104);
    assert!(media.continuation.is_none());
    assert!(media.entries.iter().all(|entry| entry.duration.is_some()));
    println!("playlist complete={}", media.entries.len());
}
