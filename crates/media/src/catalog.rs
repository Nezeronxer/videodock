use crate::{Auth, Entry, Format, Media, MediaKind, Protocol, ScopedCookie, Subtitle};
use anyhow::{Result, anyhow, bail, ensure};
use regex::Regex;
use reqwest::{Client, Response, StatusCode};
use scraper::{Html, Selector};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};
use url::Url;

const DOCUMENT_LIMIT: usize = 8 * 1024 * 1024;
const ENTRY_LIMIT: usize = 10_000;
const APP_UA: &str = "VideoDock/0.2";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36";

pub struct Catalog {
    http: Client,
    media_http: Client,
    proxy: Option<String>,
}

impl Catalog {
    pub fn new(proxy: Option<&str>) -> Result<Self> {
        Self::with_proxy(proxy, false)
    }
    pub fn new_automatic(proxy: Option<&str>) -> Result<Self> {
        Self::with_proxy(proxy, true)
    }
    fn with_proxy(proxy: Option<&str>, automatic: bool) -> Result<Self> {
        let proxy = proxy
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned);
        let dns = crate::dns::resolver(proxy.is_none())?;
        let builder = || -> Result<reqwest::ClientBuilder> {
            let mut builder = Client::builder()
                .dns_resolver(dns.clone())
                .default_headers(reqwest::header::HeaderMap::from_iter([(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static("*/*"),
                )]))
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(UA)
                .connect_timeout(Duration::from_secs(30));
            if let Some(value) = &proxy {
                let mut proxy =
                    reqwest::Proxy::all(value).map_err(|_| anyhow!("Некорректный адрес прокси"))?;
                if automatic {
                    proxy = proxy.no_proxy(reqwest::NoProxy::from_string(
                        "localhost,.localhost,127.0.0.0/8,::1",
                    ));
                }
                builder = builder.proxy(proxy);
            }
            Ok(builder)
        };
        Ok(Self {
            http: builder()?
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|_| anyhow!("Не удалось создать HTTP клиент"))?,
            // Long media responses need an idle timeout, not a total download deadline.
            media_http: builder()?
                .read_timeout(Duration::from_secs(30))
                .build()
                .map_err(|_| anyhow!("Не удалось создать HTTP клиент"))?,
            proxy,
        })
    }
    pub fn client(&self) -> Client {
        self.media_http.clone()
    }
    pub async fn search(&self, value: &str, auth: &Auth) -> Result<Media> {
        crate::youtube::YouTube::new(self.proxy.as_deref())
            .await?
            .search(value, auth)
            .await
    }
    pub async fn inspect(&self, value: &str, auth: &Auth) -> Result<Media> {
        self.inspect_inner(value, auth, 0).await
    }

    async fn inspect_inner(&self, value: &str, auth: &Auth, depth: u8) -> Result<Media> {
        ensure!(depth < 4, "Слишком много вложенных медиаплееров");
        let url = crate::validate_url(value)?;
        ensure!(
            !auth.cookie_header.chars().any(|ch| ch.is_control()),
            "Недопустимый Cookie-заголовок"
        );
        let host = url.host_str().unwrap_or_default();
        if domain(host, "youtube.com") || host == "youtu.be" || domain(host, "youtube-nocookie.com")
        {
            return crate::youtube::YouTube::new(self.proxy.as_deref())
                .await?
                .inspect(value, auth)
                .await;
        }
        if domain(host, "vine.co") {
            bail!("Vine: сервис закрыт; медиапоток не предоставляется")
        }
        if domain(host, "vimeo.com") {
            return self.vimeo(&url, auth).await;
        }
        if domain(host, "dailymotion.com") || host == "dai.ly" {
            return self.dailymotion(&url, auth).await;
        }
        if domain(host, "soundcloud.com") {
            return self.soundcloud(&url, auth).await;
        }
        if domain(host, "flickr.com") || host == "flic.kr" {
            return self.flickr(&url, auth).await;
        }
        if domain(host, "tumblr.com") {
            return self.tumblr(&url, auth, depth).await;
        }
        if domain(host, "twitch.tv") {
            return self.twitch(&url).await;
        }
        if domain(host, "bilibili.com") || host == "b23.tv" {
            return self.bilibili(&url, auth).await;
        }
        if domain(host, "rumble.com") {
            return self.rumble(&url, auth).await;
        }
        if domain(host, "bitchute.com") {
            return self.bitchute(&url).await;
        }
        if domain(host, "nicovideo.jp") || host == "nico.ms" {
            return self.niconico(&url, auth).await;
        }
        if domain(host, "likee.video") || domain(host, "likee.com") || domain(host, "likeevideo.ru")
        {
            return self.likee(&url, auth).await;
        }
        if host == "tv.naver.com" || host == "tv.naver.tv" || host == "m.tv.naver.com" {
            return self.naver(&url, auth).await;
        }
        let mut response = self.source_get(&url, &url, auth).await?;
        let tiktok_cookie = if domain(host, "tiktok.com") && auth.cookie_header.is_empty() {
            issued_cookie(
                response.url(),
                response.headers(),
                "tt_chain_token",
                "tiktok.com",
            )
        } else {
            None
        };
        let final_url = response.url().clone();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        if is_media_type(&content_type) {
            return Ok(direct_media(
                &final_url,
                &content_type,
                response.content_length(),
            ));
        }
        if direct_extension(&final_url) {
            let size = response.content_length();
            let sample = response
                .chunk()
                .await
                .map_err(|_| anyhow!("Не удалось проверить прямую медиассылку"))?
                .ok_or_else(|| anyhow!("Прямая медиассылка вернула пустой ответ"))?;
            ensure!(
                media_signature(&sample),
                "Прямая ссылка не вернула узнаваемый медиаконтейнер или манифест"
            );
            return Ok(direct_media(&final_url, &content_type, size));
        }
        let text = body(response).await?;
        if domain(host, "tiktok.com") {
            let cookie = if auth.cookie_header.is_empty() {
                tiktok_cookie
            } else {
                let anonymous = self.source_get(&url, &url, &Auth::default()).await?;
                issued_cookie(
                    anonymous.url(),
                    anonymous.headers(),
                    "tt_chain_token",
                    "tiktok.com",
                )
            };
            let mut media = parse_tiktok(&final_url, &text)?;
            if let Some(cookie) = cookie {
                for stream in &mut media.formats {
                    if Url::parse(&stream.url).ok().is_some_and(|target| {
                        target.scheme() == "https"
                            && domain(target.host_str().unwrap_or_default(), "tiktok.com")
                    }) {
                        stream.cookies.push(cookie.clone());
                    }
                }
            }
            return Ok(media);
        }
        if domain(host, "facebook.com") || domain(host, "fb.watch") {
            if let Ok(media) = parse_facebook(&final_url, &text) {
                return Ok(media);
            }
            let mut embed = Url::parse("https://www.facebook.com/plugins/video.php")?;
            embed
                .query_pairs_mut()
                .append_pair("href", final_url.as_str());
            let page = body(self.source_get(&embed, &embed, &Auth::default()).await?).await?;
            let mut media = parse_facebook(&final_url, &page)?;
            media.title = meta(&text, "og:title").unwrap_or(media.title);
            media.thumbnail = meta(&text, "og:image").unwrap_or(media.thumbnail);
            return Ok(media);
        }
        let media = parse_html(&final_url, &text)?;
        if !media.formats.is_empty() || !media.entries.is_empty() {
            return finish(media);
        }
        if let Some(embed) = embedded_player(&final_url, &text) {
            return Box::pin(self.inspect_inner(embed.as_str(), &Auth::default(), depth + 1)).await;
        }
        bail!(
            "На странице нет доступных HTML5/JSON-LD медиапотоков; для этого сайта нужен отдельный экстрактор"
        )
    }

    async fn source_get(&self, url: &Url, origin: &Url, auth: &Auth) -> Result<Response> {
        let mut current = url.clone();
        for _ in 0..6 {
            let mut request = self.http.get(current.clone());
            if direct_extension(&current) {
                request = request.header(reqwest::header::USER_AGENT, APP_UA);
            }
            if same_origin(&current, origin) && !auth.cookie_header.is_empty() {
                request = request.header(reqwest::header::COOKIE, &auth.cookie_header);
            }
            let response = request
                .send()
                .await
                .map_err(|_| anyhow!("Не удалось открыть страницу: проверьте сеть и прокси"))?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow!("Сайт вернул перенаправление без адреса"))?;
                current = crate::validate_url(
                    current
                        .join(location)
                        .map_err(|_| anyhow!("Неверное перенаправление"))?
                        .as_str(),
                )?;
                continue;
            }
            check_status(response.status())?;
            return Ok(response);
        }
        bail!("Слишком много перенаправлений")
    }
    async fn get_json(&self, url: Url) -> Result<Value> {
        let response = self
            .http
            .get(url)
            .header("User-Agent", "VideoDock/0.2")
            .send()
            .await
            .map_err(|_| anyhow!("Не удалось запросить публичный API сервиса"))?;
        json_response(response).await
    }
    async fn post_json(&self, url: &str, value: &Value) -> Result<Value> {
        let response = self
            .http
            .post(url)
            .header("User-Agent", "VideoDock/0.2")
            .json(value)
            .send()
            .await
            .map_err(|_| anyhow!("Не удалось запросить публичный API сервиса"))?;
        json_response(response).await
    }
}

fn check_status(status: StatusCode) -> Result<()> {
    match status.as_u16() {
        200..=299 => Ok(()),
        401 | 403 => bail!(
            "Сервис отказал в доступе (HTTP{}): требуется вход, подтверждение браузера или доступ в регионе",
            status.as_u16()
        ),
        429 => bail!("Сервис ограничил частоту запросов; повторите позже"),
        _ => bail!("Сервис вернул HTTP{}", status.as_u16()),
    }
}
async fn body(mut response: Response) -> Result<String> {
    let mut data = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow!("Не удалось прочитать ответ сервиса"))?
    {
        ensure!(
            data.len() + chunk.len() <= DOCUMENT_LIMIT,
            "Ответ сервиса превышает 8 MiB"
        );
        data.extend_from_slice(&chunk);
    }
    String::from_utf8(data).map_err(|_| anyhow!("Ответ сервиса не является UTF-8 документом"))
}
async fn json_response(response: Response) -> Result<Value> {
    check_status(response.status())?;
    serde_json::from_str(&body(response).await?)
        .map_err(|_| anyhow!("Сервис вернул некорректный JSON"))
}
// Only newly issued anonymous delivery leases enter CDN headers; caller Auth is never copied.
fn issued_cookie(
    origin: &Url,
    headers: &reqwest::header::HeaderMap,
    name: &str,
    allowed_domain: &str,
) -> Option<ScopedCookie> {
    if origin.scheme() != "https" || !domain(origin.host_str()?, allowed_domain) {
        return None;
    }
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .filter(|value| value.len() <= 8192)
        .find_map(|value| {
            let mut parts = value.split(';');
            let pair = parts.next()?.trim();
            let (key, content) = pair.split_once('=')?;
            if key != name || content.is_empty() {
                return None;
            }
            let raw_attributes = parts.map(str::trim).collect::<Vec<_>>();
            let attributes = raw_attributes
                .iter()
                .filter_map(|part| part.split_once('='))
                .collect::<Vec<_>>();
            if attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("max-age"))
                .and_then(|(_, value)| value.parse::<i64>().ok())
                .is_some_and(|value| value <= 0)
            {
                return None;
            }
            let cookie_domain = attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("domain"))?
                .1
                .trim_start_matches('.');
            let path = attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("path"))?
                .1;
            if !cookie_domain.eq_ignore_ascii_case(allowed_domain) || path != "/" {
                return None;
            }
            let max_age = attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("max-age"))
                .and_then(|(_, value)| value.parse::<u64>().ok());
            let expires = if let Some(seconds) = max_age {
                Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs()
                        .saturating_add(seconds),
                )
            } else {
                attributes
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case("expires"))
                    .and_then(|(_, value)| {
                        time::OffsetDateTime::parse(
                            value,
                            &time::format_description::well_known::Rfc2822,
                        )
                        .ok()
                    })
                    .and_then(|date| date.unix_timestamp().try_into().ok())
            };
            let cookie = ScopedCookie {
                name: key.to_owned(),
                value: content.to_owned(),
                domain: format!(".{}", cookie_domain.to_ascii_lowercase()),
                path: path.into(),
                secure: raw_attributes
                    .iter()
                    .any(|attribute| attribute.eq_ignore_ascii_case("secure")),
                expires,
            };
            cookie.applies_to(origin).then_some(cookie)
        })
}
fn domain(host: &str, base: &str) -> bool {
    host == base || host.ends_with(&format!(".{base}"))
}
fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}
fn api_url(base: &str, pairs: &[(&str, &str)]) -> Result<Url> {
    let mut url = Url::parse(base)?;
    url.query_pairs_mut().extend_pairs(pairs.iter().copied());
    Ok(url)
}
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .filter(|value| value.is_finite() && *value >= 0.0)
}
fn uint(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}
fn id_from_path(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut parts| parts.rfind(|part| !part.is_empty()))
        .unwrap_or_default()
        .to_owned()
}
fn source_headers(origin: &Url) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("Accept".into(), "*/*".into()),
        ("User-Agent".into(), UA.into()),
        (
            "Referer".into(),
            format!("{}/", origin.origin().ascii_serialization()),
        ),
    ])
}
fn protocol(value: &str, mime: &str) -> Protocol {
    let path = Url::parse(value)
        .ok()
        .map(|u| u.path().to_owned())
        .unwrap_or_default();
    if path.ends_with(".m3u8") || mime.contains("mpegurl") {
        Protocol::Hls
    } else if path.ends_with(".mpd") || mime.contains("dash+xml") {
        Protocol::Dash
    } else {
        Protocol::Http
    }
}
fn extension(value: &str, mime: &str, audio: bool) -> String {
    let path = Url::parse(value)
        .ok()
        .map(|u| u.path().to_owned())
        .unwrap_or_default();
    let ext = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "mp4", "webm", "m4a", "mp3", "ogg", "opus", "flac", "aac", "wav", "mov", "m4v", "3gp",
    ]
    .contains(&ext.as_str())
    {
        ext
    } else if mime.contains("webm") {
        "webm".into()
    } else if mime.contains("mpeg") && !mime.contains("mpegurl") {
        if audio { "mp3".into() } else { "mp4".into() }
    } else if audio {
        "m4a".into()
    } else {
        "mp4".into()
    }
}
fn format(origin: &Url, id: &str, value: &str, mime: &str, audio: bool) -> Result<Format> {
    ensure!(
        !value.trim().is_empty(),
        "Сервис вернул пустую ссылку медиапотока"
    );
    let url = crate::validate_url(
        origin
            .join(value)
            .map_err(|_| anyhow!("Сервис вернул неверную ссылку медиапотока"))?
            .as_str(),
    )?;
    Ok(Format {
        id: id.into(),
        protocol: protocol(url.as_str(), mime),
        ext: extension(url.as_str(), mime, audio),
        url: url.to_string(),
        headers: source_headers(origin),
        video_codec: if audio { None } else { Some("unknown".into()) },
        audio_codec: Some("unknown".into()),
        ..Default::default()
    })
}
fn finish(mut media: Media) -> Result<Media> {
    ensure!(
        media.entries.len() <= ENTRY_LIMIT,
        "Список превышает 10000 элементов"
    );
    let mut seen = HashSet::new();
    media.formats.retain(|f| seen.insert(f.url.clone()));
    ensure!(
        !media.formats.is_empty() || !media.entries.is_empty(),
        "Сервис не вернул доступных медиапотоков или элементов списка"
    );
    if media.title.is_empty() {
        media.title = media.id.clone()
    };
    Ok(media)
}
fn base_media(url: &Url) -> Media {
    Media {
        id: id_from_path(url),
        url: url.to_string(),
        title: id_from_path(url),
        ..Default::default()
    }
}
fn is_media_type(value: &str) -> bool {
    value.starts_with("video/")
        || value.starts_with("audio/")
        || value.contains("mpegurl")
        || value.contains("dash+xml")
}
fn direct_extension(url: &Url) -> bool {
    url.path().rsplit('.').next().is_some_and(|extension| {
        [
            "mp4", "m4v", "webm", "m4a", "mp3", "ogg", "opus", "flac", "aac", "wav", "mov", "3gp",
            "m3u8", "mpd",
        ]
        .contains(&extension.to_ascii_lowercase().as_str())
    })
}
fn media_signature(bytes: &[u8]) -> bool {
    bytes.get(4..8) == Some(b"ftyp")
        || bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3])
        || bytes.starts_with(b"OggS")
        || bytes.starts_with(b"fLaC")
        || (bytes.starts_with(b"RIFF") && matches!(bytes.get(8..12), Some(b"WAVE" | b"AVI ")))
        || bytes.starts_with(b"ID3")
        || (bytes.first() == Some(&0xff)
            && bytes
                .get(1)
                .is_some_and(|value| value & 0xe0 == 0xe0 && *value != 0xff))
        || String::from_utf8_lossy(bytes)
            .trim_start_matches('\u{feff}')
            .starts_with("#EXTM3U")
        || String::from_utf8_lossy(bytes).contains("<MPD")
}
fn direct_media(url: &Url, mime: &str, size: Option<u64>) -> Media {
    let extension = url
        .path()
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let audio = (mime.starts_with("audio/") && !mime.contains("mpegurl"))
        || ["mp3", "m4a", "ogg", "opus", "flac", "aac", "wav"].contains(&extension.as_str());
    let mut media = base_media(url);
    media.kind = if audio {
        MediaKind::Audio
    } else {
        MediaKind::Video
    };
    let mut stream =
        format(url, "source", url.as_str(), mime, audio).expect("already validated HTTP URL");
    stream.headers.insert("User-Agent".into(), APP_UA.into());
    stream.size = size;
    media.formats.push(stream);
    media
}
fn objects(value: &Value, depth: u8, visit: &mut impl FnMut(&Map<String, Value>)) {
    if depth >= 64 {
        return;
    }
    match value {
        Value::Object(map) => {
            visit(map);
            for value in map.values() {
                objects(value, depth + 1, visit)
            }
        }
        Value::Array(list) => {
            for value in list {
                objects(value, depth + 1, visit)
            }
        }
        _ => {}
    }
}
fn script_json(html: &str, id: &str) -> Option<Value> {
    // HTML serialization escapes signed URL ampersands; read the script's raw text.
    let document = Html::parse_document(html);
    let selector = Selector::parse("script").ok()?;
    for script in document.select(&selector) {
        if script.value().attr("id") == Some(id) {
            return serde_json::from_str(&script.text().collect::<String>()).ok();
        }
    }
    None
}
fn json_after(html: &str, marker: &str) -> Option<Value> {
    let tail = &html[html.find(marker)? + marker.len()..];
    let start = tail.find(['{', '['])?;
    serde_json::Deserializer::from_str(&tail[start..])
        .into_iter::<Value>()
        .next()?
        .ok()
}
fn capture(value: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(value)?
        .get(1)
        .map(|m| m.as_str().to_owned())
}
fn html_text(value: &str) -> String {
    Html::parse_fragment(value)
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join("")
}

impl Catalog {
    async fn vimeo(&self, url: &Url, _auth: &Auth) -> Result<Media> {
        let parts: Vec<_> = url.path_segments().into_iter().flatten().collect();
        ensure!(
            !parts
                .iter()
                .any(|part| ["showcase", "album", "channels", "groups"].contains(part)),
            "Vimeo: для коллекций нужен отдельный доступный список; поддерживается ссылка отдельного видео"
        );
        let id = parts
            .iter()
            .find(|part| part.chars().all(|c| c.is_ascii_digit()) && !part.is_empty())
            .ok_or_else(|| anyhow!("Vimeo: в ссылке нет идентификатора видео"))?;
        let mut endpoint = Url::parse(&format!("https://player.vimeo.com/video/{id}/config"))?;
        if let Some(hash) = url
            .query_pairs()
            .find(|(key, _)| key == "h")
            .map(|(_, value)| value.into_owned())
            .or_else(|| {
                parts
                    .iter()
                    .position(|part| part == id)
                    .and_then(|index| parts.get(index + 1))
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
            })
        {
            endpoint.query_pairs_mut().append_pair("h", &hash);
        }
        let data = self.get_json(endpoint).await?;
        let mut media = parse_vimeo(url, &data)?;
        let mut reachable = Vec::new();
        for mut stream in media.formats.drain(..) {
            if stream.protocol == Protocol::Http {
                reachable.push(stream);
                continue;
            }
            let kind = if stream.protocol == Protocol::Hls {
                "hls"
            } else {
                "dash"
            };
            let mut candidates = vec![stream.url.clone()];
            if let Some(cdns) = data["request"]["files"][kind]["cdns"].as_object() {
                for cdn in cdns.values() {
                    if let Some(address) = cdn["url"].as_str()
                        && !candidates.iter().any(|value| value == address)
                    {
                        candidates.push(address.to_owned())
                    }
                }
            }
            for address in candidates {
                let mut target = crate::validate_url(&address)?;
                let mut valid = false;
                for _ in 0..4 {
                    let mut request = self
                        .http
                        .get(target.clone())
                        .timeout(Duration::from_secs(8));
                    for (name, value) in &stream.headers {
                        request = request.header(name, value);
                    }
                    let Ok(response) = request.send().await else {
                        break;
                    };
                    if response.status().is_redirection() {
                        let Some(next) = response
                            .headers()
                            .get(reqwest::header::LOCATION)
                            .and_then(|value| value.to_str().ok())
                            .and_then(|value| target.join(value).ok())
                        else {
                            break;
                        };
                        target = crate::validate_url(next.as_str())?;
                        continue;
                    }
                    if !response.status().is_success() {
                        break;
                    }
                    let Ok(manifest) = body(response).await else {
                        break;
                    };
                    valid = if kind == "hls" {
                        manifest
                            .trim_start_matches('\u{feff}')
                            .starts_with("#EXTM3U")
                    } else {
                        manifest.contains("<MPD")
                    };
                    break;
                }
                if valid {
                    stream.url = target.to_string();
                    reachable.push(stream);
                    break;
                }
            }
        }
        media.formats = reachable;
        finish(media)
    }
    async fn dailymotion(&self, url: &Url, _auth: &Auth) -> Result<Media> {
        let parts: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|p| !p.is_empty())
            .collect();
        let video = parts.first().copied() == Some("video")
            || parts.get(1).copied() == Some("video")
            || url.host_str() == Some("dai.ly");
        if video {
            let id = id_from_path(url)
                .split('_')
                .next()
                .unwrap_or_default()
                .to_owned();
            ensure!(
                id.chars().all(|ch| ch.is_ascii_alphanumeric()),
                "Dailymotion: неверный идентификатор видео"
            );
            return parse_dailymotion(
                url,
                &self
                    .get_json(Url::parse(&format!(
                        "https://www.dailymotion.com/player/metadata/video/{id}"
                    ))?)
                    .await?,
            );
        }
        let (kind, id) = if parts.first().copied() == Some("playlist") {
            ("playlist", parts.get(1).copied().unwrap_or_default())
        } else {
            ("user", parts.first().copied().unwrap_or_default())
        };
        ensure!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'),
            "Dailymotion: неверная ссылка списка"
        );
        let mut media = base_media(url);
        media.kind = if kind == "playlist" {
            MediaKind::Playlist
        } else {
            MediaKind::Channel
        };
        for page in 1..=101 {
            let data = self
                .get_json(api_url(
                    &format!("https://api.dailymotion.com/{kind}/{id}/videos"),
                    &[
                        ("fields", "id,title,duration,thumbnail_url"),
                        ("limit", "100"),
                        ("page", &page.to_string()),
                    ],
                )?)
                .await?;
            ensure!(
                data.get("error").is_none(),
                "Dailymotion: список недоступен"
            );
            let entries = data["list"]
                .as_array()
                .ok_or_else(|| anyhow!("Dailymotion изменил формат списка"))?;
            for entry in entries {
                let id = text(entry, "id");
                if !id.is_empty() {
                    media.entries.push(Entry {
                        id: id.clone(),
                        url: format!("https://www.dailymotion.com/video/{id}"),
                        title: text(entry, "title"),
                        duration: number(&entry["duration"]),
                        thumbnail: text(entry, "thumbnail_url"),
                    });
                }
            }
            ensure!(
                media.entries.len() <= ENTRY_LIMIT,
                "Dailymotion: список превышает 10000 элементов"
            );
            if data["has_more"].as_bool() != Some(true) {
                return finish(media);
            }
            ensure!(
                !entries.is_empty(),
                "Dailymotion: пустая страница с продолжением списка"
            );
        }
        bail!("Dailymotion: список не выгружен полностью после 100 страниц")
    }
    async fn bilibili(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let mut source = url.clone();
        if source.host_str() == Some("b23.tv") {
            source = self.source_get(&source, url, auth).await?.url().clone();
            ensure!(
                domain(source.host_str().unwrap_or_default(), "bilibili.com"),
                "BiliBili: короткая ссылка ведёт на другой сервис"
            );
        }
        if source.path().contains("/bangumi/") {
            return self.bilibili_bangumi(&source).await;
        }
        let id = capture(source.path(), r"/(BV[A-Za-z0-9]+|av[0-9]+)").ok_or_else(|| {
            anyhow!("BiliBili: поддерживается отдельное видео или bangumi episode")
        })?;
        let id_key = if id.starts_with("BV") { "bvid" } else { "aid" };
        let id_value = if id_key == "aid" { &id[2..] } else { &id };
        let view = self
            .get_json(api_url(
                "https://api.bilibili.com/x/web-interface/view",
                &[(id_key, id_value)],
            )?)
            .await?;
        ensure!(
            view["code"].as_i64() == Some(0),
            "BiliBili: видео недоступно"
        );
        let data = &view["data"];
        let requested = source
            .query_pairs()
            .find(|(key, _)| key == "p")
            .and_then(|(_, value)| value.parse::<usize>().ok());
        let pages = data["pages"]
            .as_array()
            .ok_or_else(|| anyhow!("BiliBili изменил список частей видео"))?;
        if pages.len() > 1 && requested.is_none() {
            let mut media = bilibili_metadata(&source, data);
            media.kind = MediaKind::Playlist;
            ensure!(
                pages.len() <= ENTRY_LIMIT,
                "BiliBili: слишком много частей видео"
            );
            for page in pages {
                let index = uint(&page["page"]).unwrap_or(1);
                media.entries.push(Entry {
                    id: format!("{id}:p{index}"),
                    url: format!("https://www.bilibili.com/video/{id}?p={index}"),
                    title: text(page, "part"),
                    duration: number(&page["duration"]),
                    thumbnail: text(data, "pic"),
                });
            }
            return finish(media);
        }
        let index = requested.unwrap_or(1);
        ensure!(
            index > 0 && index <= pages.len(),
            "BiliBili: такой части видео нет"
        );
        let cid = uint(&pages[index - 1]["cid"])
            .ok_or_else(|| anyhow!("BiliBili: отсутствует cid"))?
            .to_string();
        let mut play = self
            .get_json(api_url(
                "https://api.bilibili.com/x/player/playurl",
                &[
                    (id_key, id_value),
                    ("cid", &cid),
                    ("qn", "127"),
                    ("fnval", "4048"),
                    ("fourk", "1"),
                ],
            )?)
            .await?;
        self.bilibili_delivery(&mut play, &source).await?;
        let mut media = parse_bilibili(&source, data, &play)?;
        if let Ok(subtitles) = self
            .get_json(api_url(
                "https://api.bilibili.com/x/player/v2",
                &[(id_key, id_value), ("cid", &cid)],
            )?)
            .await
            && let Some(list) = subtitles
                .pointer("/data/subtitle/subtitles")
                .and_then(Value::as_array)
        {
            for subtitle in list {
                let value = text(subtitle, "subtitle_url");
                if !value.is_empty() {
                    media.subtitles.push(Subtitle {
                        language: text(subtitle, "lan"),
                        title: text(subtitle, "lan_doc"),
                        url: media_address(&source, &value)?,
                        ext: "json".into(),
                        automatic: text(subtitle, "lan").starts_with("ai-"),
                        translatable: false,
                    });
                }
            }
        }
        Ok(media)
    }
    async fn bilibili_bangumi(&self, url: &Url) -> Result<Media> {
        let id = capture(url.path(), r"/(?:ep|ss)([0-9]+)")
            .ok_or_else(|| anyhow!("BiliBili: неверный episode/season"))?;
        let episode = url.path().contains("/ep");
        let key = if episode { "ep_id" } else { "season_id" };
        let data = self
            .get_json(api_url(
                "https://api.bilibili.com/pgc/view/web/season",
                &[(key, &id)],
            )?)
            .await?;
        ensure!(
            data["code"].as_i64() == Some(0),
            "BiliBili: сезон недоступен"
        );
        let episodes = data
            .pointer("/result/episodes")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("BiliBili: нет списка эпизодов"))?;
        if !episode {
            let mut media = base_media(url);
            media.title = text(&data["result"], "title");
            media.thumbnail = text(&data["result"], "cover");
            media.kind = MediaKind::Playlist;
            for entry in episodes {
                let id = uint(&entry["id"]).unwrap_or_default().to_string();
                media.entries.push(Entry {
                    id: id.clone(),
                    url: format!("https://www.bilibili.com/bangumi/play/ep{id}"),
                    title: format!("{} {}", text(entry, "title"), text(entry, "long_title")),
                    duration: number(&entry["duration"]).map(|v| v / 1000.),
                    thumbnail: text(entry, "cover"),
                });
            }
            return finish(media);
        }
        let entry = episodes
            .iter()
            .find(|entry| uint(&entry["id"]).map(|v| v.to_string()).as_deref() == Some(id.as_str()))
            .ok_or_else(|| anyhow!("BiliBili: эпизод не найден"))?;
        let mut play = self
            .get_json(api_url(
                "https://api.bilibili.com/pgc/player/web/playurl",
                &[
                    ("ep_id", &id),
                    ("fnval", "4048"),
                    ("qn", "127"),
                    ("fourk", "1"),
                ],
            )?)
            .await?;
        let metadata = json!({"bvid":text(entry,"bvid"),"title":format!("{} {}",text(entry,"title"),text(entry,"long_title")),"pic":text(entry,"cover"),"duration":number(&entry["duration"]).map(|v|v/1000.),"desc":text(&data["result"],"evaluate")});
        self.bilibili_delivery(&mut play, url).await?;
        parse_bilibili(url, &metadata, &play)
    }
}

fn parse_vimeo(origin: &Url, data: &Value) -> Result<Media> {
    let mut media = base_media(origin);
    let video = &data["video"];
    media.id = uint(&video["id"])
        .map(|v| v.to_string())
        .unwrap_or_else(|| id_from_path(origin));
    media.title = text(video, "title");
    media.duration = number(&video["duration"]);
    media.thumbnail = video["thumbs"]
        .as_object()
        .and_then(|m| {
            m.get("1280")
                .or_else(|| m.get("960"))
                .or_else(|| m.values().next())
        })
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    media.channel_title = text(&video["owner"], "name");
    if let Some(list) = data
        .pointer("/request/files/progressive")
        .and_then(Value::as_array)
    {
        for (index, item) in list.iter().enumerate() {
            let mut f = format(
                origin,
                &format!("http-{index}"),
                &text(item, "url"),
                "video/mp4",
                false,
            )?;
            f.width = uint(&item["width"]).map(|v| v as u32);
            f.height = uint(&item["height"]).map(|v| v as u32);
            f.fps = number(&item["fps"]);
            media.formats.push(f);
        }
    }
    for (kind, mime) in [
        ("hls", "application/vnd.apple.mpegurl"),
        ("dash", "application/dash+xml"),
    ] {
        let entry = &data["request"]["files"][kind];
        let cdn = text(entry, "default_cdn");
        let value = entry["cdns"]
            .get(&cdn)
            .or_else(|| entry["cdns"].as_object().and_then(|m| m.values().next()))
            .and_then(|v| v.get("url"))
            .and_then(Value::as_str);
        if let Some(value) = value {
            media
                .formats
                .push(format(origin, kind, value, mime, false)?);
        }
    }
    if let Some(list) = data
        .pointer("/request/text_tracks")
        .and_then(Value::as_array)
    {
        for subtitle in list {
            media.subtitles.push(Subtitle {
                language: text(subtitle, "lang"),
                title: text(subtitle, "label"),
                url: media_address(origin, &text(subtitle, "url"))?,
                ext: "vtt".into(),
                automatic: false,
                translatable: false,
            });
        }
    }
    finish(media)
}
fn parse_dailymotion(origin: &Url, data: &Value) -> Result<Media> {
    ensure!(data.get("error").is_none(), "Dailymotion: видео недоступно");
    let mut media = base_media(origin);
    media.id = text(data, "id");
    media.title = text(data, "title");
    media.description = text(data, "description");
    media.duration = number(&data["duration"]);
    media.channel_title = text(&data["owner"], "screenname");
    media.thumbnail = data["thumbnails"]
        .as_object()
        .and_then(|m| m.get("720").or_else(|| m.values().next()))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if let Some(qualities) = data["qualities"].as_object() {
        for (quality, list) in qualities {
            if let Some(list) = list.as_array() {
                for (index, item) in list.iter().enumerate() {
                    let value = text(item, "url");
                    if value.is_empty() {
                        continue;
                    }
                    let mut f = format(
                        origin,
                        &format!("{quality}-{index}"),
                        &value,
                        &text(item, "type"),
                        false,
                    )?;
                    f.height = quality.parse().ok();
                    media.formats.push(f);
                }
            }
        }
    }
    if let Some(list) = data.pointer("/subtitles/data").and_then(Value::as_object) {
        for (language, item) in list {
            if let Some(url) = item["urls"]
                .as_array()
                .and_then(|list| list.first())
                .and_then(Value::as_str)
            {
                media.subtitles.push(Subtitle {
                    language: language.clone(),
                    title: text(item, "label"),
                    url: media_address(origin, url)?,
                    ext: "srt".into(),
                    automatic: false,
                    translatable: false,
                });
            }
        }
    }
    finish(media)
}
fn bilibili_metadata(origin: &Url, data: &Value) -> Media {
    let mut media = base_media(origin);
    media.id = text(data, "bvid");
    media.title = text(data, "title");
    media.description = text(data, "desc");
    media.thumbnail = text(data, "pic");
    media.duration = number(&data["duration"]);
    media.channel_id = uint(&data["owner"]["mid"])
        .map(|v| v.to_string())
        .unwrap_or_default();
    media.channel_title = text(&data["owner"], "name");
    media
}
fn parse_bilibili(origin: &Url, metadata: &Value, response: &Value) -> Result<Media> {
    ensure!(
        response["code"].as_i64() == Some(0),
        "BiliBili: медиапотоки недоступны (возможны регион, Premium или вход)"
    );
    let data = response
        .get("data")
        .or_else(|| response.get("result"))
        .ok_or_else(|| anyhow!("BiliBili: отсутствуют данные потоков"))?;
    let mut media = bilibili_metadata(origin, metadata);
    for (kind, audio) in [("video", false), ("audio", true)] {
        if let Some(list) = data["dash"][kind].as_array() {
            for (index, item) in list.iter().enumerate() {
                let value = item
                    .get("baseUrl")
                    .or_else(|| item.get("base_url"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if value.is_empty() {
                    continue;
                }
                let mime = item
                    .get("mimeType")
                    .or_else(|| item.get("mime_type"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let mut f = format(
                    origin,
                    &format!("{kind}-{}-{index}", uint(&item["id"]).unwrap_or_default()),
                    value,
                    mime,
                    audio,
                )?;
                f.width = uint(&item["width"]).map(|v| v as u32);
                f.height = uint(&item["height"]).map(|v| v as u32);
                f.bitrate = uint(&item["bandwidth"]);
                f.fps = item
                    .get("frameRate")
                    .or_else(|| item.get("frame_rate"))
                    .and_then(Value::as_str)
                    .and_then(|s| {
                        if let Some((n, d)) = s.split_once('/') {
                            Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?)
                        } else {
                            s.parse().ok()
                        }
                    });
                let codec = text(item, "codecs");
                if audio {
                    f.audio_codec = Some(codec);
                    f.video_codec = None
                } else {
                    f.video_codec = Some(codec);
                    f.audio_codec = None
                }
                media.formats.push(f);
            }
        }
    }
    if let Some(list) = data["durl"].as_array() {
        ensure!(
            list.len() <= 1,
            "BiliBili вернул несколько последовательных сегментов; требуется отдельная сборка segmented FLV"
        );
        for item in list {
            let mut f = format(
                origin,
                "progressive",
                &text(item, "url"),
                "video/mp4",
                false,
            )?;
            f.size = uint(&item["size"]);
            media.formats.push(f);
        }
    }
    for stream in &mut media.formats {
        stream
            .headers
            .insert("User-Agent".into(), "VideoDock/0.2".into());
        // The CDN rejects full GETs without Range even when its byte probes succeed.
        stream.headers.insert("Range".into(), "bytes=0-".into());
    }
    finish(media)
}

impl Catalog {
    async fn soundcloud(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let page = body(self.source_get(url, url, auth).await?).await?;
        let hydration = json_after(&page, "window.__sc_hydration")
            .ok_or_else(|| anyhow!("SoundCloud изменил hydration документ"))?;
        let mut client_id = hydration
            .as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["hydratable"] == "apiClient")
            .map(|entry| text(&entry["data"], "id"))
            .unwrap_or_default();
        let expiring = hydration
            .as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["hydratable"] == "apiClient")
            .and_then(|entry| entry["data"]["isExpiring"].as_bool())
            .unwrap_or(false);
        if client_id.is_empty() || expiring {
            let scripts = {
                let document = Html::parse_document(&page);
                let selector = Selector::parse("script[src]").unwrap();
                document
                    .select(&selector)
                    .filter_map(|script| script.value().attr("src"))
                    .filter_map(|src| url.join(src).ok())
                    .filter(|src| domain(src.host_str().unwrap_or_default(), "sndcdn.com"))
                    .collect::<Vec<_>>()
            };
            for script in scripts.iter().take(8) {
                let script_text = body(
                    self.source_get(
                        script,
                        &Url::parse("https://soundcloud.com/")?,
                        &Auth::default(),
                    )
                    .await?,
                )
                .await?;
                if let Some(value) =
                    capture(&script_text, r#"client_id\s*:\s*["']([A-Za-z0-9]{32})["']"#)
                {
                    client_id = value;
                    break;
                }
            }
        }
        ensure!(
            !client_id.is_empty(),
            "SoundCloud: публичный идентификатор текущего клиента не найден"
        );
        let data = self
            .get_json(api_url(
                "https://api-v2.soundcloud.com/resolve",
                &[("url", url.as_str()), ("client_id", &client_id)],
            )?)
            .await?;
        match data["kind"].as_str() {
            Some("track") => self.soundcloud_track(url, &data, &client_id).await,
            Some("playlist") => {
                let mut media = soundcloud_metadata(url, &data);
                media.kind = MediaKind::Playlist;
                let list = data["tracks"]
                    .as_array()
                    .ok_or_else(|| anyhow!("SoundCloud: нет списка дорожек"))?;
                ensure!(
                    list.len() <= ENTRY_LIMIT,
                    "SoundCloud: плейлист превышает 10000 дорожек"
                );
                for batch in list.chunks(50) {
                    if batch.iter().all(|entry| entry["permalink_url"].is_string()) {
                        media.entries.extend(batch.iter().map(soundcloud_entry));
                    } else {
                        let ids = batch
                            .iter()
                            .filter_map(|entry| uint(&entry["id"]))
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(",");
                        let tracks = self
                            .get_json(api_url(
                                "https://api-v2.soundcloud.com/tracks",
                                &[("ids", &ids), ("client_id", &client_id)],
                            )?)
                            .await?;
                        let tracks = tracks.as_array().ok_or_else(|| {
                            anyhow!("SoundCloud: не удалось разрешить дорожки плейлиста")
                        })?;
                        ensure!(
                            tracks.len() == batch.len(),
                            "SoundCloud: часть дорожек плейлиста недоступна"
                        );
                        media.entries.extend(tracks.iter().map(soundcloud_entry));
                    }
                }
                finish(media)
            }
            Some("user") => {
                let id = uint(&data["id"])
                    .ok_or_else(|| anyhow!("SoundCloud: нет идентификатора автора"))?
                    .to_string();
                let mut media = soundcloud_metadata(url, &data);
                media.kind = MediaKind::Channel;
                media.title = text(&data, "username");
                let mut next = Some(api_url(
                    &format!("https://api-v2.soundcloud.com/users/{id}/tracks"),
                    &[
                        ("client_id", &client_id),
                        ("limit", "200"),
                        ("linked_partitioning", "1"),
                    ],
                )?);
                let mut seen = HashSet::new();
                while let Some(endpoint) = next.take() {
                    ensure!(
                        seen.insert(endpoint.as_str().to_owned()),
                        "SoundCloud повторил страницу"
                    );
                    ensure!(
                        endpoint.host_str() == Some("api-v2.soundcloud.com"),
                        "SoundCloud: продолжение ведёт на неизвестный API"
                    );
                    let page = self.get_json(endpoint).await?;
                    let tracks = page["collection"]
                        .as_array()
                        .ok_or_else(|| anyhow!("SoundCloud изменил формат списка"))?;
                    media.entries.extend(tracks.iter().map(soundcloud_entry));
                    ensure!(
                        media.entries.len() <= ENTRY_LIMIT,
                        "SoundCloud: канал превышает 10000 дорожек"
                    );
                    next = page["next_href"].as_str().map(Url::parse).transpose()?;
                    if let Some(endpoint) = next.as_mut()
                        && !endpoint.query_pairs().any(|(key, _)| key == "client_id")
                    {
                        endpoint
                            .query_pairs_mut()
                            .append_pair("client_id", &client_id);
                    }
                }
                finish(media)
            }
            _ => bail!("SoundCloud: этот тип ссылки не поддерживается"),
        }
    }
    async fn soundcloud_track(&self, url: &Url, data: &Value, client_id: &str) -> Result<Media> {
        ensure!(
            data["policy"]
                .as_str()
                .is_none_or(|policy| matches!(policy, "ALLOW" | "MONETIZE")),
            "SoundCloud: полная дорожка недоступна без дополнительного доступа"
        );
        let mut media = soundcloud_metadata(url, data);
        media.kind = MediaKind::Audio;
        let list = data
            .pointer("/media/transcodings")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("SoundCloud: нет доступных транскодирований"))?;
        for (index, transcode) in list.iter().enumerate() {
            let mut endpoint = crate::validate_url(&text(transcode, "url"))?;
            ensure!(
                domain(endpoint.host_str().unwrap_or_default(), "soundcloud.com"),
                "SoundCloud: неизвестный API транскодирования"
            );
            endpoint
                .query_pairs_mut()
                .append_pair("client_id", client_id);
            let authorization = text(data, "track_authorization");
            if !authorization.is_empty() {
                endpoint
                    .query_pairs_mut()
                    .append_pair("track_authorization", &authorization);
            }
            let Ok(resolved) = self.get_json(endpoint).await else {
                continue;
            };
            let value = text(&resolved, "url");
            if value.is_empty() {
                continue;
            }
            let mime = text(&transcode["format"], "mime_type");
            let mut stream = format(url, &format!("audio-{index}"), &value, &mime, true)?;
            if transcode["format"]["protocol"] == "hls" {
                stream.protocol = Protocol::Hls
            };
            media.formats.push(stream);
        }
        finish(media)
    }
    async fn twitch_gql(&self, client_id: &str, query: &str, variables: Value) -> Result<Value> {
        let response = self
            .http
            .post("https://gql.twitch.tv/gql")
            .header("Client-ID", client_id)
            .json(&json!({"query":query,"variables":variables}))
            .send()
            .await
            .map_err(|_| anyhow!("Twitch: не удалось выполнить запрос"))?;
        let data = json_response(response).await?;
        twitch_data(&data)
    }
    async fn twitch(&self, url: &Url) -> Result<Media> {
        let page = body(
            self.source_get(
                &Url::parse("https://www.twitch.tv/")?,
                url,
                &Auth::default(),
            )
            .await?,
        )
        .await?;
        let client_id = capture(&page, r#"clientId["']?\s*[:=]\s*["']([A-Za-z0-9]+)["']"#)
            .ok_or_else(|| anyhow!("Twitch: не найден идентификатор текущего веб-клиента"))?;
        let parts: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        if url.host_str() == Some("clips.twitch.tv") || parts.contains(&"clip") {
            let slug = id_from_path(url);
            let data=self.twitch_gql(&client_id,"query($slug:ID!){clip(slug:$slug){slug title durationSeconds thumbnailURL broadcaster{id displayName} videoQualities{frameRate quality sourceURL} playbackAccessToken(params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){signature value}}}",json!({"slug":slug})).await?;
            return parse_twitch_clip(url, &data["clip"]);
        }
        if parts.first().copied() == Some("videos") {
            let id = parts.get(1).copied().unwrap_or_default();
            ensure!(
                !id.is_empty() && id.chars().all(|ch| ch.is_ascii_digit()),
                "Twitch: неверный VOD id"
            );
            let data=self.twitch_gql(&client_id,"query($id:ID!){video(id:$id){id title lengthSeconds previewThumbnailURL owner{id displayName}} videoPlaybackAccessToken(id:$id,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}",json!({"id":id})).await?;
            return parse_twitch_vod(url, &data);
        }
        let login = parts.first().copied().unwrap_or_default();
        ensure!(
            !login.is_empty()
                && login
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
            "Twitch: нужна ссылка канала, VOD или клипа"
        );
        if parts.len() == 1 {
            let data=self.twitch_gql(&client_id,"query($login:String!){user(login:$login){id displayName stream{id title previewImageURL(width:1280,height:720)}} streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}",json!({"login":login})).await?;
            if data["user"]["stream"].is_object() {
                let mut media = base_media(url);
                media.title = text(&data["user"]["stream"], "title");
                media.thumbnail = text(&data["user"]["stream"], "previewImageURL");
                media.channel_id = text(&data["user"], "id");
                media.channel_title = text(&data["user"], "displayName");
                let token = &data["streamPlaybackAccessToken"];
                ensure!(
                    token["value"].is_string() && token["signature"].is_string(),
                    "Twitch: нет разрешения воспроизведения"
                );
                let endpoint = api_url(
                    &format!("https://usher.ttvnw.net/api/channel/hls/{login}.m3u8"),
                    &[
                        ("token", &text(token, "value")),
                        ("sig", &text(token, "signature")),
                        ("allow_source", "true"),
                        ("allow_audio_only", "true"),
                    ],
                )?;
                media.formats.push(format(
                    url,
                    "live",
                    endpoint.as_str(),
                    "application/vnd.apple.mpegurl",
                    false,
                )?);
                return finish(media);
            }
        }
        let mut media = base_media(url);
        media.kind = MediaKind::Channel;
        let mut cursor: Option<String> = None;
        let mut cursors = HashSet::new();
        loop {
            let data=self.twitch_gql(&client_id,"query($login:String!,$after:Cursor){user(login:$login){id displayName videos(first:100,after:$after){edges{cursor node{id title lengthSeconds previewThumbnailURL}} pageInfo{hasNextPage}}}}",json!({"login":login,"after":cursor})).await?;
            let user = &data["user"];
            ensure!(user.is_object(), "Twitch: канал не найден");
            media.title = text(user, "displayName");
            media.channel_title = media.title.clone();
            media.channel_id = text(user, "id");
            let edges = user["videos"]["edges"]
                .as_array()
                .ok_or_else(|| anyhow!("Twitch: список VOD недоступен"))?;
            for edge in edges {
                let item = &edge["node"];
                let id = text(item, "id");
                media.entries.push(Entry {
                    id: id.clone(),
                    url: format!("https://www.twitch.tv/videos/{id}"),
                    title: text(item, "title"),
                    duration: number(&item["lengthSeconds"]),
                    thumbnail: text(item, "previewThumbnailURL"),
                });
            }
            ensure!(
                media.entries.len() <= ENTRY_LIMIT,
                "Twitch: канал превышает 10000 VOD"
            );
            if user["videos"]["pageInfo"]["hasNextPage"] != true {
                break;
            }
            cursor = edges
                .last()
                .and_then(|edge| edge["cursor"].as_str())
                .map(str::to_owned);
            ensure!(
                cursor
                    .as_ref()
                    .is_some_and(|cursor| cursors.insert(cursor.clone())),
                "Twitch: неверное продолжение списка VOD"
            );
        }
        finish(media)
    }
}
fn soundcloud_metadata(origin: &Url, data: &Value) -> Media {
    let mut media = base_media(origin);
    media.id = uint(&data["id"])
        .map(|id| id.to_string())
        .unwrap_or_default();
    media.title = text(data, "title");
    media.description = text(data, "description");
    media.duration = number(&data["full_duration"])
        .or_else(|| number(&data["duration"]))
        .map(|v| v / 1000.);
    media.thumbnail = text(data, "artwork_url");
    media.channel_id = uint(&data["user"]["id"])
        .map(|id| id.to_string())
        .unwrap_or_default();
    media.channel_title = text(&data["user"], "username");
    media
}
fn soundcloud_entry(data: &Value) -> Entry {
    Entry {
        id: uint(&data["id"])
            .map(|id| id.to_string())
            .unwrap_or_default(),
        url: text(data, "permalink_url"),
        title: text(data, "title"),
        duration: number(&data["duration"]).map(|v| v / 1000.),
        thumbnail: text(data, "artwork_url"),
    }
}
fn parse_twitch_vod(origin: &Url, data: &Value) -> Result<Media> {
    let video = &data["video"];
    ensure!(video.is_object(), "Twitch: VOD недоступен");
    let token = &data["videoPlaybackAccessToken"];
    ensure!(
        token["value"].is_string() && token["signature"].is_string(),
        "Twitch: нет разрешения воспроизведения VOD"
    );
    let mut media = base_media(origin);
    media.id = text(video, "id");
    media.title = text(video, "title");
    media.duration = number(&video["lengthSeconds"]);
    media.thumbnail = text(video, "previewThumbnailURL");
    media.channel_id = text(&video["owner"], "id");
    media.channel_title = text(&video["owner"], "displayName");
    let url = api_url(
        &format!("https://usher.ttvnw.net/vod/{}.m3u8", media.id),
        &[
            ("nauth", &text(token, "value")),
            ("nauthsig", &text(token, "signature")),
            ("allow_source", "true"),
            ("allow_audio_only", "true"),
        ],
    )?;
    media.formats.push(format(
        origin,
        "vod",
        url.as_str(),
        "application/vnd.apple.mpegurl",
        false,
    )?);
    finish(media)
}
fn parse_twitch_clip(origin: &Url, data: &Value) -> Result<Media> {
    ensure!(data.is_object(), "Twitch: клип недоступен");
    let mut media = base_media(origin);
    media.id = text(data, "slug");
    media.title = text(data, "title");
    media.duration = number(&data["durationSeconds"]);
    media.thumbnail = text(data, "thumbnailURL");
    media.channel_id = text(&data["broadcaster"], "id");
    media.channel_title = text(&data["broadcaster"], "displayName");
    if let Some(list) = data["videoQualities"].as_array() {
        for (index, item) in list.iter().enumerate() {
            let mut url = crate::validate_url(&text(item, "sourceURL"))?;
            let token = &data["playbackAccessToken"];
            if token["value"].is_string() && token["signature"].is_string() {
                url.query_pairs_mut()
                    .append_pair("sig", &text(token, "signature"))
                    .append_pair("token", &text(token, "value"));
            }
            let mut stream = format(
                origin,
                &format!("clip-{index}"),
                url.as_str(),
                "video/mp4",
                false,
            )?;
            stream.height = item["quality"].as_str().and_then(|v| v.parse().ok());
            stream.fps = number(&item["frameRate"]);
            media.formats.push(stream);
        }
    }
    finish(media)
}

impl Catalog {
    async fn rumble(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let embed = if url.path().starts_with("/embed/") {
            Some(id_from_path(url))
        } else {
            None
        };
        let id = if let Some(id) = embed {
            id
        } else {
            let page = body(self.source_get(url, url, auth).await?).await?;
            if url.path().starts_with("/c/")
                || url.path().starts_with("/user/")
                || url.path().starts_with("/channel/")
            {
                return self.rumble_channel(url, auth, &page).await;
            }
            capture(
                &page,
                r#"(?:rumble\.com/embed/|"video"\s*:\s*")([A-Za-z0-9]+)"#,
            )
            .ok_or_else(|| anyhow!("Rumble: embed id не найден; требуется доступ к странице"))?
        };
        ensure!(
            id.chars().all(|ch| ch.is_ascii_alphanumeric()),
            "Rumble: неверный embed id"
        );
        let endpoint = api_url(
            "https://rumble.com/embedJS/u3/",
            &[("request", "video"), ("ver", "2"), ("v", &id)],
        )?;
        let response = self
            .http
            .get(endpoint)
            .header("User-Agent", "VideoDock/0.2")
            .header("Referer", "https://rumble.com/")
            .send()
            .await
            .map_err(|_| anyhow!("Rumble: запрос публичного embed API не выполнен"))?;
        let mut media = parse_rumble(url, &json_response(response).await?)?;
        for format in &mut media.formats {
            format
                .headers
                .insert("User-Agent".into(), "VideoDock/0.2".into());
        }
        Ok(media)
    }
    async fn rumble_channel(&self, url: &Url, auth: &Auth, first: &str) -> Result<Media> {
        let mut media = base_media(url);
        media.kind = MediaKind::Channel;
        let mut current = url.clone();
        let mut page = first.to_owned();
        let mut seen = HashSet::new();
        let mut pages = HashSet::new();
        loop {
            ensure!(
                pages.insert(current.as_str().to_owned()),
                "Rumble повторил страницу канала"
            );
            let (entries, next) = {
                let document = Html::parse_document(&page);
                let selector = Selector::parse(
                    ".video-item a[href], .video-listing-entry a[href], article a[href]",
                )
                .unwrap();
                let mut entries = Vec::new();
                for link in document.select(&selector) {
                    if let Some(target) = link
                        .value()
                        .attr("href")
                        .and_then(|value| url.join(value).ok())
                        .filter(|value| {
                            same_origin(value, url)
                                && value.path().starts_with("/v")
                                && value.path().ends_with(".html")
                        })
                        && seen.insert(target.to_string())
                    {
                        entries.push(Entry {
                            id: id_from_path(&target),
                            url: target.to_string(),
                            title: link
                                .value()
                                .attr("title")
                                .map(str::to_owned)
                                .unwrap_or_else(|| {
                                    link.text().collect::<Vec<_>>().join(" ").trim().to_owned()
                                }),
                            ..Default::default()
                        });
                    }
                }
                let next_selector = Selector::parse(
                    "a[rel=next], .pagination a.pagination__button--next, a.paginator--next",
                )
                .unwrap();
                let next = document
                    .select(&next_selector)
                    .next()
                    .and_then(|link| link.value().attr("href"))
                    .and_then(|value| url.join(value).ok());
                (entries, next)
            };
            ensure!(
                !entries.is_empty() || next.is_none(),
                "Rumble: страница канала не содержит распознанных видео"
            );
            media.entries.extend(entries);
            ensure!(
                media.entries.len() <= ENTRY_LIMIT,
                "Rumble: канал превышает 10000 видео"
            );
            let Some(next) = next else { break };
            ensure!(
                same_origin(&next, url),
                "Rumble: продолжение ведёт на другой сайт"
            );
            current = next;
            page = body(self.source_get(&current, url, auth).await?).await?;
        }
        finish(media)
    }
    async fn bitchute(&self, url: &Url) -> Result<Media> {
        let parts: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|p| !p.is_empty())
            .collect();
        let id = parts.get(1).copied().unwrap_or_default();
        ensure!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'),
            "BitChute: неверная ссылка видео или канала"
        );
        if parts.first().copied() == Some("channel") {
            let metadata = self
                .post_json(
                    "https://api.bitchute.com/api/beta9/channel",
                    &json!({"channel_id":id}),
                )
                .await?;
            let mut media = base_media(url);
            media.kind = MediaKind::Channel;
            media.title = text(&metadata, "channel_name");
            media.channel_id = id.into();
            media.channel_title = media.title.clone();
            let mut offset = 0;
            loop {
                let response=self.post_json("https://api.bitchute.com/api/beta/channel/videos",&json!({"channel_id":id,"offset":offset,"limit":100,"order_by":"new","state_id":"published"})).await?;
                let list = response["videos"]
                    .as_array()
                    .ok_or_else(|| anyhow!("BitChute изменил список видео канала"))?;
                media.entries.extend(list.iter().map(bitchute_entry));
                offset += list.len();
                ensure!(
                    media.entries.len() <= ENTRY_LIMIT,
                    "BitChute: канал превышает 10000 видео"
                );
                if list.len() < 100 {
                    break;
                }
            }
            return finish(media);
        }
        ensure!(
            matches!(parts.first().copied(), Some("video" | "embed")),
            "BitChute: этот тип ссылки не поддерживается"
        );
        let metadata = self
            .post_json(
                "https://api.bitchute.com/api/beta9/video",
                &json!({"video_id":id}),
            )
            .await?;
        let payload = self
            .post_json(
                "https://api.bitchute.com/api/beta/video/media",
                &json!({"video_id":id}),
            )
            .await?;
        parse_bitchute(url, &metadata, &payload)
    }
    async fn niconico(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let response = self.source_get(url, url, auth).await?;
        let origin = response.url().clone();
        let page = body(response).await?;
        let data = nico_metadata(&page)
            .ok_or_else(|| anyhow!("NicoVideo: текущий server-response не найден"))?;
        let mut media = parse_nico_metadata(&origin, &data);
        let id = text(&data["video"], "id");
        ensure!(
            !id.is_empty() && id.chars().all(|ch| ch.is_ascii_alphanumeric()),
            "NicoVideo: неверный идентификатор видео"
        );
        let videos = data
            .pointer("/media/contents/videos")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("NicoVideo: нет доступных видеодорожек"))?;
        let audios = data
            .pointer("/media/contents/audios")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("NicoVideo: нет доступных аудиодорожек"))?;
        let mut outputs = Vec::new();
        for video in videos.iter().filter(|item| item["isAvailable"] == true) {
            for audio in audios.iter().filter(|item| item["isAvailable"] == true) {
                outputs.push(json!([text(video, "id"), text(audio, "id")]));
            }
        }
        ensure!(
            !outputs.is_empty() && outputs.len() <= 128,
            "NicoVideo: нет разрешённых комбинаций дорожек или их слишком много"
        );
        let key = text(&data["media"], "accessRightKey");
        ensure!(
            !key.is_empty(),
            "NicoVideo: нет разрешения access-right; требуется вход или доступ к видео"
        );
        let endpoint = api_url(
            &format!("https://nvapi.nicovideo.jp/v1/watch/{id}/access-rights/hls"),
            &[("actionTrackId", &text(&data["client"], "watchTrackId"))],
        )?;
        let response = self
            .http
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("X-Request-With", "https://www.nicovideo.jp")
            .header("X-Frontend-Id", "6")
            .header("X-Frontend-Version", "0")
            .header("X-Access-Right-Key", key)
            .header("Origin", "https://www.nicovideo.jp")
            .header("Referer", "https://www.nicovideo.jp/")
            .json(&json!({"outputs":outputs}))
            .send()
            .await
            .map_err(|_| anyhow!("NicoVideo: запрос доступа к HLS не выполнен"))?;
        let session = issued_cookie(
            response.url(),
            response.headers(),
            "domand_bid",
            "nicovideo.jp",
        );
        let payload = json_response(response).await?;
        let address = payload
            .pointer("/data/contentUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("NicoVideo: HLS не разрешён"))?;
        let target = crate::validate_url(address)?;
        ensure!(
            target.scheme() == "https"
                && domain(target.host_str().unwrap_or_default(), "domand.nicovideo.jp"),
            "NicoVideo: неизвестный CDN доставки"
        );
        let mut stream = format(
            &origin,
            "hls",
            target.as_str(),
            "application/vnd.apple.mpegurl",
            false,
        )?;
        if let Some(session) = session {
            stream.cookies.push(session);
        }
        media.formats.push(stream);
        finish(media)
    }
    async fn likee(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let response = self.source_get(url, url, auth).await?;
        let origin = response.url().clone();
        let page = body(response).await?;
        let id = capture(origin.path(), r"/(?:video/)?([0-9]{8,})(?:/)?$");
        if let Some(id) = id {
            let response = self
                .post_json(
                    "https://api.like-video.com/likee-activity-flow-micro/videoApi/getVideoInfo",
                    &json!({"postIds":[id]}),
                )
                .await?;
            return parse_likee(&origin, &response, &page);
        }
        let generic = parse_html(&origin, &page)?;
        if !generic.formats.is_empty() {
            return finish(generic);
        }
        bail!(
            "Likee: профиль или ссылка не предоставили публичный post id; профильная пагинация требует отдельной проверки API"
        )
    }
    async fn naver(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let page = body(self.source_get(url, url, auth).await?).await?;
        let (video_id,key)=naver_ticket(&page).ok_or_else(||anyhow!("NaverTV: публичный playback ticket не найден; новая Clip-схема или вход требуют отдельной поддержки"))?;
        ensure!(
            video_id.chars().all(|ch| ch.is_ascii_alphanumeric()),
            "NaverTV: неверный video id"
        );
        let endpoint = api_url(
            &format!("https://apis.naver.com/rmcnmv/rmcnmv/vod/play/v2.0/{video_id}"),
            &[("key", &key)],
        )?;
        parse_naver(url, &self.get_json(endpoint).await?)
    }
}
fn parse_rumble(origin: &Url, data: &Value) -> Result<Media> {
    let mut media = base_media(origin);
    media.id = uint(&data["vid"])
        .map(|v| v.to_string())
        .unwrap_or_else(|| id_from_path(origin));
    media.title = text(data, "title");
    media.duration = number(&data["duration"]);
    media.thumbnail = text(data, "i");
    media.channel_title = text(&data["author"], "name");
    if let Some(types) = data["ua"].as_object() {
        for (kind, qualities) in types {
            if let Some(qualities) = qualities.as_object() {
                for (quality, item) in qualities {
                    let value = text(item, "url");
                    if value.is_empty() {
                        continue;
                    }
                    let mime = match kind.as_str() {
                        "hls" => "application/vnd.apple.mpegurl",
                        "dash" => "application/dash+xml",
                        "webm" => "video/webm",
                        _ => "video/mp4",
                    };
                    let mut stream =
                        format(origin, &format!("{kind}-{quality}"), &value, mime, false)?;
                    stream.height = uint(&item["meta"]["h"])
                        .or_else(|| quality.parse().ok())
                        .map(|v| v as u32);
                    stream.width = uint(&item["meta"]["w"]).map(|v| v as u32);
                    stream.bitrate = uint(&item["meta"]["bitrate"]);
                    stream.size = uint(&item["meta"]["size"]);
                    media.formats.push(stream);
                }
            }
        }
    }
    if let Some(captions) = data["cc"].as_array() {
        for caption in captions {
            let value = text(caption, "url");
            if !value.is_empty() {
                media.subtitles.push(Subtitle {
                    language: text(caption, "lang"),
                    title: text(caption, "name"),
                    url: media_address(origin, &value)?,
                    ext: "vtt".into(),
                    ..Default::default()
                });
            }
        }
    }
    finish(media)
}
fn bitchute_entry(data: &Value) -> Entry {
    let id = text(data, "video_id");
    Entry {
        id: id.clone(),
        url: format!("https://www.bitchute.com/video/{id}"),
        title: text(data, "video_name"),
        duration: duration(&data["duration"]),
        thumbnail: text(data, "thumbnail_url"),
    }
}
fn parse_bitchute(origin: &Url, metadata: &Value, payload: &Value) -> Result<Media> {
    let mut media = base_media(origin);
    media.id = text(metadata, "video_id");
    media.title = text(metadata, "video_name");
    media.description = text(metadata, "description");
    media.thumbnail = text(metadata, "thumbnail_url");
    media.duration = duration(&metadata["duration"]);
    media.channel_id = text(&metadata["channel"], "channel_id");
    media.channel_title = text(&metadata["channel"], "channel_name");
    let address = text(payload, "media_url");
    ensure!(!address.is_empty(), "BitChute: медиапоток недоступен");
    media.formats.push(format(
        origin,
        "source",
        &address,
        &text(payload, "media_type"),
        false,
    )?);
    finish(media)
}
fn nico_metadata(html: &str) -> Option<Value> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("meta[name=server-response]").ok()?;
    let content = document.select(&selector).next()?.value().attr("content")?;
    let root: Value = serde_json::from_str(content).ok()?;
    root.pointer("/data/response/$watchV4/data")
        .or_else(|| root.pointer("/data/response"))
        .cloned()
}
fn parse_nico_metadata(origin: &Url, data: &Value) -> Media {
    let mut media = base_media(origin);
    media.id = text(&data["video"], "id");
    media.title = text(&data["video"], "title");
    media.description = html_text(&text(&data["video"], "description"));
    media.duration = number(&data["video"]["duration"]);
    media.thumbnail = text(&data["video"]["thumbnail"], "player");
    if media.thumbnail.is_empty() {
        media.thumbnail = text(&data["video"]["thumbnail"], "url")
    };
    media
}
fn parse_likee(origin: &Url, data: &Value, html: &str) -> Result<Media> {
    ensure!(
        data["code"].as_i64().is_none_or(|code| code == 0),
        "Likee: видео недоступно или изменился API"
    );
    let mut media = base_media(origin);
    let mut found = None;
    objects(&data["data"], 0, &mut |object| {
        if found.is_none() && object.get("videoUrl").and_then(Value::as_str).is_some() {
            found = Some(Value::Object(object.clone()));
        }
    });
    let item = found.ok_or_else(|| anyhow!("Likee: нет публичного videoUrl"))?;
    media.id = text(&item, "postId");
    media.title = text(&item, "msg");
    if media.title.is_empty() {
        media.title = meta(html, "og:title").unwrap_or_default()
    };
    media.thumbnail = text(&item, "coverUrl");
    media.duration =
        number(&item["duration"]).or_else(|| number(&item["videoTime"]).map(|value| value / 1000.));
    media.formats.push(format(
        origin,
        "source",
        &text(&item, "videoUrl"),
        "video/mp4",
        false,
    )?);
    finish(media)
}
fn naver_ticket(html: &str) -> Option<(String, String)> {
    let video_id = capture(
        html,
        r#"["'](?:videoId|vid)["']\s*:\s*["']([A-Za-z0-9]+)["']"#,
    )?;
    let key = capture(html, r#"["'](?:inKey|inkey)["']\s*:\s*["']([^"']+)["']"#)?;
    Some((video_id, key))
}
fn parse_naver(origin: &Url, data: &Value) -> Result<Media> {
    let mut media = base_media(origin);
    media.title = text(&data["meta"], "subject");
    media.thumbnail = text(&data["meta"]["cover"], "source");
    media.duration = number(&data["meta"]["duration"]);
    if let Some(list) = data.pointer("/videos/list").and_then(Value::as_array) {
        for (index, item) in list.iter().enumerate() {
            let mut stream = format(
                origin,
                &format!("http-{index}"),
                &text(item, "source"),
                "video/mp4",
                false,
            )?;
            stream.width = uint(&item["width"]).map(|v| v as u32);
            stream.height = uint(&item["height"]).map(|v| v as u32);
            stream.bitrate = number(&item["videoBitrate"]).map(|value| (value * 1000.) as u64);
            stream.size = uint(&item["size"]);
            media.duration = media.duration.or_else(|| number(&item["duration"]));
            media.formats.push(stream);
        }
    }
    if let Some(list) = data.pointer("/streams/list").and_then(Value::as_array) {
        for (index, item) in list.iter().enumerate() {
            let value = text(item, "source");
            if !value.is_empty() {
                media.formats.push(format(
                    origin,
                    &format!("hls-{index}"),
                    &value,
                    "application/vnd.apple.mpegurl",
                    false,
                )?);
            }
        }
    }
    if let Some(list) = data.pointer("/captions/list").and_then(Value::as_array) {
        for item in list {
            let value = text(item, "source");
            if !value.is_empty() {
                media.subtitles.push(Subtitle {
                    language: text(item, "locale"),
                    title: text(item, "label"),
                    url: media_address(origin, &value)?,
                    ext: if value.contains(".vtt") {
                        "vtt".into()
                    } else {
                        "ttml".into()
                    },
                    automatic: false,
                    translatable: false,
                });
            }
        }
    }
    finish(media)
}

fn parse_tiktok(origin: &Url, html: &str) -> Result<Media> {
    let wanted = capture(origin.path(), r"/video/([0-9]+)");
    let hydration = script_json(html, "__UNIVERSAL_DATA_FOR_REHYDRATION__");
    let legacy = script_json(html, "SIGI_STATE");
    let item=hydration.as_ref().and_then(|root|root.pointer("/__DEFAULT_SCOPE__/webapp.video-detail/itemInfo/itemStruct")).filter(|item|item.is_object()).cloned().or_else(||legacy.as_ref().and_then(|root|root["ItemModule"].as_object()).and_then(|items|wanted.as_ref().and_then(|id|items.get(id)).or_else(||if items.len()==1{items.values().next()}else{None})).cloned()).ok_or_else(||anyhow!("TikTok: публичный video-detail отсутствует; может требоваться вход или проверка браузера"))?;
    if let Some(wanted) = wanted {
        ensure!(text(&item, "id") == wanted, "TikTok вернул другое видео");
    }
    let mut media = base_media(origin);
    media.id = text(&item, "id");
    media.title = text(&item, "desc");
    media.duration = number(&item["video"]["duration"]);
    media.thumbnail = text(&item["video"], "cover");
    media.channel_id = text(&item["author"], "id");
    media.channel_title = text(&item["author"], "uniqueId");
    let video = &item["video"];
    for key in ["playAddr", "downloadAddr"] {
        if let Some(value) = video[key].as_str().filter(|v| !v.is_empty()) {
            let mut stream = format(origin, key, value, "video/mp4", false)?;
            stream.width = uint(&video["width"]).map(|v| v as u32);
            stream.height = uint(&video["height"]).map(|v| v as u32);
            stream.bitrate = uint(&video["bitrate"]);
            if key == "playAddr" && text(video, "format").eq_ignore_ascii_case("dash") {
                stream.audio_codec = None;
            }
            media.formats.push(stream);
        }
    }
    if let Some(list) = video["bitrateInfo"].as_array() {
        for (index, item) in list.iter().enumerate() {
            if let Some(value) = item
                .pointer("/PlayAddr/UrlList")
                .and_then(Value::as_array)
                .and_then(|list| list.first())
                .and_then(Value::as_str)
            {
                let mut stream = format(
                    origin,
                    &format!("bitrate-{index}"),
                    value,
                    "video/mp4",
                    false,
                )?;
                stream.bitrate = uint(&item["Bitrate"]);
                stream.width = uint(&item["PlayAddr"]["Width"]).map(|v| v as u32);
                stream.height = uint(&item["PlayAddr"]["Height"]).map(|v| v as u32);
                if text(item, "Format").eq_ignore_ascii_case("dash") {
                    stream.audio_codec = None;
                }
                media.formats.push(stream);
            }
        }
    }
    if let Some(list) = video["bitrateAudioInfo"].as_array() {
        for (index, item) in list.iter().enumerate() {
            if let Some(value) = ["MainUrl", "BackupUrl"]
                .iter()
                .filter_map(|key| item["UrlList"][key].as_str())
                .find(|value| !value.is_empty())
            {
                let mut stream =
                    format(origin, &format!("audio-{index}"), value, "audio/mp4", true)?;
                stream.bitrate = uint(&item["Bitrate"]);
                stream.size = uint(&item["AudioDataSize"]);
                media.formats.push(stream);
            }
        }
    }
    if let Some(list) = video["subtitleInfos"].as_array() {
        for item in list {
            let value = text(item, "Url");
            if !value.is_empty() {
                media.subtitles.push(Subtitle {
                    language: text(item, "LanguageCodeName"),
                    title: text(item, "LanguageName"),
                    url: media_address(origin, &value)?,
                    ext: "vtt".into(),
                    automatic: item["Source"] == "ASR",
                    translatable: false,
                });
            }
        }
    }
    finish(media)
}
fn parse_facebook(origin: &Url, html: &str) -> Result<Media> {
    let mut media = base_media(origin);
    let wanted = origin
        .query_pairs()
        .find(|(key, _)| key == "v")
        .map(|(_, value)| value.into_owned())
        .or_else(|| capture(origin.path(), r"/(?:videos|video|reel)/([A-Za-z0-9_-]+)"));
    ensure!(
        wanted.is_some() || origin.path().contains("/share/v/"),
        "Facebook: нужна ссылка отдельного видео; выгрузка профиля не поддерживается"
    );
    if let Some(id) = &wanted {
        media.id = id.clone();
    }
    let mut structured_video = false;
    media.title = meta(html, "og:title").unwrap_or_default();
    media.thumbnail = meta(html, "og:image").unwrap_or_default();
    let document = Html::parse_document(html);
    let selector = Selector::parse("script").unwrap();
    for script in document.select(&selector) {
        if let Ok(data) = serde_json::from_str::<Value>(&script.text().collect::<String>()) {
            objects(&data, 0, &mut |object| {
                let has_video = [
                    "browser_native_hd_url",
                    "browser_native_sd_url",
                    "playable_url_quality_hd",
                    "playable_url",
                    "dash_manifest",
                ]
                .iter()
                .any(|key| object.get(*key).is_some_and(Value::is_string));
                if has_video {
                    structured_video = true;
                }
                if let Some(wanted) = &wanted
                    && let Some(id) = object
                        .get("id")
                        .or_else(|| object.get("video_id"))
                        .and_then(|value| {
                            value
                                .as_str()
                                .map(str::to_owned)
                                .or_else(|| uint(value).map(|id| id.to_string()))
                        })
                    && id != *wanted
                {
                    return;
                }
                for key in [
                    "browser_native_hd_url",
                    "browser_native_sd_url",
                    "playable_url_quality_hd",
                    "playable_url",
                ] {
                    if let Some(value) = object
                        .get(key)
                        .and_then(Value::as_str)
                        .filter(|v| !v.is_empty())
                        && let Ok(stream) = format(origin, key, value, "video/mp4", false)
                    {
                        media.formats.push(stream);
                    }
                }
                if let Some(manifest) = object.get("dash_manifest").and_then(Value::as_str) {
                    media.formats.extend(inline_dash(origin, manifest));
                }
                if has_video
                    && let Some(value) = object.get("playable_duration_in_ms").and_then(number)
                {
                    media.duration = Some(value / 1000.)
                }
            });
        }
    }
    if let Some(wanted) = &wanted
        && let Some(data) = json_after(html, r#""videoData":"#)
    {
        for item in data
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["video_id"].as_str() == Some(wanted))
        {
            for key in ["sd_src", "hd_src"] {
                if let Some(value) = item[key].as_str().filter(|value| !value.is_empty()) {
                    let mut stream = format(origin, key, value, "video/mp4", false)?;
                    if key == "hd_src" {
                        stream.width = item["original_width"]
                            .as_u64()
                            .and_then(|value| value.try_into().ok());
                        stream.height = item["original_height"]
                            .as_u64()
                            .and_then(|value| value.try_into().ok());
                    }
                    media.formats.push(stream);
                }
            }
        }
    }
    let pattern = Regex::new(
        r#""(browser_native_hd_url|browser_native_sd_url|playable_url_quality_hd|playable_url)"\s*:\s*("(?:[^"\\]|\\.)*")"#,
    )?;
    for captures in pattern.captures_iter(html).filter(|_| !structured_video) {
        if let Ok(value) = serde_json::from_str::<String>(&captures[2])
            && !value.is_empty()
        {
            media
                .formats
                .push(format(origin, &captures[1], &value, "video/mp4", false)?);
        }
    }
    finish(media).map_err(|_|anyhow!("Facebook: страница не отдала доступные native video URLs; нужен разрешённый доступ или актуальный parser"))
}
fn inline_dash(origin: &Url, manifest: &str) -> Vec<Format> {
    let mut result = Vec::new();
    let adaptations = Regex::new(r"(?s)<AdaptationSet\b([^>]*)>(.*?)</AdaptationSet>").unwrap();
    let representations =
        Regex::new(r"(?s)<Representation\b([^>]*)>(.*?)</Representation>").unwrap();
    for adaptation in adaptations.captures_iter(manifest) {
        let audio = adaptation[1].contains("audio/");
        for representation in representations.captures_iter(&adaptation[2]) {
            let Some(value) = capture(&representation[2], r"(?s)<BaseURL[^>]*>(.*?)</BaseURL>")
            else {
                continue;
            };
            let value = html_text(&value);
            if Url::parse(&value)
                .ok()
                .is_none_or(|url| !url.path().ends_with(".mp4"))
            {
                continue;
            }
            if let Ok(mut stream) = format(
                origin,
                &format!("dash-inline-{}", result.len()),
                &value,
                if audio { "audio/mp4" } else { "video/mp4" },
                audio,
            ) {
                stream.width =
                    capture(&representation[1], r#"width="([0-9]+)""#).and_then(|v| v.parse().ok());
                stream.height = capture(&representation[1], r#"height="([0-9]+)""#)
                    .and_then(|v| v.parse().ok());
                let codec = capture(&representation[1], r#"codecs="([^"]+)""#)
                    .or_else(|| capture(&adaptation[1], r#"codecs="([^"]+)""#));
                if audio {
                    stream.audio_codec = codec;
                    stream.video_codec = None
                } else {
                    stream.video_codec = codec.or(Some("unknown".into()));
                    stream.audio_codec = None
                }
                result.push(stream);
            }
        }
    }
    result
}
fn meta(html: &str, name: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("meta[property],meta[name]").ok()?;
    document
        .select(&selector)
        .find(|element| {
            element
                .value()
                .attr("property")
                .or_else(|| element.value().attr("name"))
                == Some(name)
        })
        .and_then(|element| element.value().attr("content"))
        .map(str::to_owned)
}
fn parse_html(origin: &Url, html: &str) -> Result<Media> {
    let mut media = base_media(origin);
    media.title = meta(html, "og:title")
        .or_else(|| {
            let document = Html::parse_document(html);
            let selector = Selector::parse("title").ok()?;
            document
                .select(&selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
        })
        .unwrap_or_else(|| media.title.clone());
    media.description = meta(html, "og:description")
        .or_else(|| meta(html, "description"))
        .unwrap_or_default();
    media.thumbnail = meta(html, "og:image").unwrap_or_default();
    let document = Html::parse_document(html);
    let selector =
        Selector::parse("video[src],audio[src],video source[src],audio source[src]").unwrap();
    for (index, element) in document.select(&selector).enumerate() {
        let value = element.value().attr("src").unwrap_or_default();
        if value.is_empty() {
            continue;
        }
        let audio = element.value().name() == "audio"
            || element
                .ancestors()
                .filter_map(scraper::ElementRef::wrap)
                .any(|parent| parent.value().name() == "audio");
        let mime =
            element
                .value()
                .attr("type")
                .unwrap_or(if audio { "audio/mp4" } else { "video/mp4" });
        let mut stream = format(origin, &format!("html5-{index}"), value, mime, audio)?;
        stream.width = element.value().attr("width").and_then(|v| v.parse().ok());
        stream.height = element.value().attr("height").and_then(|v| v.parse().ok());
        media.formats.push(stream);
        if audio {
            media.kind = MediaKind::Audio
        }
    }
    let selector = Selector::parse("track[src]").unwrap();
    for track in document.select(&selector) {
        if !matches!(
            track.value().attr("kind"),
            None | Some("subtitles" | "captions")
        ) {
            continue;
        }
        let value = track.value().attr("src").unwrap_or_default();
        if !value.is_empty() {
            media.subtitles.push(Subtitle {
                language: track.value().attr("srclang").unwrap_or("und").into(),
                title: track.value().attr("label").unwrap_or_default().into(),
                url: media_address(origin, value)?,
                ext: "vtt".into(),
                ..Default::default()
            });
        }
    }
    let selector = Selector::parse("script[type='application/ld+json']").unwrap();
    for script in document.select(&selector) {
        if let Ok(data) = serde_json::from_str::<Value>(&script.text().collect::<String>()) {
            parse_jsonld(origin, &data, &mut media);
        }
    }
    // Tumblr's NPF video blocks provide actual media URLs; OpenGraph image/video embeds do not.
    let selector=Selector::parse("script[type='application/json'],script[id='___INITIAL_STATE___'],script[id='__NEXT_DATA__']").unwrap();
    for script in document.select(&selector) {
        if let Ok(data) = serde_json::from_str::<Value>(&script.text().collect::<String>()) {
            if let Some(post_id) = tumblr_post_id(origin) {
                objects(&data, 0, &mut |object| {
                    if object
                        .get("idString")
                        .or_else(|| object.get("id"))
                        .is_some_and(|id| {
                            id.as_str() == Some(post_id)
                                || id.as_u64().is_some_and(|id| id.to_string() == post_id)
                        })
                        && let Some(content) = object.get("content")
                    {
                        parse_npf(origin, content, &mut media);
                    }
                });
            } else {
                parse_npf(origin, &data, &mut media);
            }
        }
    }
    let mut seen = HashSet::new();
    media
        .formats
        .retain(|stream| seen.insert(stream.url.clone()));
    Ok(media)
}
fn tumblr_post_id(url: &Url) -> Option<&str> {
    let host = url.host_str()?;
    if !domain(host, "tumblr.com") {
        return None;
    }
    let mut parts = url.path_segments()?.filter(|part| !part.is_empty());
    let id = if matches!(host, "www.tumblr.com" | "tumblr.com") {
        parts.nth(1)?
    } else {
        parts.find(|part| *part == "post")?;
        parts.next()?
    };
    (!id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit())).then_some(id)
}
fn parse_npf(origin: &Url, data: &Value, media: &mut Media) {
    objects(data, 0, &mut |object| {
        if object.get("type").and_then(Value::as_str) == Some("video")
            && let Some(value) = object
                .get("media")
                .and_then(|media| media.get("url"))
                .and_then(Value::as_str)
            && let Ok(stream) = format(
                origin,
                &format!("npf-{}", media.formats.len()),
                value,
                "video/mp4",
                false,
            )
        {
            media.formats.push(stream);
        }
    });
}
fn parse_jsonld(origin: &Url, data: &Value, media: &mut Media) {
    objects(data, 0, &mut |object| {
        let types = object.get("@type");
        let is_type = |name: &str| {
            types.is_some_and(|value| {
                value.as_str() == Some(name)
                    || value
                        .as_array()
                        .is_some_and(|list| list.iter().any(|value| value.as_str() == Some(name)))
            })
        };
        if (is_type("VideoObject") || is_type("AudioObject"))
            && let Some(post_id) = tumblr_post_id(origin)
            && !object
                .get("mainEntityOfPage")
                .and_then(|page| {
                    page.as_str()
                        .or_else(|| page.get("@id").and_then(Value::as_str))
                })
                .and_then(|page| origin.join(page).ok())
                .is_some_and(|page| tumblr_post_id(&page) == Some(post_id))
        {
            return;
        }
        if (is_type("VideoObject") || is_type("AudioObject"))
            && let Some(value) = object.get("contentUrl").and_then(Value::as_str)
        {
            let audio = is_type("AudioObject");
            if let Ok(stream) = format(
                origin,
                &format!("jsonld-{}", media.formats.len()),
                value,
                object
                    .get("encodingFormat")
                    .and_then(Value::as_str)
                    .unwrap_or(if audio { "audio/mp4" } else { "video/mp4" }),
                audio,
            ) {
                media.formats.push(stream);
                if audio {
                    media.kind = MediaKind::Audio
                }
                if let Some(title) = object.get("name").and_then(Value::as_str) {
                    media.title = title.into()
                }
                if let Some(value) = object.get("duration").and_then(duration) {
                    media.duration = Some(value)
                }
                if let Some(value) = object.get("thumbnailUrl").and_then(|value| {
                    value.as_str().or_else(|| {
                        value
                            .as_array()
                            .and_then(|values| values.first())
                            .and_then(Value::as_str)
                    })
                }) {
                    media.thumbnail = value.into()
                }
            }
        }
        if is_type("ItemList")
            && let Some(items) = object.get("itemListElement").and_then(Value::as_array)
        {
            for item in items {
                let item = item.get("item").unwrap_or(item);
                let address = item
                    .get("url")
                    .or_else(|| item.get("@id"))
                    .and_then(Value::as_str);
                if let Some(address) = address
                    && let Ok(url) = origin.join(address)
                    && crate::validate_url(url.as_str()).is_ok()
                {
                    media.entries.push(Entry {
                        id: id_from_path(&url),
                        url: url.to_string(),
                        title: text(item, "name"),
                        duration: item.get("duration").and_then(duration),
                        thumbnail: text(item, "thumbnailUrl"),
                    });
                }
            }
            if !media.entries.is_empty() {
                media.kind = MediaKind::Playlist
            }
        }
    });
}
fn embedded_player(origin: &Url, html: &str) -> Option<Url> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("iframe[src],embed[src]").ok()?;
    for element in document.select(&selector) {
        let target = origin.join(element.value().attr("src")?).ok()?;
        let host = target.host_str()?;
        if [
            "youtube.com",
            "youtube-nocookie.com",
            "vimeo.com",
            "dailymotion.com",
            "rumble.com",
        ]
        .iter()
        .any(|base| domain(host, base))
        {
            return Some(target);
        }
    }
    None
}
fn duration(value: &Value) -> Option<f64> {
    if let Some(number) = number(value) {
        return Some(number);
    }
    let value = value.as_str()?;
    if value.starts_with('P') {
        let pattern=Regex::new(r"^P(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$").ok()?;
        let groups = pattern.captures(value)?;
        let multipliers = [86400., 3600., 60., 1.];
        let mut total = 0.;
        let mut found = false;
        for (index, multiplier) in multipliers.into_iter().enumerate() {
            if let Some(part) = groups.get(index + 1) {
                total += part.as_str().parse::<f64>().ok()? * multiplier;
                found = true
            }
        }
        return found.then_some(total);
    }
    let parts = value
        .split(':')
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if (2..=3).contains(&parts.len()) {
        Some(parts.into_iter().fold(0., |value, part| value * 60. + part))
    } else {
        None
    }
}

impl Catalog {
    async fn flickr_api(
        &self,
        key: &str,
        method: &str,
        parameters: &[(&str, &str)],
    ) -> Result<Value> {
        let mut endpoint = api_url(
            "https://api.flickr.com/services/rest/",
            &[
                ("method", method),
                ("api_key", key),
                ("format", "json"),
                ("nojsoncallback", "1"),
            ],
        )?;
        endpoint
            .query_pairs_mut()
            .extend_pairs(parameters.iter().copied());
        let value = self.get_json(endpoint).await?;
        ensure!(
            value["stat"] == "ok",
            "Flickr: метод API отказал в доступе или контент недоступен"
        );
        Ok(value)
    }
    async fn flickr(&self, url: &Url, auth: &Auth) -> Result<Media> {
        let mut origin = url.clone();
        if url.host_str() == Some("flic.kr") {
            origin = self.source_get(url, url, auth).await?.url().clone();
            ensure!(
                domain(origin.host_str().unwrap_or_default(), "flickr.com"),
                "Flickr: короткая ссылка ведёт на другой сервис"
            );
        }
        let page = body(self.source_get(&origin, url, auth).await?).await?;
        let mut key = capture(&page, r#"api\.site_key\s*=\s*["']([A-Za-z0-9]+)["']"#);
        if key.is_none() {
            let app = body(
                self.source_get(
                    &Url::parse("https://www.flickr.com/explore")?,
                    url,
                    &Auth::default(),
                )
                .await?,
            )
            .await?;
            key = capture(&app, r#"api\.site_key\s*=\s*["']([A-Za-z0-9]+)["']"#);
        }
        let key =
            key.ok_or_else(|| anyhow!("Flickr: текущий публичный API identifier не найден"))?;
        let parts: Vec<_> = origin
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        if let Some(index) = parts
            .iter()
            .position(|part| *part == "albums" || *part == "sets")
        {
            let id = parts.get(index + 1).copied().unwrap_or_default();
            ensure!(
                !id.is_empty() && id.chars().all(|ch| ch.is_ascii_digit()),
                "Flickr: неверный photoset id"
            );
            let mut media = base_media(&origin);
            media.kind = MediaKind::Playlist;
            for page in 1..=100 {
                let value = self
                    .flickr_api(
                        &key,
                        "flickr.photosets.getPhotos",
                        &[
                            ("photoset_id", id),
                            ("media", "videos"),
                            ("extras", "media,url_m"),
                            ("per_page", "500"),
                            ("page", &page.to_string()),
                        ],
                    )
                    .await?;
                let set = &value["photoset"];
                media.title = text(set, "title");
                let list = set["photo"]
                    .as_array()
                    .ok_or_else(|| anyhow!("Flickr: не распознан список videos"))?;
                for item in list.iter().filter(|item| item["media"] == "video") {
                    media.entries.push(flickr_entry(item, &text(set, "owner")));
                }
                ensure!(
                    media.entries.len() <= ENTRY_LIMIT,
                    "Flickr: photoset превышает 10000 видео"
                );
                if uint(&set["pages"]).is_none_or(|pages| page >= pages) {
                    return finish(media);
                }
            }
            bail!("Flickr: photoset не выгружен полностью после 100 страниц")
        }
        if parts.first().copied() == Some("photos")
            && parts
                .get(2)
                .is_some_and(|id| id.chars().all(|ch| ch.is_ascii_digit()))
        {
            let id = parts[2];
            let info = self
                .flickr_api(&key, "flickr.photos.getInfo", &[("photo_id", id)])
                .await?;
            ensure!(
                info["photo"]["media"] == "video",
                "Flickr: ссылка ведёт на фотографию"
            );
            let secret = text(&info["photo"], "secret");
            let streams = self
                .flickr_api(
                    &key,
                    "flickr.video.getStreamInfo",
                    &[("photo_id", id), ("secret", &secret)],
                )
                .await?;
            return parse_flickr(&origin, &info, &streams);
        }
        ensure!(
            parts.first().copied() == Some("photos") && parts.len() >= 2,
            "Flickr: нужна ссылка видео, photoset или photostream"
        );
        let lookup = self
            .flickr_api(&key, "flickr.urls.lookupUser", &[("url", origin.as_str())])
            .await?;
        let id = text(&lookup["user"], "id");
        ensure!(!id.is_empty(), "Flickr: photostream не найден");
        let mut media = base_media(&origin);
        media.kind = MediaKind::Channel;
        media.channel_id = id.clone();
        media.title = text(&lookup["user"]["username"], "_content");
        media.channel_title = media.title.clone();
        for page in 1..=100 {
            let value = self
                .flickr_api(
                    &key,
                    "flickr.people.getPublicPhotos",
                    &[
                        ("user_id", &id),
                        ("extras", "media,url_m"),
                        ("per_page", "500"),
                        ("page", &page.to_string()),
                    ],
                )
                .await?;
            let list = value["photos"]["photo"]
                .as_array()
                .ok_or_else(|| anyhow!("Flickr: photostream не распознан"))?;
            for item in list.iter().filter(|item| item["media"] == "video") {
                media.entries.push(flickr_entry(item, &id));
            }
            ensure!(
                media.entries.len() <= ENTRY_LIMIT,
                "Flickr: photostream превышает 10000 видео"
            );
            if uint(&value["photos"]["pages"]).is_none_or(|pages| page >= pages) {
                return finish(media);
            }
        }
        bail!("Flickr: photostream не выгружен полностью после 100 страниц")
    }
    async fn tumblr(&self, url: &Url, auth: &Auth, depth: u8) -> Result<Media> {
        let page = body(self.source_get(url, url, auth).await?).await?;
        let direct = parse_html(url, &page)?;
        if !direct.formats.is_empty() {
            return finish(direct);
        }
        let host = url.host_str().unwrap_or_default();
        let parts: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect();
        let blog = if host == "www.tumblr.com" || host == "tumblr.com" {
            parts.first().copied().unwrap_or_default()
        } else {
            host.strip_suffix(".tumblr.com").unwrap_or_default()
        };
        ensure!(
            !blog.is_empty()
                && blog
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'),
            "Tumblr: неверный blog id"
        );
        let id = tumblr_post_id(url);
        let endpoint = api_url(
            &format!("https://{blog}.tumblr.com/api/read/json"),
            &[("type", "video"), ("num", "50")],
        )?;
        if let Some(id) = id {
            let mut endpoint = endpoint;
            endpoint.query_pairs_mut().append_pair("id", id);
            let payload = body(self.source_get(&endpoint, url, auth).await?).await?;
            let data = json_after(&payload, "tumblr_api_read")
                .ok_or_else(|| anyhow!("Tumblr: публичный video API не распознан"))?;
            let post = data["posts"]
                .as_array()
                .and_then(|posts| posts.first())
                .ok_or_else(|| anyhow!("Tumblr: видео недоступно"))?;
            let player = text(post, "video-player");
            let mut media = parse_html(url, &player)?;
            media.title = html_text(&text(post, "video-caption"));
            if !media.formats.is_empty() {
                return finish(media);
            }
            if let Some(embed) = embedded_player(url, &player) {
                return Box::pin(self.inspect_inner(embed.as_str(), &Auth::default(), depth + 1))
                    .await;
            }
            bail!("Tumblr: пост не содержит поддерживаемого video player")
        }
        let mut media = base_media(url);
        media.kind = MediaKind::Channel;
        let mut start = 0;
        loop {
            let mut endpoint = endpoint.clone();
            endpoint
                .query_pairs_mut()
                .append_pair("start", &start.to_string());
            let payload = body(self.source_get(&endpoint, url, auth).await?).await?;
            let data = json_after(&payload, "tumblr_api_read")
                .ok_or_else(|| anyhow!("Tumblr: публичный video API не распознан"))?;
            media.title = text(&data["tumblelog"], "title");
            let posts = data["posts"]
                .as_array()
                .ok_or_else(|| anyhow!("Tumblr: список posts недоступен"))?;
            for post in posts {
                let address = text(post, "url");
                if !address.is_empty() {
                    media.entries.push(Entry {
                        id: text(post, "id"),
                        url: address,
                        title: html_text(&text(post, "video-caption")),
                        ..Default::default()
                    });
                }
            }
            start += posts.len();
            ensure!(
                media.entries.len() <= ENTRY_LIMIT,
                "Tumblr: blog превышает 10000 видео"
            );
            if posts.len() < 50
                || uint(&data["posts-total"]).is_some_and(|total| start as u64 >= total)
            {
                break;
            }
            ensure!(!posts.is_empty(), "Tumblr: пустая страница продолжения");
        }
        finish(media)
    }
}
fn flickr_entry(item: &Value, owner: &str) -> Entry {
    let id = text(item, "id");
    Entry {
        id: id.clone(),
        url: format!("https://www.flickr.com/photos/{owner}/{id}"),
        title: text(item, "title"),
        thumbnail: text(item, "url_m"),
        ..Default::default()
    }
}
fn parse_flickr(origin: &Url, info: &Value, payload: &Value) -> Result<Media> {
    ensure!(
        info["stat"] == "ok" && payload["stat"] == "ok",
        "Flickr: видео недоступно"
    );
    let photo = &info["photo"];
    let mut media = base_media(origin);
    media.id = text(photo, "id");
    media.title = text(&photo["title"], "_content");
    media.description = html_text(&text(&photo["description"], "_content"));
    media.duration = number(&photo["video"]["duration"]);
    media.channel_id = text(&photo["owner"], "nsid");
    media.channel_title = text(&photo["owner"], "username");
    media.thumbnail = format!(
        "https://live.staticflickr.com/{}/{}_{}_b.jpg",
        text(photo, "server"),
        media.id,
        text(photo, "secret")
    );
    if let Some(list) = payload.pointer("/streams/stream").and_then(Value::as_array) {
        for item in list {
            let value = text(item, "_content");
            if value.is_empty() {
                continue;
            }
            let kind = text(item, "type");
            let mut stream = format(
                origin,
                &kind,
                &value,
                if kind == "hls" {
                    "application/vnd.apple.mpegurl"
                } else {
                    "video/mp4"
                },
                false,
            )?;
            stream.height = kind.trim_end_matches('p').parse().ok();
            media.formats.push(stream);
        }
    }
    finish(media)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> Value {
        let data: Value =
            serde_json::from_str(include_str!("../tests/catalog/fixtures.json")).unwrap();
        data[name].clone()
    }
    fn origin(host: &str) -> Url {
        Url::parse(&format!("https://{host}/video/fixture?private=placeholder")).unwrap()
    }
    #[test]
    fn vimeo_formats_and_subtitles() {
        let media = parse_vimeo(&origin("vimeo.com"), &fixture("vimeo")).unwrap();
        assert_eq!(media.formats.len(), 3);
        assert_eq!(media.subtitles.len(), 1);
        assert_eq!(media.duration, Some(62.));
        assert_eq!(media.formats[0].height, Some(720));
        assert_eq!(media.formats[0].headers["Referer"], "https://vimeo.com/");
    }
    #[test]
    fn dailymotion_hls_and_subtitles() {
        let media =
            parse_dailymotion(&origin("www.dailymotion.com"), &fixture("dailymotion")).unwrap();
        assert_eq!(media.formats[0].protocol, Protocol::Hls);
        assert_eq!(media.subtitles[0].language, "en");
        assert_eq!(media.channel_title, "Channel");
        assert!(!media.thumbnail.is_empty());
    }
    #[test]
    fn soundcloud_metadata_milliseconds_and_entries() {
        let data = fixture("soundcloud");
        let media = soundcloud_metadata(&origin("soundcloud.com"), &data);
        assert_eq!(media.duration, Some(65.));
        let entry = soundcloud_entry(&data);
        assert_eq!(entry.url, "https://soundcloud.com/channel/track");
        assert_eq!(entry.duration, Some(65.));
    }
    #[test]
    fn twitch_vod_and_clip_formats() {
        let media = parse_twitch_vod(&origin("www.twitch.tv"), &fixture("twitch")).unwrap();
        assert_eq!(media.formats[0].protocol, Protocol::Hls);
        assert_eq!(
            Url::parse(&media.formats[0].url).unwrap().host_str(),
            Some("usher.ttvnw.net")
        );
        let clip = parse_twitch_clip(&origin("clips.twitch.tv"), &fixture("twitch_clip")).unwrap();
        assert_eq!(clip.formats[0].height, Some(720));
        assert_eq!(clip.formats[0].fps, Some(60.));
    }
    #[test]
    fn bilibili_keeps_separate_audio_and_video() {
        let data = fixture("bilibili");
        let media =
            parse_bilibili(&origin("www.bilibili.com"), &data["view"], &data["play"]).unwrap();
        assert_eq!(media.formats.len(), 2);
        assert!(media.formats[0].has_video() && !media.formats[0].has_audio());
        assert!(media.formats[1].has_audio() && !media.formats[1].has_video());
        assert!((media.formats[0].fps.unwrap() - 59.940).abs() < 0.01);
        assert!(
            media
                .formats
                .iter()
                .all(|format| format.headers["Range"] == "bytes=0-")
        );
    }
    #[test]
    fn rumble_quality_metadata_and_captions() {
        let media = parse_rumble(&origin("rumble.com"), &fixture("rumble")).unwrap();
        assert_eq!(media.formats[0].height, Some(360));
        assert_eq!(media.formats[0].size, Some(1000000));
        assert_eq!(media.subtitles[0].language, "en");
    }
    #[test]
    fn bitchute_metadata_and_media_are_both_required() {
        let data = fixture("bitchute");
        let media = parse_bitchute(
            &origin("www.bitchute.com"),
            &data["metadata"],
            &data["payload"],
        )
        .unwrap();
        assert_eq!(media.duration, Some(90.));
        assert!(
            parse_bitchute(&origin("www.bitchute.com"), &data["metadata"], &json!({})).is_err()
        );
    }
    #[test]
    fn nico_current_server_response_schema() {
        let data = fixture("nico");
        let raw = json!({"data":{"response":{"$watchV4":{"data":data}}}})
            .to_string()
            .replace('&', "&amp;")
            .replace('"', "&quot;");
        let html = format!("<meta name=\"server-response\" content=\"{raw}\">");
        let data = nico_metadata(&html).unwrap();
        let media = parse_nico_metadata(&origin("www.nicovideo.jp"), &data);
        assert_eq!(media.id, "sm9");
        assert_eq!(media.duration, Some(100.));
        assert_eq!(media.description, "Video description");
    }
    #[test]
    fn likee_video_info_and_errors() {
        let media = parse_likee(&origin("likee.video"), &fixture("likee"), "").unwrap();
        assert_eq!(media.duration, Some(15.));
        assert_eq!(media.id, "123456789");
        assert!(parse_likee(&origin("likee.video"), &json!({"code":40004}), "").is_err());
    }
    #[test]
    fn naver_legacy_ticket_and_vod() {
        assert_eq!(
            naver_ticket(r#"{"videoId":"ABCD","inKey":"fixture-key"}"#),
            Some(("ABCD".into(), "fixture-key".into()))
        );
        let media = parse_naver(&origin("tv.naver.com"), &fixture("naver")).unwrap();
        assert_eq!(media.formats[0].height, Some(720));
        assert_eq!(media.subtitles[0].language, "ko");
    }
    #[test]
    fn flickr_approved_video_streams() {
        let data = fixture("flickr");
        let media =
            parse_flickr(&origin("www.flickr.com"), &data["info"], &data["streams"]).unwrap();
        assert_eq!(media.formats.len(), 2);
        assert_eq!(media.formats[0].height, Some(720));
        assert_eq!(media.formats[1].protocol, Protocol::Hls);
    }
    #[test]
    fn tiktok_current_hydration_and_missing_playback() {
        let item = fixture("tiktok");
        let payload =
            json!({"__DEFAULT_SCOPE__":{"webapp.video-detail":{"itemInfo":{"itemStruct":item}}}});
        let page = format!(
            "<script id=\"__UNIVERSAL_DATA_FOR_REHYDRATION__\" type=\"application/json\">{payload}</script>"
        );
        let media = parse_tiktok(
            &Url::parse("https://www.tiktok.com/@channel/video/123456789").unwrap(),
            &page,
        )
        .unwrap();
        assert_eq!(media.formats.len(), 3);
        assert_eq!(media.subtitles[0].language, "en");
        let signed = Url::parse(&media.formats[0].url).unwrap();
        assert!(
            signed
                .query_pairs()
                .any(|(key, value)| key == "signature" && value == "fixture")
        );
        assert!(!signed.query_pairs().any(|(key, _)| key.starts_with("amp;")));
        assert!(parse_tiktok(&origin("www.tiktok.com"), "<title>Login</title>").is_err());
    }
    #[test]
    fn tiktok_dash_video_selects_separate_audio() {
        let item = json!({"id":"123456789","video":{
            "duration":148,"width":576,"height":640,
            "playAddr":"https://media.example/muxed.mp4",
            "bitrateInfo":[{"Format":"dash","CodecType":"h265_hvc1","Bitrate":489724,
                "PlayAddr":{"Width":720,"Height":800,"UrlList":["https://media.example/video.mp4"]}}],
            "bitrateAudioInfo":[{"Bitrate":65008,"MediaType":"audio","Format":"dash",
                "AudioDataSize":"1209583","UrlList":{"MainUrl":"https://media.example/audio.m4a"}}]
        }});
        let payload =
            json!({"__DEFAULT_SCOPE__":{"webapp.video-detail":{"itemInfo":{"itemStruct":item}}}});
        let page = format!("<script id=\"__UNIVERSAL_DATA_FOR_REHYDRATION__\">{payload}</script>");
        let media = parse_tiktok(
            &Url::parse("https://www.tiktok.com/@channel/video/123456789").unwrap(),
            &page,
        )
        .unwrap();
        let video = media.formats.iter().find(|f| f.id == "bitrate-0").unwrap();
        assert!(
            !video.has_audio(),
            "TikTok DASH video was incorrectly marked as muxed"
        );
        let selected = crate::choose_formats(&media.formats, &crate::Selection::default()).unwrap();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].id, "bitrate-0");
        assert_eq!(selected[1].url, "https://media.example/audio.m4a");
        assert!(!selected[1].has_video());
        assert_eq!(selected[1].size, Some(1209583));
    }
    #[test]
    fn facebook_native_urls_and_inline_dash() {
        let page = r#"<meta property="og:title" content="Fixture"><script type="application/json">{"browser_native_hd_url":"https://media.example/video.mp4","playable_duration_in_ms":10000}</script>"#;
        let media = parse_facebook(&origin("www.facebook.com"), page).unwrap();
        assert_eq!(media.formats.len(), 1);
        assert_eq!(media.duration, Some(10.));
        let mpd = r#"<MPD><Period><AdaptationSet mimeType="video/mp4" codecs="avc1.640028"><Representation width="1280" height="720"><BaseURL>https://media.example/video.mp4?a=1&amp;b=2</BaseURL><SegmentBase/></Representation></AdaptationSet><AdaptationSet mimeType="audio/mp4" codecs="mp4a.40.2"><Representation><BaseURL>https://media.example/audio.mp4</BaseURL><SegmentBase/></Representation></AdaptationSet></Period></MPD>"#;
        let formats = inline_dash(&origin("www.facebook.com"), mpd);
        assert_eq!(formats.len(), 2);
        assert!(formats[0].has_video() && !formats[0].has_audio());
        assert!(formats[1].has_audio() && !formats[1].has_video());
    }
    #[test]
    fn generic_html5_jsonld_and_tumblr_npf() {
        let page = r#"<title>Fixture</title><video><source src="/media/video.mp4" type="video/mp4"><track src="/sub.vtt" kind="subtitles" srclang="en"></video><script type="application/ld+json">{"@type":"VideoObject","name":"Schema fixture","duration":"PT1M2.5S","contentUrl":"https://media.example/schema.mp4"}</script><script type="application/json">{"content":[{"type":"video","media":{"url":"https://media.example/npf.mp4"}}]}</script>"#;
        let media = parse_html(&origin("staff.tumblr.com"), page).unwrap();
        assert_eq!(media.formats.len(), 3);
        assert_eq!(media.duration, Some(62.5));
        assert_eq!(media.subtitles.len(), 1);
        let image_only = r#"<meta property="og:title" content="Title only"><meta property="og:image" content="https://media.example/poster.jpg"><meta property="og:video" content="https://example.com/player.html">"#;
        assert!(
            parse_html(&origin("example.com"), image_only)
                .unwrap()
                .formats
                .is_empty()
        );
        assert!(parse_facebook(&origin("www.facebook.com"), image_only).is_err());
    }
    #[test]
    fn tumblr_post_excludes_other_posts_from_jsonld_and_npf() {
        let page = r#"<script type="application/ld+json">[{"@type":"VideoObject","mainEntityOfPage":{"@id":"https://www.tumblr.com/staff/123/wanted"},"name":"Wanted","duration":"PT10S","contentUrl":"https://media.example/wanted.mp4"},{"@type":"VideoObject","mainEntityOfPage":{"@id":"https://www.tumblr.com/staff/456/recommended"},"name":"Recommendation","duration":"PT90S","contentUrl":"https://media.example/unrelated.mp4"}]</script><script id="___INITIAL_STATE___" type="application/json">{"posts":[{"idString":"123","content":[{"type":"video","media":{"url":"https://media.example/wanted.mp4"}}]},{"idString":"456","content":[{"type":"video","media":{"url":"https://media.example/unrelated.mp4"}}]}]}</script>"#;
        for address in [
            "https://www.tumblr.com/staff/123/wanted",
            "https://tumblr.com/staff/123/wanted",
            "https://staff.tumblr.com/post/123/wanted",
        ] {
            let media = parse_html(&Url::parse(address).unwrap(), page).unwrap();
            assert_eq!(media.formats.len(), 1);
            assert_eq!(media.formats[0].url, "https://media.example/wanted.mp4");
            assert_eq!(media.title, "Wanted");
            assert_eq!(media.duration, Some(10.0));
        }
        let missing = parse_html(
            &Url::parse("https://www.tumblr.com/staff/789").unwrap(),
            page,
        )
        .unwrap();
        assert!(missing.formats.is_empty());
    }
    #[test]
    fn tumblr_numeric_post_identity_scopes_npf_without_jsonld() {
        let page = r#"<script type="application/json">{"posts":[{"id":123,"content":[{"type":"video","media":{"url":"https://media.example/wanted.mp4"}}]},{"id":456,"content":[{"type":"video","media":{"url":"https://media.example/unrelated.mp4"}}]}]}</script>"#;
        let media = parse_html(
            &Url::parse("https://www.tumblr.com/staff/123").unwrap(),
            page,
        )
        .unwrap();
        assert_eq!(media.formats.len(), 1);
        assert_eq!(media.formats[0].url, "https://media.example/wanted.mp4");
        assert!(tumblr_post_id(&origin("www.tumblr.com")).is_none());
        assert!(
            tumblr_post_id(&Url::parse("https://www.tumblr.com/staff/not-a-post").unwrap())
                .is_none()
        );
        assert!(
            tumblr_post_id(&Url::parse("https://tumblr.com.attacker.example/staff/123").unwrap())
                .is_none()
        );
    }
    #[test]
    fn jsonld_playlist_and_durations() {
        let page = r#"<script type="application/ld+json">{"@type":"ItemList","itemListElement":[{"item":{"url":"https://example.com/video/1","name":"First"}},{"item":{"url":"https://example.com/video/2","name":"Second"}}]}</script>"#;
        let media = parse_html(&origin("example.com"), page).unwrap();
        assert_eq!(media.kind, MediaKind::Playlist);
        assert_eq!(media.entries.len(), 2);
        assert_eq!(duration(&json!("01:02:03")), Some(3723.));
        assert_eq!(duration(&json!("PT2H3M4S")), Some(7384.));
        assert!(!domain("notyoutube.com", "youtube.com"));
    }
}

#[cfg(test)]
mod delivery_tests {
    use super::*;
    #[test]
    fn anonymous_delivery_cookie_requires_https_domain_root_path_and_live_age() {
        let url = Url::parse("https://www.tiktok.com/video/fixture").unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::SET_COOKIE,reqwest::header::HeaderValue::from_static("tt_chain_token=fixture-lease; Domain=.tiktok.com; Path=/; Max-Age=600; Secure; HttpOnly"));
        let cookie = issued_cookie(&url, &headers, "tt_chain_token", "tiktok.com").unwrap();
        assert_eq!(cookie.name, "tt_chain_token");
        assert_eq!(cookie.value, "fixture-lease");
        assert_eq!(cookie.domain, ".tiktok.com");
        assert_eq!(cookie.path, "/");
        assert!(cookie.secure);
        assert!(cookie.applies_to(
            &Url::parse("https://v16-webapp-prime.tiktok.com/media/video.mp4").unwrap()
        ));
        assert!(!cookie.applies_to(&Url::parse("https://attacker.example/video.mp4").unwrap()));
        assert!(
            issued_cookie(
                &Url::parse("https://attacker.example/").unwrap(),
                &headers,
                "tt_chain_token",
                "tiktok.com"
            )
            .is_none()
        );
        assert!(
            issued_cookie(
                &Url::parse("http://www.tiktok.com/").unwrap(),
                &headers,
                "tt_chain_token",
                "tiktok.com"
            )
            .is_none()
        );
        headers.insert(
            reqwest::header::SET_COOKIE,
            reqwest::header::HeaderValue::from_static(
                "tt_chain_token=deleted; Domain=.tiktok.com; Path=/; Max-Age=0",
            ),
        );
        assert!(issued_cookie(&url, &headers, "tt_chain_token", "tiktok.com").is_none());
        headers.insert(
            reqwest::header::SET_COOKIE,
            reqwest::header::HeaderValue::from_static(
                "tt_chain_token=fixture-lease; Domain=.tiktok.com; Path=/private",
            ),
        );
        assert!(issued_cookie(&url, &headers, "tt_chain_token", "tiktok.com").is_none());
    }
    #[test]
    fn binary_media_sniff_does_not_accept_html_or_webp() {
        assert!(media_signature(b"\0\0\0\x18ftypisom"));
        assert!(media_signature(b"#EXTM3U\n#EXTINF:1"));
        assert!(!media_signature(b"<html>not a video</html>"));
        assert!(!media_signature(b"RIFFxxxxWEBP"));
        assert!(number(&json!("NaN")).is_none());
        assert!(number(&json!(-1)).is_none());
    }
}

fn media_address(origin: &Url, value: &str) -> Result<String> {
    ensure!(!value.trim().is_empty(), "Пустая ссылка медиаресурса");
    let target = origin
        .join(value)
        .map_err(|_| anyhow!("Неверная ссылка медиаресурса"))?;
    Ok(crate::validate_url(target.as_str())?.to_string())
}

impl Catalog {
    async fn bilibili_delivery(&self, response: &mut Value, origin: &Url) -> Result<()> {
        if response["code"].as_i64() != Some(0) {
            return Ok(());
        }
        let root = if response.get("data").is_some() {
            "data"
        } else {
            "result"
        };
        let mut plans = Vec::new();
        for kind in ["video", "audio"] {
            if let Some(items) = response[root]["dash"][kind].as_array() {
                for (index, item) in items.iter().enumerate() {
                    plans.push((
                        format!("/{root}/dash/{kind}/{index}"),
                        bilibili_candidates(item),
                    ));
                }
            }
        }
        ensure!(
            plans.len() <= 256 && plans.iter().all(|(_, candidates)| candidates.len() <= 16),
            "BiliBili вернул слишком много потоков или CDN адресов"
        );
        for batch in plans.chunks(4) {
            let mut tasks = tokio::task::JoinSet::new();
            for (pointer, candidates) in batch {
                let http = self.http.clone();
                let pointer = pointer.clone();
                let candidates = candidates.clone();
                let mut headers = source_headers(origin);
                headers.insert("User-Agent".into(), "VideoDock/0.2".into());
                tasks.spawn(async move {
                    let mut chosen = None;
                    'candidate: for address in candidates {
                        let Ok(mut target) = crate::validate_url(&address) else {
                            continue;
                        };
                        for _ in 0..4 {
                            let mut request = http
                                .get(target.clone())
                                .header("Range", "bytes=0-0")
                                .timeout(Duration::from_secs(7));
                            for (name, value) in &headers {
                                request = request.header(name, value);
                            }
                            let Ok(mut result) = request.send().await else {
                                continue 'candidate;
                            };
                            if result.status().is_redirection() {
                                let Some(next) = result
                                    .headers()
                                    .get(reqwest::header::LOCATION)
                                    .and_then(|value| value.to_str().ok())
                                    .and_then(|value| target.join(value).ok())
                                else {
                                    continue 'candidate;
                                };
                                let Ok(next) = crate::validate_url(next.as_str()) else {
                                    continue 'candidate;
                                };
                                target = next;
                                continue;
                            }
                            if !result.status().is_success() {
                                continue 'candidate;
                            }
                            let content_type = result
                                .headers()
                                .get(reqwest::header::CONTENT_TYPE)
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or_default();
                            if content_type.contains("text/html")
                                || content_type.contains("application/json")
                            {
                                continue 'candidate;
                            }
                            if result
                                .chunk()
                                .await
                                .ok()
                                .flatten()
                                .is_some_and(|chunk| !chunk.is_empty())
                            {
                                chosen = Some(target.to_string());
                                break 'candidate;
                            }
                            continue 'candidate;
                        }
                    }
                    (pointer, chosen)
                });
            }
            while let Some(result) = tasks.join_next().await {
                let (pointer, chosen) =
                    result.map_err(|_| anyhow!("BiliBili: проверка CDN прервана"))?;
                if let Some(item) = response
                    .pointer_mut(&pointer)
                    .and_then(Value::as_object_mut)
                {
                    item.insert(
                        "baseUrl".into(),
                        chosen.map(Value::String).unwrap_or(Value::Null),
                    );
                }
            }
        }
        Ok(())
    }
}
fn bilibili_candidates(item: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(value) = item
        .get("baseUrl")
        .or_else(|| item.get("base_url"))
        .and_then(Value::as_str)
    {
        urls.push(value.to_owned())
    }
    for key in ["backupUrl", "backup_url"] {
        if let Some(values) = item[key].as_array() {
            for value in values.iter().filter_map(Value::as_str) {
                if !urls.iter().any(|existing| existing == value) {
                    urls.push(value.to_owned())
                }
            }
        }
    }
    urls
}

#[cfg(test)]
mod catalog_network_regressions {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    #[tokio::test]
    async fn bilibili_tries_the_provided_backup_after_primary_connection_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!(
            "http://127.0.0.1:{}/backup.mp4",
            listener.local_addr().unwrap().port()
        );
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut headers = [0; 8192];
            let count = stream.read(&mut headers).unwrap();
            let request = String::from_utf8_lossy(&headers[..count]).to_lowercase();
            assert!(request.contains("range: bytes=0-0"));
            assert!(!request.contains("cookie:"));
            stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nContent-Length: 1\r\nContent-Range: bytes 0-0/100\r\nConnection: close\r\n\r\nx").unwrap();
        });
        let mut payload = json!({"code":0,"data":{"dash":{"video":[{"baseUrl":"http://127.0.0.1:0/unreachable.mp4","backupUrl":[address.clone()]}],"audio":[]}}});
        Catalog::new(None)
            .unwrap()
            .bilibili_delivery(
                &mut payload,
                &Url::parse("https://www.bilibili.com/").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            payload["data"]["dash"]["video"][0]["baseUrl"].as_str(),
            Some(address.as_str())
        );
        server.join().unwrap();
    }
    #[test]
    fn cookie_http_date_is_respected_and_max_age_takes_precedence() {
        let origin = Url::parse("https://nvapi.nicovideo.jp/").unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        let future = 4_102_444_800i64;
        let date = time::OffsetDateTime::from_unix_timestamp(future)
            .unwrap()
            .format(&time::format_description::well_known::Rfc2822)
            .unwrap();
        headers.insert(
            reqwest::header::SET_COOKIE,
            reqwest::header::HeaderValue::from_str(&format!(
                "domand_bid=fixture; Domain=.nicovideo.jp; Path=/; Secure; Expires={date}"
            ))
            .unwrap(),
        );
        let cookie = issued_cookie(&origin, &headers, "domand_bid", "nicovideo.jp").unwrap();
        assert_eq!(cookie.expires, Some(future as u64));
        assert!(cookie.secure);
        headers.insert(reqwest::header::SET_COOKIE,reqwest::header::HeaderValue::from_static("domand_bid=fixture; Domain=.nicovideo.jp; Path=/; Expires=Sat, 12 Jun 1993 13:25:19 GMT"));
        assert!(issued_cookie(&origin, &headers, "domand_bid", "nicovideo.jp").is_none());
        headers.insert(reqwest::header::SET_COOKIE,reqwest::header::HeaderValue::from_static("domand_bid=fixture; Domain=.nicovideo.jp; Path=/; Max-Age=600; Expires=Sat, 12 Jun 1993 13:25:19 GMT"));
        assert!(issued_cookie(&origin, &headers, "domand_bid", "nicovideo.jp").is_some());
    }
}

fn twitch_data(data: &Value) -> Result<Value> {
    if let Some(errors) = data["errors"].as_array() {
        ensure!(
            !errors.iter().any(|error| error["message"]
                .as_str()
                .is_some_and(|message| message.to_ascii_lowercase().contains("integrity"))),
            "Twitch ограничил продолжение списка; полный список сейчас недоступен"
        );
        ensure!(
            errors.is_empty(),
            "Twitch изменил публичную схему API или отказал в доступе"
        );
    } else {
        ensure!(
            data.get("errors").is_none_or(Value::is_null),
            "Twitch вернул нераспознанную ошибку API"
        );
    }
    ensure!(data["data"].is_object(), "Twitch не вернул данные");
    Ok(data["data"].clone())
}

#[cfg(test)]
mod pagination_errors {
    use super::*;
    #[test]
    fn twitch_integrity_failure_never_becomes_a_successful_partial_list() {
        let response = json!({"data":{"user":{"videos":{"edges":[{"node":{"id":"fixture"}}]}}},"errors":[{"message":"failed integrity check"}]});
        let error = twitch_data(&response).unwrap_err().to_string();
        assert!(error.contains("полный список сейчас недоступен"));
        assert!(twitch_data(&json!({"errors":[],"data":{"video":{"id":"fixture"}}})).is_ok());
    }
}

#[cfg(test)]
mod facebook_selection {
    use super::*;
    #[test]
    fn public_embed_selects_only_requested_video_and_keeps_geometry() {
        let html = r#"<script>require("VideoConfig").handle({"videoData":[{"video_id":"123","sd_src":"https://media.example/wanted-sd.mp4?a=1&signature=fixture","hd_src":"https://media.example/wanted-hd.mp4","original_height":1080,"original_width":1080},{"video_id":"456","hd_src":"https://media.example/unrelated.mp4"}]});</script>"#;
        let target = Url::parse("https://www.facebook.com/watch/?v=123").unwrap();
        let media = parse_facebook(&target, html).unwrap();
        assert_eq!(media.formats.len(), 2);
        assert!(
            media
                .formats
                .iter()
                .all(|stream| stream.url.contains("wanted-"))
        );
        let hd = media
            .formats
            .iter()
            .find(|stream| stream.id == "hd_src")
            .unwrap();
        assert_eq!((hd.width, hd.height), (Some(1080), Some(1080)));
        let sd = media
            .formats
            .iter()
            .find(|stream| stream.id == "sd_src")
            .unwrap();
        assert!(sd.url.contains("&signature=fixture"));
        assert!(
            parse_facebook(
                &Url::parse("https://www.facebook.com/watch/?v=789").unwrap(),
                html
            )
            .is_err()
        );
    }
    #[test]
    fn unrelated_recommended_videos_are_not_formats_of_requested_video() {
        let html = r#"<meta property="og:title" content="Requested"><script type="application/json">{"videos":[{"id":"123","browser_native_hd_url":"https://media.example/wanted.mp4?a=1&signature=fixture"},{"id":"456","browser_native_hd_url":"https://media.example/unrelated.mp4"}]}</script>"#;
        let media = parse_facebook(
            &Url::parse("https://www.facebook.com/watch/?v=123").unwrap(),
            html,
        )
        .unwrap();
        assert_eq!(media.formats.len(), 1);
        assert!(media.formats[0].url.contains("wanted.mp4"));
        assert!(
            Url::parse(&media.formats[0].url)
                .unwrap()
                .query_pairs()
                .any(|(key, _)| key == "signature")
        );
        assert!(
            parse_facebook(&Url::parse("https://www.facebook.com/page").unwrap(), html).is_err()
        );
    }
}
