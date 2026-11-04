use super::*;
#[path = "network.rs"]
mod network;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, path::Path};
use videodock_media::Auth;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Secrets {
    pub(super) proxy_password: String,
    #[serde(skip)]
    network: Arc<Mutex<network::Cache>>,
    #[serde(skip)]
    // Qt owns cookie persistence; replay only its currently valid profile on startup.
    cookies: Vec<Cookie>,
    sources: HashMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    expires: Option<u64>,
}

pub(super) fn load(directory: &Path) -> Result<Secrets> {
    let path = directory.join("secrets.json");
    if !path.exists() {
        return Ok(Secrets::default());
    }
    ensure!(
        std::fs::metadata(&path)?.len() <= 8 * 1024 * 1024,
        "Хранилище авторизации слишком большое"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    serde_json::from_slice(&std::fs::read(path)?)
        .context("Повреждено локальное хранилище авторизации")
}
pub(super) fn save(inner: &Inner, secrets: &Secrets) -> Result<()> {
    let temp = inner
        .directory
        .join(format!(".secrets-{}", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let data = serde_json::to_vec(secrets)?;
    ensure!(
        data.len() <= 8 * 1024 * 1024,
        "Локальное хранилище защищённых данных превышает 8 MiB"
    );
    let mut file = options.open(&temp)?;
    if let Err(error) = file.write_all(&data).and_then(|_| file.sync_all()) {
        drop(file);
        std::fs::remove_file(temp).ok();
        return Err(error.into());
    }
    drop(file);
    let target = inner.directory.join("secrets.json");
    let result = replace_file(&temp, &target);
    if result.is_err() {
        std::fs::remove_file(temp).ok();
    }
    result?;
    Ok(())
}
#[cfg(not(windows))]
pub(super) fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::rename(source, target)
}
#[cfg(windows)]
pub(super) fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, target: *const u16, flags: u32) -> i32;
    }
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 1 | 8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
pub(super) fn mutate<T>(
    inner: &Inner,
    operation: impl FnOnce(&mut Secrets) -> Result<T>,
) -> Result<T> {
    let mut guard = lock(&inner.secrets);
    let mut next = guard.clone();
    let result = operation(&mut next)?;
    save(inner, &next)?;
    *guard = next;
    Ok(result)
}
fn protected_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(
        key.as_str(),
        "sig"
            | "s"
            | "signature"
            | "token"
            | "access_token"
            | "auth"
            | "authorization"
            | "api_key"
            | "key"
            | "policy"
            | "credential"
            | "x-amz-signature"
            | "x-amz-credential"
            | "x-amz-security-token"
            | "x-goog-signature"
            | "x-goog-credential"
    ) || key.ends_with("_token")
}
pub(super) fn masked(value: &str) -> String {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() > 1 && parts.iter().all(|part| url::Url::parse(part).is_ok()) {
        let hidden = parts.iter().map(|part| masked(part)).collect::<Vec<_>>();
        return if hidden.iter().zip(&parts).any(|(a, b)| a != b) {
            hidden.join("\n")
        } else {
            value.to_owned()
        };
    }
    let Ok(mut url) = url::Url::parse(value) else {
        return value.to_owned();
    };
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let private_query = pairs.iter().any(|(key, _)| protected_key(key));
    let private_fragment = url.fragment().is_some_and(|value| {
        value
            .split('&')
            .filter_map(|part| part.split_once('='))
            .any(|(key, _)| protected_key(key))
    });
    if !private_query && !private_fragment {
        return value.to_owned();
    }
    if private_query {
        url.set_query(None);
        url.query_pairs_mut()
            .extend_pairs(pairs.iter().map(|(key, value)| {
                (
                    key.as_str(),
                    if protected_key(key) {
                        "[private]"
                    } else {
                        value.as_str()
                    },
                )
            }));
    }
    if private_fragment {
        url.set_fragment(Some("[private]"));
    }
    url.to_string()
}
pub(super) fn source(inner: &Inner, item: &Value) -> Result<String> {
    if item["_source_private"] == true {
        let id = item["id"]
            .as_str()
            .context("Нет идентификатора исходника")?;
        return lock(&inner.secrets)
            .sources
            .get(id)
            .cloned()
            .context("Защищённая исходная ссылка недоступна. Вставьте её повторно.");
    }
    Ok(settings::text(&item["url"])?.to_owned())
}
pub(super) fn seal_sources(inner: &Inner, state: &mut Value) -> Result<()> {
    let mut updates = Vec::new();
    for key in ["jobs", "subscriptions"] {
        if let Some(items) = state[key].as_array_mut() {
            for item in items {
                if let Some(title) = item["title"].as_str() {
                    let hidden = masked(title);
                    if hidden != title {
                        item["title"] = json!(hidden);
                    }
                }
                if item["_source_private"] == true {
                    continue;
                }
                if let Some(value) = item["url"].as_str() {
                    let hidden = masked(value);
                    if hidden != value {
                        let id = item["id"]
                            .as_str()
                            .context("Нет идентификатора исходника")?
                            .to_owned();
                        updates.push((id, value.to_owned()));
                        item["url"] = json!(hidden);
                        item["_source_private"] = json!(true);
                        item["private_source"] = json!(true);
                    }
                }
            }
        }
    }
    if let Some(manual) = state["manual"].as_object_mut() {
        for key in ["url", "title"] {
            if let Some(value) = manual.get(key).and_then(Value::as_str) {
                let hidden = masked(value);
                if hidden != value {
                    manual.insert(key.into(), json!(hidden));
                    manual.insert("private_source".into(), json!(true));
                }
            }
        }
        if let Some(items) = manual.get_mut("items").and_then(Value::as_array_mut) {
            for item in items {
                if let Some(title) = item["title"].as_str() {
                    item["title"] = json!(masked(title));
                }
            }
        }
    }
    if !updates.is_empty() {
        mutate(inner, |secrets| {
            for (id, value) in updates {
                secrets.sources.insert(id, value);
            }
            Ok(())
        })?;
    }
    Ok(())
}
pub(super) fn forget(inner: &Inner, ids: &HashSet<String>) -> Result<()> {
    mutate(inner, |secrets| {
        secrets.sources.retain(|id, _| !ids.contains(id));
        Ok(())
    })
}
fn applies(cookie: &Cookie, url: &url::Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    let domain = cookie.domain.trim_start_matches('.');
    let domain_match =
        host == domain || (cookie.domain.starts_with('.') && host.ends_with(&format!(".{domain}")));
    let path = url.path();
    let prefix = cookie.path.as_str();
    domain_match
        && cookie.expires.is_none_or(|expires| expires > now())
        && (!cookie.secure || url.scheme() == "https")
        && path.starts_with(prefix)
        && (prefix.ends_with('/')
            || path.len() == prefix.len()
            || path.as_bytes().get(prefix.len()) == Some(&b'/'))
}
pub(super) fn auth(inner: &Inner, source: &str) -> Auth {
    let Ok(url) = url::Url::parse(source) else {
        return Auth::default();
    };
    let secrets = lock(&inner.secrets);
    let mut cookies = secrets
        .cookies
        .iter()
        .filter(|cookie| applies(cookie, &url))
        .collect::<Vec<_>>();
    cookies.sort_by_key(|cookie| std::cmp::Reverse(cookie.path.len()));
    Auth {
        cookie_header: cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; "),
    }
}
pub(super) fn context(inner: &Inner, origin: &str) -> Vec<videodock_media::ScopedCookie> {
    let Ok(url) = url::Url::parse(origin) else {
        return Vec::new();
    };
    lock(&inner.secrets)
        .cookies
        .iter()
        .map(|cookie| videodock_media::ScopedCookie {
            name: cookie.name.clone(),
            value: cookie.value.clone(),
            domain: cookie.domain.clone(),
            path: cookie.path.clone(),
            secure: cookie.secure,
            expires: cookie.expires,
        })
        .filter(|cookie| cookie.matches_domain(&url))
        .collect()
}
pub(super) fn auth_in_context(inner: &Inner, target: &str, origin: &str) -> Auth {
    let Ok(url) = url::Url::parse(target) else {
        return Auth::default();
    };
    let mut cookies = context(inner, origin)
        .into_iter()
        .filter(|cookie| cookie.applies_to(&url))
        .collect::<Vec<_>>();
    cookies.sort_by_key(|cookie| std::cmp::Reverse(cookie.path.len()));
    Auth {
        cookie_header: cookies
            .into_iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; "),
    }
}
pub(super) fn proxy(inner: &Inner) -> Result<Option<String>> {
    proxy_policy(inner).map(|(proxy, _)| proxy)
}
pub(super) fn proxy_policy(inner: &Inner) -> Result<(Option<String>, bool)> {
    let settings = lock(&inner.state)["settings"].clone();
    if settings["proxyEnabled"] != true {
        let cache = Arc::clone(&lock(&inner.secrets).network);
        return lock(&cache).proxy().map(|proxy| (proxy, true));
    }
    let scheme = match settings["proxyType"].as_str().unwrap_or("SOCKS5") {
        "SOCKS4" => "socks4",
        "SOCKS5" => "socks5h",
        "HTTPS" => "https",
        _ => "http",
    };
    let mut url = url::Url::parse(&format!("{scheme}://localhost"))?;
    let host = settings["proxyHost"]
        .as_str()
        .context("Не указан сервер прокси")?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        url.set_ip_host(ip)
            .map_err(|_| anyhow!("Некорректный адрес прокси"))?;
    } else {
        url.set_host(Some(host))
            .map_err(|_| anyhow!("Некорректный адрес прокси"))?;
    }
    url.set_port(Some(settings["proxyPort"].as_u64().unwrap_or(1080) as u16))
        .map_err(|_| anyhow!("Некорректный порт прокси"))?;
    let login = settings["proxyLogin"].as_str().unwrap_or_default();
    if !login.is_empty() {
        url.set_username(login)
            .map_err(|_| anyhow!("Недопустимый логин прокси"))?;
        url.set_password(Some(&lock(&inner.secrets).proxy_password))
            .map_err(|_| anyhow!("Недопустимые данные прокси"))?;
    }
    Ok((Some(url.to_string()), false))
}
pub(super) fn verify_debounced(inner: &Arc<Inner>) {
    let generation = inner.auth_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let weak = Arc::downgrade(inner);
    let Ok(runtime) = runtime() else { return };
    runtime.spawn(async move{
        tokio::time::sleep(Duration::from_secs(2)).await;
        let Some(inner)=weak.upgrade()else{return};if inner.shutdown.is_cancelled()||inner.auth_generation.load(Ordering::SeqCst)!=generation{return}
        let auth=auth(&inner,"https://www.youtube.com/");
        if auth.cookie_header.is_empty(){let _=change(&inner,|state|{state["account"]=json!({"logged_in":false,"user_name":"","avatar_url":""});Ok(())});return}
        let _=change(&inner,|state|{state["account"]["checking"]=json!(true);Ok(())});
        let result=async{
            let proxy=proxy(&inner)?;
            let youtube=videodock_media::youtube::YouTube::new(proxy.as_deref()).await?;
            tokio::select!{_ = inner.shutdown.cancelled()=>bail!("Проверка отменена"),result=youtube.account_info(&auth)=>result}
        }.await;
        if inner.auth_generation.load(Ordering::SeqCst)!=generation||inner.shutdown.is_cancelled(){return}
        let verified=result.is_ok();let _=change(&inner,|state|{state["account"]=match &result{Ok(account)=>json!({"logged_in":true,"user_name":account.user_name,"avatar_url":account.avatar_url,"checking":false}),Err(_)=>json!({"logged_in":false,"user_name":"","avatar_url":"","checking":false})};if !verified{state["notice"]=json!("Не удалось подтвердить авторизацию YouTube. Повторите вход в браузере.");}Ok(())});
    });
}
pub(super) fn account(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    match action["command"]
        .as_str()
        .context("Не указано действие аккаунта")?
    {
        "login" => Ok(
            json!({"url":"https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F"}),
        ),
        "logout" => {
            mutate(inner, |secrets| {
                secrets.cookies.clear();
                Ok(())
            })?;
            inner.auth_generation.fetch_add(1, Ordering::SeqCst);
            change(inner, |state| {
                state["account"] = json!({"logged_in":false,"user_name":"","avatar_url":""});
                Ok(())
            })?;
            Ok(json!({}))
        }
        "liked" | "watch_later" => {
            ensure!(
                lock(&inner.state)["account"]["logged_in"] == true,
                "Сначала войдите в YouTube и дождитесь проверки авторизации"
            );
            let list = if action["command"] == "liked" {
                "LL"
            } else {
                "WL"
            };
            super::queue::parse(
                inner,
                &json!({"url":format!("https://www.youtube.com/playlist?list={list}"),"force_manual":action["subscribe"]==true,"mode":if action["subscribe"]==true{"subscribe"}else{"download"}}),
            )
        }
        "cookie" => {
            let name = settings::bounded(&action["name"], 1024)?;
            ensure!(
                !name.is_empty()
                    && !name
                        .chars()
                        .any(|ch| !ch.is_ascii() || "()<>@,;:\\\"/[]?={} \t".contains(ch)),
                "Недопустимое имя cookie"
            );
            let value = settings::bounded(&action["value"], 65536)?;
            ensure!(!value.contains(';'), "Недопустимое значение cookie");
            let domain = settings::bounded(&action["domain"], 253)?.to_ascii_lowercase();
            let bare = domain.trim_start_matches('.');
            ensure!(
                !bare.is_empty()
                    && (bare.parse::<std::net::IpAddr>().is_ok()
                        || bare
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')),
                "Недопустимый домен cookie"
            );
            let path = settings::bounded(&action["path"], 4096)?;
            let path = if path.is_empty() { "/".into() } else { path };
            ensure!(path.starts_with('/'), "Недопустимый путь cookie");
            let secure = action["secure"].as_bool().unwrap_or(false);
            let expires = if action["expires"].is_null() {
                None
            } else {
                Some(
                    action["expires"]
                        .as_u64()
                        .context("Недопустимое время cookie")?,
                )
            };
            let removed = action["removed"] == true;
            let auth_changed = bare == "youtube.com"
                || bare.ends_with(".youtube.com")
                || bare == "google.com"
                || bare.ends_with(".google.com");
            mutate(inner, |secrets| {
                secrets.cookies.retain(|cookie| {
                    cookie.name != name || cookie.domain != domain || cookie.path != path
                });
                if !removed && expires.is_none_or(|expires| expires > now()) {
                    ensure!(
                        secrets.cookies.len() < 4096,
                        "Слишком много cookies в профиле"
                    );
                    secrets.cookies.push(Cookie {
                        name,
                        value,
                        domain,
                        path,
                        secure,
                        expires,
                    });
                }
                Ok(())
            })?;
            if auth_changed {
                verify_debounced(inner);
            }
            Ok(json!({}))
        }
        _ => bail!("Неизвестное действие аккаунта"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_cookies_keep_expiry_and_are_restored_only_by_the_browser() {
        let data = tempfile::tempdir().unwrap();
        let engine = Engine::new(data.path().to_owned()).unwrap();
        let expires = now() + 60;
        engine.dispatch(json!({"action":"account","command":"cookie","name":"temporary","value":"synthetic-expiry-only","domain":"example.invalid","path":"/","secure":true,"expires":expires})).unwrap();
        let cookies = context(&engine.inner, "https://example.invalid/");
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].expires, Some(expires));
        engine.dispatch(json!({"action":"account","command":"cookie","name":"expired","value":"synthetic-expired-only","domain":"example.invalid","path":"/","secure":true,"expires":now()-1})).unwrap();
        assert_eq!(context(&engine.inner, "https://example.invalid/").len(), 1);
        let saved: Value =
            serde_json::from_slice(&std::fs::read(data.path().join("secrets.json")).unwrap())
                .unwrap();
        assert!(
            saved.get("cookies").is_none(),
            "the browser owns persistence and expiration of its cookies"
        );
        drop(engine);
        let engine = Engine::new(data.path().to_owned()).unwrap();
        assert!(context(&engine.inner, "https://example.invalid/").is_empty());
    }
    #[test]
    fn cookie_scope_never_crosses_host_path_or_secure_boundary() {
        let cookie = Cookie {
            name: "x".into(),
            value: "synthetic".into(),
            domain: ".youtube.com".into(),
            path: "/api".into(),
            secure: true,
            expires: None,
        };
        for (url, expected) in [
            ("https://www.youtube.com/api/player", true),
            ("https://youtube.com/api", true),
            ("http://youtube.com/api", false),
            ("https://youtube.com/apibad", false),
            ("https://youtube.com.evil.invalid/api", false),
            ("https://google.com/api", false),
        ] {
            assert_eq!(
                applies(&cookie, &url::Url::parse(url).unwrap()),
                expected,
                "{url}"
            );
        }
    }

    #[test]
    fn media_context_ignores_path_but_preserves_source_domain_and_secure_boundary() {
        let data = tempfile::tempdir().unwrap();
        let engine = Engine::new(data.path().to_owned()).unwrap();
        for (name, domain, secure) in [
            ("stream", ".example.invalid", false),
            ("secure", ".example.invalid", true),
            ("bank", "bank.invalid", false),
        ] {
            engine.dispatch(json!({"action":"account","command":"cookie","name":name,"value":"synthetic-context-value","domain":domain,"path":"/captions","secure":secure})).unwrap();
        }
        let plain = context(&engine.inner, "http://media.example.invalid/watch");
        assert_eq!(
            plain
                .iter()
                .map(|cookie| cookie.name.as_str())
                .collect::<Vec<_>>(),
            vec!["stream"]
        );
        assert!(
            plain[0]
                .applies_to(&url::Url::parse("http://cdn.example.invalid/captions/en").unwrap())
        );
        assert!(
            !plain[0]
                .applies_to(&url::Url::parse("http://cdn.example.invalid/captions-other").unwrap())
        );
        let secure = context(&engine.inner, "https://media.example.invalid/watch");
        assert_eq!(secure.len(), 2);
        assert!(context(&engine.inner, "https://foreign.invalid/").is_empty());
    }
}
