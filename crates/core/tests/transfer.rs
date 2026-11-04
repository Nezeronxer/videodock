use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use videodock_core::transfer::{Limiter, download};

#[tokio::test]
async fn media_retries_are_bounded_and_do_not_retry_access_denials() {
    for (status, expected_requests) in [(403, 1), (503, 3)] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/media", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..expected_requests {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                assert!(socket.read(&mut buffer).await.unwrap() > 0);
                socket.write_all(format!("HTTP/1.1 {status} Unavailable\r\nContent-Length: 0\r\nRetry-After: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
            }
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                    .await
                    .is_err()
            );
        });
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("media.part");
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            download(
                &reqwest::Client::new(),
                &url,
                &BTreeMap::new(),
                &path,
                &CancellationToken::new(),
                &Limiter::new(0),
                Arc::new(|_| {}),
            ),
        )
        .await
        .unwrap();
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains(&status.to_string())
        );
        assert!(!path.exists(), "an error page was saved as media");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn temporary_media_server_errors_are_retried_before_download_fails() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = requests.clone();
    let server = tokio::spawn(async move {
        for status in [503, 429, 200] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            assert!(socket.read(&mut buffer).await.unwrap() > 0);
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let body = if status == 200 { "abcdef" } else { "" };
            socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nRetry-After: 0\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    download(
        &reqwest::Client::new(),
        &url,
        &BTreeMap::new(),
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"abcdef");
    assert_eq!(requests.load(std::sync::atomic::Ordering::Relaxed), 3);
    server.await.unwrap();
}

#[tokio::test]
async fn cancellation_interrupts_retry_after_without_another_request() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        assert!(socket.read(&mut buffer).await.unwrap() > 0);
        socket.write_all(b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nRetry-After: 30\r\nConnection: close\r\n\r\n").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        trigger.cancel();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    let temp = tempfile::tempdir().unwrap();
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        download(
            &reqwest::Client::new(),
            &url,
            &BTreeMap::new(),
            &temp.path().join("media.part"),
            &cancel,
            &Limiter::new(0),
            Arc::new(|_| {}),
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains("приостановлена"), "{error}");
    server.await.unwrap();
}

#[tokio::test]
async fn range_resume_never_appends_a_full_200_response() {
    let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/video", socket.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = socket.accept().await.unwrap();
        let mut request = [0; 4096];
        let n = stream.read(&mut request).await.unwrap();
        assert!(
            String::from_utf8_lossy(&request[..n])
                .to_lowercase()
                .contains("range: bytes=3-")
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef")
            .await
            .unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    tokio::fs::write(&path, b"abc").await.unwrap();
    download(
        &reqwest::Client::new(),
        &url,
        &BTreeMap::new(),
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"abcdef");
    server.await.unwrap();
}

#[tokio::test]
async fn mismatching_content_range_is_rejected_before_writing() {
    let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/video", socket.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = socket.accept().await.unwrap();
        let mut req = [0; 4096];
        assert!(stream.read(&mut req).await.unwrap() > 0);
        stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 1-5/6\r\nContent-Length: 5\r\nConnection: close\r\n\r\nbcdef").await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    tokio::fs::write(&path, b"abc").await.unwrap();
    assert!(
        download(
            &reqwest::Client::new(),
            &url,
            &BTreeMap::new(),
            &path,
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {})
        )
        .await
        .is_err()
    );
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"abc");
    server.await.unwrap();
}

#[tokio::test]
async fn resume_preserves_a_known_full_file_range_end() {
    let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/video", socket.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = socket.accept().await.unwrap();
        let mut request = [0; 4096];
        let count = stream.read(&mut request).await.unwrap();
        assert!(
            String::from_utf8_lossy(&request[..count])
                .to_lowercase()
                .contains("range: bytes=3-5")
        );
        stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 3-5/6\r\nContent-Length: 3\r\nConnection: close\r\n\r\ndef").await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    tokio::fs::write(&path, b"abc").await.unwrap();
    download(
        &reqwest::Client::new(),
        &url,
        &BTreeMap::from([("Range".into(), "bytes=0-5".into())]),
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"abcdef");
    server.await.unwrap();
}

const CHUNK_SIZE: usize = 1024 * 1024;
async fn chunk_source(
    body: Vec<u8>,
    changed_etag: bool,
    truncated: bool,
) -> (
    String,
    Arc<std::sync::Mutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = requests.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            let count = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]);
            let range = request
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("range: bytes=")
                        .map(str::to_owned)
                })
                .unwrap();
            let (start, end) = range.split_once('-').unwrap();
            let start: usize = start.parse().unwrap();
            let end = end
                .parse::<usize>()
                .unwrap_or(body.len() - 1)
                .min(body.len() - 1);
            let index = {
                let mut requests = observed.lock().unwrap();
                requests.push(range.clone());
                requests.len()
            };
            let etag = if changed_etag && index > 1 {
                "new"
            } else {
                "original"
            };
            let head = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nETag: \"{etag}\"\r\nConnection: close\r\n\r\n",
                body.len(),
                end - start + 1
            );
            if socket.write_all(head.as_bytes()).await.is_ok() {
                let actual_end = if truncated {
                    start + (end - start) / 2
                } else {
                    end
                };
                socket.write_all(&body[start..=actual_end]).await.ok();
            }
        }
    });
    (url, requests, task)
}

async fn chunked(full_end: bool, resume: bool) {
    let bytes = (0..CHUNK_SIZE * 2 + 17)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    let (url, requests, server) = chunk_source(bytes.clone(), false, false).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    let offset = if resume { 3 } else { 0 };
    if resume {
        tokio::fs::write(&path, &bytes[..offset]).await.unwrap();
    }
    let range = if full_end {
        format!("bytes=0-{}", bytes.len() - 1)
    } else {
        "bytes=0-".into()
    };
    download(
        &reqwest::Client::new(),
        &url,
        &BTreeMap::from([("Range".into(), range)]),
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for (index, range) in requests.iter().enumerate() {
        let start = offset + index * CHUNK_SIZE;
        assert_eq!(
            range,
            &format!("{start}-{}", (start + CHUNK_SIZE - 1).min(bytes.len() - 1))
        );
    }
    server.abort();
}
#[tokio::test]
async fn bounded_full_file_range_downloads_every_chunk() {
    chunked(true, false).await;
}

#[tokio::test]
async fn media_ranges_fit_the_audio_cdn_one_mib_limit() {
    let bytes = vec![7; 1024 * 1024 + 17];
    let (url, requests, server) = chunk_source(bytes.clone(), false, false).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    download(
        &reqwest::Client::new(),
        &url,
        &BTreeMap::from([("Range".into(), format!("bytes=0-{}", bytes.len() - 1))]),
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), bytes);
    for range in requests.lock().unwrap().iter() {
        let (start, end) = range.split_once('-').unwrap();
        assert!(
            end.parse::<u64>().unwrap() - start.parse::<u64>().unwrap() < 1024 * 1024,
            "audio CDN rejects ranges larger than 1 MiB: {range}"
        );
    }
    server.abort();
}
#[tokio::test]
async fn open_full_file_range_discovers_total_and_downloads_every_chunk() {
    chunked(false, false).await;
}
#[tokio::test]
async fn bounded_chunks_resume_without_repeating_existing_bytes() {
    chunked(true, true).await;
}
#[tokio::test]
async fn changed_etag_between_chunks_preserves_the_original_partial_file() {
    let bytes = vec![b'a'; CHUNK_SIZE + 17];
    let (url, requests, server) = chunk_source(bytes, true, false).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    assert!(
        download(
            &reqwest::Client::new(),
            &url,
            &BTreeMap::from([("Range".into(), "bytes=0-".into())]),
            &path,
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {})
        )
        .await
        .is_err()
    );
    assert_eq!(requests.lock().unwrap().len(), 2);
    assert_eq!(
        tokio::fs::metadata(&path).await.unwrap().len(),
        CHUNK_SIZE as u64
    );
    server.abort();
}
#[tokio::test]
async fn a_truncated_chunk_is_never_marked_complete_or_skipped() {
    let bytes = vec![b'a'; CHUNK_SIZE + 17];
    let (url, requests, server) = chunk_source(bytes, false, true).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    assert!(
        download(
            &reqwest::Client::new(),
            &url,
            &BTreeMap::from([("Range".into(), "bytes=0-".into())]),
            &path,
            &CancellationToken::new(),
            &Limiter::new(0),
            Arc::new(|_| {})
        )
        .await
        .is_err()
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert!(tokio::fs::metadata(path).await.unwrap().len() < (CHUNK_SIZE as u64));
    server.abort();
}
#[tokio::test]
async fn cancellation_between_chunks_prevents_the_next_request() {
    let bytes = vec![b'a'; CHUNK_SIZE + 17];
    let (url, requests, server) = chunk_source(bytes, false, false).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    assert!(
        download(
            &reqwest::Client::new(),
            &url,
            &BTreeMap::from([("Range".into(), "bytes=0-".into())]),
            &path,
            &cancel,
            &Limiter::new(0),
            Arc::new(move |progress| {
                if progress.downloaded >= CHUNK_SIZE as u64 {
                    trigger.cancel();
                }
            })
        )
        .await
        .is_err()
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert_eq!(
        tokio::fs::metadata(&path).await.unwrap().len(),
        CHUNK_SIZE as u64
    );
    server.abort();
}
