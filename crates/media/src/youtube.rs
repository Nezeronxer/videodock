pub use crate::proof::register_proof_worker;
use crate::{Auth, Entry, Format, Media, MediaKind, Protocol, Subtitle, VideoProjection};
use anyhow::{Result, anyhow, ensure};
use rustypipe::{
    client::{ClientType, RustyPipe, RustyPipeQuery},
    error::{Error as PipeError, ExtractionError},
    model::{
        AudioTrackType, Thumbnail, UrlTarget, VideoItem, VideoPlayer, paginator::Paginator,
        richtext::ToPlaintext, traits::FileFormat,
    },
};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

const ENTRY_LIMIT: usize = 10_000;
const ACCOUNT_JSON_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountInfo {
    pub user_name: String,
    pub avatar_url: String,
}

impl std::fmt::Debug for AccountInfo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccountInfo")
            .field("identity", &"[redacted]")
            .finish()
    }
}

#[path = "youtube_sabr.rs"]
mod native_sabr;

pub struct YouTube {
    proxy_url: Option<String>,
    proxy: Option<wreq::Proxy>,
}

impl YouTube {
    pub async fn new(proxy: Option<&str>) -> Result<Self> {
        let proxy_url = proxy
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned);
        let proxy = proxy
            .filter(|value| !value.trim().is_empty())
            .map(|value| wreq::Proxy::all(value).map_err(|_| anyhow!("Некорректный адрес прокси")))
            .transpose()?;
        Ok(Self { proxy, proxy_url })
    }

    async fn client(&self, auth: &Auth) -> Result<RustyPipe> {
        self.client_identity(auth, None).await
    }

    async fn client_identity(
        &self,
        auth: &Auth,
        proof: Option<&crate::proof::Proof>,
    ) -> Result<RustyPipe> {
        ensure!(
            !auth.cookie_header.contains(['\r', '\n']),
            "Недопустимый Cookie-заголовок"
        );
        // shortcut: wreq 0.16 player POSTs reset HTTP/2 streams here; restore HTTP/2 after a verified transport update.
        let mut http = wreq::Client::builder()
            .no_proxy()
            .http1_only()
            .dns_resolver(crate::dns::resolver(self.proxy.is_none())?);
        if let Some(proxy) = &self.proxy {
            http = http.proxy(proxy.clone());
        }
        // Credentials stay in this request's memory; disable filesystem reports and helper discovery.
        let mut builder = RustyPipe::builder()
            .strict()
            .no_storage()
            .no_reporter()
            .no_botguard()
            .timeout(Duration::from_secs(30))
            .n_http_retries(1);
        if let Some(proof) = proof {
            builder = builder
                .user_agent(&proof.user_agent)
                .visitor_data(&proof.visitor_data);
        }
        let client = builder.build_with_client(http).map_err(pipe_error)?;
        if has_account_cookie(auth) {
            client
                .user_auth_set_cookie(auth.cookie_header.clone())
                .await
                .map_err(pipe_error)?;
        }
        Ok(client)
    }

    pub async fn inspect(&self, value: &str, auth: &Auth) -> Result<Media> {
        let url = checked_video_url(value)?;
        let client = self.client(auth).await?;
        let query = query(&client, auth);
        let target = query
            .clone()
            .resolve_url(url.as_str(), false)
            .await
            .map_err(pipe_error)?;
        match target {
            UrlTarget::Video { id, .. } => {
                if !has_account_cookie(auth)
                    && let Some(worker) = crate::proof::registered_worker()
                {
                    let proof =
                        crate::proof::request(worker, &id, self.proxy_url.as_deref()).await?;
                    let attested = self.client_identity(auth, Some(&proof)).await?;
                    let attested = attested
                        .query()
                        .unauthenticated()
                        .visitor_data(&proof.visitor_data);
                    let (raw, version) = attested
                        .attested_player(&id, &proof.visitor_data, &proof.content_po)
                        .await
                        .map_err(pipe_error)?;
                    let data: serde_json::Value = serde_json::from_str(&raw)
                        .map_err(|_| anyhow!("Некорректный ответ YouTube"))?;
                    if data["videoDetails"]["isLive"].as_bool() != Some(true) {
                        let delivery = native_sabr::delivery_url(&data)?;
                        crate::sabr::validate_url(delivery)?;
                        let delivery = attested
                            .decipher_stream_url(delivery)
                            .await
                            .map_err(pipe_error)?;
                        return native_sabr::map(
                            data,
                            &id,
                            delivery,
                            version,
                            proof,
                            self.sabr_http()?,
                        );
                    }
                }
                let player = query.player(&id).await.map_err(pipe_error)?;
                map_player(player, &query)
            }
            UrlTarget::Playlist { id } => {
                let mut playlist = query.playlist(&id).await.map_err(pipe_error)?;
                complete(&mut playlist.videos, &query).await?;
                let channel = playlist.channel;
                Ok(Media {
                    id: id.clone(),
                    url: format!("https://www.youtube.com/playlist?list={id}"),
                    title: playlist.name,
                    kind: MediaKind::Playlist,
                    description: playlist
                        .description
                        .map(|text| text.to_plaintext())
                        .unwrap_or_default(),
                    thumbnail: thumbnail(&playlist.thumbnail),
                    channel_id: channel.as_ref().map(|c| c.id.clone()).unwrap_or_default(),
                    channel_title: channel.map(|c| c.name).unwrap_or_default(),
                    entries: playlist.videos.items.into_iter().map(map_entry).collect(),
                    ..Default::default()
                })
            }
            UrlTarget::Channel { id } => {
                let mut channel = query.channel_videos(&id).await.map_err(pipe_error)?;
                complete(&mut channel.content, &query).await?;
                Ok(Media {
                    id: id.clone(),
                    url: format!("https://www.youtube.com/channel/{id}"),
                    title: channel.name.clone(),
                    kind: MediaKind::Channel,
                    description: channel.description,
                    thumbnail: thumbnail(&channel.avatar),
                    channel_id: id,
                    channel_title: channel.name,
                    entries: channel.content.items.into_iter().map(map_entry).collect(),
                    ..Default::default()
                })
            }
            _ => Err(anyhow!("Этот тип ссылки YouTube пока не поддерживается")),
        }
    }

    fn sabr_http(&self) -> Result<reqwest::Client> {
        // shortcut: direct CDN HTTP/2 stalls after segments; restore after verified full downloads.
        let mut client = reqwest::Client::builder()
            .no_proxy()
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(20))
            .read_timeout(Duration::from_secs(30))
            .dns_resolver(crate::dns::resolver(self.proxy_url.is_none())?);
        if let Some(proxy) = &self.proxy_url {
            client = client.proxy(
                reqwest::Proxy::all(proxy).map_err(|_| anyhow!("Некорректный прокси SABR"))?,
            );
        }
        client
            .build()
            .map_err(|_| anyhow!("Не удалось создать транспорт SABR"))
    }

    pub async fn verify_auth(&self, auth: &Auth) -> Result<()> {
        ensure!(
            has_account_cookie(auth),
            "Нет данных авторизации YouTube"
        );
        self.client(auth)
            .await?
            .user_auth_check_cookie()
            .await
            .map_err(pipe_error)
    }

    pub async fn account_info(&self, auth: &Auth) -> Result<AccountInfo> {
        ensure!(
            has_account_cookie(auth),
            "Нет данных авторизации YouTube"
        );
        let client = self.client(auth).await?;
        client.user_auth_check_cookie().await.map_err(pipe_error)?;
        let response = client
            .query()
            .authenticated()
            .raw_bounded(
                ClientType::Desktop,
                "account/account_menu",
                &serde_json::json!({"deviceTheme":"DEVICE_THEME_SELECTED"}),
                ACCOUNT_JSON_LIMIT,
            )
            .await
            .map_err(pipe_error)?;
        parse_account_info(&response)
    }

    pub async fn search(&self, value: &str, auth: &Auth) -> Result<Media> {
        let value = value.trim();
        ensure!(
            !value.is_empty() && value.len() <= 2048 && !value.chars().any(char::is_control),
            "Введите поисковый запрос до 2048 символов"
        );
        let client = self.client(auth).await?;
        let query = query(&client, auth);
        let mut result = query
            .search::<VideoItem, _>(value)
            .await
            .map_err(pipe_error)?;
        complete(&mut result.items, &query).await?;
        let mut url = url::Url::parse("https://www.youtube.com/results")?;
        url.query_pairs_mut().append_pair("search_query", value);
        Ok(Media {
            id: value.to_owned(),
            url: url.to_string(),
            title: value.to_owned(),
            kind: MediaKind::Search,
            entries: result.items.items.into_iter().map(map_entry).collect(),
            ..Default::default()
        })
    }
}

fn has_account_cookie(auth: &Auth) -> bool {
    auth.cookie_header.split(';').any(|cookie| {
        cookie.trim().strip_prefix("SAPISID=").is_some_and(|value| !value.trim().is_empty())
    })
}

fn query(client: &RustyPipe, auth: &Auth) -> RustyPipeQuery {
    if !has_account_cookie(auth) {
        client.query().unauthenticated()
    } else {
        client.query()
    }
}

// Dropping/aborting the inspect future cancels its in-flight HTTP requests and pagination.
async fn complete(items: &mut Paginator<VideoItem>, query: &RustyPipeQuery) -> Result<()> {
    let mut tokens = HashSet::new();
    while let Some(token) = items.ctoken.as_ref() {
        ensure!(
            items.items.len() < ENTRY_LIMIT,
            "YouTube: достигнут предел 10000 элементов; список не выгружен полностью"
        );
        ensure!(
            tokens.insert(token.clone()),
            "YouTube повторил страницу списка; выгрузка остановлена"
        );
        items.extend(query).await.map_err(pipe_error)?;
    }
    ensure!(
        items.items.len() <= ENTRY_LIMIT,
        "YouTube: список превышает предел 10000 элементов"
    );
    Ok(())
}

fn map_player(player: VideoPlayer, query: &RustyPipeQuery) -> Result<Media> {
    ensure!(player.drm.is_none(), "YouTube: видео защищено DRM");
    let user_agent = query.user_agent(player.client_type).into_owned();
    let headers = BTreeMap::from([
        ("User-Agent".to_owned(), user_agent),
        ("Referer".to_owned(), "https://www.youtube.com/".to_owned()),
    ]);
    let mut formats = Vec::new();
    for (streams, muxed) in [
        (&player.video_streams, true),
        (&player.video_only_streams, false),
    ] {
        for stream in streams {
            let (video_codec, audio_codec) = codecs(&stream.mime);
            formats.push(Format {
                id: stream.itag.to_string(),
                url: stream.url.clone(),
                protocol: Protocol::Http,
                ext: stream.format.extension().trim_start_matches('.').to_owned(),
                width: Some(stream.width),
                height: Some(stream.height),
                fps: Some(stream.fps.into()),
                vr: stream
                    .projection_type
                    .as_deref()
                    .is_some_and(|value| matches!(value, "MESH" | "EQUIRECTANGULAR" | "360")),
                // Mesh/stereo flags alone do not establish angular coverage or a precise VR mode.
                projection: (stream.projection_type.as_deref() == Some("RECTANGULAR"))
                    .then_some(VideoProjection::Flat),
                projection_type: stream.projection_type.clone(),
                stereo_layout: stream.stereo_layout.clone(),
                video_codec,
                audio_codec: if muxed { audio_codec } else { None },
                bitrate: Some(stream.bitrate.into()),
                size: stream.size,
                headers: stream_headers(&headers, stream.size),
                ..Default::default()
            });
        }
    }
    for stream in &player.audio_streams {
        let (_, audio_codec) = codecs(&stream.mime);
        let track = stream.track.as_ref();
        formats.push(Format {
            id: track
                .map(|track| format!("{}:{}", stream.itag, track.id))
                .unwrap_or_else(|| stream.itag.to_string()),
            url: stream.url.clone(),
            protocol: Protocol::Http,
            ext: stream.format.extension().trim_start_matches('.').to_owned(),
            audio_codec,
            audio_language: track.and_then(|track| track.lang.clone()),
            audio_title: track.map(|track| track.lang_name.clone()),
            audio_track_id: track.map(|track| track.id.clone()),
            audio_default: track.map(|track| track.is_default),
            audio_original: track.and_then(|track| {
                track
                    .track_type
                    .map(|kind| kind == AudioTrackType::Original)
            }),
            bitrate: Some(stream.bitrate.into()),
            size: Some(stream.size),
            headers: stream_headers(&headers, Some(stream.size)),
            ..Default::default()
        });
    }
    if let Some(url) = &player.hls_manifest_url {
        formats.push(Format {
            id: "hls".into(),
            url: url.clone(),
            protocol: Protocol::Hls,
            ext: "mp4".into(),
            headers: headers.clone(),
            ..Default::default()
        });
    }
    if let Some(url) = &player.dash_manifest_url {
        formats.push(Format {
            id: "dash".into(),
            url: url.clone(),
            protocol: Protocol::Dash,
            ext: "mp4".into(),
            headers,
            ..Default::default()
        });
    }
    ensure!(
        !formats.is_empty(),
        "YouTube не вернул медиаформаты: требуется обновить подписи/PO token или войти в аккаунт"
    );
    let subtitles = player
        .subtitles
        .into_iter()
        .map(|subtitle| {
            Ok(Subtitle {
                language: subtitle.lang,
                title: subtitle.lang_name,
                url: subtitle_url(&subtitle.url)?,
                ext: "vtt".into(),
                automatic: subtitle.auto_generated,
                translatable: false,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let details = player.details;
    Ok(Media {
        id: details.id.clone(),
        url: format!("https://www.youtube.com/watch?v={}", details.id),
        title: details.name.unwrap_or_else(|| details.id.clone()),
        description: details.description.unwrap_or_default(),
        kind: MediaKind::Video,
        duration: if details.duration == 0 {
            None
        } else {
            Some(details.duration.into())
        },
        thumbnail: thumbnail(&details.thumbnail),
        channel_id: details.channel_id,
        channel_title: details.channel_name.unwrap_or_default(),
        formats,
        subtitles,
        ..Default::default()
    })
}

fn stream_headers(
    headers: &BTreeMap<String, String>,
    size: Option<u64>,
) -> BTreeMap<String, String> {
    let mut headers = headers.clone();
    // YouTube throttles unbounded GETs; request the advertised file range without changing its URL.
    if let Some(end) = size.and_then(|size| size.checked_sub(1)) {
        headers.insert("Range".into(), format!("bytes=0-{end}"));
    }
    headers
}

fn map_entry(item: VideoItem) -> Entry {
    Entry {
        id: item.id.clone(),
        url: format!("https://www.youtube.com/watch?v={}", item.id),
        title: item.name,
        duration: item.duration.map(Into::into),
        thumbnail: thumbnail(&item.thumbnail),
    }
}

fn thumbnail(images: &[Thumbnail]) -> String {
    images
        .iter()
        .max_by_key(|image| u64::from(image.width) * u64::from(image.height))
        .map(|image| image.url.clone())
        .unwrap_or_default()
}

fn codecs(mime: &str) -> (Option<String>, Option<String>) {
    let codecs = mime
        .split(';')
        .find_map(|part| part.trim().strip_prefix("codecs="))
        .unwrap_or_default()
        .trim_matches('"')
        .split(',')
        .map(str::trim);
    let mut result = (None, None);
    for codec in codecs.filter(|codec| !codec.is_empty()) {
        if ["mp4a", "opus", "vorbis", "ac-3", "ec-3"]
            .iter()
            .any(|prefix| codec.starts_with(prefix))
        {
            result.1 = Some(codec.to_owned());
        } else {
            result.0 = Some(codec.to_owned());
        }
    }
    result
}

fn subtitle_url(value: &str) -> Result<String> {
    let mut url =
        url::Url::parse(value).map_err(|_| anyhow!("YouTube вернул неверную ссылку субтитров"))?;
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "fmt")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(pairs)
        .append_pair("fmt", "vtt");
    Ok(url.to_string())
}

fn checked_video_url(value: &str) -> Result<url::Url> {
    let mut url = crate::validate_url(value)?;
    ensure!(is_youtube_host(url.host_str().unwrap_or_default()), "Нужна ссылка YouTube");
    if let Some(path) = url.path().strip_prefix("/shorts/") {
        let id = path.trim_end_matches('/');
        ensure!(id.len() == 11 && id.bytes().all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_'), "Некорректная ссылка YouTube Shorts");
        let id = id.to_owned();
        url.set_path("/watch");
        url.set_query(None);
        url.query_pairs_mut().append_pair("v", &id);
    }
    Ok(url)
}

fn is_youtube_host(host: &str) -> bool {
    matches!(host, "youtube.com" | "youtu.be" | "youtube-nocookie.com")
        || host.ends_with(".youtube.com")
        || host.ends_with(".youtube-nocookie.com")
}

#[cfg(test)]
mod input_links {
    use super::*;
    #[test]
    fn anonymous_browser_cookies_do_not_select_the_account_download_path() {
        assert!(!has_account_cookie(&Auth {cookie_header:"VISITOR_INFO1_LIVE=fixture; YSC=fixture; CONSENT=YES".into()}));
        assert!(!has_account_cookie(&Auth {cookie_header:"XSAPISID=fixture; SAPISID= ".into()}));
        assert!(has_account_cookie(&Auth {cookie_header:"YSC=fixture; SAPISID=fixture".into()}));
    }
    #[test]
    fn shorts_link_resolves_directly_to_its_video_without_navigation_api() {
        let url = checked_video_url("https://www.youtube.com/shorts/4wayF-jA2Os?feature=share").unwrap();
        assert_eq!(url.path(), "/watch");
        assert!(url.query_pairs().any(|(key,value)| key=="v" && value=="4wayF-jA2Os"));
        assert!(checked_video_url("https://youtube.com.example.invalid/shorts/4wayF-jA2Os").is_err());
        assert!(checked_video_url("https://www.youtube.com/shorts/not-an-id").is_err());
        let watch = checked_video_url("https://www.youtube.com/watch?v=w1Pr_lXdW2A&t=12").unwrap();
        assert_eq!(watch.path(), "/watch");
        assert!(watch.query_pairs().any(|(key,value)| key=="t" && value=="12"));
    }
}

fn pipe_error(error: PipeError) -> anyhow::Error {
    let message = match error {
        PipeError::Auth(_) => "YouTube: обновите вход в аккаунт",
        PipeError::HttpStatus(403, _) => {
            "YouTube отклонил запрос. Повторите загрузку. Если видео открывается только через VPN или прокси, выберите это подключение в настройках VideoDock."
        }
        PipeError::HttpStatus(429, _) => "YouTube ограничил частоту запросов; повторите позже",
        PipeError::Extraction(ExtractionError::Deobfuscation(_)) => {
            "YouTube изменил способ загрузки видео. Проверьте обновления VideoDock."
        }
        PipeError::Extraction(ExtractionError::Botguard(_)) => {
            "Не удалось пройти проверку YouTube. Повторите загрузку. Если ошибка повторяется, проверьте подключение и обновления VideoDock."
        }
        PipeError::Extraction(ExtractionError::Unavailable { .. }) => {
            "YouTube не разрешил воспроизведение. Проверьте видео во встроенном браузере; для закрытого видео войдите в аккаунт YouTube в настройках."
        }
        PipeError::Extraction(ExtractionError::NotFound { .. }) => "YouTube: контент не найден",
        PipeError::Extraction(_) => {
            "YouTube изменил ответ; список или форматы не удалось получить полностью"
        }
        PipeError::Http(_) | PipeError::HttpStatus(_, _) => {
            "Не удалось выполнить запрос к YouTube; проверьте сеть и прокси"
        }
        PipeError::Other(_) => "YouTube: не удалось извлечь данные",
    };
    anyhow!(message)
}

fn parse_account_info(response: &str) -> Result<AccountInfo> {
    ensure!(
        response.len() <= ACCOUNT_JSON_LIMIT,
        "Ответ аккаунта YouTube слишком велик"
    );
    let data: serde_json::Value = serde_json::from_str(response)
        .map_err(|_| anyhow!("YouTube вернул некорректные данные аккаунта"))?;
    let mut stack = vec![&data];
    let mut headers = Vec::new();
    let mut nodes = 0usize;
    while let Some(node) = stack.pop() {
        nodes += 1;
        ensure!(nodes <= 65_536, "Ответ аккаунта YouTube слишком сложен");
        match node {
            serde_json::Value::Object(object) => {
                if let Some(header) = object.get("activeAccountHeaderRenderer") {
                    headers.push(header);
                }
                stack.extend(object.values());
            }
            serde_json::Value::Array(values) => stack.extend(values),
            _ => {}
        }
    }
    ensure!(
        headers.len() == 1,
        "YouTube не подтвердил единственный активный аккаунт"
    );
    let header = headers[0];
    let user_name = header["accountName"]["simpleText"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            header["accountName"]["runs"].as_array().map(|runs| {
                runs.iter()
                    .filter_map(|run| run["text"].as_str())
                    .collect::<String>()
            })
        })
        .unwrap_or_default()
        .trim()
        .to_owned();
    ensure!(
        !user_name.is_empty()
            && user_name.chars().count() <= 256
            && !user_name.chars().any(char::is_control),
        "YouTube не вернул имя активного аккаунта"
    );
    let avatar_url = header["accountPhoto"]["thumbnails"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|image| {
            let url = crate::validate_url(image["url"].as_str()?).ok()?;
            if url.scheme() != "https" {
                return None;
            }
            let size = image["width"]
                .as_u64()
                .unwrap_or(0)
                .saturating_mul(image["height"].as_u64().unwrap_or(0));
            Some((size, url.to_string()))
        })
        .max_by_key(|(size, _)| *size)
        .map(|(_, url)| url)
        .unwrap_or_default();
    Ok(AccountInfo {
        user_name,
        avatar_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_ranges_handle_unknown_zero_and_max_size_without_changing_shared_headers() {
        let base = BTreeMap::from([("User-Agent".into(), "Fixture public agent".into())]);
        for size in [None, Some(0)] {
            assert!(!stream_headers(&base, size).contains_key("Range"));
        }
        assert_eq!(stream_headers(&base, Some(1))["Range"], "bytes=0-0");
        assert_eq!(stream_headers(&base, Some(1024))["Range"], "bytes=0-1023");
        assert_eq!(
            stream_headers(&base, Some(u64::MAX))["Range"],
            "bytes=0-18446744073709551614"
        );
        assert!(!base.contains_key("Range"));
        let headers = crate::request_headers(
            &stream_headers(&base, Some(1024)),
            "https://www.youtube.com/",
            "https://cdn.example/fixture",
            &[],
        )
        .unwrap();
        assert_eq!(headers["Range"], "bytes=0-1023");
        assert!(!headers.keys().any(|name| matches!(
            name.to_ascii_lowercase().as_str(),
            "cookie" | "authorization" | "proxy-authorization"
        )));
    }
    #[tokio::test]
    async fn offline_player_preserves_track_identity_default_and_original_metadata() {
        let stream = |itag, track| {
            serde_json::json!({
                "url":"https://media.example/fixture-audio?token=synthetic-stream-token", "itag":itag,
                "bitrate":128000, "average_bitrate":128000, "size":1024,
                "mime":"audio/mp4; codecs=\"mp4a.40.2\"", "format":"m4a", "codec":"mp4a", "track":track,
                "drm_systems":[]
            })
        };
        let player: VideoPlayer = serde_json::from_value(serde_json::json!({
            "details":{"id":"fixture0001","name":"Fixture video","duration":1,"thumbnail":[],"channel_id":"fixture-channel","view_count":1,"keywords":[],"is_live":false,"is_live_content":false},
            "video_streams":[], "video_only_streams":[],
            "audio_streams":[
                stream(140,serde_json::json!({"id":"en.0","lang":"en-US","lang_name":"English","is_default":false,"track_type":"original"})),
                stream(141,serde_json::json!({"id":"ru.1","lang":"ru","lang_name":"Russian","is_default":true,"track_type":"dubbed"})),
                stream(142,serde_json::json!({"id":"unknown.0","lang":"en","lang_name":"Unknown type","is_default":false,"track_type":null})),
                stream(143,serde_json::Value::Null)
            ],
            "subtitles":[],"preview_frames":[],"expires_in_seconds":3600,"valid_until":"2026-11-03T00:00:00Z","client_type":"desktop"
        })).unwrap();
        let pipe = RustyPipe::builder()
            .no_storage()
            .no_reporter()
            .no_botguard()
            .build()
            .unwrap();
        let media = map_player(player, &pipe.query()).unwrap();
        let mapped = serde_json::to_value(&media.formats).unwrap();
        assert!(
            media
                .formats
                .iter()
                .all(|format| format.headers["Range"] == "bytes=0-1023")
        );
        assert_eq!(mapped[0]["audio_track_id"], "en.0");
        assert_eq!(mapped[0]["audio_default"], false);
        assert_eq!(mapped[0]["audio_original"], true);
        assert_eq!(mapped[1]["audio_default"], true);
        assert_eq!(mapped[1]["audio_original"], false);
        assert!(mapped[2]["audio_original"].is_null());
        assert!(mapped[3]["audio_default"].is_null());
        assert!(mapped[3]["audio_track_id"].is_null());
        assert!(!format!("{:?}", media.formats).contains("synthetic-stream-token"));
    }
    #[tokio::test]
    async fn offline_player_preserves_projection_and_stereo_without_inventing_exact_geometry() {
        let stream = |itag, projection, stereo| {
            serde_json::json!({
                "url":"https://media.example/fixture-video?token=synthetic-stream-token","itag":itag,
                "bitrate":128000,"average_bitrate":128000,"size":1024,"width":1920,"height":1080,"fps":30,"quality":"1080p","hdr":false,
                "projection_type":projection,"stereo_layout":stereo,"mime":"video/mp4; codecs=\"avc1.4d401f, mp4a.40.2\"","format":"mp4","codec":"avc1","drm_systems":[]
            })
        };
        let player: VideoPlayer = serde_json::from_value(serde_json::json!({
            "details":{"id":"fixture0001","name":"Fixture video","duration":1,"thumbnail":[],"channel_id":"fixture-channel","view_count":1,"keywords":[],"is_live":false,"is_live_content":false},
            "video_streams":[stream(18,Some("RECTANGULAR"),None),stream(19,Some("EQUIRECTANGULAR"),None),stream(20,Some("MESH"),Some("LEFT_RIGHT"))],
            "video_only_streams":[],"audio_streams":[],"subtitles":[],"preview_frames":[],"expires_in_seconds":3600,"valid_until":"2026-11-03T00:00:00Z","client_type":"desktop"
        })).unwrap();
        let pipe = RustyPipe::builder()
            .no_storage()
            .no_reporter()
            .no_botguard()
            .build()
            .unwrap();
        let media = map_player(player, &pipe.query()).unwrap();
        assert!(
            media
                .formats
                .iter()
                .all(|format| format.headers["Range"] == "bytes=0-1023")
        );
        assert_eq!(media.formats[0].projection, Some(VideoProjection::Flat));
        assert!(!media.formats[0].is_vr());
        assert!(media.formats[1].is_vr());
        assert_eq!(
            media.formats[1].projection_type.as_deref(),
            Some("EQUIRECTANGULAR")
        );
        assert_eq!(media.formats[1].projection, None);
        assert_eq!(media.formats[2].projection_type.as_deref(), Some("MESH"));
        assert_eq!(
            media.formats[2].stereo_layout.as_deref(),
            Some("LEFT_RIGHT")
        );
        assert_eq!(media.formats[2].projection, None);
        let selection = crate::Selection {
            vr_mode: crate::VideoVrMode::Stereo180,
            ..Default::default()
        };
        assert!(crate::choose_formats(&media.formats, &selection).is_err());
        assert!(!format!("{:?}", media.formats).contains("synthetic-stream-token"));
    }
    #[test]
    fn parses_muxed_and_audio_mime() {
        assert_eq!(
            codecs("video/mp4; codecs=\"avc1.42001E, mp4a.40.2\""),
            (Some("avc1.42001E".into()), Some("mp4a.40.2".into()))
        );
        assert_eq!(
            codecs("audio/webm; codecs=\"opus\""),
            (None, Some("opus".into()))
        );
    }
    #[test]
    fn replaces_subtitle_format_without_losing_other_parameters() {
        let value = subtitle_url("https://www.youtube.com/api/timedtext?v=public&fmt=srv3&lang=en")
            .unwrap();
        let parsed = url::Url::parse(&value).unwrap();
        assert_eq!(
            parsed
                .query_pairs()
                .filter(|(key, _)| key == "fmt")
                .collect::<Vec<_>>(),
            vec![("fmt".into(), "vtt".into())]
        );
        assert!(
            parsed
                .query_pairs()
                .any(|(key, value)| key == "lang" && value == "en")
        );
    }
    #[test]
    fn rejects_youtube_lookalike_hosts_and_sanitizes_errors() {
        assert!(!is_youtube_host("youtube.com.attacker.example"));
        assert!(!is_youtube_host("notyoutube.com"));
        assert!(is_youtube_host("music.youtube.com"));
        let error = pipe_error(PipeError::Http("secret-cookie-and-signed-url".into())).to_string();
        assert!(!error.contains("secret-cookie"));
    }
}

#[cfg(test)]
mod account_tests {
    use super::*;
    #[test]
    fn account_identity_uses_only_active_header_and_debug_is_redacted() {
        let response=serde_json::json!({"accountName":{"simpleText":"Inactive placeholder"},"actions":[{"openPopupAction":{"popup":{"multiPageMenuRenderer":{"header":{"activeAccountHeaderRenderer":{"accountName":{"runs":[{"text":"Fixture "},{"text":"account"}]},"accountPhoto":{"thumbnails":[{"url":"https://yt3.ggpht.com/small-fixture","width":32,"height":32},{"url":"https://yt3.ggpht.com/large-fixture","width":88,"height":88}]}}}}}}}]}).to_string();
        let account = parse_account_info(&response).unwrap();
        assert_eq!(account.user_name, "Fixture account");
        assert!(account.avatar_url.ends_with("large-fixture"));
        let debug = format!("{account:?}");
        assert!(!debug.contains("Fixture account"));
        assert!(!debug.contains("ggpht"));
    }
    #[test]
    fn missing_ambiguous_and_oversized_identity_are_errors() {
        assert!(
            parse_account_info(r#"{"accountName":{"simpleText":"Not proof of login"}}"#).is_err()
        );
        let header = serde_json::json!({"activeAccountHeaderRenderer":{"accountName":{"simpleText":"Fixture"}}});
        assert!(
            parse_account_info(&serde_json::json!([header.clone(), header]).to_string()).is_err()
        );
        assert!(parse_account_info(&" ".repeat(ACCOUNT_JSON_LIMIT + 1)).is_err());
        let error = parse_account_info(r#"{"error":"fixture-cookie-should-never-be-echoed"}"#)
            .err()
            .unwrap()
            .to_string();
        assert!(!error.contains("fixture-cookie"));
    }
    #[test]
    fn avatar_is_optional_and_cannot_be_a_local_or_insecure_url() {
        let data = serde_json::json!({"activeAccountHeaderRenderer":{"accountName":{"simpleText":"Fixture"},"accountPhoto":{"thumbnails":[{"url":"file:///private/fixture","width":999,"height":999},{"url":"http://example.com/insecure","width":999,"height":999}]}}});
        assert!(
            parse_account_info(&data.to_string())
                .unwrap()
                .avatar_url
                .is_empty()
        );
    }
    #[tokio::test]
    async fn empty_credentials_never_become_logged_in_identity() {
        let youtube = YouTube::new(None).await.unwrap();
        let error = youtube
            .account_info(&Auth::default())
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("Нет данных авторизации"));
    }
}
