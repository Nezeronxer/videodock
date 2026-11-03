use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use videodock_core::{dash, transfer::Limiter};
use videodock_media::{Format, Media, Protocol};

async fn server(files: BTreeMap<String, Vec<u8>>) -> (String, tokio::task::JoinHandle<()>) {
    let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", socket.local_addr().unwrap());
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = socket.accept().await else {
                break;
            };
            let mut request = vec![0; 8192];
            let Ok(n) = stream.read(&mut request).await else {
                continue;
            };
            let request = String::from_utf8_lossy(&request[..n]);
            let path = request.split_whitespace().nth(1).unwrap_or("");
            let (mut status, mut body) = match files.get(path) {
                Some(v) => ("200 OK", v.as_slice()),
                None => ("404 Not Found", b"missing".as_slice()),
            };
            let mut content_range = String::new();
            if let Some(range) = request.lines().find_map(|l| {
                l.to_lowercase()
                    .strip_prefix("range: bytes=")
                    .map(str::to_owned)
            }) {
                let (start, end) = range.split_once('-').unwrap();
                let start: usize = start.parse().unwrap();
                let end: usize = if end.is_empty() {
                    body.len() - 1
                } else {
                    end.parse().unwrap()
                };
                if start >= body.len() {
                    content_range = format!("Content-Range: bytes */{}\r\n", body.len());
                    body = &[];
                    status = "416 Range Not Satisfiable";
                } else {
                    assert!(start <= end && end < body.len());
                    content_range =
                        format!("Content-Range: bytes {start}-{end}/{}\r\n", body.len());
                    body = &body[start..=end];
                    status = "206 Partial Content";
                }
            }
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{content_range}ETag: \"fixture\"\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(body).await;
        }
    });
    (base, task)
}

#[tokio::test]
async fn pause_after_last_chunk_resumes_using_confirmed_416_size() {
    let xml = r#"<MPD mediaPresentationDuration="PT1S"><Period><AdaptationSet mimeType="audio/mp4"><Representation id="x"><SegmentTemplate media="chunk" duration="1"/></Representation></AdaptationSet></Period></MPD>"#;
    let (base, task) = server(BTreeMap::from([
        ("/m.mpd".into(), xml.as_bytes().to_vec()),
        ("/chunk".into(), b"complete".to_vec()),
    ]))
    .await;
    let client = reqwest::Client::new();
    let mut item = media(format!("{base}/m.mpd"));
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    let folder = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let pause = cancel.clone();
    assert!(
        dash::download_dash(
            &client,
            &item.formats[0],
            folder.path(),
            &cancel,
            &Limiter::new(0),
            Arc::new(move |p| if p.speed > 0.0 {
                pause.cancel()
            })
        )
        .await
        .is_err()
    );
    let out = dash::download_dash(
        &client,
        &item.formats[0],
        folder.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(out).await.unwrap(), b"complete");
    task.abort();
}

#[tokio::test]
async fn truncated_segment_resumes_with_matching_validator_and_forwarded_headers() {
    let xml=r#"<MPD mediaPresentationDuration="PT1S"><Period><AdaptationSet mimeType="audio/mp4"><Representation id="x"><SegmentTemplate media="chunk" duration="1"/></Representation></AdaptationSet></Period></MPD>"#.as_bytes().to_vec();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        for index in 0..5 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 8192];
            let n = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]).to_lowercase();
            assert!(request.contains("x-fixture: keep"));
            let (header, data) = if [0, 1, 3].contains(&index) {
                (
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        xml.len()
                    ),
                    xml.as_slice(),
                )
            } else if index == 2 {
                ("HTTP/1.1 200 OK\r\nContent-Length: 10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n".into(),b"0123".as_slice())
            } else {
                assert!(request.contains("range: bytes=4-"));
                assert!(request.contains("if-range: \"stable\""));
                ("HTTP/1.1 206 Partial Content\r\nContent-Length: 6\r\nContent-Range: bytes 4-9/10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n".into(),b"456789".as_slice())
            };
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(data).await.unwrap();
        }
    });
    let client = reqwest::Client::new();
    let mut item = media(format!("{base}/m.mpd"));
    item.formats[0]
        .headers
        .insert("X-Fixture".into(), "keep".into());
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    let folder = tempfile::tempdir().unwrap();
    assert!(
        dash::download_dash(
            &client,
            &item.formats[0],
            folder.path(),
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {})
        )
        .await
        .is_err()
    );
    let output = dash::download_dash(
        &client,
        &item.formats[0],
        folder.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(output).await.unwrap(), b"0123456789");
    task.await.unwrap();
}

#[tokio::test]
async fn invalid_timeline_successor_and_dimensions_are_rejected() {
    for inner in [
        r#"<Representation id="x" height="0"><BaseURL>x.mp4</BaseURL></Representation>"#,
        r#"<Representation id="x"><SegmentTemplate media="$Time$"><SegmentTimeline><S d="2" r="-1"/><S d="2"/></SegmentTimeline></SegmentTemplate></Representation>"#,
    ] {
        let xml = format!(
            r#"<MPD mediaPresentationDuration="PT6S"><Period><AdaptationSet mimeType="video/mp4">{inner}</AdaptationSet></Period></MPD>"#
        );
        let (base, task) =
            server(BTreeMap::from([("/m.mpd".into(), xml.as_bytes().to_vec())])).await;
        let mut item = media(format!("{base}/m.mpd"));
        assert!(
            dash::expand(
                &reqwest::Client::new(),
                &mut item,
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        task.abort();
    }
}

#[tokio::test]
async fn segment_list_ranges_and_completed_segments_survive_pause() {
    let xml = r#"<MPD mediaPresentationDuration="PT2S"><Period><AdaptationSet mimeType="video/mp4" codecs="avc1.4d401f"><Representation id="r"><BaseURL>blob</BaseURL><SegmentList><Initialization range="0-3"/><SegmentURL mediaRange="4-6"/><SegmentURL mediaRange="7-9"/></SegmentList></Representation></AdaptationSet></Period></MPD>"#;
    let (base, task) = server(BTreeMap::from([
        ("/m.mpd".into(), xml.as_bytes().to_vec()),
        ("/blob".into(), b"INIToneTWO".to_vec()),
    ]))
    .await;
    let client = reqwest::Client::new();
    let mut item = media(format!("{base}/m.mpd"));
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let pause = cancel.clone();
    assert!(
        dash::download_dash(
            &client,
            &item.formats[0],
            dir.path(),
            &cancel,
            &Limiter::new(0),
            Arc::new(move |p| if p.speed == 0.0 && p.downloaded > 0 {
                pause.cancel()
            })
        )
        .await
        .is_err()
    );
    let out = dash::download_dash(
        &client,
        &item.formats[0],
        dir.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(out).await.unwrap(), b"INIToneTWO");
    task.abort();
}

#[tokio::test]
async fn timeline_negative_repeat_is_bounded_by_next_s_and_period_offset() {
    let xml = r#"<MPD mediaPresentationDuration="PT6S"><Period><AdaptationSet mimeType="audio/mp4" codecs="mp4a.40.2"><SegmentTemplate timescale="2" presentationTimeOffset="10" initialization="init" media="$Time$"><SegmentTimeline><S t="10" d="4" r="-1"/><S t="18" d="4" r="-1"/></SegmentTimeline></SegmentTemplate><Representation id="a"/></AdaptationSet></Period></MPD>"#;
    let files = BTreeMap::from([
        ("/m.mpd".into(), xml.as_bytes().to_vec()),
        ("/init".into(), b"I".to_vec()),
        ("/10".into(), b"A".to_vec()),
        ("/14".into(), b"B".to_vec()),
        ("/18".into(), b"C".to_vec()),
    ]);
    let (base, task) = server(files).await;
    let client = reqwest::Client::new();
    let mut item = media(format!("{base}/m.mpd"));
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dash::download_dash(
        &client,
        &item.formats[0],
        dir.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(out).await.unwrap(), b"IABC");
    task.abort();
}
fn media(url: String) -> Media {
    Media {
        formats: vec![Format {
            id: "mpd".into(),
            url,
            protocol: Protocol::Dash,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn compatible_periods_form_a_safe_local_concat_and_incompatible_periods_fail() {
    let period = |id: &str, codec: &str| {
        format!(
            r#"<Period duration="PT1S"><AdaptationSet mimeType="video/mp4" codecs="{codec}"><Representation id="{id}" width="320" height="180"><SegmentTemplate media="{id}-$Number$" duration="1"/></Representation></AdaptationSet></Period>"#
        )
    };
    for compatible in [true, false] {
        let xml = format!(
            "<MPD mediaPresentationDuration=\"PT2S\">{}{}</MPD>",
            period("p0", "avc1.4d401f"),
            period(
                "p1",
                if compatible {
                    "avc1.4d401f"
                } else {
                    "hvc1.1.6.L93"
                }
            )
        );
        let (base, task) = server(BTreeMap::from([
            ("/m.mpd".into(), xml.into_bytes()),
            ("/p0-1".into(), b"first".to_vec()),
            ("/p1-1".into(), b"second".to_vec()),
        ]))
        .await;
        let client = reqwest::Client::new();
        let mut item = media(format!("{base}/m.mpd"));
        let expanded = dash::expand(&client, &mut item, &CancellationToken::new()).await;
        if compatible {
            expanded.unwrap();
            assert_eq!(item.formats.len(), 1);
            let folder = tempfile::tempdir().unwrap();
            let out = dash::download_dash(
                &client,
                &item.formats[0],
                folder.path(),
                &CancellationToken::new(),
                &Limiter::new(0),
                Arc::new(|_| {}),
            )
            .await
            .unwrap();
            assert_eq!(out.extension().unwrap(), "ffconcat");
            let contents = tokio::fs::read_to_string(&out).await.unwrap();
            assert!(contents.starts_with("ffconcat version 1.0\n"));
            assert!(contents.contains("file 'period-0000.mp4'"));
            assert!(contents.contains("file 'period-0001.mp4'"));
            assert!(!contents.contains("http"));
        } else {
            assert!(expanded.is_err());
        }
        task.abort();
    }
}

#[tokio::test]
async fn hostile_urls_and_numeric_or_xml_bounds_fail_closed() {
    let bad = [
        r#"<MPD mediaPresentationDuration="P1DT"><Period><AdaptationSet><Representation><BaseURL>x.mp4</BaseURL></Representation></AdaptationSet></Period></MPD>"#,
        r#"<MPD><Period><AdaptationSet><Representation><BaseURL>file:///etc/passwd</BaseURL></Representation></AdaptationSet></Period></MPD>"#,
        r#"<MPD><Period><AdaptationSet><Representation><BaseURL>https://user:password@example.test/x.mp4</BaseURL></Representation></AdaptationSet></Period></MPD>"#,
        r#"<!DOCTYPE MPD [<!ENTITY x SYSTEM "file:///etc/passwd">]><MPD><Period>&x;</Period></MPD>"#,
        r#"<MPD mediaPresentationDuration="PT2S"><Period><AdaptationSet><Representation><SegmentTemplate media="$Number$" duration="1" timescale="0"/></Representation></AdaptationSet></Period></MPD>"#,
        r#"<MPD><Period><AdaptationSet><Representation><SegmentTemplate media="$Time$"><SegmentTimeline><S d="1" r="100000"/></SegmentTimeline></SegmentTemplate></Representation></AdaptationSet></Period></MPD>"#,
        r#"<MPD><Period><AdaptationSet><Representation><BaseURL>file</BaseURL><SegmentList><SegmentURL mediaRange="9-3"/></SegmentList></Representation></AdaptationSet></Period></MPD>"#,
    ];
    for xml in bad {
        let (base, task) =
            server(BTreeMap::from([("/m.mpd".into(), xml.as_bytes().to_vec())])).await;
        let mut item = media(format!("{base}/m.mpd"));
        assert!(
            dash::expand(
                &reqwest::Client::new(),
                &mut item,
                &CancellationToken::new()
            )
            .await
            .is_err(),
            "accepted invalid MPD: {xml}"
        );
        task.abort();
    }
}

#[tokio::test]
async fn byte_range_200_or_mismatching_206_never_becomes_media() {
    for status in ["200 OK", "206 Partial Content"] {
        let xml=r#"<MPD><Period><AdaptationSet mimeType="audio/mp4"><Representation id="x"><BaseURL>blob</BaseURL><SegmentList><SegmentURL mediaRange="2-4"/></SegmentList></Representation></AdaptationSet></Period></MPD>"#.as_bytes().to_vec();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            for index in 0..3 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0; 8192];
                let n = stream.read(&mut request).await.unwrap();
                let request = String::from_utf8_lossy(&request[..n]).to_lowercase();
                let (header, data) = if index < 2 {
                    (
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            xml.len()
                        ),
                        xml.as_slice(),
                    )
                } else {
                    assert!(request.contains("range: bytes=2-4"));
                    (
                        format!(
                            "HTTP/1.1 {status}\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nConnection: close\r\n\r\n"
                        ),
                        b"bad".as_slice(),
                    )
                };
                stream.write_all(header.as_bytes()).await.unwrap();
                stream.write_all(data).await.unwrap();
            }
        });
        let client = reqwest::Client::new();
        let mut item = media(format!("{base}/m.mpd"));
        dash::expand(&client, &mut item, &CancellationToken::new())
            .await
            .unwrap();
        let folder = tempfile::tempdir().unwrap();
        assert!(
            dash::download_dash(
                &client,
                &item.formats[0],
                folder.path(),
                &CancellationToken::new(),
                &Limiter::new(0),
                Arc::new(|_| {})
            )
            .await
            .is_err()
        );
        task.await.unwrap();
    }
}

#[tokio::test]
async fn redirect_and_nested_origin_changes_do_not_forward_manifest_credentials() {
    for redirect in [false, true] {
        let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let first_url = format!("http://{}", first.local_addr().unwrap());
        let second = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let second_url = format!("http://{}", second.local_addr().unwrap());
        let base_value = if redirect {
            "parts/".to_owned()
        } else {
            format!("{second_url}/parts/")
        };
        let manifest = format!(
            r#"<MPD mediaPresentationDuration="PT1S"><Period><AdaptationSet mimeType="audio/mp4" codecs="mp4a.40.2"><Representation id="a"><BaseURL>{base_value}</BaseURL><SegmentTemplate initialization="init" media="part" duration="1"/></Representation></AdaptationSet></Period></MPD>"#
        );
        let manifest2 = manifest.clone();
        let target = second_url.clone();
        let origin = tokio::spawn(async move {
            loop {
                let (mut stream, _) = first.accept().await.unwrap();
                let mut bytes = [0; 8192];
                let n = stream.read(&mut bytes).await.unwrap();
                let req = String::from_utf8_lossy(&bytes[..n]).to_lowercase();
                assert!(req.contains("cookie: fixture=not-a-secret"));
                assert!(req.contains("authorization: bearer fixture"));
                let response = if redirect {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: {target}/nested/main.mpd\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{manifest}",
                        manifest.len()
                    )
                };
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let cdn = tokio::spawn(async move {
            loop {
                let (mut stream, _) = second.accept().await.unwrap();
                let mut bytes = [0; 8192];
                let n = stream.read(&mut bytes).await.unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                let lowered = request.to_lowercase();
                assert!(!lowered.contains("cookie:"));
                assert!(!lowered.contains("authorization:"));
                assert!(!lowered.contains("private-path"));
                let path = request.split_whitespace().nth(1).unwrap();
                let data = if path == "/nested/main.mpd" {
                    manifest2.as_bytes()
                } else if path
                    == if redirect {
                        "/nested/parts/init"
                    } else {
                        "/parts/init"
                    }
                {
                    b"I"
                } else if path
                    == if redirect {
                        "/nested/parts/part"
                    } else {
                        "/parts/part"
                    }
                {
                    b"P"
                } else {
                    panic!("wrong redirected resolution: {path}")
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    data.len()
                );
                stream.write_all(header.as_bytes()).await.unwrap();
                stream.write_all(data).await.unwrap();
            }
        });
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap();
        let mut item = media(format!("{first_url}/start"));
        item.formats[0].headers = BTreeMap::from([
            ("Cookie".into(), "fixture=not-a-secret".into()),
            ("Authorization".into(), "Bearer fixture".into()),
            (
                "Referer".into(),
                format!("{first_url}/private-path?token=fixture"),
            ),
        ]);
        dash::expand(&client, &mut item, &CancellationToken::new())
            .await
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let result = dash::download_dash(
            &client,
            &item.formats[0],
            dir.path(),
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
        assert_eq!(tokio::fs::read(result).await.unwrap(), b"IP");
        origin.abort();
        cdn.abort();
    }
}

#[tokio::test]
async fn typed_cookie_context_respects_same_origin_paths_expiry_and_secure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let xml=r#"<MPD mediaPresentationDuration="PT1S"><Period><AdaptationSet mimeType="audio/mp4"><Representation id="x"><BaseURL>../public/</BaseURL><SegmentTemplate initialization="init" media="chunk" duration="1"/></Representation></AdaptationSet></Period></MPD>"#.as_bytes().to_vec();
    let server = tokio::spawn(async move {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 8192];
            let n = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            let lower = request.to_lowercase();
            let path = request.split_whitespace().nth(1).unwrap();
            assert!(!lower.contains("expired="));
            assert!(!lower.contains("secure_only="));
            let body = if path == "/private/m.mpd" {
                assert!(lower.contains("cookie: session=fixture"));
                xml.as_slice()
            } else {
                assert!(!lower.contains("cookie:"));
                if path == "/public/init" {
                    b"I"
                } else if path == "/public/chunk" {
                    b"C"
                } else {
                    panic!("wrong path {path}")
                }
            };
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(body).await.unwrap();
        }
    });
    let mut item = media(format!("{base}/private/m.mpd"));
    item.cookies = vec![
        videodock_media::ScopedCookie {
            name: "session".into(),
            value: "fixture".into(),
            domain: "127.0.0.1".into(),
            path: "/private/".into(),
            secure: false,
            expires: None,
        },
        videodock_media::ScopedCookie {
            name: "expired".into(),
            value: "fixture".into(),
            domain: "127.0.0.1".into(),
            path: "/".into(),
            secure: false,
            expires: Some(0),
        },
        videodock_media::ScopedCookie {
            name: "secure_only".into(),
            value: "fixture".into(),
            domain: "127.0.0.1".into(),
            path: "/".into(),
            secure: true,
            expires: None,
        },
    ];
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(item.formats[0].cookies.len(), 3);
    let dir = tempfile::tempdir().unwrap();
    let output = dash::download_dash(
        &client,
        &item.formats[0],
        dir.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(output).await.unwrap(), b"IC");
    server.await.unwrap();
}

#[tokio::test]
async fn vod_templates_expand_video_and_audio_metadata_and_download_real_parts() {
    let xml = r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" mediaPresentationDuration="PT4S"><BaseURL>media/</BaseURL><Period><AdaptationSet contentType="video" mimeType="video/mp4" codecs="avc1.64001f"><SegmentTemplate timescale="1" duration="2" initialization="$RepresentationID$-init" media="$RepresentationID$-$Number%03d$" startNumber="7"/><Representation id="v" bandwidth="800000" width="1280" height="720" frameRate="30000/1001"/></AdaptationSet><AdaptationSet contentType="audio" mimeType="audio/mp4" codecs="mp4a.40.2" lang="fr"><SegmentTemplate timescale="2" initialization="a-init" media="a-$Time$"><SegmentTimeline><S t="10" d="4" r="1"/></SegmentTimeline></SegmentTemplate><Representation id="a" bandwidth="128000"/></AdaptationSet></Period></MPD>"#;
    let files = BTreeMap::from([
        ("/master.mpd".into(), xml.as_bytes().to_vec()),
        ("/media/v-init".into(), b"init".to_vec()),
        ("/media/v-007".into(), b"one".to_vec()),
        ("/media/v-008".into(), b"two".to_vec()),
    ]);
    let (base, task) = server(files).await;
    let mut item = media(format!("{base}/master.mpd"));
    let client = reqwest::Client::new();
    dash::expand(&client, &mut item, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(item.formats.len(), 2);
    let v = &item.formats[0];
    assert_eq!(v.height, Some(720));
    assert_eq!(v.bitrate, Some(800000));
    assert!(v.has_video() && !v.has_audio());
    assert!((v.fps.unwrap() - 29.97002997).abs() < 0.0001);
    assert_eq!(item.formats[1].audio_language.as_deref(), Some("fr"));
    assert!(!item.formats[1].has_video() && item.formats[1].has_audio());
    let dir = tempfile::tempdir().unwrap();
    let out = dash::download_dash(
        &client,
        v,
        dir.path(),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(out).await.unwrap(), b"initonetwo");
    task.abort();
}

#[tokio::test]
async fn progressive_base_url_inherits_and_becomes_http() {
    let xml = r#"<MPD mediaPresentationDuration="PT1S"><BaseURL>folder/</BaseURL><Period><BaseURL>nested/</BaseURL><AdaptationSet mimeType="audio/mp4" lang="en"><Representation id="one" codecs="mp4a.40.2" bandwidth="123"><BaseURL>audio.m4a?sig=x&amp;y=z</BaseURL></Representation></AdaptationSet></Period></MPD>"#;
    let (base, task) = server(BTreeMap::from([("/m.mpd".into(), xml.as_bytes().to_vec())])).await;
    let mut item = media(format!("{base}/m.mpd"));
    dash::expand(
        &reqwest::Client::new(),
        &mut item,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(item.formats[0].protocol, Protocol::Http);
    assert_eq!(
        item.formats[0].url,
        format!("{base}/folder/nested/audio.m4a?sig=x&y=z")
    );
    task.abort();
}

#[tokio::test]
async fn drm_live_and_unbounded_timeline_are_errors_without_mutating_formats() {
    for xml in [
        r#"<MPD><Period><AdaptationSet><ContentProtection schemeIdUri="urn:uuid:test"/><Representation id="x"/></AdaptationSet></Period></MPD>"#,
        r#"<MPD type="dynamic"><Period/></MPD>"#,
        r#"<MPD><Period><AdaptationSet mimeType="video/mp4"><Representation id="x"><SegmentTemplate media="$Time$"><SegmentTimeline><S d="2" r="-1"/></SegmentTimeline></SegmentTemplate></Representation></AdaptationSet></Period></MPD>"#,
    ] {
        let (base, task) =
            server(BTreeMap::from([("/m.mpd".into(), xml.as_bytes().to_vec())])).await;
        let original = format!("{base}/m.mpd");
        let mut item = media(original.clone());
        assert!(
            dash::expand(
                &reqwest::Client::new(),
                &mut item,
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        assert_eq!(item.formats[0].url, original);
        task.abort();
    }
}
