use super::{codecs, subtitle_url};
use crate::{
    Format, Media, MediaKind, Protocol, Subtitle, VideoProjection,
    proof::Proof,
    sabr::{Delivery, Session},
};
use anyhow::{Result, ensure};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};
fn integer(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}
fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|v| !v.is_empty()).map(str::to_owned)
}
fn format_tags(format: &Value) -> Option<String> {
    text(&format["xtags"]).or_else(|| {
        let source = format["url"]
            .as_str()
            .filter(|value| value.len() <= 64 * 1024)
            .map(str::to_owned)
            .or_else(|| {
                let cipher = format["signatureCipher"]
                    .as_str()
                    .or_else(|| format["cipher"].as_str())?;
                if cipher.len() > 64 * 1024 {
                    return None;
                }
                url::form_urlencoded::parse(cipher.as_bytes())
                    .find(|(key, _)| key == "url")
                    .map(|(_, value)| value.into_owned())
            })?;
        url::Url::parse(&source)
            .ok()?
            .query_pairs()
            .find(|(key, value)| key == "xtags" && !value.is_empty())
            .map(|(_, value)| value.into_owned())
    })
}
fn binary(value: &str) -> Result<Vec<u8>> {
    ensure!(value.len() <= 128 * 1024, "Параметры SABR слишком велики");
    data_encoding::BASE64URL_NOPAD
        .decode(value.trim_end_matches('=').as_bytes())
        .map_err(|_| anyhow::anyhow!("Некорректная кодировка SABR"))
}

pub(super) fn delivery_url(data: &Value) -> Result<&str> {
    match data["playabilityStatus"]["status"].as_str() {
        Some("OK") => {}
        Some("LOGIN_REQUIRED" | "AGE_CHECK_REQUIRED" | "CONTENT_CHECK_REQUIRED") => {
            anyhow::bail!(
                "YouTube просит подтвердить доступ. Откройте видео во встроенном браузере; при необходимости выберите вход в аккаунт YouTube в настройках."
            )
        }
        Some("UNPLAYABLE" | "ERROR" | "LIVE_STREAM_OFFLINE") => {
            anyhow::bail!(
                "Это видео сейчас недоступно на YouTube. Откройте ссылку в браузере и проверьте, воспроизводится ли оно."
            )
        }
        _ => anyhow::bail!("Не удалось получить ответ YouTube. Повторите загрузку позже."),
    }
    data["streamingData"]["serverAbrStreamingUrl"]
        .as_str()
        .filter(|url| !url.is_empty())
        .ok_or_else(|| anyhow::anyhow!("YouTube не предоставил поток для скачивания. Повторите загрузку. Если ошибка повторяется, проверьте обновления VideoDock."))
}

pub(super) fn map(
    data: Value,
    id: &str,
    url: String,
    version: String,
    proof: Proof,
    http: reqwest::Client,
) -> Result<Media> {
    ensure!(
        data["playabilityStatus"]["status"] == "OK",
        "YouTube не разрешил анонимное воспроизведение"
    );
    let details = &data["videoDetails"];
    ensure!(details["videoId"] == id, "YouTube вернул другое видео");
    ensure!(
        details["isLive"].as_bool() != Some(true),
        "Для SABR требуется завершённое видео"
    );
    let duration = integer(&details["lengthSeconds"])
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow::anyhow!("YouTube не указал длительность"))?;
    let streaming = &data["streamingData"];
    ensure!(
        streaming["drmParams"].is_null(),
        "YouTube: видео защищено DRM"
    );
    let config=data["playerConfig"]["mediaCommonConfig"]["mediaUstreamerRequestConfig"]["videoPlaybackUstreamerConfig"].as_str().ok_or_else(||anyhow::anyhow!("Нет конфигурации SABR"))?;
    let session = Arc::new(Session::new(
        url,
        id.to_owned(),
        binary(config)?,
        binary(&proof.content_po)?,
        proof.user_agent,
        version,
        http,
    )?);
    let source = format!("https://www.youtube.com/watch?v={id}");
    let entries = streaming["adaptiveFormats"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Нет форматов SABR"))?;
    ensure!(entries.len() <= 2048, "Слишком много форматов SABR");
    let mut formats = Vec::new();
    for f in entries {
        if f["drmFamilies"].as_array().is_some_and(|v| !v.is_empty())
            || !f["drmTrackType"].is_null()
        {
            continue;
        }
        let Some(itag) = integer(&f["itag"]).and_then(|n| u32::try_from(n).ok()) else {
            continue;
        };
        let Some(last_modified) = integer(&f["lastModified"]) else {
            continue;
        };
        let Some(size) = integer(&f["contentLength"]).filter(|n| *n > 0) else {
            continue;
        };
        let Some(mime) = f["mimeType"].as_str() else {
            continue;
        };
        let (video_codec, audio_codec) = codecs(mime);
        if video_codec.is_none() && audio_codec.is_none() {
            continue;
        }
        let ext = match mime.split(';').next().unwrap_or("") {
            "video/mp4" => "mp4",
            "audio/mp4" => "m4a",
            "video/webm" | "audio/webm" => "webm",
            _ => continue,
        };
        let xtags = format_tags(f);
        ensure!(
            xtags
                .as_ref()
                .is_none_or(|v| v.len() <= 1024 && !v.chars().any(char::is_control)),
            "Некорректные метки SABR"
        );
        let tags: BTreeMap<_, _> = xtags
            .as_deref()
            .unwrap_or("")
            .split(':')
            .filter_map(|v| v.split_once('='))
            .collect();
        let track = &f["audioTrack"];
        let audio_track_id = text(&track["id"]);
        ensure!(
            audio_track_id
                .as_ref()
                .is_none_or(|v| v.len() <= 1024 && !v.chars().any(char::is_control)),
            "Некорректная аудиодорожка SABR"
        );
        let track_id = text(&track["id"]).or_else(|| audio_codec.as_ref().and(xtags.clone()));
        let language = tags.get("lang").map(|v| (*v).to_owned());
        let original = tags.get("acont").map(|v| *v == "original");
        let duration_ms = integer(&f["approxDurationMs"]).unwrap_or(duration.saturating_mul(1000));
        let projection = text(&f["projectionType"]);
        let stereo = text(&f["stereoLayout"]);
        formats.push(Format {
            id: track_id
                .as_ref()
                .map(|t| format!("{itag}:{t}"))
                .unwrap_or_else(|| itag.to_string()),
            url: source.clone(),
            protocol: Protocol::Sabr,
            ext: ext.into(),
            sabr: Some(Delivery {
                session: session.clone(),
                itag,
                last_modified,
                xtags,
                audio_track_id,
                drc_enabled: f["isDrc"] == true && f["isVb"] != true,
                voice_boost: f["isVb"] == true,
                duration_ms,
                mime_type: mime.to_owned(),
                width: integer(&f["width"])
                    .and_then(|n| n.try_into().ok())
                    .unwrap_or(0),
                height: integer(&f["height"])
                    .and_then(|n| n.try_into().ok())
                    .unwrap_or(0),
                fps: integer(&f["fps"])
                    .and_then(|n| n.try_into().ok())
                    .unwrap_or(0),
                bitrate: integer(&f["bitrate"]).unwrap_or(0),
                channels: integer(&f["audioChannels"])
                    .and_then(|n| n.try_into().ok())
                    .unwrap_or(2),
            }),
            width: integer(&f["width"]).and_then(|n| n.try_into().ok()),
            height: integer(&f["height"]).and_then(|n| n.try_into().ok()),
            fps: f["fps"].as_f64(),
            vr: projection
                .as_deref()
                .is_some_and(|v| matches!(v, "MESH" | "EQUIRECTANGULAR" | "360")),
            projection: (projection.as_deref() == Some("RECTANGULAR"))
                .then_some(VideoProjection::Flat),
            projection_type: projection,
            stereo_layout: stereo,
            video_codec,
            audio_codec,
            audio_track_id: track_id,
            audio_language: language.clone(),
            audio_title: text(&track["displayName"]).or(language),
            audio_default: track["audioIsDefault"].as_bool(),
            audio_original: original,
            bitrate: integer(&f["bitrate"]),
            size: Some(size),
            ..Default::default()
        });
    }
    ensure!(
        !formats.is_empty(),
        "YouTube не вернул полные публичные SABR-форматы"
    );
    let mut subtitles = Vec::new();
    if let Some(tracks) =
        data["captions"]["playerCaptionsTracklistRenderer"]["captionTracks"].as_array()
    {
        ensure!(tracks.len() <= 256, "Слишком много субтитров");
        for track in tracks {
            let Some(url) = track["baseUrl"].as_str() else {
                continue;
            };
            let Some(lang) = track["languageCode"].as_str() else {
                continue;
            };
            subtitles.push(Subtitle {
                language: lang.into(),
                title: track["name"]["simpleText"].as_str().unwrap_or(lang).into(),
                url: subtitle_url(url)?,
                ext: "vtt".into(),
                automatic: track["kind"] == "asr",
                translatable: track["isTranslatable"].as_bool().unwrap_or(false),
            });
        }
    }
    let thumbnail = details["thumbnail"]["thumbnails"]
        .as_array()
        .and_then(|v| v.last())
        .and_then(|v| v["url"].as_str())
        .unwrap_or_default()
        .to_owned();
    Ok(Media {
        id: id.into(),
        url: source,
        title: details["title"].as_str().unwrap_or(id).into(),
        description: details["shortDescription"]
            .as_str()
            .unwrap_or_default()
            .into(),
        kind: MediaKind::Video,
        duration: Some(duration as f64),
        thumbnail,
        channel_id: details["channelId"].as_str().unwrap_or_default().into(),
        channel_title: details["author"].as_str().unwrap_or_default().into(),
        formats,
        subtitles,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn format_identity_reads_nested_cipher_tags_without_using_the_signature() {
        let source = "https://r.googlevideo.com/videoplayback?xtags=fixture%3Dvariant";
        let cipher = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("url", source)
            .append_pair("s", "synthetic-secret")
            .finish();
        for key in ["signatureCipher", "cipher"] {
            let mut format = serde_json::json!({});
            format[key] = Value::String(cipher.clone());
            assert_eq!(format_tags(&format).as_deref(), Some("fixture=variant"));
        }
        assert_eq!(
            format_tags(&serde_json::json!({"xtags":"direct=1","url":source})).as_deref(),
            Some("direct=1")
        );
        assert!(format_tags(&serde_json::json!({"url":"not a URL"})).is_none());
        assert!(
            format_tags(&serde_json::json!({"signatureCipher":"url=not-a-url&s=synthetic-secret"}))
                .is_none()
        );
        assert!(format_tags(&serde_json::json!({"url":"x".repeat(64 * 1024 + 1)})).is_none());
    }
    #[test]
    fn player_refusal_is_reported_before_looking_for_stream_url() {
        for (status, expected) in [
            ("LOGIN_REQUIRED", "подтвердить"),
            ("UNPLAYABLE", "недоступно"),
            ("ERROR", "недоступно"),
        ] {
            let data = serde_json::json!({"playabilityStatus":{"status":status,"reason":"private-response-must-not-be-echoed"}});
            let error = delivery_url(&data).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("SABR") && !error.contains("private-response"));
        }
        let missing = delivery_url(&serde_json::json!({"playabilityStatus":{"status":"OK"}}))
            .unwrap_err()
            .to_string();
        assert!(missing.contains("Повторите") && !missing.contains("SABR"));
        let live = serde_json::json!({"playabilityStatus":{"status":"OK"},"streamingData":{"serverAbrStreamingUrl":"https://r.googlevideo.com/videoplayback"}});
        assert_eq!(
            delivery_url(&live).unwrap(),
            "https://r.googlevideo.com/videoplayback"
        );
    }
    #[tokio::test]
    async fn native_metadata_preserves_quality_and_keeps_proof_and_signed_url_out_of_serialization()
    {
        let mut data = serde_json::json!({"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"fixture0001","lengthSeconds":"2","title":"Fixture"},"streamingData":{"adaptiveFormats":[{"itag":136,"url":"https://r.googlevideo.com/videoplayback?token=synthetic-signed-secret&xtags=fixture%3Dvariant","lastModified":"1","contentLength":"9","approxDurationMs":"2000","mimeType":"video/mp4; codecs=\"avc1.4d401f\"","width":1280,"height":720,"fps":30}]},"playerConfig":{"mediaCommonConfig":{"mediaUstreamerRequestConfig":{"videoPlaybackUstreamerConfig":"YWJj"}}}});
        for (drc, voice_boost) in [(false, false), (true, false), (true, true)] {
            data["streamingData"]["adaptiveFormats"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "itag":251,"lastModified":"2","contentLength":"9",
                    "mimeType":"audio/webm; codecs=\"opus\"","audioChannels":2,
                    "audioTrack":{"id":"en.4"},"isDrc":drc,"isVb":voice_boost
                }));
        }
        let media = map(
            data,
            "fixture0001",
            "https://r.googlevideo.com/videoplayback?token=synthetic-signed-secret".into(),
            "2.20261104".into(),
            Proof {
                content_po: "c3ludGhldGljLXBvLXNlY3JldA".into(),
                visitor_data: "fixture".into(),
                user_agent: "Fixture Agent".into(),
            },
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(media.formats[0].height, Some(720));
        assert_eq!(media.formats[0].sabr.as_ref().unwrap().duration_ms, 2000);
        for (audio, (drc, voice_boost)) in
            media.formats[1..]
                .iter()
                .zip([(false, false), (true, false), (false, true)])
        {
            let delivery = audio.sabr.as_ref().unwrap();
            assert_eq!(delivery.audio_track_id.as_deref(), Some("en.4"));
            assert_eq!(
                (delivery.drc_enabled, delivery.voice_boost),
                (drc, voice_boost)
            );
        }
        assert_eq!(
            media.formats[0].sabr.as_ref().unwrap().xtags.as_deref(),
            Some("fixture=variant")
        );
        let serialized = serde_json::to_string(&media).unwrap();
        assert!(!serialized.contains("synthetic-signed-secret"));
        assert!(!serialized.contains("c3ludGhldGljLXBvLXNlY3JldA"));
        assert!(!format!("{media:?}").contains("synthetic-signed-secret"));
    }
}
