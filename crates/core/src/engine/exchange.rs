use super::*;
use std::{fs::OpenOptions, io::Write, path::Path};

fn preferences(spec: &Value) -> Result<Value> {
    let selected: Selection = serde_json::from_value(spec["selection"].clone())?;
    let audio = json!({"codec":if selected.audio_codec=="any"{"Any"}else{&selected.audio_codec},"bitrate":if selected.audio_bitrate==0{-1}else{i64::from(selected.audio_bitrate)*1000}});
    let mut format = json!({"container":selected.container.to_ascii_uppercase(),"audio":audio});
    if selected.media_type == "video" {
        format["video"] = json!({"codec":if selected.video_codec=="any"{"Any"}else{&selected.video_codec},"resolution":if selected.quality==0{"Original".into()}else{format!("{}p",selected.quality)},"fps":if selected.fps==0{"maximum fps".into()}else{format!("{}fps",selected.fps)},"dimension":if selected.vr{"360"}else{"2D"}});
    }
    let subtitles = selected
        .subtitles
        .iter()
        .map(|id| {
            if let Some(lang) = id.strip_suffix(":auto") {
                json!({"lang":lang,"type":"transcribed"})
            } else if let Some((target, source)) = id.split_once(":translated:") {
                json!({"lang":target,"type":"translated","videodock_id":id,"source_lang":source})
            } else {
                json!({"lang":id})
            }
        })
        .collect::<Vec<_>>();
    Ok(json!({"format":format,"outputDir":spec["output_dir"],"subtitles":subtitles}))
}
fn service(url: &url::Url) -> String {
    let host = url
        .host_str()
        .unwrap_or_default()
        .trim_start_matches("www.");
    for (domain, name) in [
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("soundcloud.com", "soundcloud"),
        ("vimeo.com", "vimeo"),
        ("tiktok.com", "tiktok"),
        ("facebook.com", "facebook"),
        ("dailymotion.com", "dailymotion"),
        ("twitch.tv", "twitch"),
        ("bilibili.com", "bilibili"),
        ("rumble.com", "rumble"),
        ("naver.com", "naver"),
        ("likee.video", "likee"),
        ("nicovideo.jp", "niconico"),
        ("bitchute.com", "bitchute"),
    ] {
        if host == domain || host.ends_with(&format!(".{domain}")) {
            return name.into();
        }
    }
    "html".into()
}
pub(super) fn export(inner: &Inner, action: &Value) -> Result<Value> {
    let path = PathBuf::from(settings::text(&action["path"])?);
    ensure!(path.is_absolute(), "Нужен абсолютный путь экспорта");
    let state = lock(&inner.state).clone();
    let mut items = Vec::new();
    let mut skipped = 0;
    fn row(item: &Value, subscription: bool, all: &[Value], skipped: &mut usize) -> Result<Value> {
        let url = videodock_media::validate_url(settings::text(&item["url"])?)?;
        let kind = item["kind"].as_str().unwrap_or("");
        let mut result = json!({"urlDesc":{"serviceName":service(&url),"handlerName":kind,"type":if kind=="playlist"||kind=="channel"{kind}else{""},"url":url.as_str()},"isSubscription":subscription,"playlistTitle":item["title"],"downloadPreferences":preferences(&item["_spec"])?});
        if item["_group"] == true {
            let mut children = Vec::new();
            for child in all.iter().filter(|child| child["_parent"] == item["id"]) {
                if child["_source_private"] == true {
                    *skipped += 1;
                } else {
                    children.push(row(child, false, &[], skipped)?);
                }
            }
            result["items"] = json!(children);
        }
        Ok(result)
    }
    for (key, subscription) in [("jobs", false), ("subscriptions", true)] {
        if !subscription && action["subscriptions_only"] == true {
            continue;
        }
        let list = state[key].as_array().unwrap();
        for item in list {
            if item.get("_parent").is_some() || item["_spec"].is_null() {
                continue;
            }
            if item["_group"] == true
                && item["url"]
                    .as_str()
                    .is_some_and(|url| videodock_media::validate_url(url).is_err())
            {
                for child in list.iter().filter(|child| child["_parent"] == item["id"]) {
                    if child["_source_private"] == true {
                        skipped += 1;
                    } else {
                        items.push(row(child, false, &[], &mut skipped)?);
                    }
                }
                continue;
            }
            if item["_source_private"] == true {
                skipped += 1;
                continue;
            }
            items.push(row(item, subscription, list, &mut skipped)?);
        }
    }
    let bytes = serde_json::to_vec_pretty(&json!({"json_schema_version":"1.0.0","items":items}))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .context("Файл экспорта уже существует или недоступен; выберите новое имя")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    if skipped > 0 {
        notice(
            inner,
            format!(
                "Экспортировано: {}. Защищённые исходные ссылки пропущены: {skipped}.",
                items.len()
            ),
        );
    }
    Ok(json!({"exported":items.len(),"skipped_private":skipped}))
}
fn imported_selection(item: &Value, settings: &Value, use_file: bool) -> Result<Selection> {
    let mut selected = selection(settings, &json!({}))?;
    if !use_file {
        return Ok(selected);
    }
    let prefs = &item["downloadPreferences"];
    if prefs.is_null() {
        return Ok(selected);
    }
    let format = prefs
        .get("format")
        .context("Не указан формат импортируемого элемента")?;
    ensure!(format.is_object(), "Некорректный формат импорта");
    if let Some(value) = format.get("container") {
        selected.container = settings::normalize("container", value)?
            .as_str()
            .unwrap()
            .into();
    }
    if let Some(video) = format.get("video") {
        selected.media_type = "video".into();
        if let Some(value) = video.get("resolution") {
            selected.quality = settings::quality(value)?;
        }
        if let Some(value) = video.get("codec") {
            selected.video_codec = settings::normalize("videoCodec", value)?
                .as_str()
                .unwrap()
                .into();
        }
        if let Some(value) = video.get("fps").and_then(Value::as_str) {
            selected.fps = if value == "maximum fps" {
                0
            } else {
                value
                    .trim_end_matches("fps")
                    .parse()
                    .context("Некорректная частота кадров")?
            };
            ensure!(selected.fps <= 240, "Слишком большая частота кадров");
        }
        if let Some(value) = video.get("dimension") {
            ensure!(
                ["Any", "2D", "360", "VR"].contains(&settings::text(value)?),
                "Неизвестный вид проекции видео"
            );
            selected.vr = matches!(settings::text(value)?, "360" | "VR");
        }
    } else {
        selected.media_type = "audio".into();
    }
    if let Some(audio) = format.get("audio") {
        if let Some(value) = audio.get("codec") {
            selected.audio_codec = settings::normalize("audioCodec", value)?
                .as_str()
                .unwrap()
                .into();
        }
        if let Some(value) = audio.get("bitrate") {
            let bitrate = value.as_i64().context("Некорректный битрейт")?;
            if bitrate > 0 {
                ensure!(
                    (32000..=1536000).contains(&bitrate),
                    "Битрейт вне диапазона"
                );
                selected.audio_bitrate = (bitrate / 1000) as u32;
            } else {
                ensure!(bitrate == 0 || bitrate == -1, "Неизвестный битрейт");
                selected.audio_bitrate = 0;
            }
        }
    }
    if let Some(values) = prefs["subtitles"].as_array() {
        ensure!(values.len() <= 64, "Слишком много языков");
        selected.subtitles = values
            .iter()
            .map(|value| {
                if let Some(id) = value["videodock_id"].as_str() {
                    return settings::bounded(&json!(id), 128);
                }
                let lang = settings::bounded(&value["lang"], 128)?;
                Ok(if value["type"] == "transcribed" {
                    format!("{lang}:auto")
                } else {
                    lang
                })
            })
            .collect::<Result<_>>()?;
    }
    Ok(selected)
}
pub(super) fn import(inner: &Inner, action: &Value) -> Result<()> {
    let path = PathBuf::from(settings::text(&action["path"])?);
    ensure!(path.is_absolute(), "Нужен абсолютный путь импорта");
    ensure!(
        std::fs::metadata(&path)?.len() <= 16 * 1024 * 1024,
        "Файл импорта превышает 16 MiB"
    );
    let document: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(
        document["json_schema_version"] == "1.0.0",
        "Поддерживается JSON schema 1.0.0"
    );
    let items = document["items"].as_array().context("Нет списка items")?;
    let settings = lock(&inner.state)["settings"].clone();
    let mut rows = Vec::new();
    fn visit(
        items: &[Value],
        settings: &Value,
        action: &Value,
        rows: &mut Vec<(bool, Value)>,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth < 16, "Слишком много вложенных списков");
        ensure!(items.len() <= 10000, "Слишком много элементов импорта");
        for item in items {
            ensure!(rows.len() < 10000, "Слишком много элементов импорта");
            let desc = item.get("urlDesc").context("Нет urlDesc")?;
            ensure!(
                desc["serviceName"].is_string() && desc["type"].is_string(),
                "Неполное описание urlDesc"
            );
            let url = videodock_media::validate_url(settings::text(&desc["url"])?)?.to_string();
            let selected =
                imported_selection(item, settings, action["preferences_from_file"] != false)?;
            let output = action["output_dir"]
                .as_str()
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    if action["preferences_from_file"] != false {
                        item["downloadPreferences"]["outputDir"].as_str()
                    } else {
                        None
                    }
                })
                .or_else(|| settings["path"].as_str())
                .context("Не указана папка загрузки")?;
            ensure!(
                Path::new(output).is_absolute(),
                "Нужна абсолютная папка загрузки"
            );
            let kind = match desc["type"].as_str().unwrap_or("") {
                "playlist" => "playlist",
                "channel" => "channel",
                _ => {
                    if selected.media_type == "audio" {
                        "audio"
                    } else {
                        "video"
                    }
                }
            };
            let subscription = item["isSubscription"] == true;
            ensure!(
                !subscription || kind == "playlist" || kind == "channel",
                "Подписка должна ссылаться на канал или плейлист"
            );
            let children = item["items"].as_array();
            if !subscription
                && let Some(children) = children
                && action["download_complete_playlist"] == false
            {
                visit(children, settings, action, rows, depth + 1)?;
                continue;
            }
            let row = json!({"id":uuid::Uuid::new_v4().to_string(),"url":url,"title":item["playlistTitle"].as_str().unwrap_or("Импортированный элемент"),"kind":kind,"media_type":selected.media_type,"state":if subscription{"subscribed"}else{"queued"},"progress":0,"created_at":now(),"last_checked":now(),"_seen_ids":[],"_baseline":false,"_spec":{"selection":selected,"output_dir":output,"filename":null},"_expand":kind=="playlist"||kind=="channel"});
            rows.push((subscription, row));
        }
        Ok(())
    }
    visit(items, &settings, action, &mut rows, 0)?;
    change(inner, |state| {
        for (subscription, row) in rows {
            let key = if subscription {
                "subscriptions"
            } else {
                "jobs"
            };
            if !state[key]
                .as_array()
                .unwrap()
                .iter()
                .any(|existing| existing["url"] == row["url"])
            {
                state[key].as_array_mut().unwrap().push(row);
            }
        }
        Ok(())
    })
}
