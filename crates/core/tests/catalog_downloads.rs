//! Opt-in complete public downloads; no credentials and no signed URLs in output.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use videodock_core::{
    pipeline::{self, Options},
    transfer::Limiter,
};
use videodock_media::{Auth, Selection, catalog::Catalog};

async fn full_sample(service: &str, address: &str, audio: bool) {
    let proxy = std::env::var("VIDEODOCK_TEST_PROXY").ok();
    let catalog = Catalog::new(proxy.as_deref()).unwrap();
    let started = Instant::now();
    let media = tokio::time::timeout(
        Duration::from_secs(90),
        catalog.inspect(address, &Auth::default()),
    )
    .await
    .expect("metadata deadline")
    .unwrap_or_else(|error| {
        let message = error.to_string();
        let safe = if message.contains("http") {
            "metadata transport error"
        } else {
            &message
        };
        panic!("{service}: metadata failed: {safe}")
    });
    let metadata_seconds = started.elapsed().as_secs_f64();
    let expected_duration = media.duration;
    println!(
        "{service}: metadata_seconds={metadata_seconds:.3} duration={expected_duration:?} formats={}",
        media.formats.len()
    );
    let temp = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(600),
        pipeline::run(
            media,
            catalog.client(),
            Options {
                selection: Selection {
                    media_type: if audio { "audio" } else { "video" }.into(),
                    container: if audio { "m4a" } else { "mp4" }.into(),
                    quality: 1080,
                    ..Default::default()
                },
                output_dir: std::env::var_os("VIDEODOCK_TEST_OUTPUT_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| temp.path().join("output")),
                filename: Some(service.into()),
                embed_subtitles: false,
                tags: false,
                skip_duplicates: false,
            },
            temp.path().join("staging"),
            CancellationToken::new(),
            Arc::new(Limiter::new(0)),
            Arc::new(|_| {}),
        ),
    )
    .await
    .expect("complete download deadline")
    .unwrap_or_else(|error| {
        let message = error.to_string();
        let safe = if message.contains("http")
            || message.contains("token")
            || message.contains("signature")
        {
            "transport error (URL omitted)"
        } else {
            &message
        };
        panic!("{service}: pipeline failed: {safe}")
    });
    let download_seconds = started.elapsed().as_secs_f64();
    let info = pipeline::probe(&result.path).await.unwrap();
    let streams = info["streams"].as_array().unwrap();
    assert!(
        streams.iter().any(|stream| stream["codec_type"] == "audio"),
        "{service}: missing audio"
    );
    if !audio {
        assert!(
            streams.iter().any(|stream| stream["codec_type"] == "video"),
            "{service}: missing video"
        );
    }
    let duration = info["format"]["duration"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!(duration > 0.0 && result.size > 1024);
    if let Some(expected) = expected_duration.filter(|duration| *duration > 0.0) {
        if (duration - expected).abs() > (expected * 0.02).max(3.0) {
            let retained = temp.keep();
            println!(
                "{service}: incomplete artifacts retained at {}",
                retained.display()
            );
        }
        assert!(
            (duration - expected).abs() <= (expected * 0.02).max(3.0),
            "{service}: incomplete duration {duration}/{expected}"
        );
    }
    println!(
        "{service}: PASS bytes={} download_seconds={download_seconds:.3} MiB_per_second={:.3} duration={duration:.3}",
        result.size,
        result.size as f64 / 1_048_576.0 / download_seconds
    );
}

macro_rules! sample {
    ($name:ident, $service:literal, $url:literal, $audio:literal) => {
        #[tokio::test]
        #[ignore = "complete public provider download with FFmpeg/ffprobe"]
        async fn $name() {
            full_sample($service, $url, $audio).await;
        }
    };
}
sample!(vimeo, "Vimeo", "https://vimeo.com/76979871", false);
sample!(
    dailymotion,
    "Dailymotion",
    "https://www.dailymotion.com/video/x84sh87",
    false
);
sample!(
    soundcloud,
    "SoundCloud",
    "https://soundcloud.com/forss/flickermood",
    true
);
sample!(
    twitch,
    "Twitch",
    "https://clips.twitch.tv/CrispyJollyGullHassaanChop-nPlLKGxGRcBj37e4",
    false
);
sample!(
    bilibili,
    "BiliBili",
    "https://www.bilibili.com/video/BV1xx411c7mD",
    false
);
sample!(rumble, "Rumble", "https://rumble.com/embed/v5hzoa5/", false);
sample!(
    niconico,
    "NicoVideo",
    "https://www.nicovideo.jp/watch/sm9",
    false
);
sample!(
    flickr,
    "Flickr",
    "https://www.flickr.com/photos/24354425@N03/55620471687",
    false
);
sample!(
    bitchute,
    "BitChute",
    "https://www.bitchute.com/video/aF9jQK57yx0B",
    false
);
sample!(
    tiktok,
    "TikTok",
    "https://www.tiktok.com/@tiktok/video/7106594312292453675",
    false
);
sample!(
    tumblr,
    "Tumblr",
    "https://www.tumblr.com/staff/653731604461682688/potatoes-are-neat-right-heres-one-cooking-in-a",
    false
);
sample!(
    generic,
    "Generic",
    "https://media.w3.org/2010/05/sintel/trailer.mp4",
    false
);

#[tokio::test]
#[ignore = "checks a streaming response longer than the 30-second metadata deadline"]
async fn media_client_has_idle_timeout_instead_of_total_deadline() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}/slow", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).await.unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        for _ in 0..7 {
            tokio::time::sleep(Duration::from_secs(5)).await;
            stream.write_all(b"x").await.unwrap();
        }
    });
    let response = Catalog::new(None)
        .unwrap()
        .client()
        .get(address)
        .send()
        .await
        .unwrap();
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"xxxxxxx");
    server.await.unwrap();
}
