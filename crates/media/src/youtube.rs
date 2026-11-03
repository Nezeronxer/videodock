use crate::{Auth, Entry, Format, Media, MediaKind, Protocol, Subtitle};
use anyhow::{Result, anyhow, ensure};
use rustypipe::{
    client::{ClientType, RustyPipe, RustyPipeQuery},
    error::{Error as PipeError, ExtractionError},
    model::{
        Thumbnail, UrlTarget, VideoItem, VideoPlayer, paginator::Paginator, richtext::ToPlaintext,
        traits::FileFormat,
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

pub struct YouTube {
    proxy: Option<wreq::Proxy>,
}

impl YouTube {
    pub async fn new(proxy: Option<&str>) -> Result<Self> {
        let proxy = proxy
            .filter(|value| !value.trim().is_empty())
            .map(|value| wreq::Proxy::all(value).map_err(|_| anyhow!("Некорректный адрес прокси")))
            .transpose()?;
        Ok(Self { proxy })
    }

    async fn client(&self, auth: &Auth) -> Result<RustyPipe> {
        ensure!(
            !auth.cookie_header.contains(['\r', '\n']),
            "Недопустимый Cookie-заголовок"
        );
        // shortcut: wreq 0.16 player POSTs reset HTTP/2 streams here; restore HTTP/2 after a verified transport update.
        let mut http = wreq::Client::builder().no_proxy().http1_only();
        if let Some(proxy) = &self.proxy {
            http = http.proxy(proxy.clone());
        }
        // Credentials stay in this request's memory; disable filesystem reports and helper discovery.
        let client = RustyPipe::builder()
            .strict()
            .no_storage()
            .no_reporter()
            .no_botguard()
            .timeout(Duration::from_secs(30))
            .n_http_retries(1)
            .build_with_client(http)
            .map_err(pipe_error)?;
        if !auth.cookie_header.trim().is_empty() {
            client
                .user_auth_set_cookie(auth.cookie_header.clone())
                .await
                .map_err(pipe_error)?;
        }
        Ok(client)
    }

    pub async fn inspect(&self, value: &str, auth: &Auth) -> Result<Media> {
        let url = crate::validate_url(value)?;
        ensure!(
            is_youtube_host(url.host_str().unwrap_or_default()),
            "Нужна ссылка YouTube"
        );
        let client = self.client(auth).await?;
        let query = query(&client, auth);
        let target = query
            .clone()
            .resolve_url(url.as_str(), false)
            .await
            .map_err(pipe_error)?;
        match target {
            UrlTarget::Video { id, .. } => {
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

    pub async fn verify_auth(&self, auth: &Auth) -> Result<()> {
        ensure!(
            !auth.cookie_header.trim().is_empty(),
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
            !auth.cookie_header.trim().is_empty(),
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

fn query(client: &RustyPipe, auth: &Auth) -> RustyPipeQuery {
    if auth.cookie_header.trim().is_empty() {
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
                video_codec,
                audio_codec: if muxed { audio_codec } else { None },
                bitrate: Some(stream.bitrate.into()),
                size: stream.size,
                headers: headers.clone(),
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
            bitrate: Some(stream.bitrate.into()),
            size: Some(stream.size),
            headers: headers.clone(),
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

fn is_youtube_host(host: &str) -> bool {
    matches!(host, "youtube.com" | "youtu.be" | "youtube-nocookie.com")
        || host.ends_with(".youtube.com")
        || host.ends_with(".youtube-nocookie.com")
}

fn pipe_error(error: PipeError) -> anyhow::Error {
    let message = match error {
        PipeError::Auth(_) => "YouTube: обновите вход в аккаунт",
        PipeError::HttpStatus(403, _) => "YouTube: HTTP403; обновите ссылку, подпись или PO token",
        PipeError::HttpStatus(429, _) => "YouTube ограничил частоту запросов; повторите позже",
        PipeError::Extraction(ExtractionError::Deobfuscation(_)) => {
            "YouTube изменил подписи ссылок; нужен актуальный экстрактор"
        }
        PipeError::Extraction(ExtractionError::Botguard(_)) => {
            "YouTube требует актуальный PO token"
        }
        PipeError::Extraction(ExtractionError::Unavailable { .. }) => {
            "YouTube не разрешил воспроизведение: проверьте доступ, вход в аккаунт и PO token"
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
