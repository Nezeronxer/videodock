//! OS DNS first; a bounded encrypted fallback only for public YouTube infrastructure.
use serde::Deserialize;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

const ENDPOINT: &str = "https://cloudflare-dns.com/dns-query";
const BODY_LIMIT: usize = 32 * 1024;
const ANONYMOUS_HOSTS: [&str; 2] = ["www.youtube.com", "www.google.com"];
const DNS_TIMEOUT: Duration = Duration::from_secs(3);
const DOH_TIMEOUT: Duration = Duration::from_secs(5);
// shortcut: fallback is IPv4-only; add AAAA/bootstrap after native IPv6 path verification.

#[derive(Clone)]
pub(crate) struct Resolver {
    doh: Option<reqwest::Client>,
    endpoint: String,
}

pub(crate) fn resolver(direct: bool) -> anyhow::Result<Arc<Resolver>> {
    // Proxies resolve their targets remotely; never make an out-of-proxy DNS request.
    let doh = if direct {
        Some(
            reqwest::Client::builder()
                .no_proxy()
                .http1_only()
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(2))
                .timeout(DOH_TIMEOUT)
                // Bootstrap HTTPS without asking the failing system resolver for the DoH host.
                .resolve("cloudflare-dns.com", SocketAddr::from(([1, 1, 1, 1], 443)))
                .build()
                .map_err(|_| anyhow::anyhow!("Не удалось создать защищённый DNS-клиент"))?,
        )
    } else {
        None
    };
    Ok(Arc::new(Resolver {
        doh,
        endpoint: ENDPOINT.into(),
    }))
}
/// Resolver overrides for the anonymous Qt worker; a selected proxy always owns DNS.
pub async fn anonymous_dns_bootstrap(
    proxy_configured: bool,
) -> anyhow::Result<Vec<(String, IpAddr)>> {
    if proxy_configured {
        return Ok(Vec::new());
    }
    let dns = resolver(true)?;
    let (youtube, google) = tokio::join!(
        async {
            dns.bootstrap_after_system(ANONYMOUS_HOSTS[0], system(ANONYMOUS_HOSTS[0]).await)
                .await
        },
        async {
            dns.bootstrap_after_system(ANONYMOUS_HOSTS[1], system(ANONYMOUS_HOSTS[1]).await)
                .await
        },
    );
    [youtube, google]
        .into_iter()
        .collect::<io::Result<Vec<_>>>()
        .map(|values| values.into_iter().flatten().collect())
        .map_err(|_| anyhow::anyhow!("Не удалось подготовить DNS анонимного подтверждения"))
}
async fn system(host: &str) -> io::Result<Vec<SocketAddr>> {
    tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host((host, 0)))
        .await
        .map_err(|_| failure())
        .and_then(|result| result.map(|values| values.collect()))
}
fn failure() -> io::Error {
    io::Error::other("Не удалось безопасно разрешить сетевой адрес")
}
fn normalized(host: &str) -> String {
    host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase()
}
fn eligible(host: &str) -> bool {
    let host = normalized(host);
    if host.len() > 253
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
    {
        return false;
    }
    [
        "youtube.com",
        "youtube-nocookie.com",
        "youtu.be",
        "googlevideo.com",
        "ytimg.com",
        "ggpht.com",
        "googleusercontent.com",
        "gstatic.com",
    ]
    .iter()
    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        || matches!(
            host.as_str(),
            "www.google.com"
                | "accounts.google.com"
                | "waa-pa.clients6.google.com"
                | "youtubei.googleapis.com"
                | "youtube.googleapis.com"
                | "youtubeembeddedplayer.googleapis.com"
                | "jnn-pa.googleapis.com"
        )
}
fn public(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_documentation()
        && a != 0
        && a < 224
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 192 && b == 0 && c == 0)
        && !(a == 192 && b == 88 && c == 99)
        && !(a == 198 && (18..=19).contains(&b))
}
#[derive(Deserialize)]
struct Question {
    name: String,
    #[serde(rename = "type")]
    kind: u16,
}
#[derive(Deserialize)]
struct Answer {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}
#[derive(Deserialize)]
struct Response {
    #[serde(rename = "Status")]
    status: u32,
    #[serde(rename = "TC", default)]
    truncated: bool,
    #[serde(rename = "Question")]
    questions: Vec<Question>,
    #[serde(rename = "Answer", default)]
    answers: Vec<Answer>,
}
fn parse(bytes: &[u8], host: &str) -> io::Result<Vec<SocketAddr>> {
    if bytes.len() > BODY_LIMIT {
        return Err(failure());
    }
    let response: Response = serde_json::from_slice(bytes).map_err(|_| failure())?;
    if response.status != 0
        || response.truncated
        || response.questions.len() != 1
        || response.questions[0].kind != 1
        || normalized(&response.questions[0].name) != normalized(host)
        || response.answers.len() > 64
    {
        return Err(failure());
    }
    let mut addresses = Vec::new();
    for answer in response
        .answers
        .into_iter()
        .filter(|answer| answer.kind == 1)
    {
        let ip: Ipv4Addr = answer.data.parse().map_err(|_| failure())?;
        if !public(ip) {
            return Err(failure());
        }
        let address = SocketAddr::new(IpAddr::V4(ip), 0);
        if !addresses.contains(&address) {
            addresses.push(address);
        }
        if addresses.len() > 16 {
            return Err(failure());
        }
    }
    if addresses.is_empty() {
        return Err(failure());
    }
    Ok(addresses)
}
impl Resolver {
    async fn bootstrap_after_system(
        &self,
        host: &'static str,
        original: io::Result<Vec<SocketAddr>>,
    ) -> io::Result<Option<(String, IpAddr)>> {
        if !ANONYMOUS_HOSTS.contains(&host) {
            return Err(failure());
        }
        if original.as_ref().is_ok_and(|values| !values.is_empty()) {
            return Ok(None);
        }
        self.after_system(host, original).await.map(|values| {
            values
                .first()
                .map(|address| (host.to_owned(), address.ip()))
        })
    }
    async fn after_system(
        &self,
        host: &str,
        system: io::Result<Vec<SocketAddr>>,
    ) -> io::Result<Vec<SocketAddr>> {
        // Keep trusted OS mappings, including a VPN's fake-IP route and local fixtures.
        if let Ok(addresses) = system
            && !addresses.is_empty()
        {
            return Ok(addresses);
        }
        let Some(client) = &self.doh else {
            return Err(failure());
        };
        if !eligible(host) {
            return Err(failure());
        }
        let request = async {
            let mut response = client
                .get(&self.endpoint)
                .header(reqwest::header::ACCEPT, "application/dns-json")
                .query(&[("name", normalized(host)), ("type", "A".into())])
                .send()
                .await
                .map_err(|error| {
                    #[cfg(test)]
                    eprintln!(
                        "DoH send failure: timeout={} connect={}",
                        error.is_timeout(),
                        error.is_connect()
                    );
                    #[cfg(test)]
                    eprintln!("DoH send source: {:?}", error.without_url());
                    #[cfg(not(test))]
                    let _ = error;
                    failure()
                })?;
            let mime = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|header| header.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim();
            if !response.status().is_success()
                || !matches!(mime, "application/dns-json" | "application/json")
                || response
                    .content_length()
                    .is_some_and(|length| length > BODY_LIMIT as u64)
            {
                return Err(failure());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|error| {
                #[cfg(test)]
                eprintln!(
                    "DoH body failure: timeout={} body={}",
                    error.is_timeout(),
                    error.is_body()
                );
                let _ = error;
                failure()
            })? {
                if chunk.len() > BODY_LIMIT - bytes.len() {
                    return Err(failure());
                }
                bytes.extend_from_slice(&chunk);
            }
            let result = parse(&bytes, host);
            #[cfg(test)]
            if result.is_err() {
                eprintln!("DoH parse failure: bytes={}", bytes.len());
            }
            result
        };
        tokio::time::timeout(DOH_TIMEOUT, request)
            .await
            .map_err(|_| failure())?
    }
    fn future(&self, host: String) -> reqwest::dns::Resolving {
        let this = self.clone();
        Box::pin(async move {
            let system = system(&host).await;
            let values = this.after_system(&host, system).await?;
            Ok(Box::new(values.into_iter()) as reqwest::dns::Addrs)
        })
    }
}
impl reqwest::dns::Resolve for Resolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        self.future(name.as_str().to_owned())
    }
}
impl wreq::dns::Resolve for Resolver {
    fn resolve(&self, name: wreq::dns::Name) -> wreq::dns::Resolving {
        self.future(name.as_str().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::atomic::{AtomicUsize, Ordering},
    };
    fn answer(ip: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({"Status":0,"Question":[{"name":"www.youtube.com","type":1}],"Answer":[{"type":1,"data":ip}]})).unwrap()
    }
    fn fixture(
        status: &str,
        mime: &str,
        body: Vec<u8>,
        declared: Option<usize>,
    ) -> (Resolver, Arc<AtomicUsize>, std::thread::JoinHandle<()>) {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(DOH_TIMEOUT)
            .build()
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let reads = count.clone();
        let status = status.to_owned();
        let mime = mime.to_owned();
        let thread = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            while start.elapsed() < DOH_TIMEOUT + Duration::from_secs(1) {
                if let Ok((mut socket, _)) = listener.accept() {
                    // Windows/BSD accept inherits the listener's nonblocking mode.
                    socket.set_nonblocking(false).unwrap();
                    reads.fetch_add(1, Ordering::SeqCst);
                    socket.set_read_timeout(Some(DOH_TIMEOUT)).unwrap();
                    let mut request = Vec::new();
                    let mut byte = [0];
                    while !request.ends_with(b"\r\n\r\n") && request.len() < 16384 {
                        if socket.read_exact(&mut byte).is_err() {
                            return;
                        }
                        request.push(byte[0]);
                    }
                    assert!(
                        String::from_utf8_lossy(&request)
                            .starts_with("GET /dns-query?name=www.youtube.com&type=A ")
                    );
                    let length = declared
                        .map(|value| format!("Content-Length: {value}\r\n"))
                        .unwrap_or_default();
                    let headers = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\n{length}Connection: close\r\n\r\n"
                    );
                    let response = [headers.as_bytes(), body.as_slice()].concat();
                    socket.write_all(&response).ok();
                    socket.shutdown(std::net::Shutdown::Write).ok();
                    socket.read_exact(&mut [0]).ok();
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        (
            Resolver {
                doh: Some(client),
                endpoint: format!("http://{address}/dns-query"),
            },
            count,
            thread,
        )
    }
    #[test]
    fn scope_and_private_answers_cannot_expand_to_local_networks() {
        for host in [
            "www.youtube.com",
            "r1.googlevideo.com",
            "jnn-pa.googleapis.com",
            "www.google.com",
            "WWW.YOUTUBE.COM.",
        ] {
            assert!(eligible(host));
        }
        for host in [
            "localhost",
            "router.local",
            "192.168.1.1",
            "youtube.com.evil.invalid",
            "evil-youtube.com",
            "www.youtube.com..",
            "internal.google.com",
            "private-proxy.googleapis.com",
            "tenant.storage.googleapis.com",
            "www.youtube.com/private",
        ] {
            assert!(!eligible(host));
        }
        for ip in [
            "0.0.0.0",
            "10.1.2.3",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.2",
            "172.16.0.1",
            "192.168.1.1",
            "198.18.0.1",
            "192.0.2.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::1",
            "::ffff:127.0.0.1",
            "not-an-address",
        ] {
            assert!(parse(&answer(ip), "www.youtube.com").is_err());
        }
        assert_eq!(
            parse(&answer("142.250.74.206"), "www.youtube.com")
                .unwrap()
                .len(),
            1
        );
    }
    #[tokio::test]
    async fn normal_system_dns_and_both_http_resolver_traits_remain_available() {
        let resolver = resolver(false).unwrap();
        let a = reqwest::dns::Resolve::resolve(resolver.as_ref(), "localhost".parse().unwrap())
            .await
            .unwrap()
            .collect::<Vec<_>>();
        let b = wreq::dns::Resolve::resolve(resolver.as_ref(), wreq::dns::Name::from("localhost"))
            .await
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!a.is_empty() && !b.is_empty());
        assert!(a.iter().all(|address| address.ip().is_loopback()));
        let saved = vec![SocketAddr::from(([198, 18, 0, 1], 0))];
        assert_eq!(
            resolver
                .after_system("www.youtube.com", Ok(saved.clone()))
                .await
                .unwrap(),
            saved
        );
        assert!(
            resolver
                .after_system("www.youtube.com", Err(failure()))
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn both_http_clients_accept_the_shared_resolver_and_preserve_ports() {
        let dns = resolver(false).unwrap();
        let (server, count, thread) = fixture("200 OK", "text/plain", b"adapter-ok".to_vec(), None);
        let target =
            server.endpoint.replace("127.0.0.1", "localhost") + "?name=www.youtube.com&type=A";
        let http = reqwest::Client::builder()
            .no_proxy()
            .dns_resolver(dns.clone())
            .timeout(DOH_TIMEOUT)
            .build()
            .unwrap();
        assert_eq!(
            http.get(target)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .as_ref(),
            b"adapter-ok"
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        let (server, count, thread) = fixture("200 OK", "text/plain", b"adapter-ok".to_vec(), None);
        let target =
            server.endpoint.replace("127.0.0.1", "localhost") + "?name=www.youtube.com&type=A";
        let http = wreq::Client::builder()
            .no_proxy()
            .dns_resolver(dns)
            .timeout(DOH_TIMEOUT)
            .build()
            .unwrap();
        assert_eq!(
            http.get(target)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .as_ref(),
            b"adapter-ok"
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn fixture_waits_for_request_headers_after_accept() {
        let body = answer("142.250.74.206");
        let (resolver, count, thread) =
            fixture("200 OK", "application/dns-json", body.clone(), None);
        let endpoint = url::Url::parse(&resolver.endpoint).unwrap();
        let mut client =
            std::net::TcpStream::connect(("127.0.0.1", endpoint.port().unwrap())).unwrap();
        client.set_read_timeout(Some(DOH_TIMEOUT)).unwrap();
        let deadline = std::time::Instant::now();
        while count.load(Ordering::SeqCst) == 0 && deadline.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        std::thread::sleep(Duration::from_millis(30));
        client.write_all(b"GET /dns-query?name=www.youtube.com&type=A HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        drop(client);
        thread.join().unwrap();
        assert!(
            response.ends_with(&body),
            "Fixture closed before delivering the response"
        );
    }
    #[tokio::test]
    async fn fixture_outlives_delayed_client_start() {
        let (resolver, count, thread) = fixture(
            "200 OK",
            "application/dns-json",
            answer("142.250.74.206"),
            None,
        );
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(
            resolver
                .bootstrap_after_system("www.youtube.com", Err(failure()))
                .await
                .unwrap()
                .is_some()
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn anonymous_worker_bootstrap_is_pinned_and_never_bypasses_a_proxy() {
        assert!(anonymous_dns_bootstrap(true).await.unwrap().is_empty());
        let (resolver, count, thread) = fixture(
            "200 OK",
            "application/dns-json",
            answer("142.250.74.206"),
            None,
        );
        let trusted_os = vec![SocketAddr::from(([198, 18, 0, 1], 0))];
        assert!(
            resolver
                .bootstrap_after_system("www.youtube.com", Ok(trusted_os))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            resolver
                .bootstrap_after_system("m.youtube.com", Err(failure()))
                .await
                .is_err()
        );
        let result = resolver
            .bootstrap_after_system("www.youtube.com", Err(failure()))
            .await;
        assert!(
            result.is_ok(),
            "Bootstrap fixture requests={}",
            count.load(Ordering::SeqCst)
        );
        assert_eq!(
            result.unwrap(),
            Some(("www.youtube.com".into(), IpAddr::from([142, 250, 74, 206])))
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn failed_system_dns_uses_bounded_local_doh_fixture() {
        let (resolver, count, thread) = fixture(
            "200 OK",
            "application/dns-json",
            answer("142.250.74.206"),
            None,
        );
        assert_eq!(
            resolver
                .after_system("www.youtube.com", Err(failure()))
                .await
                .unwrap(),
            vec![SocketAddr::from(([142, 250, 74, 206], 0))]
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn disallowed_names_and_disabled_fallback_never_contact_doh() {
        let (mut resolver, count, thread) = fixture(
            "200 OK",
            "application/dns-json",
            answer("142.250.74.206"),
            None,
        );
        let trusted_os_mapping = vec![SocketAddr::from(([198, 18, 0, 1], 0))];
        assert_eq!(
            resolver
                .after_system("www.youtube.com", Ok(trusted_os_mapping.clone()))
                .await
                .unwrap(),
            trusted_os_mapping
        );
        assert!(
            resolver
                .after_system("private-proxy.invalid", Err(failure()))
                .await
                .is_err()
        );
        resolver.doh = None;
        assert!(
            resolver
                .after_system("www.youtube.com", Err(failure()))
                .await
                .is_err()
        );
        thread.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn invalid_status_content_length_and_private_json_are_redacted() {
        for (status, mime, body, declared) in [
            (
                "403 Forbidden",
                "application/dns-json",
                answer("142.250.74.206"),
                None,
            ),
            (
                "200 OK",
                "text/html",
                b"private synthetic fixture".to_vec(),
                None,
            ),
            ("200 OK", "application/dns-json", answer("127.0.0.1"), None),
            (
                "200 OK",
                "application/dns-json",
                answer("142.250.74.206"),
                Some(BODY_LIMIT + 1),
            ),
            (
                "200 OK",
                "application/dns-json",
                vec![b' '; BODY_LIMIT + 1],
                None,
            ),
        ] {
            let (resolver, _, thread) = fixture(status, mime, body, declared);
            let error = resolver
                .after_system("www.youtube.com", Err(failure()))
                .await
                .unwrap_err()
                .to_string();
            assert!(
                !error.contains("youtube")
                    && !error.contains("127.0.0.1")
                    && !error.contains("synthetic")
            );
            thread.join().unwrap();
        }
        assert!(parse(b"{}", "www.youtube.com").is_err());
        assert!(
            parse(
                &serde_json::to_vec(
                    &json!({"Status":3,"Question":[{"name":"www.youtube.com","type":1}]})
                )
                .unwrap(),
                "www.youtube.com"
            )
            .is_err()
        );
        assert!(parse(&answer("142.250.74.206"), "different.youtube.com").is_err());
        let mut truncated: serde_json::Value =
            serde_json::from_slice(&answer("142.250.74.206")).unwrap();
        truncated["TC"] = json!(true);
        assert!(parse(&serde_json::to_vec(&truncated).unwrap(), "www.youtube.com").is_err());
        let mut records: serde_json::Value =
            serde_json::from_slice(&answer("142.250.74.206")).unwrap();
        records["Answer"] = json!(
            (1..=17)
                .map(|last| json!({"type":1,"data":format!("142.250.74.{last}")}))
                .collect::<Vec<_>>()
        );
        assert!(parse(&serde_json::to_vec(&records).unwrap(), "www.youtube.com").is_err());
        records["Answer"] = json!(
            (0..65)
                .map(|_| json!({"type":5,"data":"public.youtube.com"}))
                .collect::<Vec<_>>()
        );
        assert!(parse(&serde_json::to_vec(&records).unwrap(), "www.youtube.com").is_err());
    }
}
