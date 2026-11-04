use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
use videodock_media::{Auth, catalog::Catalog};

#[tokio::test]
async fn automatic_proxy_bypasses_local_media_but_manual_proxy_remains_explicit() {
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener, time::{timeout, Duration}};
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}/video.mp4", origin.local_addr().unwrap());
    let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("http://fixture:synthetic@{}", proxy.local_addr().unwrap());
    let origin_task = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = timeout(Duration::from_secs(5), origin.accept()).await.unwrap().unwrap();
            let mut request = [0; 8192];
            let count = stream.read(&mut request).await.unwrap();
            assert!(!String::from_utf8_lossy(&request[..count]).to_lowercase().contains("proxy-authorization"));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 16\r\nConnection: close\r\n\r\n\0\0\0\x18ftypisom\0\0\0\0").await.unwrap();
        }
    });
    let proxy_task = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = timeout(Duration::from_secs(5), proxy.accept()).await.unwrap().unwrap();
            let mut request = [0; 8192];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).to_lowercase().contains("proxy-authorization"));
            stream.write_all(b"HTTP/1.1 418 Proxy marker\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        }
    });
    let automatic = Catalog::new_automatic(Some(&proxy_url)).unwrap();
    assert_eq!(automatic.inspect(&address, &Auth::default()).await.unwrap().formats.len(), 1);
    assert_eq!(automatic.client().get(&address).send().await.unwrap().status().as_u16(), 200);
    assert_eq!(automatic.client().get("http://media.example.invalid/video.mp4").send().await.unwrap().status().as_u16(), 418);
    let manual = Catalog::new(Some(&proxy_url)).unwrap();
    assert_eq!(manual.client().get(&address).send().await.unwrap().status().as_u16(), 418);
    origin_task.await.unwrap();
    proxy_task.await.unwrap();
}

#[tokio::test]
async fn direct_media_uses_its_application_identity_for_metadata_and_transfer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/video.mp4", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 8192];
        let count = stream.read(&mut buffer).unwrap();
        let headers = String::from_utf8_lossy(&buffer[..count]).to_lowercase();
        if headers.contains("user-agent: videodock/0.2\r\n") {
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 16\r\nConnection: close\r\n\r\n\0\0\0\x18ftypisom\0\0\0\0").unwrap();
        } else {
            stream
                .write_all(
                    b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        }
    });
    let media = Catalog::new(None)
        .unwrap()
        .inspect(&address, &Auth::default())
        .await
        .unwrap();
    assert_eq!(media.formats[0].headers["User-Agent"], "VideoDock/0.2");
    assert_eq!(media.formats[0].size, Some(16));
    server.join().unwrap();
}

#[tokio::test]
async fn redirect_does_not_forward_source_credentials_or_export_them() {
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_port = source.local_addr().unwrap().port();
    let target_port = target.local_addr().unwrap().port();
    let first = thread::spawn(move || {
        let (mut stream, _) = source.accept().unwrap();
        let mut data = [0; 8192];
        let count = stream.read(&mut data).unwrap();
        let request = String::from_utf8_lossy(&data[..count]).to_lowercase();
        assert!(request.contains("cookie: fixture-session=placeholder"));
        write!(stream,"HTTP/1.1 302 Found\r\nLocation: http://localhost:{target_port}/player\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let second = thread::spawn(move || {
        let (mut stream, _) = target.accept().unwrap();
        let mut data = [0; 8192];
        let count = stream.read(&mut data).unwrap();
        let request = String::from_utf8_lossy(&data[..count]).to_lowercase();
        assert!(!request.contains("cookie:"));
        let body =
            "<title>Local fixture</title><video src=\"https://media.example/video.mp4\"></video>";
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    });
    let media = Catalog::new(None)
        .unwrap()
        .inspect(
            &format!("http://127.0.0.1:{source_port}/start"),
            &Auth {
                cookie_header: "fixture-session=placeholder".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(media.title, "Local fixture");
    assert_eq!(media.formats.len(), 1);
    assert!(
        !media.formats[0]
            .headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("cookie"))
    );
    assert!(
        !serde_json::to_string(&media)
            .unwrap()
            .contains("fixture-session")
    );
    first.join().unwrap();
    second.join().unwrap();
}

#[tokio::test]
async fn closed_vine_and_invalid_proxy_are_explicit_errors() {
    assert!(Catalog::new(Some("not a valid proxy")).is_err());
    let error = Catalog::new(None)
        .unwrap()
        .inspect("https://vine.co/v/fixture", &Auth::default())
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("закрыт"));
}

#[tokio::test]
async fn binary_direct_link_is_sniffed_and_html_masquerading_as_mp4_is_rejected() {
    for (payload, accepted) in [
        (b"\0\0\0\x18ftypisom\0\0\0\0".as_slice(), true),
        (b"<html>Not media</html>".as_slice(), false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let payload = payload.to_vec();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 8192];
            assert!(stream.read(&mut request).unwrap() > 0);
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",payload.len()).unwrap();
            stream.write_all(&payload).unwrap();
        });
        let result = Catalog::new(None)
            .unwrap()
            .inspect(
                &format!("http://127.0.0.1:{port}/asset.MP4"),
                &Auth::default(),
            )
            .await;
        if accepted {
            let media = result.unwrap();
            assert_eq!(media.formats[0].ext, "mp4");
            assert_eq!(media.formats[0].size, Some(16));
        } else {
            assert!(result.is_err());
        }
        server.join().unwrap();
    }
}
