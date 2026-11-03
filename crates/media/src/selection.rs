use crate::{Format, Selection};
use anyhow::{Result, ensure};

pub fn choose_formats(formats: &[Format], settings: &Selection) -> Result<Vec<Format>> {
    ensure!(
        matches!(settings.media_type.as_str(), "audio" | "video"),
        "Неизвестный тип медиа"
    );
    let matches_codec = |codec: &Option<String>, requested: &str| {
        requested.eq_ignore_ascii_case("any")
            || requested.is_empty()
            || codec.as_deref().is_some_and(|value| {
                let value = value.to_ascii_lowercase();
                match requested.to_ascii_lowercase().as_str() {
                    "h264" => value.starts_with("avc") || value.starts_with("h264"),
                    "h265" | "hevc" => value.starts_with("hev") || value.starts_with("hvc"),
                    "av1" => value.starts_with("av01") || value.starts_with("av1"),
                    "vp9" => value.starts_with("vp09") || value.starts_with("vp9"),
                    "aac" => value.starts_with("mp4a") || value.starts_with("aac"),
                    other => value.starts_with(other),
                }
            })
    };
    let mut chosen = Vec::new();
    if settings.media_type == "audio"
        && let Some(id) = &settings.format_id
    {
        let audio = formats
            .iter()
            .find(|f| &f.id == id && f.has_audio())
            .ok_or_else(|| anyhow::anyhow!("Выбранный аудиоформат недоступен"))?;
        return Ok(vec![audio.clone()]);
    }
    if settings.media_type == "video" {
        let video = if let Some(id) = &settings.format_id {
            formats
                .iter()
                .find(|f| &f.id == id && f.has_video() && (!settings.vr || f.vr))
        } else {
            formats
                .iter()
                .filter(|f| {
                    f.has_video()
                        && (!settings.vr || f.vr)
                        && (settings.quality == 0
                            || f.height.is_none_or(|height| height <= settings.quality))
                        && (settings.fps == 0
                            || f.fps.is_none_or(|fps| fps <= f64::from(settings.fps)))
                        && matches_codec(&f.video_codec, &settings.video_codec)
                })
                .max_by_key(|f| {
                    (
                        f.height.unwrap_or_default(),
                        (f.fps.unwrap_or_default() * 100.0) as u64,
                        f.bitrate.unwrap_or_default(),
                    )
                })
        };
        let video = video
            .ok_or_else(|| anyhow::anyhow!("Нет видео с выбранными фильтрами качества/кодека"))?;
        chosen.push(video.clone());
        if video.has_audio() && settings.audio_tracks.is_empty() {
            return Ok(chosen);
        }
    }
    let candidates: Vec<_> = formats
        .iter()
        .filter(|f| f.has_audio() && matches_codec(&f.audio_codec, &settings.audio_codec))
        .collect();
    if settings.audio_tracks.is_empty() {
        let audio = candidates
            .iter()
            .max_by_key(|f| (!f.has_video(), f.bitrate.unwrap_or_default()))
            .ok_or_else(|| anyhow::anyhow!("Нет подходящей аудиодорожки"))?;
        chosen.push((*audio).clone());
    } else {
        for language in &settings.audio_tracks {
            let audio = candidates
                .iter()
                .filter(|f| {
                    f.audio_language.as_deref() == Some(language)
                        || &f.id == language
                        || (language == "default" && f.audio_language.is_none())
                })
                .max_by_key(|f| (!f.has_video(), f.bitrate.unwrap_or_default()))
                .ok_or_else(|| anyhow::anyhow!("Выбранная аудиодорожка недоступна: {language}"))?;
            if !chosen.iter().any(|f| f.id == audio.id) {
                chosen.push((*audio).clone());
            }
        }
    }
    ensure!(!chosen.is_empty(), "Нет доступных медиапотоков");
    Ok(chosen)
}
