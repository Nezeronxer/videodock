use super::*;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs::OpenOptions, io::Write, path::Path};
use videodock_media::{Entry, Format, MediaKind};

#[derive(Clone, Serialize, Deserialize)]
struct Spec {
    selection: Selection,
    output_dir: PathBuf,
    filename: Option<String>,
    #[serde(default)]
    embed_subtitles: Option<bool>,
    #[serde(default)]
    tags: Option<bool>,
    #[serde(default)]
    skip_duplicates: Option<bool>,
    #[serde(default)]
    smart: bool,
    #[serde(default)]
    auth_origin: Option<String>,
}
pub(super) fn catalog(inner: &Inner) -> Result<videodock_media::catalog::Catalog> {
    let proxy = secrets::proxy(inner)?;
    videodock_media::catalog::Catalog::new(proxy.as_deref())
}
fn spec(settings: &Value, action: &Value) -> Result<Spec> {
    let output = action["output_dir"]
        .as_str()
        .filter(|v| !v.is_empty())
        .or_else(|| settings["path"].as_str())
        .context("Не указана папка загрузки")?;
    ensure!(
        Path::new(output).is_absolute(),
        "Нужен абсолютный путь к папке загрузки"
    );
    let filename = action["filename"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|value| settings::bounded(&json!(value), 240))
        .transpose()?;
    ensure!(
        filename
            .as_ref()
            .is_none_or(|value| !value.contains(['/', '\\'])),
        "Имя файла не должно содержать путь"
    );
    Ok(Spec {
        selection: selection(settings, action)?,
        output_dir: output.into(),
        filename,
        embed_subtitles: Some(settings["embedSubtitles"] == true),
        tags: Some(settings["tagsEnabled"] == true),
        skip_duplicates: Some(settings["skipDuplicates"] == true),
        smart: action["_smart"] == true,
        auth_origin: action["_auth_origin"].as_str().map(str::to_owned),
    })
}
fn origin(source: &str) -> Option<String> {
    let url = videodock_media::validate_url(source).ok()?;
    let host = url.host_str().unwrap_or_default();
    Some(
        if host == "youtu.be"
            || host == "youtube.com"
            || host.ends_with(".youtube.com")
            || host == "youtube-nocookie.com"
            || host.ends_with(".youtube-nocookie.com")
        {
            "https://www.youtube.com".into()
        } else {
            url.origin().ascii_serialization()
        },
    )
}
fn source_kind(media: &Media) -> String {
    serde_json::to_value(&media.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "video".into())
}
fn row(url: &str, title: &str, kind: &str, spec: &Spec) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"url":url,"title":title,"kind":kind,"media_type":spec.selection.media_type,"state":"queued","progress":0,"created_at":now(),"error":"","thumbnail":"","format":spec.selection.container.to_ascii_uppercase(),"quality":if spec.selection.media_type=="audio"{if spec.selection.audio_bitrate==0{"Original".into()}else{format!("{} kbps",spec.selection.audio_bitrate)}}else if spec.selection.quality==0{"Original".into()}else{format!("≤{}p",spec.selection.quality)},"requested_quality":spec.selection.quality,"requested_audio_bitrate":spec.selection.audio_bitrate,"_spec":spec})
}
fn add_media(
    state: &mut Value,
    media: &Media,
    action: &Value,
    settings: &Value,
    subscription: Option<&str>,
) -> Result<Vec<String>> {
    let mut spec = spec(settings, action)?;
    if media.kind == MediaKind::Audio && spec.smart && spec.selection.media_type == "video" {
        spec.selection.media_type = "audio".into();
        if ["mp4", "mkv", "webm"].contains(&spec.selection.container.as_str()) {
            spec.selection.container = "m4a".into();
        }
    }
    if spec.auth_origin.is_none() {
        spec.auth_origin = origin(&media.url);
    }
    let batch = matches!(
        media.kind,
        MediaKind::Playlist | MediaKind::Channel | MediaKind::Search
    );
    if !batch {
        let mut item = row(
            &media.url,
            &media.title,
            if spec.selection.media_type == "audio" {
                "audio"
            } else {
                "video"
            },
            &spec,
        );
        item["thumbnail"] = json!(if media.thumbnail.starts_with("file:") {
            &media.thumbnail
        } else {
            ""
        });
        item["channel_title"] = json!(media.channel_title);
        item["channel_id"] = json!(media.channel_id);
        if let Some(subscription) = subscription {
            item["_subscription"] = json!(subscription);
        }
        let id = item["id"].as_str().unwrap().to_owned();
        state["jobs"].as_array_mut().unwrap().push(item);
        return Ok(vec![id]);
    }
    let selection = action
        .get("selected_ids")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).collect::<HashSet<_>>());
    let entries = media
        .entries
        .iter()
        .filter(|entry| {
            selection
                .as_ref()
                .is_none_or(|ids| ids.contains(entry.id.as_str()))
        })
        .collect::<Vec<_>>();
    ensure!(!entries.is_empty(), "Не выбраны элементы для загрузки");
    spec.selection.format_id = None;
    if settings["separateFolderCreationEnabled"] == true {
        spec.output_dir = spec
            .output_dir
            .join(crate::pipeline::safe_name(&media.title));
    }
    let mut group = row(&media.url, &media.title, &source_kind(media), &spec);
    let group_id = group["id"].as_str().unwrap().to_owned();
    group["_group"] = json!(true);
    group["path"] = json!(spec.output_dir);
    group["_m3u"] = json!(
        settings["m3uEnabled"] == true
            && matches!(media.kind, MediaKind::Playlist | MediaKind::Search)
    );
    if let Some(subscription) = subscription {
        group["_subscription"] = json!(subscription);
    }
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        videodock_media::validate_url(&entry.url)?;
        if settings["skipDuplicates"] == true {
            if !seen.insert(entry.url.clone()) {
                continue;
            }
            if state["jobs"].as_array().unwrap().iter().any(|job| {
                job["url"] == entry.url
                    && job["state"] != "error"
                    && (job["state"] != "complete"
                        || job["path"]
                            .as_str()
                            .is_some_and(|path| Path::new(path).is_file()))
            }) {
                continue;
            }
        }
        let mut child_spec = spec.clone();
        if settings["numerationEnabled"] == true {
            child_spec.filename = Some(format!("{:03} {}", index + 1, entry.title));
        }
        if child_spec.auth_origin.is_none() {
            child_spec.auth_origin = origin(&entry.url);
        }
        let mut child = row(
            &entry.url,
            &entry.title,
            if spec.selection.media_type == "audio" {
                "audio"
            } else {
                "video"
            },
            &child_spec,
        );
        child["_parent"] = json!(group_id);
        child["_index"] = json!(index);
        if let Some(subscription) = subscription {
            child["_subscription"] = json!(subscription);
        }
        ids.push(child["id"].as_str().unwrap().to_owned());
        state["jobs"].as_array_mut().unwrap().push(child);
    }
    ensure!(
        !ids.is_empty(),
        "Все выбранные элементы уже имеются в списке"
    );
    state["jobs"].as_array_mut().unwrap().push(group);
    Ok(ids)
}
fn add_subscription(inner: &Inner, media: &Media, action: &Value) -> Result<Vec<String>> {
    ensure!(
        matches!(media.kind, MediaKind::Channel | MediaKind::Playlist),
        "Подписаться можно только на канал или плейлист"
    );
    let settings = lock(&inner.state)["settings"].clone();
    let mut spec = spec(&settings, action)?;
    if spec.auth_origin.is_none() {
        spec.auth_origin = origin(&media.url);
    }
    let editing = lock(&inner.state)["manual"]["id"]
        .as_str()
        .map(str::to_owned);
    change(inner, |state| {
        if let Some(existing) = state["subscriptions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["url"] == secrets::masked(&media.url))
        {
            ensure!(
                editing.as_deref() == existing["id"].as_str(),
                "Вы уже подписаны на этот канал или плейлист"
            );
            existing["media_type"] = json!(spec.selection.media_type);
            existing["_spec"] = json!(spec);
            existing["title"] = json!(media.title);
            return Ok(Vec::new());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let item = json!({"id":id,"url":media.url,"title":media.title,"kind":source_kind(media),"media_type":spec.selection.media_type,"state":"subscribed","created_at":now(),"last_checked":now(),"new_count":0,"_baseline":true,"_seen_ids":media.entries.iter().map(|entry|entry.id.clone()).collect::<Vec<_>>(),"_spec":spec});
        let ids = if action["download_existing"] == true {
            let mut action = action.clone();
            action.as_object_mut().unwrap().remove("selected_ids");
            add_media(state, media, &action, &settings, Some(&id))?
        } else {
            Vec::new()
        };
        state["subscriptions"].as_array_mut().unwrap().push(item);
        Ok(ids)
    })
}
fn parsed_media(inner: &Inner, action: &Value) -> Result<Media> {
    let media = lock(&inner.manual)
        .clone()
        .context("Нет разобранного источника. Сначала вставьте ссылку.")?;
    if let Some(url) = action["url"].as_str() {
        ensure!(
            url == secrets::masked(&media.url),
            "Источник диалога изменился; откройте его заново"
        );
    }
    Ok(media)
}
pub(super) fn preview_output(inner: &Inner, action: &Value) -> Result<Value> {
    let media = parsed_media(inner, action)?;
    ensure!(
        !matches!(
            media.kind,
            MediaKind::Playlist | MediaKind::Channel | MediaKind::Search
        ),
        "Для списка выберите папку загрузки, а не отдельный файл"
    );
    let settings = lock(&inner.state)["settings"].clone();
    let spec = spec(&settings, action)?;
    videodock_media::choose_formats(&media.formats, &spec.selection)?;
    let path = crate::pipeline::preview_output_path(
        &media,
        &spec.output_dir,
        spec.filename.as_deref(),
        &spec.selection.container,
    )?;
    Ok(json!({"download_path":path}))
}
pub(super) fn download(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    let media = parsed_media(inner, action)?;
    let ids = if action["mode"] == "subscribe" {
        add_subscription(inner, &media, action)?
    } else {
        let settings = lock(&inner.state)["settings"].clone();
        change(inner, |state| {
            add_media(state, &media, action, &settings, None)
        })?
    };
    *lock(&inner.manual) = None;
    change(inner, |state| {
        state["manual"] = Value::Null;
        Ok(())
    })?;
    Ok(json!({"ids":ids}))
}
pub(super) fn parse(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    let source = settings::text(&action["url"])?.trim().to_owned();
    ensure!(
        !source.is_empty() && source.len() <= 16384,
        "Вставьте ссылку HTTP(S)"
    );
    let urls = source.split_whitespace().collect::<Vec<_>>();
    ensure!(urls.len() <= 1000, "Слишком много ссылок");
    for url in urls {
        videodock_media::validate_url(url)?;
    }
    let generation = inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
    inner.parse_paused.store(false, Ordering::SeqCst);
    inner.parse_wake.notify_one();
    let cancel = inner.shutdown.child_token();
    if let Some(previous) = lock(&inner.parse_cancel).replace(cancel.clone()) {
        previous.cancel();
    }
    *lock(&inner.manual) = None;
    let settings = lock(&inner.state)["settings"].clone();
    let smart = settings["smartMode"] == true
        && action["force_manual"] != true
        && action["mode"] != "subscribe";
    if smart {
        let mut spec = spec(&settings, &json!({"_smart":true}))?;
        spec.auth_origin = origin(&source);
        change(inner, |state| {
            state["manual"] = Value::Null;
            state["notice"] = json!("");
            state["jobs"].as_array_mut().unwrap().push(row(
                &source,
                &source,
                &spec.selection.media_type,
                &spec,
            ));
            Ok(())
        })?;
        return Ok(json!({"request_id":generation}));
    }
    change(inner, |state| {
        state["manual"] = json!({"title":source,"url":source,"state":"parsing","kind":"video","formats":[],"items":[],"subtitles":[],"audio_tracks":[]});
        state["notice"] = json!("");
        Ok(())
    })?;
    let weak = Arc::downgrade(inner);
    let action = action.clone();
    runtime()?.spawn(async move {
        let Some(inner) = weak.upgrade() else { return };
        let result = parse_async(&inner, &source, &action, generation, &cancel).await;
        if !cancel.is_cancelled()
            && inner.generation.load(Ordering::SeqCst) == generation
            && let Err(error) = result
        {
            let message = error.to_string();
            let _ = change(&inner, |state| {
                state["notice"] = json!(message);
                state["manual"]["state"] = json!("error");
                state["manual"]["error"] = json!(message);
                Ok(())
            });
        }
    });
    Ok(json!({"request_id":generation}))
}
pub(super) fn parsing_control(inner: &Inner, action: &Value) -> Result<Value> {
    let paused = action["action"] == "manual_pause";
    change(inner, |state| {
        ensure!(
            matches!(
                state["manual"]["state"].as_str(),
                Some("parsing" | "parsing_paused")
            ),
            "Разбор сейчас не выполняется"
        );
        inner.parse_paused.store(paused, Ordering::SeqCst);
        state["manual"]["state"] = json!(if paused { "parsing_paused" } else { "parsing" });
        Ok(())
    })?;
    if !paused {
        inner.parse_wake.notify_one();
    }
    Ok(json!({}))
}
async fn await_parsing(inner: &Inner, cancel: &CancellationToken) -> Result<()> {
    while inner.parse_paused.load(Ordering::SeqCst) {
        tokio::select! {
            _ = cancel.cancelled() => bail!("Разбор отменён"),
            _ = inner.parse_wake.notified() => {}
        }
    }
    ensure!(!cancel.is_cancelled(), "Разбор отменён");
    Ok(())
}
async fn inspect(
    inner: &Inner,
    source: &str,
    cancel: &CancellationToken,
) -> Result<(Media, reqwest::Client)> {
    inspect_in_context(inner, source, None, cancel).await
}
async fn inspect_in_context(
    inner: &Inner,
    source: &str,
    auth_origin: Option<&str>,
    cancel: &CancellationToken,
) -> Result<(Media, reqwest::Client)> {
    let catalog = catalog(inner)?;
    let own_origin = origin(source);
    let auth_origin = auth_origin.or(own_origin.as_deref()).unwrap_or(source);
    let header_target = if own_origin.as_deref() == Some("https://www.youtube.com") {
        "https://www.youtube.com/"
    } else {
        source
    };
    let auth = secrets::auth_in_context(inner, header_target, auth_origin);
    let operation = async {
        let sources = source.split_whitespace().collect::<Vec<_>>();
        if sources.len() > 1 {
            use sha2::{Digest, Sha256};
            let entries = sources
                .iter()
                .map(|source| {
                    videodock_media::validate_url(source)?;
                    Ok(Entry {
                        id: format!("{:x}", Sha256::digest(source.as_bytes())),
                        url: source.to_string(),
                        title: source.to_string(),
                        ..Default::default()
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            return Ok(Media {
                id: "clipboard".into(),
                url: source.into(),
                title: "Список ссылок".into(),
                kind: MediaKind::Playlist,
                entries,
                ..Default::default()
            });
        }
        let url = videodock_media::validate_url(source)?;
        if url
            .host_str()
            .is_some_and(|host| host == "youtube.com" || host.ends_with(".youtube.com"))
            && url.path() == "/results"
        {
            let query = url
                .query_pairs()
                .find(|(key, _)| key == "search_query")
                .map(|(_, value)| value.into_owned())
                .context("Нет поискового запроса")?;
            return catalog.search(&query, &auth).await;
        }
        catalog.inspect(source, &auth).await
    };
    let mut media =
        tokio::select! {_ = cancel.cancelled()=>bail!("Разбор отменён"),result=operation=>result?};
    // Titles can fall back to signed URLs; sanitize before deriving filenames and folders.
    media.title = secrets::masked(&media.title);
    for entry in &mut media.entries {
        entry.title = secrets::masked(&entry.title);
    }
    media.cookies = secrets::context(inner, auth_origin);
    Ok((media, catalog.client()))
}
async fn parse_async(
    inner: &Arc<Inner>,
    source: &str,
    action: &Value,
    generation: u64,
    cancel: &CancellationToken,
) -> Result<()> {
    await_parsing(inner, cancel).await?;
    let (mut media, client) = inspect(inner, source, cancel).await?;
    media.url = source.to_owned();
    ensure!(!cancel.is_cancelled(), "Разбор отменён");
    let settings = lock(&inner.state)["settings"].clone();
    let batch = matches!(
        media.kind,
        MediaKind::Playlist | MediaKind::Channel | MediaKind::Search
    );
    let mut errors = HashMap::new();
    if batch {
        ensure!(
            !media.entries.is_empty(),
            "В списке нет доступных элементов"
        );
        let mut preview = media.clone();
        preview.formats.clear();
        let mut keys = HashSet::new();
        for (index, entry) in media.entries.iter().enumerate() {
            await_parsing(inner, cancel).await?;
            ensure!(!cancel.is_cancelled(), "Разбор отменён");
            let context = origin(&media.url);
            let result = async {
                let (mut clip, http) =
                    inspect_in_context(inner, &entry.url, context.as_deref(), cancel).await?;
                crate::pipeline::prepare(&http, &mut clip, cancel).await?;
                Ok::<_, anyhow::Error>(clip)
            }
            .await;
            match result {
                Ok(clip) => merge_batch_metadata(&mut preview, &mut keys, clip),
                Err(error) => {
                    errors.insert(entry.id.clone(), error.to_string());
                }
            }
            if inner.generation.load(Ordering::SeqCst) != generation {
                return Ok(());
            }
            let mut view = manual_view(&preview, &settings, &errors);
            view["parsed_count"] = json!(index + 1);
            view["total_count"] = json!(media.entries.len());
            change(inner, |state| {
                view["state"] = json!(if inner.parse_paused.load(Ordering::SeqCst) {
                    "parsing_paused"
                } else {
                    "parsing"
                });
                state["manual"] = view;
                Ok(())
            })?;
        }
        media.formats = preview.formats;
        media.subtitles = preview.subtitles;
        ensure!(
            !media.formats.is_empty(),
            "Не найдено доступных форматов ни для одного элемента"
        );
    } else {
        crate::pipeline::prepare(&client, &mut media, cancel).await?;
    }
    await_parsing(inner, cancel).await?;
    if cancel.is_cancelled() || inner.generation.load(Ordering::SeqCst) != generation {
        return Ok(());
    }
    media.thumbnail = thumbnail(inner, &client, &media.thumbnail, cancel)
        .await
        .unwrap_or_default();
    let mut view = manual_view(&media, &settings, &errors);
    view["state"] = json!("ready");
    view["mode"] = json!(if action["mode"] == "subscribe" {
        "subscribe"
    } else {
        "download"
    });
    if let Some(id) = action["subscription_id"].as_str()
        && let Some(subscription) = lock(&inner.state)["subscriptions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|subscription| subscription["id"] == id)
    {
        view["id"] = json!(id);
        view["editing"] = json!(true);
        view["preferences"] = subscription["_spec"]["selection"].clone();
        view["output_dir"] = subscription["_spec"]["output_dir"].clone();
        if let Ok(mut selection) = serde_json::from_value::<Selection>(view["preferences"].clone())
        {
            selection.format_id = None;
            if let Ok(formats) = videodock_media::choose_formats(&media.formats, &selection)
                && let Some(format) = formats.iter().find(|format| {
                    if selection.media_type == "audio" {
                        format.has_audio()
                    } else {
                        format.has_video()
                    }
                })
            {
                view["selected_format_id"] = json!(format.id);
            }
        }
    }
    *lock(&inner.manual) = Some(media);
    change(inner, |state| {
        state["manual"] = view;
        Ok(())
    })?;
    Ok(())
}
fn merge_batch_metadata(preview: &mut Media, keys: &mut HashSet<String>, clip: Media) {
    for format in clip.formats {
        let key = format!(
            "{:?}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{:?}",
            format.protocol,
            format.ext,
            format.height,
            format.fps,
            format.video_codec,
            format.audio_codec,
            format.bitrate,
            format.projection,
            format.vr,
            format.projection_type,
            format.stereo_layout,
            (
                &format.audio_track_id,
                &format.audio_language,
                &format.audio_title,
                format.audio_default,
                format.audio_original,
            )
        );
        if keys.insert(key) {
            preview.formats.push(format);
        }
    }
    for subtitle in clip.subtitles {
        if let Some(existing) = preview.subtitles.iter_mut().find(|existing| {
            existing.language == subtitle.language && existing.automatic == subtitle.automatic
        }) {
            existing.translatable |= subtitle.translatable;
        } else {
            preview.subtitles.push(subtitle);
        }
    }
}
fn manual_view(media: &Media, settings: &Value, errors: &HashMap<String, String>) -> Value {
    let mut formats = Vec::new();
    for format in &media.formats {
        let compatible = media.formats.iter().any(|other| {
            other.has_video() && !modern_codec(other) && other.height == format.height
        });
        for video in [true, false] {
            if (video && !format.has_video()) || (!video && !format.has_audio()) {
                continue;
            }
            if video && settings["displayAllCodecs"] != true && modern_codec(format) && compatible {
                continue;
            }
            let quality = if video {
                format
                    .video_quality()
                    .map(|height| format!("{height}p"))
                    .unwrap_or_else(|| "Original".into())
            } else if format.has_video() {
                "Original".into()
            } else {
                format
                    .bitrate
                    .map(|bitrate| format!("{} kbps", bitrate / 1000))
                    .unwrap_or_else(|| "Original".into())
            };
            let codec = if video {
                format.video_codec.as_deref()
            } else {
                format.audio_codec.as_deref()
            }
            .unwrap_or("");
            formats.push(json!({"id":format.id,"label":format!("{} · {} · {}",format.ext.to_ascii_uppercase(),quality,codec),
                "media_type":if video{"video"}else{"audio"},"container":format.ext.to_ascii_uppercase(),"quality":quality,
                "video_codec":format.video_codec,"audio_codec":format.audio_codec,
                "fps":if video{format.fps.map(|fps|fps.to_string()).unwrap_or_default()}else{String::new()},
                "vr":if video {format.projection.map(|projection|json!(projection)).unwrap_or_else(||json!(if format.is_vr(){"Unknown"}else{"None"}))}else{json!("None")},"projection":format.projection,"size":format.size.map(bytes)}));
        }
    }
    let only_audio =
        !media.formats.iter().any(Format::has_video) && media.formats.iter().any(Format::has_audio);
    let mut tracks = BTreeSet::new();
    for format in &media.formats {
        if format.has_audio() {
            tracks.insert((
                format
                    .audio_language
                    .clone()
                    .unwrap_or_else(|| "default".into()),
                format.audio_title.clone().unwrap_or_else(|| {
                    format
                        .audio_language
                        .clone()
                        .unwrap_or_else(|| "Default".into())
                }),
            ));
        }
    }
    let mut view = json!({"url":media.url,"title":media.title,"kind":if only_audio&&!matches!(media.kind,MediaKind::Playlist|MediaKind::Channel|MediaKind::Search){"audio".into()}else{source_kind(media)},
        "media_type":if only_audio{"audio"}else{settings["mediaType"].as_str().unwrap_or("video")},
        "thumbnail":if media.thumbnail.starts_with("file:"){&media.thumbnail}else{""},
        "formats":formats,"items":media.entries.iter().map(|entry|json!({"id":entry.id,"title":entry.title,
            "duration":entry.duration.map(|duration|format!("{}:{:02}",duration as u64/60,duration as u64%60)),"error":errors.get(&entry.id)})).collect::<Vec<_>>(),
        "subtitles":media.subtitles.iter().map(|subtitle|json!({"id":if subtitle.automatic{format!("{}:auto",subtitle.language)}else{subtitle.language.clone()},
            "language":subtitle.language,"label":subtitle.title,"automatic":subtitle.automatic})).collect::<Vec<_>>(),
        "can_translate_subtitles":media.subtitles.iter().any(|subtitle|subtitle.translatable),
        "audio_tracks":tracks.into_iter().map(|(language,title)|json!({"id":language,"language":language,"label":title,"default":language=="default"})).collect::<Vec<_>>(),
        "output_dir":settings["path"]});
    if !matches!(
        media.kind,
        MediaKind::Playlist | MediaKind::Channel | MediaKind::Search
    ) {
        match crate::pipeline::preview_output_path(
            media,
            Path::new(settings["path"].as_str().unwrap_or_default()),
            None,
            settings["container"].as_str().unwrap_or_default(),
        ) {
            Ok(path) => view["download_path"] = json!(path),
            Err(error) => view["download_path_error"] = json!(error.to_string()),
        }
    }
    view
}
fn modern_codec(format: &Format) -> bool {
    format.video_codec.as_deref().is_some_and(|codec| {
        codec.starts_with("av01")
            || codec.starts_with("av1")
            || codec.starts_with("vp9")
            || codec.starts_with("vp09")
    })
}
fn bytes(value: u64) -> String {
    if value >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", value as f64 / 1073741824.0)
    } else if value >= 1024 * 1024 {
        format!("{:.1} MB", value as f64 / 1048576.0)
    } else {
        format!("{} KB", value / 1024)
    }
}
async fn thumbnail(
    inner: &Inner,
    client: &reqwest::Client,
    value: &str,
    cancel: &CancellationToken,
) -> Result<String> {
    use sha2::{Digest, Sha256};
    if value.is_empty() {
        return Ok(String::new());
    }
    let url = videodock_media::validate_url(value)?;
    let digest = format!("{:x}", Sha256::digest(value.as_bytes()));
    let folder = inner.directory.join("thumbnails");
    tokio::fs::create_dir_all(&folder).await?;
    let path = folder.join(format!("{digest}.img"));
    if !path.exists() {
        let mut response=tokio::select!{_ = cancel.cancelled()=>bail!("Отменено"),result=client.get(url).send()=>result?}.error_for_status()?;
        let mut data = Vec::new();
        loop {
            let chunk = tokio::select! {_ = cancel.cancelled()=>bail!("Отменено"),result=response.chunk()=>result?};
            let Some(chunk) = chunk else { break };
            ensure!(
                data.len() + chunk.len() <= 8 * 1024 * 1024,
                "Слишком большое изображение"
            );
            data.extend_from_slice(&chunk);
        }
        ensure!(
            data.starts_with(b"\x89PNG\r\n\x1a\n")
                || data.starts_with(b"\xff\xd8\xff")
                || data.starts_with(b"GIF8")
                || (data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP")),
            "Неподдерживаемый формат изображения"
        );
        tokio::fs::write(&path, data).await?;
    }
    Ok(url::Url::from_file_path(path)
        .map_err(|_| anyhow!("Некорректный путь изображения"))?
        .to_string())
}
fn actual_info(item: &mut Value, info: &Value) {
    item["media_info"] = info.clone();
    let streams = info["streams"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let video = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video");
    let audio = streams
        .iter()
        .find(|stream| stream["codec_type"] == "audio");
    if let Some(video) = video {
        if let Some(height) = video["height"].as_u64() {
            let quality = video["width"]
                .as_u64()
                .filter(|width| *width > 0)
                .map_or(height, |width| width.min(height));
            item["quality"] = json!(format!("{quality}p"));
            item["height"] = json!(height);
        }
        item["width"] = video["width"].clone();
        item["video_codec"] = video["codec_name"].clone();
        if let Some(rate) = video["avg_frame_rate"].as_str() {
            let fps = if let Some((n, d)) = rate.split_once('/') {
                n.parse::<f64>()
                    .ok()
                    .zip(d.parse::<f64>().ok())
                    .filter(|(_, d)| *d > 0.0)
                    .map(|(n, d)| n / d)
            } else {
                rate.parse().ok()
            };
            if let Some(fps) = fps.filter(|fps| fps.is_finite() && *fps > 0.0) {
                item["fps"] = json!(
                    format!("{fps:.2}")
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                );
            }
        }
    }
    if let Some(audio) = audio {
        item["audio_codec"] = audio["codec_name"].clone();
        if video.is_none()
            && let Some(bitrate) = audio["bit_rate"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
                .or_else(|| audio["bit_rate"].as_u64())
                .or_else(|| {
                    info["format"]["bit_rate"]
                        .as_str()
                        .and_then(|value| value.parse::<u64>().ok())
                })
        {
            item["quality"] = json!(format!("{} kbps", bitrate / 1000));
        }
    }
    item["codec"] = json!(
        [
            video.and_then(|stream| stream["codec_name"].as_str()),
            audio.and_then(|stream| stream["codec_name"].as_str())
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" / ")
    );
    if let Some(duration) = info["format"]["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
    {
        item["duration"] = json!(duration);
    }
}
pub(super) fn public_state(state: &Value) -> Value {
    let mut result = state.clone();
    let all = state["jobs"].as_array().cloned().unwrap_or_default();
    let mut visible = Vec::new();
    for item in &all {
        if item.get("_parent").is_some() || item.get("_remove").is_some() || item["_hide"] == true {
            continue;
        }
        let mut item = item.clone();
        if item["_group"] == true {
            let children = all
                .iter()
                .filter(|child| child["_parent"] == item["id"] && child.get("_remove").is_none())
                .cloned()
                .collect::<Vec<_>>();
            if !children.is_empty() {
                item["progress"] = json!(
                    children
                        .iter()
                        .map(|child| child["progress"].as_f64().unwrap_or(0.0))
                        .sum::<f64>()
                        / children.len() as f64
                );
                let status = if children.iter().all(|c| c["state"] == "complete") {
                    if item["_aux_error"].is_string() {
                        "error"
                    } else {
                        "complete"
                    }
                } else if children.iter().any(|c| {
                    matches!(
                        c["state"].as_str(),
                        Some("downloading" | "parsing" | "processing")
                    )
                }) {
                    "downloading"
                } else if children.iter().any(|c| c["state"] == "queued") {
                    "queued"
                } else if children.iter().any(|c| c["state"] == "error") {
                    "error"
                } else {
                    "paused"
                };
                item["state"] = json!(status);
                item["size"] = json!(bytes(
                    children
                        .iter()
                        .filter_map(|c| c["_size_bytes"].as_u64())
                        .sum()
                ));
            }
            item["items"] = json!(children);
        }
        visible.push(item);
    }
    result["jobs"] = json!(visible);
    fn clean(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.retain(|key, _| !key.starts_with('_') && key != "proxyPassword");
                for key in ["error", "notice"] {
                    if let Some(Value::String(message)) = map.get_mut(key) {
                        let diagnostic = message.to_ascii_lowercase();
                        if diagnostic.contains("sabr")
                            || diagnostic.contains("po token")
                            || diagnostic.contains("botguard")
                        {
                            *message = if diagnostic.contains("403") {
                                "YouTube отклонил загрузку. Повторите попытку. Если видео открывается только через VPN или прокси, выберите это подключение в настройках VideoDock."
                            } else {
                                "Не удалось получить видео с YouTube. Повторите загрузку. Если ошибка повторяется, проверьте воспроизведение во встроенном браузере и обновления VideoDock."
                            }.to_owned();
                        }
                    }
                }
                for value in map.values_mut() {
                    clean(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    clean(value);
                }
            }
            _ => {}
        }
    }
    clean(&mut result);
    result
}

pub(super) fn start(inner: &Arc<Inner>) -> Result<()> {
    let weak = Arc::downgrade(inner);
    let shutdown = inner.shutdown.clone();
    let wake = inner.wake.clone();
    runtime()?.spawn(async move{loop{
        let Some(inner)=weak.upgrade()else{break};if shutdown.is_cancelled(){break}
        if let Err(error)=kick(&inner){notice(&inner,error);}
        check_subscriptions(&inner);check_library(&inner);drop(inner);
        tokio::select!{_ = shutdown.cancelled()=>break,_ = wake.notified()=>{},_ = tokio::time::sleep(Duration::from_millis(250))=>{}}
    }});
    Ok(())
}
fn job<'a>(state: &'a Value, id: &str) -> Option<&'a Value> {
    state["jobs"].as_array()?.iter().find(|job| job["id"] == id)
}
fn job_mut<'a>(state: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    state["jobs"]
        .as_array_mut()?
        .iter_mut()
        .find(|job| job["id"] == id)
}
fn kick(inner: &Arc<Inner>) -> Result<()> {
    let mut active = lock(&inner.active);
    let state = lock(&inner.state).clone();
    let capacity = state["settings"]["simultaneousDownloads"]
        .as_u64()
        .unwrap_or(4)
        .clamp(1, 8) as usize;
    let active_downloads = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| active.contains_key(item["id"].as_str().unwrap_or_default()))
        .count();
    let available = capacity.saturating_sub(active_downloads);
    let ids = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|job| {
            job["state"] == "queued"
                && job["_group"] != true
                && job.get("_remove").is_none()
                && !active.contains_key(job["id"].as_str().unwrap_or_default())
        })
        .take(available)
        .map(|job| job["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    for id in ids {
        let cancel = inner.shutdown.child_token();
        active.insert(id.clone(), cancel.clone());
        if let Err(error) = change(inner, |state| {
            let item = job_mut(state, &id).context("Элемент исчез из очереди")?;
            item["state"] = json!("parsing");
            item["error"] = json!("");
            Ok(())
        }) {
            active.remove(&id);
            return Err(error);
        }
        let weak = Arc::downgrade(inner);
        runtime()?.spawn(async move {
            run_job(weak, id, cancel).await;
        });
    }
    Ok(())
}
const ATTESTATION_REFRESH_LIMIT: u64 = 4;
fn refresh_pending_attestation(item: &mut Value, error: &anyhow::Error) -> bool {
    let refreshes = item["_attestation_refreshes"]
        .as_u64()
        .unwrap_or(u64::from(item["_sabr_refreshed"] == true));
    if !(error.is::<crate::sabr::PendingAttestation>()
        || error.is::<crate::sabr::Retryable>()
        || error.is::<videodock_media::AttestationRetryable>())
        || refreshes >= ATTESTATION_REFRESH_LIMIT
        || item["state"] == "paused"
        || item.get("_remove").is_some()
    {
        return false;
    }
    item.as_object_mut().unwrap().remove("_sabr_refreshed");
    item["_attestation_refreshes"] = json!(refreshes + 1);
    item["_reset_stage"] = json!(true);
    item["state"] = json!("queued");
    item["error"] = json!("");
    item["progress"] = json!(0);
    item["downloaded"] = json!(0);
    item["remaining"] = json!("");
    true
}
async fn run_job(weak: Weak<Inner>, id: String, cancel: CancellationToken) {
    let Some(inner) = weak.upgrade() else { return };
    let staging = inner
        .directory
        .join("staging")
        .join(crate::pipeline::safe_name(&id));
    let result=async{
        let saved=job(&lock(&inner.state),&id).cloned().context("Элемент удалён")?;
        if saved["_reset_stage"]==true{if staging.exists(){tokio::fs::remove_dir_all(&staging).await?;}change(&inner,|state|{job_mut(state,&id).context("Элемент удалён")?.as_object_mut().unwrap().remove("_reset_stage");Ok(())})?;}
        let spec:Spec=serde_json::from_value(saved["_spec"].clone()).context("Повреждены настройки элемента")?;
        let source=secrets::source(&inner,&saved)?;
        let (mut media,client)=inspect_in_context(&inner,&source,spec.auth_origin.as_deref(),&cancel).await?;
        media.url=source.clone();
        ensure!(!cancel.is_cancelled(), "Загрузка отменена");
        if matches!(media.kind,MediaKind::Playlist|MediaKind::Channel|MediaKind::Search){
            let settings=lock(&inner.state)["settings"].clone();
            let action=json!({"format":serde_json::to_value(&spec.selection)?,"output_dir":spec.output_dir,"filename":spec.filename,"_smart":spec.smart,"_auth_origin":spec.auth_origin});
            change(&inner,|state|{
                state["jobs"].as_array_mut().unwrap().retain(|item|item["id"]!=id);
                add_media(state,&media,&action,&settings,None)?;
                let replacement=state["jobs"].as_array().unwrap().iter().rev().find(|item|item["_group"]==true&&item["url"]==media.url).map(|item|item["id"].clone()).context("Не создана группа импорта")?;
                for item in state["jobs"].as_array_mut().unwrap(){if item["id"]==replacement{item["id"]=json!(id);item["added_order"]=saved["added_order"].clone();}
if item["_parent"]==replacement{item["_parent"]=json!(id);}}
                Ok(())
            })?;return Ok(None)
        }
        media.thumbnail=thumbnail(&inner,&client,&media.thumbnail,&cancel).await.unwrap_or_default();
        if !cancel.is_cancelled(){change(&inner,|state|{if let Some(item)=job_mut(state,&id){item["title"]=json!(media.title);item["thumbnail"]=json!(media.thumbnail);item["channel_title"]=json!(media.channel_title);item["channel_id"]=json!(media.channel_id);}Ok(())})?;}
        let settings=lock(&inner.state)["settings"].clone();
        if settings["displayAllCodecs"]!=true&&spec.selection.video_codec=="any"{
            let compatible=media.formats.iter().filter(|f|f.has_video()&&!modern_codec(f)).map(|f|f.height).collect::<HashSet<_>>();media.formats.retain(|f|!modern_codec(f)||!compatible.contains(&f.height));
        }
        let mut selection=spec.selection.clone();
        crate::pipeline::prepare(&client,&mut media,&cancel).await?;
        if spec.smart&&selection.media_type=="video"&&!media.formats.iter().any(Format::has_video)&&media.formats.iter().any(Format::has_audio){selection.media_type="audio".into();if ["mp4","mkv","webm"].contains(&selection.container.as_str()){selection.container="m4a".into();}change(&inner,|state|{if let Some(item)=job_mut(state,&id){item["kind"]=json!("audio");item["media_type"]=json!("audio");item["_spec"]["selection"]=json!(selection);item["format"]=json!(selection.container.to_ascii_uppercase());}Ok(())})?;}
        if spec.smart&&videodock_media::choose_formats(&media.formats,&selection).is_err(){
            selection.format_id=None;selection.video_codec="any".into();selection.audio_codec="any".into();selection.fps=0;
            if videodock_media::choose_formats(&media.formats,&selection).is_err(){selection.quality=media.formats.iter().filter_map(videodock_media::Format::video_quality).min().unwrap_or(0);}
            notice(&inner,"Для одного из источников выбрано ближайшее доступное качество к настройкам умного режима.");
        }
        let callback={let weak=Arc::downgrade(&inner);let id=id.clone();let cancel=cancel.clone();Arc::new(move|progress:crate::pipeline::PipelineProgress|{
            let Some(inner)=weak.upgrade()else{return};if cancel.is_cancelled()||inner.shutdown.is_cancelled(){return}
            let mut state=lock(&inner.state);if let Some(item)=job_mut(&mut state,&id){
                if item["state"]=="paused"||item["state"]=="queued"||item.get("_remove").is_some(){return}
                item["state"]=json!(if progress.phase=="complete"{"processing"}else{&progress.phase});item["progress"]=json!(progress.percent.clamp(0.0,99.0));item["speed"]=json!(format!("{}/s",bytes(progress.speed.max(0.0) as u64)));item["downloaded"]=json!(progress.downloaded);
                if let Some(total)=progress.total{item["total"]=json!(total);item["remaining"]=json!(if progress.speed>0.0{format!("{}s",((total.saturating_sub(progress.downloaded)) as f64/progress.speed) as u64)}else{String::new()});}
            }
        }) as Arc<dyn Fn(crate::pipeline::PipelineProgress)+Send+Sync>};
        let _inhibit=if settings["preventSleepEnabled"]==true{Some(crate::platform::SleepGuard::new()?)}else{None};
        let needs_preview=media.thumbnail.is_empty();
        let output=crate::pipeline::run(media,client,crate::pipeline::Options{selection,output_dir:spec.output_dir,filename:spec.filename,embed_subtitles:spec.embed_subtitles.unwrap_or(settings["embedSubtitles"]==true),tags:spec.tags.unwrap_or(settings["tagsEnabled"]==true),skip_duplicates:spec.skip_duplicates.unwrap_or(settings["skipDuplicates"]==true)},staging.clone(),cancel.clone(),inner.limiter.clone(),callback).await?;
        if needs_preview {
            let directory=inner.directory.join("thumbnails");
            let image=tokio::select!{_ = cancel.cancelled()=>return Ok(Some(output)),result=crate::library::thumbnail(&output.path,&directory)=>result};
            if let Ok(image)=image && !inner.shutdown.is_cancelled() { change(&inner,|state|{if let Some(item)=job_mut(state,&id){item["thumbnail"]=json!(image);}Ok(())})?; }
        }
        Ok::<_,anyhow::Error>(Some(output))
    }.await;
    if !inner.shutdown.is_cancelled() {
        let remove = job(&lock(&inner.state), &id)
            .and_then(|job| job.get("_remove"))
            .cloned();
        if let Some(remove) = remove {
            if remove == true
                && let Ok(Some(output)) = &result
            {
                for path in &output.files {
                    std::fs::remove_file(path).ok();
                }
            }
            if let Err(error) = remove_records(&inner, &HashSet::from([id.clone()]), remove == true)
            {
                notice(&inner, error);
            }
        } else {
            match result {
                Ok(Some(output)) => {
                    let _ = change(&inner, |state| {
                        let notifications = state["settings"]["notificationEnabled"] == true;
                        let remove_completed =
                            state["settings"]["removeCompletedDownloads"] == true;
                        if let Some(item) = job_mut(state, &id) {
                            let queued_again = item["state"] == "queued" && cancel.is_cancelled();
                            let mut files = item["files"].as_array().cloned().unwrap_or_default();
                            files.extend(output.files.iter().map(|path| json!(path)));
                            item["files"] = json!(files);
                            item["path"] = json!(output.path);
                            item["_size_bytes"] = json!(output.size);
                            item["size"] = json!(bytes(output.size));
                            item["speed"] = json!("");
                            actual_info(item, &output.info);
                            if !queued_again {
                                item["state"] = json!("complete");
                                item["progress"] = json!(100);
                                item["error"] = json!("");
                                if remove_completed && item.get("_parent").is_none() {
                                    item["_hide"] = json!(true);
                                }
                            }
                            if notifications && !queued_again {
                                notify(
                                    state,
                                    "Загрузка завершена",
                                    output
                                        .path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("Медиафайл"),
                                );
                            }
                        }
                        Ok(())
                    });
                    if let Err(error) = write_m3u(&inner, &id) {
                        notice(&inner, error);
                    }
                    tokio::fs::remove_dir_all(&staging).await.ok();
                }
                Ok(None) => {}
                Err(error) => {
                    let message = error.to_string();
                    let _ = change(&inner, |state| {
                        if let Some(item) = job_mut(state, &id) {
                            if cancel.is_cancelled() {
                                if item["state"] != "queued" {
                                    item["state"] = json!("paused");
                                }
                            } else if !refresh_pending_attestation(item, &error) {
                                item["state"] = json!("error");
                                item["error"] = json!(message);
                            }
                            item["speed"] = json!("");
                        }
                        Ok(())
                    });
                }
            }
        }
        let reset = job(&lock(&inner.state), &id).is_some_and(|job| job["_reset_stage"] == true);
        if reset && (tokio::fs::remove_dir_all(&staging).await.is_ok() || !staging.exists()) {
            let _ = change(&inner, |state| {
                if let Some(item) = job_mut(state, &id) {
                    item.as_object_mut().unwrap().remove("_reset_stage");
                }
                Ok(())
            });
        }
    }
    if !inner.shutdown.is_cancelled() {
        let remove = job(&lock(&inner.state), &id)
            .and_then(|item| item.get("_remove"))
            .map(|value| value == true);
        if let Some(delete) = remove
            && let Err(error) = remove_records(&inner, &HashSet::from([id.clone()]), delete)
        {
            notice(&inner, error);
        }
    }
    lock(&inner.active).remove(&id);
    inner.wake.notify_one();
}
fn requested_ids(state: &Value, action: &Value, all_allowed: bool) -> Result<HashSet<String>> {
    let values = action["ids"].as_array().context("Нужен список ids")?;
    ensure!(values.len() <= 100000, "Слишком много выбранных элементов");
    if values.is_empty() {
        ensure!(all_allowed, "Не выбраны элементы");
        return Ok(state["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|job| job["id"].as_str().unwrap().to_owned())
            .collect());
    }
    let mut ids = HashSet::new();
    for value in values {
        let id = settings::text(value)?;
        ensure!(job(state, id).is_some(), "Выбранный элемент не найден");
        ids.insert(id.to_owned());
    }
    let parents = ids.clone();
    for item in state["jobs"].as_array().unwrap() {
        if item["_parent"]
            .as_str()
            .is_some_and(|parent| parents.contains(parent))
        {
            ids.insert(item["id"].as_str().unwrap().into());
        }
    }
    Ok(ids)
}
pub(super) fn jobs(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    let command = action["command"]
        .as_str()
        .context("Не указано действие списка")?;
    let state = lock(&inner.state).clone();
    let ids = requested_ids(
        &state,
        action,
        matches!(command, "pause" | "resume" | "remove"),
    )?;
    if command == "redownload" || command == "force" {
        for id in &ids {
            let item = job(&state, id).context("Элемент не найден")?;
            ensure!(
                !item["_spec"].is_null() && item["url"].is_string(),
                "У файла библиотеки нет исходной ссылки для повторной загрузки"
            );
        }
    }
    match command {
        "pause" | "resume" | "force" | "redownload" => {
            ensure!(
                ["pause", "resume"].contains(&command) || !ids.is_empty(),
                "Не выбраны элементы"
            );
            if command == "pause" || command == "redownload" {
                let active = lock(&inner.active);
                for id in &ids {
                    if let Some(cancel) = active.get(id) {
                        cancel.cancel();
                    }
                }
            }
            change(inner, |state| {
                if command == "redownload" {
                    let parents = ids
                        .iter()
                        .filter_map(|id| job(state, id))
                        .filter_map(|item| {
                            if item["_group"] == true {
                                item["id"].as_str()
                            } else {
                                item["_parent"].as_str()
                            }
                        })
                        .map(str::to_owned)
                        .collect::<HashSet<_>>();
                    for parent in parents {
                        if let Some(item) = job_mut(state, &parent) {
                            item["_m3u_written"] = json!(false);
                            item.as_object_mut().unwrap().remove("_m3u_failed");
                            item.as_object_mut().unwrap().remove("_aux_error");
                        }
                    }
                }
                for id in &ids {
                    if let Some(item) = job_mut(state, id) {
                        if item["_group"] == true {
                            continue;
                        }
                        match command {
                            "pause" => {
                                if item["state"] != "complete" && item["state"] != "error" {
                                    item["state"] = json!("paused");
                                }
                            }
                            "resume" | "force" => {
                                if item["state"] == "paused"
                                    || item["state"] == "error"
                                    || item["state"] == "queued"
                                {
                                    item["state"] = json!("queued");
                                    item["error"] = json!("");
                                    item.as_object_mut().unwrap().remove("_sabr_refreshed");
                                    item.as_object_mut().unwrap().remove("_attestation_refreshes");
                                }
                            }
                            "redownload" => {
                                item["state"] = json!("queued");
                                item["progress"] = json!(0);
                                item["error"] = json!("");
                                item["_reset_stage"] = json!(true);
                                item.as_object_mut().unwrap().remove("_sabr_refreshed");
                                item.as_object_mut().unwrap().remove("_attestation_refreshes");
                                item["_spec"]["skip_duplicates"] = json!(false);
                            }
                            _ => {}
                        }
                    }
                }
                if command == "force" {
                    let all = state["jobs"].as_array_mut().unwrap();
                    let mut chosen = Vec::new();
                    let mut rest = Vec::new();
                    for item in all.drain(..) {
                        if ids.contains(item["id"].as_str().unwrap_or_default()) {
                            chosen.push(item);
                        } else {
                            rest.push(item);
                        }
                    }
                    chosen.extend(rest);
                    *all = chosen;
                }
                Ok(())
            })?;
        }
        "remove" => {
            let delete = action["delete_files"] == true;
            let active = lock(&inner.active).clone();
            for id in &ids {
                if let Some(cancel) = active.get(id) {
                    cancel.cancel();
                }
            }
            let inactive = ids
                .iter()
                .filter(|id| !active.contains_key(*id))
                .cloned()
                .collect::<HashSet<_>>();
            remove_records(inner, &inactive, delete)?;
            change(inner, |state| {
                for id in ids.iter().filter(|id| active.contains_key(*id)) {
                    if let Some(item) = job_mut(state, id) {
                        item["_remove"] = json!(delete);
                        item["state"] = json!("removing");
                    }
                }
                Ok(())
            })?;
        }
        "copy_url" => {
            let direct = action["ids"].as_array().unwrap();
            let urls = ids
                .iter()
                .filter(|id| direct.iter().any(|value| value == id.as_str()))
                .filter_map(|id| job(&state, id))
                .filter(|item| item["url"].is_string())
                .map(|item| secrets::source(inner, item))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                !urls.is_empty(),
                "У библиотечного файла нет исходной ссылки"
            );
            return Ok(json!({"clipboard":urls.join("\n")}));
        }
        "open" | "reveal" | "open_url" => {
            let direct = action["ids"].as_array().unwrap();
            for id in &ids {
                let item = job(&state, id).context("Элемент не найден")?;
                if !direct.iter().any(|value| value == id.as_str()) {
                    continue;
                }
                if command == "open_url" {
                    let source = secrets::source(inner, item)?;
                    for url in source.split_whitespace() {
                        crate::platform::open_url(url)?;
                    }
                } else {
                    let path = Path::new(
                        item["path"]
                            .as_str()
                            .context("У элемента ещё нет локального файла")?,
                    );
                    if command == "reveal" && item["_group"] != true {
                        crate::platform::reveal(path)?;
                    } else {
                        crate::platform::open_path(path)?;
                    }
                }
            }
        }
        _ => bail!("Неизвестное действие списка"),
    }
    Ok(json!({}))
}
fn remove_records(inner: &Inner, ids: &HashSet<String>, delete: bool) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let state = lock(&inner.state).clone();
    if delete {
        for id in ids {
            if let Some(item) = job(&state, id)
                && let Some(files) = item["files"].as_array()
            {
                for file in files {
                    let path = Path::new(settings::text(file)?);
                    if path.exists() {
                        std::fs::remove_file(path)
                            .context("Не удалось удалить файл с устройства")?;
                    }
                }
            }
        }
    }
    change(inner, |state| {
        state["jobs"]
            .as_array_mut()
            .unwrap()
            .retain(|item| !ids.contains(item["id"].as_str().unwrap_or_default()));
        Ok(())
    })?;
    super::secrets::forget(inner, ids)?;
    for id in ids {
        let path = inner
            .directory
            .join("staging")
            .join(crate::pipeline::safe_name(id));
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}
fn write_m3u(inner: &Inner, id: &str) -> Result<()> {
    // Serialize the bounded local playlist write with state changes, including concurrent job completion.
    let mut guard = lock(&inner.state);
    ensure!(!inner.shutdown.is_cancelled(), "Операция отменена");
    let mut state = guard.clone();
    let Some(item) = job(&state, id) else {
        return Ok(());
    };
    let Some(parent) = item["_parent"].as_str().map(str::to_owned) else {
        return Ok(());
    };
    let Some(group) = job(&state, &parent).cloned() else {
        return Ok(());
    };
    if group["_m3u"] != true || group["_m3u_written"] == true || group["_m3u_failed"] == true {
        return Ok(());
    }
    let mut children = state["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|child| child["_parent"] == parent)
        .cloned()
        .collect::<Vec<_>>();
    if !children
        .iter()
        .all(|child| matches!(child["state"].as_str(), Some("complete" | "error")))
    {
        return Ok(());
    }
    children.sort_by_key(|child| child["_index"].as_u64().unwrap_or(0));
    let folder = PathBuf::from(settings::text(&group["_spec"]["output_dir"])?);
    let name = crate::pipeline::safe_name(group["title"].as_str().unwrap_or("playlist"));
    let path = folder.join(format!("{name} [{parent}].m3u"));
    let mut content = String::from("#EXTM3U\n");
    for child in &children {
        if child["state"] != "complete" {
            continue;
        }
        if let Some(value) = child["path"].as_str() {
            let file = Path::new(value);
            let title = child["title"]
                .as_str()
                .unwrap_or("")
                .chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch })
                .collect::<String>();
            content.push_str(&format!(
                "#EXTINF:-1,{title}\n{}\n",
                file.strip_prefix(&folder).unwrap_or(file).to_string_lossy()
            ));
        }
    }
    let written = (|| -> Result<()> {
        if path.exists() && std::fs::read(&path)? == content.as_bytes() {
            return Ok(());
        }
        if path.exists() {
            ensure!(
                group["files"]
                    .as_array()
                    .is_some_and(|files| files.contains(&json!(path))),
                "Существующий плейлист не принадлежит этой загрузке; файл сохранён"
            );
            let temporary = folder.join(format!(".playlist-{}.tmp", uuid::Uuid::new_v4()));
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            if let Err(error) = output
                .write_all(content.as_bytes())
                .and_then(|_| output.sync_all())
            {
                drop(output);
                std::fs::remove_file(temporary).ok();
                return Err(error.into());
            }
            drop(output);
            let result = secrets::replace_file(&temporary, &path);
            if result.is_err() {
                std::fs::remove_file(temporary).ok();
            }
            result?;
        } else {
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)?;
            if let Err(error) = output
                .write_all(content.as_bytes())
                .and_then(|_| output.sync_all())
            {
                drop(output);
                std::fs::remove_file(&path).ok();
                return Err(error.into());
            }
        }
        Ok(())
    })();
    if let Some(item) = job_mut(&mut state, &parent) {
        match &written {
            Ok(()) => {
                item["_m3u_written"] = json!(true);
                let mut files = item["files"].as_array().cloned().unwrap_or_default();
                if !files.contains(&json!(path)) {
                    files.push(json!(path));
                }
                item["files"] = json!(files);
                item.as_object_mut().unwrap().remove("_aux_error");
            }
            Err(error) => {
                item["_m3u_failed"] = json!(true);
                item["_aux_error"] = json!(format!(
                    "Файлы скачаны, но плейлист создать не удалось: {error}"
                ));
                item["error"] = item["_aux_error"].clone();
            }
        }
    }
    let saved = lock(&inner.store).save(&persisted(&state));
    *guard = state;
    saved?;
    written
}

pub(super) fn subscriptions(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    let command = action["command"]
        .as_str()
        .context("Не указано действие подписки")?;
    let state = lock(&inner.state).clone();
    let values = action["ids"].as_array().context("Нужен список подписок")?;
    let ids = values
        .iter()
        .map(|value| settings::text(value).map(str::to_owned))
        .collect::<Result<HashSet<_>>>()?;
    ensure!(
        ids.iter().all(|id| state["subscriptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == *id)),
        "Подписка не найдена"
    );
    match command {
        "unsubscribe" => {
            ensure!(!ids.is_empty(), "Не выбраны подписки");
            change(inner, |state| {
                state["subscriptions"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|item| !ids.contains(item["id"].as_str().unwrap_or_default()));
                Ok(())
            })?;
            secrets::forget(inner, &ids)?;
        }
        "edit" => {
            ensure!(ids.len() == 1, "Выберите одну подписку");
            let id = ids.iter().next().unwrap();
            let item = state["subscriptions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == *id)
                .unwrap();
            return parse(
                inner,
                &json!({"url":secrets::source(inner,item)?,"force_manual":true,"mode":"subscribe","subscription_id":id}),
            );
        }
        "refresh" => {
            change(inner, |state| {
                for item in state["subscriptions"].as_array_mut().unwrap() {
                    if ids.is_empty() || ids.contains(item["id"].as_str().unwrap_or_default()) {
                        item["_force_check"] = json!(true);
                    }
                }
                Ok(())
            })?;
        }
        _ => bail!("Неизвестное действие подписки"),
    }
    Ok(json!({}))
}
fn check_subscriptions(inner: &Arc<Inner>) {
    let state = lock(&inner.state).clone();
    let period = state["settings"]["subscriptionUpdatePeriod"]
        .as_u64()
        .unwrap_or(86400);
    let due = state["subscriptions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["_force_check"] == true
                || (period > 0
                    && now().saturating_sub(item["last_checked"].as_u64().unwrap_or(0)) >= period)
        })
        .map(|item| item["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    for id in due {
        if !lock(&inner.checking_subscriptions).insert(id.clone()) {
            continue;
        }
        let weak = Arc::downgrade(inner);
        let Ok(runtime) = runtime() else { continue };
        runtime.spawn(async move {
            let Some(inner) = weak.upgrade() else { return };
            let result = scan_subscription(&inner, &id).await;
            if !inner.shutdown.is_cancelled()
                && let Err(error) = result
            {
                let message = error.to_string();
                let _ = change(&inner, |state| {
                    if let Some(item) = state["subscriptions"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|item| item["id"] == id)
                    {
                        item["error"] = json!(message);
                        item["last_checked"] = json!(now());
                        item.as_object_mut().unwrap().remove("_force_check");
                    }
                    Ok(())
                });
            }
            lock(&inner.checking_subscriptions).remove(&id);
        });
    }
}
async fn scan_subscription(inner: &Arc<Inner>, id: &str) -> Result<()> {
    let saved = lock(&inner.state)["subscriptions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .cloned()
        .context("Подписка удалена")?;
    let source = secrets::source(inner, &saved)?;
    let (mut media, _) = inspect(inner, &source, &inner.shutdown).await?;
    media.url = source;
    ensure!(
        matches!(media.kind, MediaKind::Channel | MediaKind::Playlist),
        "Источник подписки больше не является каналом или плейлистом"
    );
    let settings = lock(&inner.state)["settings"].clone();
    let selected: Spec = serde_json::from_value(saved["_spec"].clone())?;
    let seen = saved["_seen_ids"]
        .as_array()
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let new = media
        .entries
        .iter()
        .filter(|entry| !seen.contains(&entry.id))
        .cloned()
        .collect::<Vec<_>>();
    let baseline = saved["_baseline"] == true;
    let mut all_seen = seen;
    all_seen.extend(media.entries.iter().map(|entry| entry.id.clone()));
    ensure!(
        all_seen.len() <= 500000,
        "История подписки превышает 500000 элементов"
    );
    change(inner, |state| {
        if !state["subscriptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == id)
        {
            return Ok(());
        }
        if baseline && !new.is_empty() {
            let mut fresh = media.clone();
            fresh.entries = new.clone();
            let action = json!({"format":selected.selection,"output_dir":selected.output_dir,"_smart":true,"_auth_origin":selected.auth_origin});
            add_media(state, &fresh, &action, &settings, Some(id))?;
            if settings["remindNotificationEnabled"] == true {
                notify(
                    state,
                    "Новые видео по подписке",
                    &format!("{}: {}", media.title, new.len()),
                );
            }
        }
        let item = state["subscriptions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == id)
            .unwrap();
        item["title"] = json!(media.title);
        item["last_checked"] = json!(now());
        item["new_count"] = json!(if baseline { new.len() } else { 0 });
        item["_seen_ids"] = json!(all_seen);
        item["_baseline"] = json!(true);
        item["error"] = json!("");
        item.as_object_mut().unwrap().remove("_force_check");
        Ok(())
    })
}
fn check_library(inner: &Arc<Inner>) {
    if now().saturating_sub(inner.library_last.load(Ordering::SeqCst)) < 60
        || inner.library_scanning.swap(true, Ordering::SeqCst)
    {
        return;
    }
    inner.library_last.store(now(), Ordering::SeqCst);
    let weak = Arc::downgrade(inner);
    let Ok(runtime) = runtime() else {
        inner.library_scanning.store(false, Ordering::SeqCst);
        return;
    };
    runtime.spawn(async move{
        let Some(inner)=weak.upgrade()else{return};let folder=lock(&inner.state)["settings"]["path"].as_str().map(PathBuf::from);
        let result=async{
            let folder=folder.context("Не задана папка библиотеки")?;if !folder.exists(){return Ok(())}
            let files=tokio::select!{_ = inner.shutdown.cancelled()=>return Ok(()),result=crate::library::scan(&folder)=>result?};
            let mut previews = HashMap::new();
            let thumbnail_directory = inner.directory.join("thumbnails");
            for file in &files {
                let image = tokio::select!{_ = inner.shutdown.cancelled()=>return Ok(()),result=crate::library::thumbnail(&file.path,&thumbnail_directory)=>result};
                if let Ok(image) = image { previews.insert(file.path.clone(),image); }
            }
            if inner.shutdown.is_cancelled(){return Ok(())}
            change(&inner,|state|{
                if state["settings"]["path"].as_str().map(PathBuf::from).as_ref()!=Some(&folder){return Ok(())}
                use sha2::{Digest,Sha256};
                for item in state["jobs"].as_array_mut().unwrap() {
                    if item["thumbnail"].as_str().is_none_or(str::is_empty)
                        && let Some(path) = item["path"].as_str().and_then(|path|std::fs::canonicalize(path).ok())
                        && let Some(image) = previews.get(&path) { item["thumbnail"] = json!(image); }
                }
                let mut tracked=HashSet::new();
                for item in state["jobs"].as_array().unwrap(){if item["_library"]==true{continue}
if let Some(path)=item["path"].as_str()&& let Ok(path)=std::fs::canonicalize(path){tracked.insert(path);}
if let Some(paths)=item["files"].as_array(){for path in paths.iter().filter_map(Value::as_str){if let Ok(path)=std::fs::canonicalize(path){tracked.insert(path);}}}}
                let mut library=Vec::new();
                for file in files{if tracked.contains(&file.path){continue}let id=format!("library-{:x}",Sha256::digest(file.path.to_string_lossy().as_bytes()));let kind=serde_json::to_value(&file.kind)?;
                    library.push(json!({"id":id,"title":file.title,"kind":kind,"media_type":kind,"state":"complete","progress":100,"thumbnail":previews.get(&file.path),"path":file.path,"files":[file.path],"size":bytes(file.size),"_size_bytes":file.size,"duration":file.duration,"format":file.path.extension().and_then(|ext|ext.to_str()).unwrap_or_default().to_ascii_uppercase(),"_library":true,"created_at":0}));
                }
                let jobs=state["jobs"].as_array_mut().unwrap();jobs.retain(|item|item["_library"]!=true);ensure!(jobs.len()+library.len()<=250000,"Библиотека превышает 250000 элементов");jobs.extend(library);
                Ok(())
            })
        }.await;
        if let Err(error)=result{notice(&inner,error);}inner.library_scanning.store(false,Ordering::SeqCst);
    });
}

#[cfg(test)]
mod user_error_tests {
    use super::*;
    #[test]
    fn attestation_refresh_is_bounded_and_preserves_job_identity() {
        for error in [
            anyhow::Error::new(crate::sabr::PendingAttestation).context("transfer"),
            anyhow::Error::new(crate::sabr::Retryable("Соединение SABR прервано"))
                .context("transfer"),
            videodock_media::AttestationRetryable::Failed.into(),
            videodock_media::AttestationRetryable::Timeout.into(),
        ] {
            let mut item = json!({"id":"same-job","state":"downloading","progress":80,"downloaded":42});
            for expected in 1..=ATTESTATION_REFRESH_LIMIT {
                assert!(refresh_pending_attestation(&mut item, &error));
                assert_eq!(item["id"], "same-job");
                assert_eq!(item["state"], "queued");
                assert_eq!(item["progress"], 0);
                assert_eq!(item["downloaded"], 0);
                assert_eq!(item["_reset_stage"], true);
                assert_eq!(item["_attestation_refreshes"], expected);
            }
            let after = item.clone();
            assert!(!refresh_pending_attestation(&mut item, &error));
            assert_eq!(item, after);
        }
    }
    #[test]
    fn attestation_refresh_never_retries_rejection_paused_or_removed_jobs() {
        for message in ["Google отклонил PO; повторите проверку", "HTTP 403", "disk full"] {
            let mut item = json!({"state":"downloading"});
            assert!(!refresh_pending_attestation(&mut item, &anyhow::anyhow!(message)));
            assert_eq!(item, json!({"state":"downloading"}));
        }
        for mut item in [json!({"state":"paused"}), json!({"state":"downloading","_remove":false})] {
            let before = item.clone();
            assert!(!refresh_pending_attestation(&mut item, &crate::sabr::PendingAttestation.into()));
            assert_eq!(item, before);
        }
    }
    #[test]
    fn batch_preview_preserves_child_subtitles_and_audio_languages() {
        let clip = |language: &str| Media {
            formats: vec![Format {
                id: format!("140:{language}"),
                ext: "m4a".into(),
                audio_codec: Some("mp4a.40.2".into()),
                bitrate: Some(128_000),
                audio_language: Some(language.into()),
                audio_track_id: Some(format!("{language}.0")),
                ..Default::default()
            }],
            subtitles: vec![videodock_media::Subtitle {
                language: language.into(),
                title: language.into(),
                url: format!("https://example.invalid/{language}.vtt"),
                ext: "vtt".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let english = clip("en");
        let russian = clip("ru");
        let mut repeated = english.clone();
        repeated.subtitles[0].translatable = true;
        let mut automatic = repeated.subtitles[0].clone();
        automatic.automatic = true;
        repeated.subtitles.push(automatic);
        let mut alternate = english.formats[0].clone();
        alternate.id = "140:en-alternate".into();
        alternate.audio_track_id = Some("en.1".into());
        repeated.formats.push(alternate);
        let mut playlist = Media {
            kind: MediaKind::Playlist,
            ..Default::default()
        };
        let mut keys = HashSet::new();
        for child in [&english, &russian, &repeated] {
            merge_batch_metadata(&mut playlist, &mut keys, child.clone());
        }
        let view = manual_view(&playlist, &defaults(), &HashMap::new());
        assert_eq!(playlist.formats.len(), 3);
        assert_eq!(
            view["audio_tracks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|track| track["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["en", "ru"]
        );
        assert_eq!(
            view["subtitles"]
                .as_array()
                .unwrap()
                .iter()
                .map(|track| track["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["en", "ru", "en:auto"]
        );
        assert_eq!(view["can_translate_subtitles"], true);
        let all_audio = videodock_media::choose_formats(
            &playlist.formats,
            &Selection {
                media_type: "audio".into(),
                audio_tracks: vec!["all".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(all_audio.len(), 3);
        for (child, language) in [(&english, "en"), (&russian, "ru")] {
            let selected = videodock_media::choose_formats(
                &child.formats,
                &Selection {
                    media_type: "audio".into(),
                    audio_tracks: vec![language.into()],
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(selected[0].audio_language.as_deref(), Some(language));
            let (subtitle, _) = crate::subtitles::select(&child.subtitles, language).unwrap();
            assert_eq!(subtitle.language, language);
        }
        let (automatic, _) = crate::subtitles::select(&playlist.subtitles, "en:auto").unwrap();
        assert!(automatic.automatic);
    }
    #[test]
    fn portrait_labels_preserve_dimensions_and_use_nominal_quality() {
        let media = Media {
            formats: vec![videodock_media::Format {
                id: "portrait".into(),
                width: Some(720),
                height: Some(1280),
                video_codec: Some("avc1".into()),
                audio_codec: Some("aac".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let settings = json!({"mediaType":"video","container":"mp4","path":std::env::temp_dir(),"displayAllCodecs":true});
        let manual = manual_view(&media, &settings, &HashMap::new());
        assert_eq!(manual["formats"][0]["quality"], "720p");
        let mut item = json!({});
        actual_info(
            &mut item,
            &json!({"streams":[{"codec_type":"video","width":720,"height":1280}]}),
        );
        assert_eq!(item["quality"], "720p");
        assert_eq!(item["width"], 720);
        assert_eq!(item["height"], 1280);
    }
    #[test]
    fn transient_errors_are_readable_and_normal_errors_are_preserved() {
        let state = json!({"jobs":[],"manual":{"error":"YouTube требует актуальный PO token"},"notice":"Нет конфигурации SABR","other":{"error":"Недостаточно места на диске"}});
        let public = public_state(&state);
        for value in [&public["manual"]["error"], &public["notice"]] {
            let text = value.as_str().unwrap();
            assert!(
                !text.contains("SABR") && !text.contains("PO token") && text.contains("Повторите")
            );
        }
        assert_eq!(public["other"]["error"], state["other"]["error"]);
        assert!(
            state["manual"]["error"]
                .as_str()
                .unwrap()
                .contains("PO token")
        );
    }
}
