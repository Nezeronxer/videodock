pub mod catalog;
pub mod model;
mod selection;
pub mod youtube;
pub use model::*;
pub use selection::choose_formats;

pub fn scoped_headers(
    headers: &std::collections::BTreeMap<String, String>,
    source: &str,
    target: &str,
) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
    let source = validate_url(source)?;
    let target = validate_url(target)?;
    let same = source.origin() == target.origin();
    let mut result = headers.clone();
    if !same {
        result.retain(|name, _| {
            !matches!(
                name.to_ascii_lowercase().as_str(),
                "cookie" | "authorization" | "proxy-authorization" | "host"
            )
        });
        for (name, value) in &mut result {
            if name.eq_ignore_ascii_case("referer")
                && let Ok(url) = url::Url::parse(value)
            {
                *value = format!("{}/", url.origin().ascii_serialization());
            }
        }
        if source.scheme() == "https" && target.scheme() != "https" {
            result.retain(|name, _| !name.eq_ignore_ascii_case("referer"));
        }
    }
    Ok(result)
}

pub fn request_headers(
    headers: &std::collections::BTreeMap<String, String>,
    source: &str,
    target: &str,
    cookies: &[ScopedCookie],
) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
    let mut headers = scoped_headers(headers, source, target)?;
    let target = validate_url(target)?;
    let mut selected = cookies
        .iter()
        .filter(|cookie| cookie.applies_to(&target))
        .collect::<Vec<_>>();
    selected.sort_by_key(|cookie| std::cmp::Reverse(cookie.path.len()));
    let existing_key = headers
        .keys()
        .find(|name| name.eq_ignore_ascii_case("cookie"))
        .cloned();
    let mut value = existing_key
        .as_ref()
        .and_then(|key| headers.get(key))
        .cloned()
        .unwrap_or_default();
    let mut names = value
        .split(';')
        .filter_map(|pair| pair.trim().split_once('=').map(|(name, _)| name.to_owned()))
        .collect::<std::collections::HashSet<_>>();
    for cookie in selected {
        anyhow::ensure!(
            !cookie.name.is_empty()
                && !cookie
                    .name
                    .chars()
                    .any(|ch| ch.is_control() || ";= ".contains(ch))
                && !cookie.value.chars().any(|ch| ch.is_control() || ch == ';'),
            "Недопустимое значение cookie"
        );
        if names.insert(cookie.name.clone()) {
            if !value.is_empty() {
                value.push_str("; ");
            }
            value.push_str(&cookie.name);
            value.push('=');
            value.push_str(&cookie.value);
        }
    }
    if !value.is_empty() {
        if let Some(key) = existing_key {
            headers.remove(&key);
        }
        headers.insert("Cookie".into(), value);
    }
    Ok(headers)
}

pub fn validate_url(value: &str) -> anyhow::Result<url::Url> {
    anyhow::ensure!(
        !value.chars().any(char::is_control) && value.len() <= 16384,
        "Недопустимые символы в ссылке"
    );
    let url = url::Url::parse(value.trim())?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "Нужна ссылка HTTP(S)"
    );
    anyhow::ensure!(
        url.username().is_empty() && url.password().is_none(),
        "Ссылка не должна содержать пароль"
    );
    Ok(url)
}
