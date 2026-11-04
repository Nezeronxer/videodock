//! One-shot private IPC. This branch never opens the normal profile or Engine.
use serde_json::{json, Value};
use std::io::{Read, Write};

const LIMIT: usize = 32 * 1024;
const STAGES: &[&str] = &[
    "validation",
    "network",
    "homepage",
    "challenge",
    "interpreter",
    "snapshot",
    "integrity",
    "mint",
    "timeout",
    "internal",
];

fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, ch)| {
            if [8, 13, 18, 23].contains(&index) {
                ch == b'-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}

fn request(bytes: &[u8]) -> Option<Value> {
    if bytes.len() > LIMIT {
        return None;
    }
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let fields = value.as_object()?;
    if !matches!(fields.len(), 4 | 5)
        || !fields.contains_key("proxy")
        || fields.keys().any(|key| {
            !matches!(
                key.as_str(),
                "version" | "request_id" | "video_id" | "proxy" | "dns_bootstrap"
            )
        })
        || value["version"].as_u64() != Some(1)
        || !uuid(value["request_id"].as_str()?)
    {
        return None;
    }
    let video = value["video_id"].as_str()?;
    if video.len() != 11
        || !video
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    {
        return None;
    }
    if !value["proxy"].is_null() {
        let proxy = value["proxy"].as_str()?;
        if proxy.len() > 8192 || proxy.bytes().any(|ch| ch.is_ascii_control()) {
            return None;
        }
        let address = url::Url::parse(proxy).ok()?;
        if !matches!(address.scheme(), "http" | "socks5" | "socks5h")
            || address.host_str().is_none()
            || address.port_or_known_default().is_none_or(|port| port == 0)
            || address.scheme() != "http"
                && (!address.username().is_empty() || address.password().is_some())
            || !matches!(address.path(), "" | "/")
            || address.query().is_some()
            || address.fragment().is_some()
        {
            return None;
        }
    }
    bootstrap(&value)?;
    Some(value)
}

fn bootstrap(value: &Value) -> Option<String> {
    let Some(entries) = value.get("dns_bootstrap") else {
        return Some(String::new());
    };
    let entries = entries.as_array()?;
    if entries.len() > 2 || !value["proxy"].is_null() && !entries.is_empty() {
        return None;
    }
    let mut hosts = Vec::new();
    let mut rules = Vec::new();
    for entry in entries {
        if entry.as_object()?.len() != 2 {
            return None;
        }
        let host = entry["host"].as_str()?;
        if !matches!(host, "www.youtube.com" | "www.google.com") || hosts.contains(&host) {
            return None;
        }
        let address: std::net::Ipv4Addr = entry["address"].as_str()?.parse().ok()?;
        let [a, b, c, _] = address.octets();
        if address.is_private()
            || address.is_loopback()
            || address.is_link_local()
            || address.is_documentation()
            || address.is_broadcast()
            || a == 0
            || a >= 224
            || a == 100 && (64..=127).contains(&b)
            || a == 192 && b == 0 && c == 0
            || a == 192 && b == 88 && c == 99
            || a == 198 && (18..=19).contains(&b)
        {
            return None;
        }
        hosts.push(host);
        rules.push(format!("MAP {host} {address}"));
    }
    Some(rules.join(", "))
}

fn response(raw: &str, id: &str) -> Value {
    let mut result = json!({"version":1,"request_id":id,"ok":false,"stage":"internal"});
    if raw.len() > LIMIT {
        return result;
    }
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return result;
    };
    if value["ok"] == false {
        if let Some(stage) = value["stage"]
            .as_str()
            .filter(|stage| STAGES.contains(stage))
        {
            result["stage"] = json!(stage);
        }
    } else if value["ok"] == true {
        for (name, maximum) in [
            ("content_po", 8192),
            ("session_po", 8192),
            ("visitor_data", 4096),
            ("user_agent", 512),
        ] {
            let Some(text) = value[name].as_str().filter(|text| {
                !text.is_empty()
                    && text.len() <= maximum
                    && text.bytes().all(|ch| (32..=126).contains(&ch))
            }) else {
                return json!({"version":1,"request_id":id,"ok":false,"stage":"internal"});
            };
            result[name] = json!(text);
        }
        result["ok"] = json!(true);
        result.as_object_mut().unwrap().remove("stage");
    }
    result
}

pub fn run() {
    let mut bytes = Vec::new();
    let read = std::io::stdin()
        .lock()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes);
    let value = if read.is_ok() { request(&bytes) } else { None };
    let rejected_id = if bytes.len() <= LIMIT {
        serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|value| {
                value["request_id"]
                    .as_str()
                    .filter(|id| uuid(id))
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let reply = match value {
        Some(value) => {
            let id = value["request_id"].as_str().unwrap();
            crate::bridge::initialize_proof_runtime();
            crate::bridge::initialize_webengine();
            let _application = crate::bridge::create_proof_application(&bootstrap(&value).unwrap());
            let raw = crate::bridge::run_youtube_proof(
                value["video_id"].as_str().unwrap(),
                value["proxy"].as_str().unwrap_or(""),
                value
                    .get("dns_bootstrap")
                    .and_then(Value::as_array)
                    .and_then(|entries| {
                        entries
                            .iter()
                            .find(|entry| entry["host"] == "www.google.com")
                    })
                    .and_then(|entry| entry["address"].as_str())
                    .unwrap_or(""),
            );
            response(&raw, id)
        }
        None => json!({"version":1,"request_id":rejected_id,"ok":false,"stage":"validation"}),
    };
    let mut output = std::io::stdout().lock();
    let _ = serde_json::to_writer(&mut output, &reply);
    let _ = output.write_all(b"\n");
    let _ = output.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_schema_proxy_and_output_do_not_echo_untrusted_errors() {
        let valid = json!({"version":1,"request_id":"12345678-1234-1234-1234-123456789abc","video_id":"aqz-KE-bpKQ","proxy":null});
        assert!(request(&serde_json::to_vec(&valid).unwrap()).is_some());
        assert!(request(&vec![b' '; LIMIT + 1]).is_none());
        for host in ["127.0.0.1", "router.local", "www.youtube.com,MAP *"] {
            let mut value = valid.clone();
            value["dns_bootstrap"] = json!([{"host":host,"address":"142.250.74.206"}]);
            assert!(request(&serde_json::to_vec(&value).unwrap()).is_none());
        }
        for address in [
            "127.0.0.1",
            "100.64.0.1",
            "192.168.1.1",
            "198.18.0.1",
            "224.0.0.1",
            "::1",
        ] {
            let mut value = valid.clone();
            value["dns_bootstrap"] = json!([{"host":"www.youtube.com","address":address}]);
            assert!(request(&serde_json::to_vec(&value).unwrap()).is_none());
        }
        let mut mapped = valid.clone();
        mapped["dns_bootstrap"] = json!([{"host":"www.youtube.com","address":"142.250.74.206"}]);
        assert_eq!(
            bootstrap(&mapped).unwrap(),
            "MAP www.youtube.com 142.250.74.206"
        );
        mapped["proxy"] = json!("socks5h://127.0.0.1:1080");
        assert!(request(&serde_json::to_vec(&mapped).unwrap()).is_none());
        for proxy in [
            "socks5h://user:private@localhost:8080",
            "https://localhost:8080",
            "socks4://localhost:1080",
            "http://localhost:80/path",
            "http://localhost:80?x=1",
        ] {
            let mut value = valid.clone();
            value["proxy"] = json!(proxy);
            assert!(request(&serde_json::to_vec(&value).unwrap()).is_none());
        }
        let mut authenticated_http = valid.clone();
        authenticated_http["proxy"] = json!("http://proof:synthetic@127.0.0.1:8080");
        assert!(request(&serde_json::to_vec(&authenticated_http).unwrap()).is_some());
        let result = response(r#"{"ok":false,"stage":"private remote message"}"#, "id");
        assert_eq!(result["stage"], "internal");
        assert!(!result.to_string().contains("private"));
        assert_eq!(
            response(r#"{"ok":true,"content_po":"x"}"#, "id")["ok"],
            false
        );
    }
}
