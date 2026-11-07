use super::*;
use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

fn preferences(spec: &Value) -> Result<Value> {
    let selected: Selection = serde_json::from_value(spec["selection"].clone())?;
    let audio = json!({"codec":if selected.audio_codec=="any"{"Any"}else{&selected.audio_codec},"bitrate":if selected.audio_bitrate==0{-1}else{i64::from(selected.audio_bitrate)*1000}});
    let mut format = json!({"container":selected.container.to_ascii_uppercase(),"audio":audio});
    if selected.media_type == "video" {
        let dimension = match selected.vr_mode {
            videodock_media::VideoVrMode::Any if selected.vr => json!("VR"),
            videodock_media::VideoVrMode::Flat => json!("2D"),
            mode => json!(mode),
        };
        format["video"] = json!({"codec":if selected.video_codec=="any"{"Any"}else{&selected.video_codec},"resolution":if selected.quality==0{"Original".into()}else{format!("{}p",selected.quality)},"fps":if selected.fps==0{"maximum fps".into()}else{format!("{}fps",selected.fps)},"dimension":dimension});
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
    Ok(
        json!({"format":format,"outputDir":spec["output_dir"],"subtitles":subtitles,"audioTracks":selected.audio_tracks}),
    )
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
            let normalized = settings::normalize("videoVR", value)?;
            selected.vr = normalized == true;
            selected.vr_mode = if normalized == false {
                videodock_media::VideoVrMode::Flat
            } else if normalized.is_boolean() {
                videodock_media::VideoVrMode::Any
            } else {
                serde_json::from_value(normalized)?
            };
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
    if let Some(tracks) = prefs.get("audioTracks") {
        selected.audio_tracks =
            serde_json::from_value(settings::normalize("audioTracks", tracks)?)?;
    }
    Ok(selected)
}
// Legacy DownloadsSerializer Version 1 profile, reconstructed from the official
// Linux 26.3.5 ELF (SHA256 fbadf3d2b97fcd3fa8b8a422181780505544697b22861ad9e0199ce716c47c7c).
// Header constructor 0x50ba40; csvDeserialize 0x71a810; delimiter detector
// 0x18f4c00; row decoder 0x18f5230. These are data-format facts, not copied code.
// Other old subscription layouts are deliberately not inferred from extensions.
fn legacy_text(bytes: &[u8]) -> Result<Cow<'_, str>> {
    // FileSystem::readLines uses QTextStream with UTF-8 and its default Unicode
    // BOM detection. Check UTF-32 before UTF-16 because the LE BOMs overlap.
    for (bom, little_endian) in [(&b"\xff\xfe\0\0"[..], true), (&b"\0\0\xfe\xff"[..], false)] {
        if let Some(bytes) = bytes.strip_prefix(bom) {
            ensure!(bytes.len() % 4 == 0, "Неполный символ UTF-32");
            let text = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&bytes| {
                    let value = if little_endian {
                        u32::from_le_bytes(bytes)
                    } else {
                        u32::from_be_bytes(bytes)
                    };
                    char::from_u32(value).context("Некорректный символ UTF-32")
                })
                .collect::<Result<String>>()?;
            return Ok(Cow::Owned(text));
        }
    }
    for (bom, little_endian) in [(&b"\xff\xfe"[..], true), (&b"\xfe\xff"[..], false)] {
        if let Some(bytes) = bytes.strip_prefix(bom) {
            ensure!(bytes.len() % 2 == 0, "Неполный символ UTF-16");
            let units = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&bytes| {
                    if little_endian {
                        u16::from_le_bytes(bytes)
                    } else {
                        u16::from_be_bytes(bytes)
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Cow::Owned(
                String::from_utf16(&units).context("Некорректный UTF-16")?,
            ));
        }
    }
    Ok(Cow::Borrowed(
        std::str::from_utf8(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
            .context("Некорректный UTF-8")?,
    ))
}

fn legacy_document(bytes: &[u8]) -> Result<Value> {
    const VIDEO: &str = "URL,Title,Type,Format,AudioBitrate,VideoResolution,FrameRate,Subtitles,OutputDir,Version 1";
    const AUDIO: &str = "URL,Title,Type,Format,AudioBitrate,OutputDir,Version 1";
    let text = legacy_text(bytes)?;
    let mut lines = text
        .split_terminator('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let header = lines.next().context("Пустой файл импорта")?;
    // Original detectDelimeter tests comma first, then semicolon, in the header.
    let delimiter = if header.contains(',') { ',' } else { ';' };
    let columns = header.split(delimiter).collect::<Vec<_>>();
    let video_profile = columns == VIDEO.split(',').collect::<Vec<_>>();
    ensure!(
        video_profile || columns == AUDIO.split(',').collect::<Vec<_>>(),
        "Неизвестный legacy-формат: нужен заголовок Downloads Version 1 (CSV)"
    );
    let mut items = Vec::new();
    for (index, line) in lines.enumerate() {
        ensure!(items.len() < 10000, "Слишком много элементов импорта");
        let fields = legacy_csv_row(line, delimiter)
            .with_context(|| format!("Некорректная CSV-строка {}", index + 2))?;
        ensure!(
            fields.len() == columns.len(),
            "Неверное число колонок в CSV-строке {}",
            index + 2
        );
        let is_video = match fields[2].as_str() {
            "Video" => true,
            "Audio" => false,
            _ => anyhow::bail!("Неизвестный Type в CSV-строке {}", index + 2),
        };
        let container = fields[3].to_ascii_uppercase();
        ensure!(
            ["MP4", "MKV", "FLV", "3GP", "M4A", "MP3", "OGG", "Any"]
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&container)),
            "Неизвестный Format в CSV-строке {}",
            index + 2
        );
        // QString::toInt(base 10) returns the decoder's Any (-1) fallback on
        // malformed text or int32 overflow; video rows always use Any bitrate.
        let bitrate = if is_video {
            -1
        } else {
            fields[4].trim().parse::<i32>().unwrap_or(-1)
        };
        let mut format = json!({"container":container,"audio":{"codec":"Any","bitrate":bitrate}});
        if is_video {
            let resolution = if video_profile {
                fields[5].as_str()
            } else {
                ""
            };
            let resolution = match resolution.to_ascii_lowercase().as_str() {
                "" | "any" | "original" => "Original",
                "qcif" => "144p",
                "240p" => "240p",
                "360p" => "360p",
                "480p" => "480p",
                "720p" => "720p",
                "1080p" => "1080p",
                "2k" => "1440p",
                "4k" => "4K",
                "8k" => "8K",
                _ => anyhow::bail!(
                    "Неподдерживаемый VideoResolution в CSV-строке {}",
                    index + 2
                ),
            };
            let fps = if video_profile {
                fields[6].to_ascii_lowercase()
            } else {
                String::new()
            };
            let fps = match fps.as_str() {
                "" | "any" | "max" | "maximum fps" => "maximum fps",
                "25" | "25fps" => "25fps",
                "30" | "30fps" => "30fps",
                "50" | "50fps" => "50fps",
                "60" | "60fps" => "60fps",
                "120" | "120fps" => "120fps",
                "240" | "240fps" => "240fps",
                _ => anyhow::bail!("Неизвестный FrameRate в CSV-строке {}", index + 2),
            };
            // csvDeserialize constructs VideoInfo with dimension enum 3,
            // confirmed as 2D by dimension::string (0x146c950).
            format["video"] =
                json!({"codec":"Any","resolution":resolution,"fps":fps,"dimension":"2D"});
        }
        let subtitles = if video_profile {
            // csvDeserialize 0x71b4b2: split on '-' before SubtitlesOption set.
            fields[7]
                .split('-')
                .filter(|lang| !lang.is_empty())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .map(|lang| json!({"lang":lang}))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let output = &fields[if video_profile { 8 } else { 5 }];
        let url = videodock_media::validate_url(&fields[0])?;
        items.push(json!({"urlDesc":{"serviceName":service(&url),"type":"","url":url.as_str()},"isSubscription":false,"playlistTitle":fields[1],"downloadPreferences":{"format":format,"subtitles":subtitles,"outputDir":output}}));
    }
    Ok(json!({"json_schema_version":"1.0.0","items":items}))
}

fn legacy_csv_row(line: &str, delimiter: char) -> Result<Vec<String>> {
    let mut chars = line.chars().peekable();
    let mut fields = Vec::new();
    loop {
        ensure!(fields.len() < 10, "Слишком много колонок CSV");
        let mut field = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            loop {
                match chars.next() {
                    Some('"') if chars.peek() == Some(&'"') => {
                        chars.next();
                        field.push('"');
                    }
                    Some('"') => break,
                    Some(ch) => field.push(ch),
                    None => anyhow::bail!("Незакрытая кавычка CSV"),
                }
            }
            ensure!(
                chars.peek().is_none() || chars.peek() == Some(&delimiter),
                "Лишний текст после кавычки CSV"
            );
        } else {
            while let Some(&ch) = chars.peek() {
                if ch == delimiter {
                    break;
                }
                ensure!(ch != '"', "Кавычка внутри неэкранированной CSV-колонки");
                field.push(ch);
                chars.next();
            }
        }
        fields.push(field);
        if chars.next().is_none() {
            break;
        }
    }
    Ok(fields)
}

pub(super) fn import(inner: &Inner, action: &Value) -> Result<()> {
    let path = PathBuf::from(settings::text(&action["path"])?);
    ensure!(path.is_absolute(), "Нужен абсолютный путь импорта");
    const LIMIT: u64 = 16 * 1024 * 1024;
    let file = std::fs::File::open(&path)?;
    ensure!(
        file.metadata()?.len() <= LIMIT,
        "Файл импорта превышает 16 MiB"
    );
    // Enforce the allocation bound even if the file grows after metadata().
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "Файл импорта превышает 16 MiB");
    let document: Value = if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        serde_json::from_slice(&bytes)?
    } else {
        legacy_document(&bytes)?
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_track_preferences_survive_export_import_without_using_global_tracks() {
        let selected = Selection {
            audio_tracks: vec!["ru".into(), "en".into()],
            ..Default::default()
        };
        let prefs = preferences(&json!({"selection":selected,"output_dir":"/fixture"})).unwrap();
        let mut settings = settings::defaults();
        settings["audioTracks"] = json!(["de"]);
        let item = json!({"downloadPreferences":prefs});
        assert_eq!(
            imported_selection(&item, &settings, true)
                .unwrap()
                .audio_tracks,
            ["ru", "en"]
        );
        assert_eq!(
            imported_selection(&item, &settings, false)
                .unwrap()
                .audio_tracks,
            ["de"]
        );
    }

    #[test]
    fn imported_audio_tracks_are_optional_but_validated_when_present() {
        let mut settings = settings::defaults();
        settings["audioTracks"] = json!(["de"]);
        let mut item = json!({"downloadPreferences":{"format":{"container":"MP4","video":{}}}});
        assert_eq!(
            imported_selection(&item, &settings, true)
                .unwrap()
                .audio_tracks,
            ["de"]
        );
        item["downloadPreferences"]["audioTracks"] = json!([]);
        assert!(
            imported_selection(&item, &settings, true)
                .unwrap()
                .audio_tracks
                .is_empty()
        );
        for invalid in [json!("ru"), json!(["bad\ntrack"]), json!(vec!["ru"; 257])] {
            item["downloadPreferences"]["audioTracks"] = invalid;
            assert!(imported_selection(&item, &settings, true).is_err());
        }
    }
}
