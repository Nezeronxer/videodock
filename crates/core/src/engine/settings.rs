use super::*;
use anyhow::bail;
use std::path::Path;

pub(super) fn defaults() -> Value {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_default();
    json!({"language":"ru","smartMode":true,"mediaType":"video","quality":1080,"container":"mp4","videoCodec":"any","audioCodec":"any","audioBitrate":192,"frameRate":0,"videoVR":false,"subtitles":[],"audioTracks":[],"path":home.join(if cfg!(target_os="macos"){"Movies"}else{"Videos"}).join("VideoDock"),"systemTrayIconVisible":false,"autoStart":false,"autoUpdate":false,"betaAccessEnabled":false,"simultaneousDownloads":4,"subscriptionUpdatePeriod":86400,"preventSleepEnabled":false,"separateFolderCreationEnabled":true,"numerationEnabled":false,"skipDuplicates":false,"m3uEnabled":false,"embedSubtitles":false,"tagsEnabled":true,"removeCompletedDownloads":false,"recommendationServiceEnabled":true,"aiProcessingAllowed":true,"displayAllCodecs":false,"speedLimit":0,"proxyEnabled":false,"proxyType":"SOCKS5","proxyHost":"","proxyPort":1080,"proxyLogin":"","proxyPasswordSet":false,"notificationEnabled":true,"processingNotificationsEnabled":true,"remindNotificationEnabled":true,"similarVideosNotificationEnabled":true,"askQuit":true,"askDownloadChannel":true,"systemNotificationsEnabled":true,"notificationSound":false})
}
pub(super) fn text(value: &Value) -> Result<&str> {
    value.as_str().context("Ожидается строка")
}
pub(super) fn bounded(value: &Value, limit: usize) -> Result<String> {
    let value = text(value)?;
    ensure!(
        value.len() <= limit && !value.chars().any(char::is_control),
        "Недопустимая строка"
    );
    Ok(value.to_owned())
}
fn integer(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|v| v.trim().parse().ok()))
        .context("Ожидается положительное целое число")
}
pub(super) fn quality(value: &Value) -> Result<u32> {
    if value.is_number() {
        let n = integer(value)?;
        ensure!(n <= 16384, "Слишком большое разрешение");
        return Ok(n as u32);
    }
    let value = text(value)?.trim().to_ascii_lowercase();
    let n = match value.as_str() {
        "any" | "best" | "original" | "original quality" => 0,
        "8k" => 4320,
        "4k" => 2160,
        _ => value
            .trim_end_matches('p')
            .parse()
            .context("Неизвестное качество")?,
    };
    ensure!(n <= 16384, "Слишком большое разрешение");
    Ok(n)
}
fn interval(value: &Value) -> Result<u64> {
    if value.is_number() {
        let seconds = integer(value)?;
        ensure!(
            (60..=604800).contains(&seconds),
            "Интервал должен быть от минуты до недели"
        );
        return Ok(seconds);
    }
    let mut words = text(value)?.split_whitespace();
    let count: u64 = words.next().context("Не указан интервал")?.parse()?;
    let unit = words.next().unwrap_or("sec");
    let multiplier = match unit {
        "min" | "minute" | "minutes" => 60,
        "hour" | "hours" => 3600,
        "day" | "days" => 86400,
        "sec" | "seconds" => 1,
        _ => bail!("Неизвестная единица интервала"),
    };
    ensure!(words.next().is_none(), "Некорректный интервал");
    let seconds = count
        .checked_mul(multiplier)
        .context("Слишком большой интервал")?;
    ensure!(
        (60..=604800).contains(&seconds),
        "Интервал должен быть от минуты до недели"
    );
    Ok(seconds)
}
fn speed_limit(value: &Value) -> Result<u64> {
    if value.is_number() {
        let speed = integer(value)?;
        ensure!(speed <= 1_000_000_000, "Слишком большой лимит скорости");
        return Ok(speed);
    }
    let value = text(value)?.trim();
    if value.eq_ignore_ascii_case("unlimited") {
        return Ok(0);
    }
    let mut words = value.split_whitespace();
    let amount: u64 = words.next().context("Не указана скорость")?.parse()?;
    let multiplier = match words.next().unwrap_or("B/s") {
        "kbps" => 125,
        "Mbps" => 125000,
        "B/s" => 1,
        _ => bail!("Неизвестная единица скорости"),
    };
    ensure!(words.next().is_none(), "Некорректная скорость");
    let speed = amount
        .checked_mul(multiplier)
        .context("Слишком большая скорость")?;
    ensure!(speed <= 1_000_000_000, "Слишком большой лимит скорости");
    Ok(speed)
}
pub(super) fn normalize(key: &str, value: &Value) -> Result<Value> {
    let template = defaults();
    ensure!(
        template.get(key).is_some() && key != "proxyPasswordSet",
        "Неизвестная настройка: {key}"
    );
    Ok(match key {
        "quality" => json!(quality(value)?),
        "subscriptionUpdatePeriod" => json!(interval(value)?),
        "speedLimit" => json!(speed_limit(value)?),
        "simultaneousDownloads" => {
            let n = integer(value)?;
            ensure!(
                [1, 2, 4, 6, 8].contains(&n),
                "Допустимо 1, 2, 4, 6 или 8 загрузок"
            );
            json!(n)
        }
        "audioBitrate" => {
            let n = if value.as_str().is_some_and(|value| {
                ["original", "maximum", "any", "best"]
                    .contains(&value.to_ascii_lowercase().as_str())
            }) {
                0
            } else if let Some(value) = value.as_str() {
                integer(&json!(value.trim_end_matches("kbps").trim()))?
            } else {
                integer(value)?
            };
            ensure!(
                n == 0 || (32..=1536).contains(&n),
                "Битрейт должен быть Original или от 32 до 1536 kbps"
            );
            json!(n)
        }
        "frameRate" => {
            let n = if value
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case("any") || v.eq_ignore_ascii_case("maximum"))
            {
                0
            } else if let Some(value) = value.as_str().and_then(|value| value.parse::<f64>().ok()) {
                ensure!(
                    value.is_finite() && value >= 0.0,
                    "Некорректная частота кадров"
                );
                value.ceil() as u64
            } else {
                integer(value)?
            };
            ensure!(n <= 240, "Слишком большая частота кадров");
            json!(n)
        }
        "proxyPort" => {
            let n = integer(value)?;
            ensure!((1..=65535).contains(&n), "Порт должен быть от 1 до 65535");
            json!(n)
        }
        "language" => {
            let value = text(value)?;
            ensure!(
                ["en", "ru"].contains(&value),
                "Этот язык интерфейса пока не поставляется"
            );
            json!(value)
        }
        "mediaType" => {
            let value = text(value)?;
            ensure!(["video", "audio"].contains(&value), "Неизвестный тип медиа");
            json!(value)
        }
        "container" => {
            let value = text(value)?.to_ascii_lowercase();
            ensure!(
                ["mp4", "mkv", "webm", "mp3", "m4a", "ogg", "flac", "wav"]
                    .contains(&value.as_str()),
                "Неизвестный контейнер"
            );
            json!(value)
        }
        "videoCodec" | "audioCodec" => {
            let value = text(value)?.to_ascii_lowercase();
            let value = if value.starts_with("avc") {
                "h264"
            } else if value.starts_with("hev") || value.starts_with("hvc") {
                "h265"
            } else if value.starts_with("vp09") {
                "vp9"
            } else if value.starts_with("av01") {
                "av1"
            } else if value.starts_with("mp4a") {
                "aac"
            } else if value == "unknown" || value == "none" {
                "any"
            } else {
                value.as_str()
            };
            ensure!(
                [
                    "any",
                    "h264",
                    "h265",
                    "hevc",
                    "vp9",
                    "av1",
                    "aac",
                    "mp3",
                    "opus",
                    "vorbis",
                    "flac",
                    "pcm_s16le"
                ]
                .contains(&value),
                "Неизвестный кодек"
            );
            json!(value)
        }
        "videoVR" => {
            let enabled = match value {
                Value::Bool(v) => *v,
                Value::String(v) if ["Any", "2D", "false"].contains(&v.as_str()) => false,
                Value::String(v) if ["3D", "360", "true", "VR"].contains(&v.as_str()) => true,
                _ => bail!("Некорректный режим VR"),
            };
            json!(enabled)
        }
        "subtitles" | "audioTracks" => {
            let values = value.as_array().context("Ожидается список языков")?;
            ensure!(values.len() <= 64, "Слишком много языков");
            json!(
                values
                    .iter()
                    .map(|v| bounded(v, 128))
                    .collect::<Result<Vec<_>>>()?
            )
        }
        "proxyType" => {
            let value = text(value)?.to_ascii_uppercase();
            ensure!(
                ["HTTP", "HTTPS", "SOCKS4", "SOCKS5"].contains(&value.as_str()),
                "Неизвестный тип прокси"
            );
            json!(value)
        }
        "proxyHost" => {
            let value = bounded(value, 253)?;
            ensure!(
                value.is_empty()
                    || value.parse::<std::net::IpAddr>().is_ok()
                    || value
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-'),
                "Недопустимый сервер прокси"
            );
            json!(value)
        }
        "proxyLogin" => json!(bounded(value, 1024)?),
        "path" => {
            let value = bounded(value, 4096)?;
            ensure!(
                Path::new(&value).is_absolute(),
                "Нужен абсолютный путь к папке"
            );
            json!(value)
        }
        _ if template[key].is_boolean() => json!(
            value
                .as_bool()
                .context("Ожидается логический переключатель")?
        ),
        _ => bail!("Не поддерживается настройка {key}"),
    })
}
pub(super) fn update(inner: &Inner, values: &Value) -> Result<()> {
    let values = values
        .as_object()
        .context("Настройки должны быть объектом")?;
    ensure!(values.len() <= 64, "Слишком много настроек");
    let mut settings = lock(&inner.state)["settings"].clone();
    let previous_password = lock(&inner.secrets).proxy_password.clone();
    let mut password = previous_password.clone();
    for (key, value) in values {
        if key == "proxyPassword" {
            password = bounded(value, 8192)?;
        } else {
            settings[key] = normalize(key, value)?;
        }
    }
    if settings["proxyEnabled"] == true {
        ensure!(
            !settings["proxyHost"]
                .as_str()
                .unwrap_or_default()
                .is_empty(),
            "Не указан сервер прокси"
        );
    }
    if !settings["proxyLogin"]
        .as_str()
        .unwrap_or_default()
        .is_empty()
    {
        ensure!(!password.is_empty(), "Для логина прокси нужен пароль");
    } else if values.contains_key("proxyLogin") {
        ensure!(
            !values.contains_key("proxyPassword") || password.is_empty(),
            "Для пароля прокси нужен логин"
        );
        password.clear();
    }
    if values.get("autoUpdate") == Some(&json!(true)) {
        bail!("Автообновление требует настроенного подписанного источника релизов")
    }
    if values.contains_key("autoStart") {
        crate::platform::set_autostart(settings["autoStart"] == true)?;
    }
    settings["proxyPasswordSet"] = json!(!password.is_empty());
    let password_changed = password != previous_password;
    if password_changed {
        super::secrets::mutate(inner, |secrets| {
            secrets.proxy_password = password;
            Ok(())
        })?;
    }
    if let Err(error) = change(inner, |state| {
        state["settings"] = settings.clone();
        Ok(())
    }) {
        if password_changed {
            super::secrets::mutate(inner, |secrets| {
                secrets.proxy_password = previous_password;
                Ok(())
            })
            .ok();
        }
        return Err(error);
    }
    if values.contains_key("path") {
        inner.library_last.store(0, Ordering::SeqCst);
    }
    inner
        .limiter
        .set(settings["speedLimit"].as_u64().unwrap_or(0));
    Ok(())
}
pub(super) fn selection(settings: &Value, action: &Value) -> Result<Selection> {
    let mut values = settings.clone();
    let format = &action["format"];
    for (source, target) in [
        ("media_type", "mediaType"),
        ("container", "container"),
        ("quality", "quality"),
        ("video_codec", "videoCodec"),
        ("audio_codec", "audioCodec"),
        ("audio_bitrate", "audioBitrate"),
        ("fps", "frameRate"),
        ("vr", "videoVR"),
    ] {
        if let Some(value) = format.get(source) {
            if source == "container"
                && value
                    .as_str()
                    .is_some_and(|v| v.eq_ignore_ascii_case("any"))
            {
                continue;
            }
            if source == "quality"
                && values["mediaType"] == "audio"
                && value.as_str().is_some_and(|v| v.ends_with("kbps"))
            {
                let bitrate = value
                    .as_str()
                    .unwrap()
                    .trim_end_matches("kbps")
                    .trim()
                    .parse::<u32>()?;
                values["audioBitrate"] = normalize("audioBitrate", &json!(bitrate))?;
                continue;
            }
            if source == "quality"
                && values["mediaType"] == "audio"
                && value
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case("original"))
            {
                values["audioBitrate"] = json!(0);
                continue;
            }
            values[target] = normalize(target, value)?;
        }
    }
    for (source, target) in [("subtitles", "subtitles"), ("audio_tracks", "audioTracks")] {
        if let Some(value) = action.get(source).or_else(|| format.get(source)) {
            values[target] = normalize(target, value)?;
        }
    }
    Ok(Selection {
        media_type: values["mediaType"].as_str().unwrap_or("video").into(),
        container: values["container"].as_str().unwrap_or("mp4").into(),
        quality: quality(&values["quality"])?,
        video_codec: values["videoCodec"].as_str().unwrap_or("any").into(),
        audio_codec: values["audioCodec"].as_str().unwrap_or("any").into(),
        audio_bitrate: values["audioBitrate"].as_u64().unwrap_or(192) as u32,
        fps: values["frameRate"].as_u64().unwrap_or(0) as u32,
        vr: values["videoVR"] == true,
        subtitles: serde_json::from_value(values["subtitles"].clone())?,
        audio_tracks: serde_json::from_value(values["audioTracks"].clone())?,
        format_id: action["format_id"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(str::to_owned),
    })
}
