use videodock_media::{Auth, Protocol, catalog::Catalog};

async fn public_sample(service: &str, address: &str) {
    let catalog = Catalog::new(None).unwrap();
    let media = catalog
        .inspect(address, &Auth::default())
        .await
        .unwrap_or_else(|error| panic!("{service}: inspect failed: {error}"));
    assert!(
        !media.title.is_empty() && !media.formats.is_empty(),
        "{service}: missing title/formats"
    );
    let stream = media
        .formats
        .iter()
        .find(|format| format.protocol == Protocol::Http)
        .unwrap_or(&media.formats[0]);
    let first = url::Url::parse(&stream.url).unwrap();
    let mut current = first.clone();
    let mut content = None;
    for _ in 0..6 {
        let headers = videodock_media::request_headers(
            &stream.headers,
            &stream.url,
            current.as_str(),
            &stream.cookies,
        )
        .unwrap();
        let mut request = catalog
            .client()
            .get(current.clone())
            .header("Range", "bytes=0-65535");
        for (name, value) in &headers {
            if !name.eq_ignore_ascii_case("range") {
                request = request.header(name, value);
            }
        }
        let mut response = request
            .send()
            .await
            .unwrap_or_else(|_| panic!("{service}: media HTTP transport failed"));
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_else(|| panic!("{service}: empty redirect"));
            current = current.join(location).unwrap();
            continue;
        }
        assert!(
            response.status().is_success(),
            "{service}: media HTTP{}",
            response.status().as_u16()
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .unwrap_or_else(|_| panic!("{service}: media body failed"))
        {
            let remaining = 65536usize.saturating_sub(bytes.len());
            bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            if bytes.len() >= 65536 {
                break;
            }
        }
        content = Some(bytes);
        break;
    }
    let bytes = content.unwrap_or_else(|| panic!("{service}: redirect loop"));
    assert!(bytes.len() > 32, "{service}: empty body");
    match stream.protocol {
        Protocol::Hls => assert!(
            String::from_utf8_lossy(&bytes)
                .trim_start_matches('\u{feff}')
                .starts_with("#EXTM3U"),
            "{service}: expected HLS"
        ),
        Protocol::Dash => assert!(
            String::from_utf8_lossy(&bytes).contains("<MPD"),
            "{service}: expected MPD"
        ),
        Protocol::Http => assert!(
            bytes
                .windows(4)
                .any(|value| value == b"ftyp" || value == b"styp")
                || bytes.starts_with(b"OggS")
                || bytes.starts_with(b"ID3")
                || bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]),
            "{service}: body has no recognizable media container"
        ),
    }
    println!(
        "{service}: formats={} subtitles={} sample_bytes={} protocol={:?}",
        media.formats.len(),
        media.subtitles.len(),
        bytes.len(),
        stream.protocol
    );
}
macro_rules! sample {
    ($name:ident,$service:literal,$url:literal) => {
        #[tokio::test]
        #[ignore = "public provider metadata and limited media HTTP requests"]
        async fn $name() {
            public_sample($service, $url).await;
        }
    };
}
sample!(vimeo, "Vimeo", "https://vimeo.com/76979871");
sample!(
    dailymotion,
    "Dailymotion",
    "https://www.dailymotion.com/video/x84sh87"
);
sample!(
    soundcloud,
    "SoundCloud",
    "https://soundcloud.com/forss/flickermood"
);
sample!(twitch, "Twitch", "https://www.twitch.tv/videos/2915087502");
sample!(
    bilibili,
    "BiliBili",
    "https://www.bilibili.com/video/BV1xx411c7mD"
);
sample!(rumble, "Rumble", "https://rumble.com/embed/v5hzoa5/");
sample!(niconico, "NicoVideo", "https://www.nicovideo.jp/watch/sm9");
sample!(
    flickr,
    "Flickr",
    "https://www.flickr.com/photos/24354425@N03/55620471687"
);
sample!(
    bitchute,
    "BitChute",
    "https://www.bitchute.com/video/aF9jQK57yx0B"
);
sample!(
    tiktok,
    "TikTok",
    "https://www.tiktok.com/@tiktok/video/7106594312292453675"
);
