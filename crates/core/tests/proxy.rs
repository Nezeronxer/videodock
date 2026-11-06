//! Explicit proxy routing, authentication and fail-closed regression checks.
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;
use videodock_core::transfer::{Limiter, download};
use videodock_media::catalog::Catalog;

struct Proxy {
    url: String,
    accepted: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn headers(socket: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 16384);
        let mut byte = [0];
        if socket.read_exact(&mut byte).await.is_err() {
            break;
        }
        bytes.push(byte[0]);
    }
    bytes
}
async fn proxy(socks: bool, auth: bool) -> Proxy {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = accepted.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut downstream, _) = listener.accept().await.unwrap();
            let count = count.clone();
            tokio::spawn(async move {
                count.fetch_add(1, Ordering::SeqCst);
                if socks {
                    let mut greeting = [0; 2];
                    downstream.read_exact(&mut greeting).await.unwrap();
                    assert_eq!(greeting[0], 5);
                    let mut methods = vec![0; greeting[1] as usize];
                    downstream.read_exact(&mut methods).await.unwrap();
                    let method = if auth { 2 } else { 0 };
                    assert!(methods.contains(&method));
                    downstream.write_all(&[5, method]).await.unwrap();
                    if auth {
                        let mut auth_header = [0; 2];
                        downstream.read_exact(&mut auth_header).await.unwrap();
                        assert_eq!(auth_header[0], 1);
                        let mut username = vec![0; auth_header[1] as usize];
                        downstream.read_exact(&mut username).await.unwrap();
                        let password_length = downstream.read_u8().await.unwrap();
                        let mut password = vec![0; password_length as usize];
                        downstream.read_exact(&mut password).await.unwrap();
                        let valid = username == b"test" && password == b"synthetic";
                        downstream
                            .write_all(&[1, if valid { 0 } else { 1 }])
                            .await
                            .unwrap();
                        if !valid {
                            return;
                        }
                    }
                    let mut request = [0; 4];
                    downstream.read_exact(&mut request).await.unwrap();
                    assert_eq!(&request[..3], &[5, 1, 0]);
                    let host = match request[3] {
                        1 => {
                            let mut ip = [0; 4];
                            downstream.read_exact(&mut ip).await.unwrap();
                            std::net::Ipv4Addr::from(ip).to_string()
                        }
                        3 => {
                            let length = downstream.read_u8().await.unwrap();
                            let mut domain = vec![0; length as usize];
                            downstream.read_exact(&mut domain).await.unwrap();
                            String::from_utf8(domain).unwrap()
                        }
                        other => panic!("unexpected address kind {other}"),
                    };
                    let port = downstream.read_u16().await.unwrap();
                    let Ok(mut upstream) = TcpStream::connect((
                        if host == "fixture.invalid" {
                            "127.0.0.1"
                        } else {
                            host.as_str()
                        },
                        port,
                    ))
                    .await
                    else {
                        return;
                    };
                    downstream
                        .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0])
                        .await
                        .unwrap();
                    tokio::io::copy_bidirectional(&mut downstream, &mut upstream)
                        .await
                        .ok();
                } else {
                    let request = headers(&mut downstream).await;
                    let text = String::from_utf8(request).unwrap();
                    let first = text.lines().next().unwrap();
                    let auth_line = text
                        .lines()
                        .find(|line| line.to_lowercase().starts_with("proxy-authorization:"));
                    if auth
                        && auth_line != Some("proxy-authorization: Basic dGVzdDpzeW50aGV0aWM=")
                        && auth_line != Some("Proxy-Authorization: Basic dGVzdDpzeW50aGV0aWM=")
                    {
                        downstream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.ok();
                        return;
                    }
                    let target = first.split_whitespace().nth(1).unwrap();
                    let connect = first.starts_with("CONNECT ");
                    let address = if connect {
                        format!("https://{target}")
                    } else {
                        target.to_owned()
                    };
                    let url = url::Url::parse(&address).unwrap();
                    let Ok(mut upstream) = TcpStream::connect((
                        if url.host_str() == Some("fixture.invalid") {
                            "127.0.0.1"
                        } else {
                            url.host_str().unwrap()
                        },
                        url.port_or_known_default().unwrap(),
                    ))
                    .await
                    else {
                        return;
                    };
                    if connect {
                        downstream
                            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                            .await
                            .unwrap();
                    } else {
                        let path = match url.query() {
                            Some(query) => format!("{}?{query}", url.path()),
                            None => url.path().to_owned(),
                        };
                        let mut forwarded = first.replacen(target, &path, 1) + "\r\n";
                        for line in text.lines().skip(1).filter(|line| {
                            !line.is_empty()
                                && !line.to_lowercase().starts_with("proxy-authorization:")
                        }) {
                            forwarded.push_str(line);
                            forwarded.push_str("\r\n");
                        }
                        forwarded.push_str("\r\n");
                        upstream.write_all(forwarded.as_bytes()).await.unwrap();
                    }
                    tokio::io::copy_bidirectional(&mut downstream, &mut upstream)
                        .await
                        .ok();
                }
            });
        }
    });
    Proxy {
        url: format!(
            "{}://{}{address}",
            if socks { "socks5h" } else { "http" },
            if auth { "test:synthetic@" } else { "" }
        ),
        accepted,
        server,
    }
}
async fn source() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}/media", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let reads = count.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = headers(&mut socket).await;
            assert!(
                !String::from_utf8_lossy(&request)
                    .to_lowercase()
                    .contains("proxy-authorization"),
                "proxy credentials reached origin"
            );
            reads.fetch_add(1, Ordering::SeqCst);
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef",
                )
                .await
                .ok();
        }
    });
    (address, count, server)
}
async fn routed(socks: bool, auth: bool) {
    let proxy = proxy(socks, auth).await;
    let (address, reads, server) = source().await;
    let catalog = Catalog::new(Some(&proxy.url)).unwrap();
    let response = catalog.client().get(&address).send().await.unwrap();
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"abcdef");
    let temp = tempfile::tempdir().unwrap();
    download(
        &catalog.client(),
        &address,
        &BTreeMap::new(),
        &temp.path().join("media"),
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(
        tokio::fs::read(temp.path().join("media")).await.unwrap(),
        b"abcdef"
    );
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    assert_eq!(proxy.accepted.load(Ordering::SeqCst), 2);
    server.abort();
}
#[tokio::test]
async fn http_proxy_routes_metadata_and_transfer() {
    routed(false, false).await;
}
#[tokio::test]
async fn http_proxy_auth_routes_metadata_and_transfer() {
    routed(false, true).await;
}
#[tokio::test]
async fn socks5_proxy_routes_metadata_and_transfer() {
    routed(true, false).await;
}
#[tokio::test]
async fn socks5_proxy_auth_routes_metadata_and_transfer() {
    routed(true, true).await;
}
#[tokio::test]
async fn dead_proxy_never_falls_back_to_origin() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let (address, reads, server) = source().await;
    for scheme in ["http", "socks5h"] {
        if let Ok(catalog) = Catalog::new(Some(&format!("{scheme}://127.0.0.1:{port}"))) {
            assert!(
                tokio::time::timeout(
                    Duration::from_secs(5),
                    catalog.client().get(&address).send()
                )
                .await
                .unwrap()
                .is_err()
            );
        }
    }
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    server.abort();
}
#[tokio::test]
async fn wrong_proxy_password_never_falls_back_to_origin() {
    let (address, reads, server) = source().await;
    for socks in [false, true] {
        let proxy = proxy(socks, true).await;
        let catalog = Catalog::new(Some(&proxy.url.replace("synthetic@", "wrong@"))).unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            catalog.client().get(&address).send(),
        )
        .await
        .unwrap();
        assert!(
            result.is_err()
                || result.unwrap().status() == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED
        );
        assert!(proxy.accepted.load(Ordering::SeqCst) > 0);
    }
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    server.abort();
}

async fn youtube_routed(socks: bool) {
    let proxy = proxy(socks, true).await;
    let start = std::time::Instant::now();
    let catalog = Catalog::new(Some(&proxy.url)).unwrap();
    let media = catalog
        .inspect(
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            &videodock_media::Auth::default(),
        )
        .await
        .unwrap();
    let metadata_seconds = start.elapsed().as_secs_f64();
    let metadata_connections = proxy.accepted.load(Ordering::SeqCst);
    assert!(metadata_connections > 0, "metadata bypassed proxy");
    let temp = tempfile::tempdir().unwrap();
    let output = videodock_core::pipeline::run(
        media,
        catalog.client(),
        videodock_core::pipeline::Options {
            selection: videodock_media::Selection {
                quality: 480,
                ..Default::default()
            },
            output_dir: temp.path().join("output"),
            filename: None,
            embed_subtitles: false,
            tags: false,
            skip_duplicates: false,
        },
        temp.path().join("staging"),
        CancellationToken::new(),
        Arc::new(Limiter::new(0)),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    let streams = output.info["streams"].as_array().unwrap();
    assert!(streams.iter().any(|stream| stream["codec_type"] == "video"));
    assert!(streams.iter().any(|stream| stream["codec_type"] == "audio"));
    assert!(
        proxy.accepted.load(Ordering::SeqCst) > metadata_connections,
        "media bypassed proxy"
    );
    println!(
        "YouTube proxy={} auth=yes metadata={metadata_seconds:.2}s total={:.2}s bytes={} connections={}",
        if socks { "SOCKS5" } else { "CONNECT" },
        start.elapsed().as_secs_f64(),
        output.size,
        proxy.accepted.load(Ordering::SeqCst)
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "downloads a complete public YouTube sample through a loopback authenticated HTTP CONNECT proxy"]
async fn youtube_complete_via_http_connect_auth() {
    youtube_routed(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "downloads a complete public YouTube sample through a loopback authenticated SOCKS5 proxy"]
async fn youtube_complete_via_socks5_auth() {
    youtube_routed(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn engine_proxy_routes_hls_segments_and_subtitles() {
    use serde_json::json;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    tokio::fs::create_dir(&source).await.unwrap();
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x90:rate=15",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440",
            "-t",
            "2",
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
    tokio::fs::write(source.join("entry"),b"<html><title>Proxy HLS</title><video src=\"/stream.m3u8\"><track kind=\"subtitles\" src=\"/captions.vtt\" srclang=\"en\" label=\"English\"></video></html>").await.unwrap();
    tokio::fs::write(
        source.join("captions.vtt"),
        b"WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nSynthetic caption\n",
    )
    .await
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let reads = requests.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let source = source.clone();
            let reads = reads.clone();
            tokio::spawn(async move {
                let bytes = headers(&mut socket).await;
                let text = String::from_utf8_lossy(&bytes);
                assert!(!text.to_lowercase().contains("proxy-authorization"));
                let path = text.split_whitespace().nth(1).unwrap();
                assert!(!path.contains(".."));
                reads.lock().unwrap().push(path.to_owned());
                let body = tokio::fs::read(source.join(path.trim_start_matches('/')))
                    .await
                    .unwrap();
                let mime = if path == "/entry" {
                    "text/html"
                } else if path.ends_with(".m3u8") {
                    "application/vnd.apple.mpegurl"
                } else if path.ends_with(".vtt") {
                    "text/vtt"
                } else {
                    "video/mp2t"
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if socket.write_all(head.as_bytes()).await.is_ok() {
                    socket.write_all(&body).await.ok();
                }
            });
        }
    });
    for (index, (socks, auth)) in [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        let proxy = proxy(socks, auth).await;
        let proxy_url = url::Url::parse(&proxy.url).unwrap();
        let before = requests.lock().unwrap().len();
        let engine =
            videodock_core::Engine::new(temp.path().join(format!("data-{index}"))).unwrap();
        engine.dispatch(json!({"action":"settings","values":{
            "path":temp.path().join(format!("output-{index}")),"tagsEnabled":false,"smartMode":true,"quality":180,"container":"mp4","subtitles":["en"],"embedSubtitles":false,
            "proxyEnabled":true,"proxyType":if socks {"SOCKS5"} else {"HTTP"},"proxyHost":"127.0.0.1","proxyPort":proxy_url.port().unwrap(),"proxyLogin":if auth {"test"} else {""},"proxyPassword":if auth {"synthetic"} else {""}
        }})).unwrap();
        assert!(
            !engine.snapshot().to_string().contains("synthetic"),
            "proxy password escaped into snapshot"
        );
        engine
            .dispatch(json!({"action":"parse","url":format!("{address}/entry")}))
            .unwrap();
        let started = std::time::Instant::now();
        let job = loop {
            let state = engine.snapshot();
            assert!(
                state["notice"].as_str().unwrap_or_default().is_empty(),
                "metadata failed"
            );
            if let Some(job) = state["jobs"].as_array().and_then(|jobs| {
                jobs.iter()
                    .find(|job| job["url"] == format!("{address}/entry"))
            }) {
                assert_ne!(job["state"], "error", "proxy queue failed");
                if job["state"] == "complete" {
                    break job.clone();
                }
            }
            assert!(
                started.elapsed() < Duration::from_secs(15),
                "queue deadline"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let info =
            videodock_core::pipeline::probe(std::path::Path::new(job["path"].as_str().unwrap()))
                .await
                .unwrap();
        let streams = info["streams"].as_array().unwrap();
        assert!(streams.iter().any(|stream| stream["codec_type"] == "video"));
        assert!(streams.iter().any(|stream| stream["codec_type"] == "audio"));
        let requests = requests.lock().unwrap();
        let routed = &requests[before..];
        assert!(routed.iter().any(|path| path.ends_with(".ts")));
        assert!(routed.iter().any(|path| path.ends_with(".vtt")));
        assert!(routed.iter().any(|path| path.ends_with(".m3u8")));
        assert_eq!(
            proxy.accepted.load(Ordering::SeqCst),
            routed.len(),
            "a request bypassed the explicit proxy"
        );
        println!(
            "Engine proxy={} auth={auth} requests={} HLS+captions+ffprobe=pass",
            if socks { "SOCKS5" } else { "HTTP" },
            routed.len()
        );
    }
    for socks in [false, true] {
        let proxy = proxy(socks, true).await;
        let before = requests.lock().unwrap().len();
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "direct_mode_and_explicit_proxy_configuration",
            "--nocapture",
        ]);
        for key in [
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
            "http_proxy",
            "HTTP_PROXY",
        ] {
            command.env_remove(key);
        }
        let result = command
            .env("HTTPS_PROXY", &proxy.url)
            .env("NO_PROXY", "*")
            .env("VIDEODOCK_AUTO_PROXY_CHILD", "hls")
            .env(
                "VIDEODOCK_AUTO_SOURCE",
                format!("{address}/entry"),
            )
            .output()
            .await
            .unwrap();
        assert!(
            result.status.success(),
            "automatic HLS child failed socks={socks}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let requests = requests.lock().unwrap();
        let routed = &requests[before..];
        assert!(routed.iter().any(|path| path.ends_with(".ts")));
        assert!(routed.iter().any(|path| path.ends_with(".vtt")));
        assert_eq!(
            proxy.accepted.load(Ordering::SeqCst),
            0,
            "disabled proxy unexpectedly used ambient configuration"
        );
        println!(
            "Engine direct route ignores ambient proxy={} requests={} complete-HLS+captions+duration+ffprobe=pass",
            if socks { "SOCKS5" } else { "HTTP" },
            routed.len()
        );
    }
    server.abort();
}

// Separate processes avoid mutating the test runner's global proxy environment.
#[tokio::test]
async fn direct_mode_and_explicit_proxy_configuration() {
    const TEST: &str = "direct_mode_and_explicit_proxy_configuration";
    if let Ok(mode) = std::env::var("VIDEODOCK_AUTO_PROXY_CHILD") {
        let temp = tempfile::tempdir().unwrap();
        let engine = videodock_core::Engine::new(temp.path().join("profile")).unwrap();
        if mode == "manual" {
            let selected =
                url::Url::parse(&std::env::var("VIDEODOCK_AUTO_SELECTED").unwrap()).unwrap();
            engine.dispatch(serde_json::json!({"action":"settings","values":{
                "proxyEnabled":true,"proxyType":"HTTP","proxyHost":"127.0.0.1",
                "proxyPort":selected.port().unwrap(),"proxyLogin":"test","proxyPassword":"synthetic"
            }})).unwrap();
        }
        if mode == "hls" {
            engine
                .dispatch(serde_json::json!({"action":"settings","values":{
                    "path":temp.path().join("output"),"tagsEnabled":false,"smartMode":true,
                    "quality":180,"container":"mp4","subtitles":["en"],"embedSubtitles":false
                }}))
                .unwrap();
            let source = std::env::var("VIDEODOCK_AUTO_SOURCE").unwrap();
            engine
                .dispatch(serde_json::json!({"action":"parse","url":source}))
                .unwrap();
            let started = std::time::Instant::now();
            let job = loop {
                let state = engine.snapshot();
                assert!(
                    !state.to_string().contains("synthetic"),
                    "automatic proxy credentials escaped into snapshot"
                );
                assert!(
                    state["notice"].as_str().unwrap_or_default().is_empty(),
                    "automatic metadata failed"
                );
                if let Some(job) = state["jobs"].as_array().and_then(|jobs| jobs.first()) {
                    assert_ne!(job["state"], "error", "automatic HLS queue failed");
                    if job["state"] == "complete" {
                        break job.clone();
                    }
                }
                assert!(
                    started.elapsed() < Duration::from_secs(20),
                    "automatic HLS queue deadline"
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            };
            let info = videodock_core::pipeline::probe(std::path::Path::new(
                job["path"].as_str().unwrap(),
            ))
            .await
            .unwrap();
            let streams = info["streams"].as_array().unwrap();
            assert!(streams.iter().any(|stream| stream["codec_type"] == "video"));
            assert!(streams.iter().any(|stream| stream["codec_type"] == "audio"));
            let duration = info["format"]["duration"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap();
            assert!((duration - 2.0).abs() < 0.3);
            return;
        }
        if mode == "invalid" {
            assert!(engine.network_proxy().unwrap().is_none(), "disabled proxy inherited malformed ambient configuration");
            assert!(!engine.snapshot().to_string().contains("synthetic"));
            return;
        }
        let selected = engine.network_proxy().unwrap();
        assert_eq!(selected.is_some(), mode == "manual", "disabled proxy must use the direct OS route");
        assert!(!engine.snapshot().to_string().contains("synthetic"));
        let catalog = if mode == "manual" {
            Catalog::new(selected.as_deref())
        } else {
            // The library retains its explicitly requested automatic mode.
            Catalog::new_automatic(std::env::var("HTTPS_PROXY").ok().as_deref())
        }
        .unwrap();
        let source = std::env::var("VIDEODOCK_AUTO_SOURCE").unwrap();
        let response = catalog.client().get(&source).send().await;
        if mode == "dead" || mode == "wrong" {
            assert!(
                response.is_err()
                    || response
                        .as_ref()
                        .is_ok_and(|response| response.status().is_client_error()
                            || response.status().is_server_error()),
                "failed configured proxy silently bypassed to origin"
            );
            return;
        }
        assert_eq!(response.unwrap().bytes().await.unwrap().as_ref(), b"abcdef");
        download(
            &catalog.client(),
            &source,
            &BTreeMap::new(),
            &temp.path().join("media"),
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
        assert_eq!(
            tokio::fs::read(temp.path().join("media")).await.unwrap(),
            b"abcdef"
        );
        return;
    }
    let (source, reads, server) = source().await;
    for (mode, socks) in [
        ("auto", false),
        ("auto", true),
        ("manual", false),
        ("wrong", false),
        ("wrong", true),
        ("dead", false),
        ("invalid", false),
    ] {
        let proxy = proxy(socks, true).await;
        let before = reads.load(Ordering::SeqCst);
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture"]);
        for key in [
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
            "http_proxy",
            "HTTP_PROXY",
        ] {
            command.env_remove(key);
        }
        let configured = match mode {
            "manual" | "invalid" => "file://test:synthetic@localhost/private".to_owned(),
            "dead" => {
                let dead = TcpListener::bind("127.0.0.1:0").await.unwrap();
                format!("http://{}", dead.local_addr().unwrap())
            }
            "wrong" => proxy.url.replace("synthetic@", "wrong@"),
            _ => proxy.url.clone(),
        };
        let result = command
            .env("HTTPS_PROXY", configured)
            .env("NO_PROXY", "*")
            .env("VIDEODOCK_AUTO_PROXY_CHILD", mode)
            .env(
                "VIDEODOCK_AUTO_SOURCE",
                source.replace("127.0.0.1", "fixture.invalid"),
            )
            .env("VIDEODOCK_AUTO_SELECTED", &proxy.url)
            .output()
            .await
            .unwrap();
        assert!(
            result.status.success(),
            "auto route child failed mode={mode} socks={socks}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let expected = if matches!(mode, "auto" | "manual") {
            2
        } else {
            0
        };
        assert_eq!(
            reads.load(Ordering::SeqCst) - before,
            expected,
            "configured proxy route changed"
        );
        if expected > 0 {
            assert_eq!(proxy.accepted.load(Ordering::SeqCst), expected);
        }
    }
    server.abort();
}
