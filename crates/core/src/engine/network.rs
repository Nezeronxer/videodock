//! Explicit settings win. Automatic proxies come from configuration or a verified VPN-owned socket.
#[cfg(any(target_os = "linux", target_os = "macos", test))]
use anyhow::Context;
use anyhow::{Result, bail};
use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[cfg(windows)]
#[path = "network/windows.rs"]
mod windows;

#[derive(Default)]
pub(super) struct Cache(Option<(Instant, Option<String>)>);
impl Cache {
    pub(super) fn proxy(&mut self) -> Result<Option<String>> {
        if let Some((at, proxy)) = &self.0
            && at.elapsed() < Duration::from_secs(5)
        {
            return Ok(proxy.clone());
        }
        let proxy = discover()?;
        self.0 = Some((Instant::now(), proxy.clone()));
        Ok(proxy)
    }
}

fn normalized(value: &str, scheme: &str) -> Result<String> {
    let value = value.trim();
    let address = if value.contains("://") {
        value.to_owned()
    } else {
        format!("{scheme}://{value}")
    };
    let mut url =
        url::Url::parse(&address).map_err(|_| anyhow::anyhow!("Некорректный системный прокси"))?;
    if !matches!(
        url.scheme(),
        "http" | "https" | "socks4" | "socks4a" | "socks5" | "socks5h"
    ) || url.host_str().is_none()
        || url.port_or_known_default().is_none_or(|port| port == 0)
        || url.path() != "/" && !url.path().is_empty()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("Неподдерживаемый системный прокси");
    }
    // Resolve SOCKS names at the VPN endpoint, as in the manual proxy setting.
    if url.scheme() == "socks5" {
        let _ = url.set_scheme("socks5h");
    }
    Ok(url.to_string())
}
fn environment(mut get: impl FnMut(&str) -> Option<String>) -> Result<Option<String>> {
    for key in [
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
        "http_proxy",
        "HTTP_PROXY",
    ] {
        if let Some(value) = get(key).filter(|value| !value.trim().is_empty()) {
            return normalized(&value, "http").map(Some);
        }
    }
    Ok(None)
}
#[cfg(any(windows, test))]
fn windows_list(value: &str) -> Result<Option<String>> {
    if !value.contains('=') || url::Url::parse(value).is_ok_and(|url| url.host_str().is_some()) {
        return normalized(value, "http").map(Some);
    }
    let entries = value
        .split(|ch: char| ch == ';' || ch.is_whitespace())
        .filter_map(|entry| entry.trim().split_once('='))
        .collect::<Vec<_>>();
    for key in ["https", "socks", "http"] {
        if let Some((_, address)) = entries
            .iter()
            .find(|(kind, _)| kind.eq_ignore_ascii_case(key))
        {
            return normalized(address, if key == "socks" { "socks5h" } else { "http" }).map(Some);
        }
    }
    bail!("Системный прокси не поддерживает HTTP/HTTPS или SOCKS")
}
fn discover() -> Result<Option<String>> {
    if let Some(proxy) = environment(|key| std::env::var(key).ok())? {
        return Ok(Some(proxy));
    }
    #[cfg(windows)]
    if let Some(proxy) = windows::static_proxy().map_err(|_| {
        anyhow::anyhow!(
            "Системный PAC-прокси пока не поддерживается. Укажите прокси в настройках VideoDock."
        )
    })? {
        return windows_list(&proxy);
    }
    #[cfg(target_os = "macos")]
    if let Some(output) = output("scutil", &["--proxy"]) {
        if let Some(proxy) = macos_proxy(&output)? {
            return Ok(Some(proxy));
        }
    }
    #[cfg(target_os = "linux")]
    if let Some(proxy) = gnome_proxy()? {
        return Ok(Some(proxy));
    }
    // shortcut: explicit PAC and unconfigured HTTP-only VPN endpoints need manual proxy settings.
    // A TUN interface and zapret require no application proxy: the OS routes direct sockets.
    Ok(vpn_listeners()
        .into_iter()
        .take(16)
        .find_map(|address| verified_socks(address).then(|| format!("socks5h://{address}"))))
}
fn output(program: &str, arguments: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut bytes = Vec::new();
                child
                    .stdout
                    .take()?
                    .take(128 * 1024)
                    .read_to_end(&mut bytes)
                    .ok()?;
                return String::from_utf8(bytes).ok();
            }
            Ok(None) if start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}
#[cfg(target_os = "linux")]
fn gnome_proxy() -> Result<Option<String>> {
    let get = |schema: &str, key: &str| {
        output("gsettings", &["get", schema, key]).map(|s| s.trim().trim_matches('\'').to_owned())
    };
    match get("org.gnome.system.proxy", "mode").as_deref() {
        Some("manual") => {}
        Some("auto") => bail!(
            "Системный PAC-прокси пока не поддерживается. Укажите прокси в настройках VideoDock."
        ),
        _ => return Ok(None),
    }
    for (kind, scheme) in [("https", "http"), ("socks", "socks5h"), ("http", "http")] {
        let schema = format!("org.gnome.system.proxy.{kind}");
        if let Some(host) = get(&schema, "host").filter(|host| !host.is_empty()) {
            let port = get(&schema, "port").context("Не указан порт системного прокси")?;
            let host = if host.parse::<std::net::Ipv6Addr>().is_ok() {
                format!("[{host}]")
            } else {
                host
            };
            return normalized(&format!("{host}:{port}"), scheme).map(Some);
        }
    }
    Ok(None)
}
#[cfg(any(target_os = "macos", test))]
fn macos_proxy(value: &str) -> Result<Option<String>> {
    let entries = value
        .lines()
        .filter_map(|line| line.trim().split_once(" : "))
        .collect::<Vec<_>>();
    let get = |key: &str| {
        entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, value)| *value)
    };
    for (prefix, scheme) in [("HTTPS", "http"), ("SOCKS", "socks5h"), ("HTTP", "http")] {
        if get(&format!("{prefix}Enable")) == Some("1") {
            let host =
                get(&format!("{prefix}Proxy")).context("Не указан сервер системного прокси")?;
            let port = get(&format!("{prefix}Port")).context("Не указан порт системного прокси")?;
            let host = if host.parse::<std::net::Ipv6Addr>().is_ok() {
                format!("[{host}]")
            } else {
                host.to_owned()
            };
            return normalized(&format!("{host}:{port}"), scheme).map(Some);
        }
    }
    if get("ProxyAutoConfigEnable") == Some("1") {
        bail!(
            "Системный PAC-прокси пока не поддерживается. Укажите прокси в настройках VideoDock."
        );
    }
    Ok(None)
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
const VPN_NAMES: &[&str] = &[
    "mihomo",
    "clash",
    "clash-meta",
    "sing-box",
    "xray",
    "v2ray",
    "hiddify",
    "nekobox",
    "nekoray",
    "v2rayn",
    "clash-verge",
    "outline-ss-server",
    "ss-local",
    "happ",
    "throne",
    "flclash",
    "clash-nyanpasu",
];
fn loopback(value: &str) -> Option<SocketAddr> {
    let address = value.parse::<SocketAddr>().ok()?;
    if address.port() == 0 {
        return None;
    }
    if address.ip().is_loopback() {
        return Some(address);
    }
    if address.ip().is_unspecified() {
        return Some(SocketAddr::new(
            if address.is_ipv4() {
                IpAddr::from([127, 0, 0, 1])
            } else {
                IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            },
            address.port(),
        ));
    }
    None
}
fn vpn_listeners() -> Vec<SocketAddr> {
    let mut addresses = Vec::new();
    #[cfg(target_os = "linux")]
    if let Some(text) = output("ss", &["-H", "-ltnp"]) {
        for line in text.lines() {
            if VPN_NAMES
                .iter()
                .any(|name| line.to_ascii_lowercase().contains(&format!("\"{name}\"")))
                && let Some(address) = line.split_whitespace().nth(3).and_then(loopback)
            {
                addresses.push(address);
            }
        }
    }
    #[cfg(windows)]
    if let Some(text) = output(
        "powershell.exe",
        &[
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$p=@{}; Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -in @('mihomo','clash','clash-meta','sing-box','xray','v2ray','hiddify','nekobox','nekoray','v2rayN','clash-verge','ss-local','happ','throne','flclash','clash-nyanpasu') } | ForEach-Object {$p[$_.Id]=1}; if($p.Count){Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object {$p.ContainsKey([int]$_.OwningProcess)} | ForEach-Object {if($_.LocalAddress -like '*:*'){ '['+$_.LocalAddress+']:'+$_.LocalPort }else{$_.LocalAddress+':'+$_.LocalPort}}}",
        ],
    ) {
        addresses.extend(text.lines().filter_map(|line| loopback(line.trim())));
    }
    #[cfg(target_os = "macos")]
    if let Some(text) = output("lsof", &["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"]) {
        let mut known = false;
        for line in text.lines() {
            if let Some(name) = line.strip_prefix('c') {
                known = VPN_NAMES.contains(&name.to_ascii_lowercase().as_str());
            }
            if known && let Some(address) = line.strip_prefix('n').and_then(loopback) {
                addresses.push(address);
            }
        }
    }
    addresses.sort_unstable();
    addresses.dedup();
    addresses
}
fn verified_socks(address: SocketAddr) -> bool {
    let result = (|| -> std::io::Result<bool> {
        let mut socket = TcpStream::connect_timeout(&address, Duration::from_millis(200))?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        socket.set_write_timeout(Some(Duration::from_millis(200)))?;
        socket.write_all(&[5, 1, 0])?;
        let mut reply = [0; 2];
        socket.read_exact(&mut reply)?;
        // A credential-protected endpoint needs explicit settings; never bypass its authentication.
        Ok(reply == [5, 0])
    })();
    result.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(target_os = "linux", windows, target_os = "macos"))]
    #[test]
    fn process_owned_vpn_socket_is_discovered_without_scanning_ports() {
        const CHILD: &str = "VIDEODOCK_VPN_FIXTURE_READY";
        if let Some(ready) = std::env::var_os(CHILD) {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            std::fs::write(ready, listener.local_addr().unwrap().to_string()).unwrap();
            let started = Instant::now();
            loop {
                if let Ok((mut socket, _)) = listener.accept() {
                    socket
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut greeting = [0; 3];
                    socket.read_exact(&mut greeting).unwrap();
                    assert_eq!(greeting, [5, 1, 0]);
                    socket.write_all(&[5, 0]).unwrap();
                    return;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(10),
                    "fixture handshake deadline"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let directory = tempfile::tempdir().unwrap();
        for known in [false, true] {
            let filename = if known { "mihomo" } else { "unrelated" };
            let program = directory
                .path()
                .join(format!("{filename}{}", std::env::consts::EXE_SUFFIX));
            std::fs::copy(std::env::current_exe().unwrap(), &program).unwrap();
            let ready = directory.path().join(format!("ready-{known}"));
            let mut child = Command::new(program)
                .args(["process_owned_vpn_socket_is_discovered_without_scanning_ports"])
                .env(CHILD, &ready)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let started = Instant::now();
            while !ready.exists() && started.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(10));
            }
            let address = std::fs::read_to_string(&ready)
                .ok()
                .and_then(|text| text.parse().ok());
            let discovered = address.is_some_and(|address| vpn_listeners().contains(&address));
            let handshake = discovered && address.is_some_and(verified_socks);
            if !handshake {
                let _ = child.kill();
            }
            let _ = child.wait();
            assert!(address.is_some(), "native process fixture did not start");
            assert_eq!(
                discovered, known,
                "process ownership detection is incorrect"
            );
            assert_eq!(
                handshake, known,
                "discovered endpoint failed protocol verification"
            );
        }
    }

    #[test]
    fn environment_priority_and_errors_do_not_expose_credentials() {
        let mut values = std::collections::HashMap::from([
            ("HTTPS_PROXY", "http://test:synthetic@127.0.0.1:1080"),
            ("ALL_PROXY", "socks5://127.0.0.1:1081"),
        ]);
        assert_eq!(
            environment(|key| values.get(key).map(|s| s.to_string()))
                .unwrap()
                .unwrap(),
            "http://test:synthetic@127.0.0.1:1080/"
        );
        values.insert("HTTPS_PROXY", "file://test:synthetic@localhost/private");
        let error = environment(|key| values.get(key).map(|s| s.to_string()))
            .unwrap_err()
            .to_string();
        assert!(!error.contains("synthetic"));
        assert!(environment(|_| None).unwrap().is_none());
    }
    #[test]
    fn static_os_protocol_lists_select_https_then_socks() {
        assert_eq!(
            windows_list("http=127.0.0.1:80;https=127.0.0.1:81;socks=127.0.0.1:82")
                .unwrap()
                .unwrap(),
            "http://127.0.0.1:81/"
        );
        assert_eq!(
            windows_list("socks=[::1]:1080").unwrap().unwrap(),
            "socks5h://[::1]:1080"
        );
        assert!(windows_list("ftp=127.0.0.1:80").is_err());
        assert_eq!(macos_proxy("<dictionary> {\n HTTPSEnable : 0\n HTTPEnable : 1\n HTTPProxy : 127.0.0.1\n HTTPPort : 7890\n}").unwrap().unwrap(), "http://127.0.0.1:7890/");
        assert!(
            macos_proxy("HTTPEnable : 0\nHTTPProxy : 127.0.0.1")
                .unwrap()
                .is_none()
        );
        assert!(macos_proxy("ProxyAutoConfigEnable : 1").is_err());
        assert!(normalized("socks5://127.0.0.1:0", "http").is_err());
        assert_eq!(
            windows_list("https=127.0.0.1:81 socks=127.0.0.1:82")
                .unwrap()
                .unwrap(),
            "http://127.0.0.1:81/"
        );
        assert_eq!(
            windows_list("http://test:a=b@127.0.0.1:81")
                .unwrap()
                .unwrap(),
            "http://test:a%3Db@127.0.0.1:81/"
        );
    }
    #[test]
    fn auto_detection_never_probes_nonlocal_addresses() {
        assert!(loopback("192.0.2.1:1080").is_none());
        assert!(loopback("localhost:1080").is_none());
        assert_eq!(
            loopback("0.0.0.0:1080").unwrap().to_string(),
            "127.0.0.1:1080"
        );
        assert_eq!(loopback("[::]:1080").unwrap().to_string(), "[::1]:1080");
    }
    #[test]
    fn a_listening_port_is_not_a_proxy_and_auth_is_not_bypassed() {
        for response in [[5, 0], [5, 2], [5, 255], [72, 84]] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let thread = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 3];
                socket.read_exact(&mut request).unwrap();
                assert_eq!(request, [5, 1, 0]);
                socket.write_all(&response).unwrap();
            });
            assert_eq!(verified_socks(address), response == [5, 0]);
            thread.join().unwrap();
        }
    }
}
