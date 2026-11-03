use std::{collections::BTreeMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use videodock_core::transfer::{Limiter, download};

#[tokio::test]
async fn manual_redirect_keeps_public_headers_and_strips_credentials() {
    let source = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/source", source.local_addr().unwrap());
    let location = format!("http://{}/media", target.local_addr().unwrap());
    let first = tokio::spawn(async move {
        let (mut stream, _) = source.accept().await.unwrap();
        let mut input = [0; 4096];
        let n = stream.read(&mut input).await.unwrap();
        assert!(
            String::from_utf8_lossy(&input[..n])
                .to_lowercase()
                .contains("cookie: synthetic=value")
        );
        stream.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    });
    let second = tokio::spawn(async move {
        let (mut stream, _) = target.accept().await.unwrap();
        let mut input = [0; 4096];
        let n = stream.read(&mut input).await.unwrap();
        let text = String::from_utf8_lossy(&input[..n]).to_lowercase();
        assert!(!text.contains("cookie:") && !text.contains("authorization:"));
        assert!(text.contains("user-agent: videodock-test"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncontent")
            .await
            .unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.part");
    let headers = BTreeMap::from([
        ("Cookie".into(), "synthetic=value".into()),
        ("Authorization".into(), "Bearer synthetic".into()),
        ("User-Agent".into(), "VideoDock-test".into()),
    ]);
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    download(
        &client,
        &url,
        &headers,
        &path,
        &CancellationToken::new(),
        &Limiter::new(0),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"content");
    first.await.unwrap();
    second.await.unwrap();
}
