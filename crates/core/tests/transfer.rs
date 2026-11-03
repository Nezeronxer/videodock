use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use videodock_core::transfer::{Limiter, download};

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
