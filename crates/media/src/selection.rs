use crate::{Format, Selection};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

fn language_tag(value: &str) -> String {
    let lower = value.to_ascii_lowercase().replace('_', "-");
    let (base, region) = lower.split_once('-').unwrap_or((&lower, ""));
    let base = match base {
        "iw" => "he",
        "fl" | "tl" => "fil",
        value => value,
    };
    if region.is_empty() {
        base.into()
    } else {
        format!("{base}-{region}")
    }
}

fn matches_language(actual: &str, requested: &str) -> bool {
    let actual = language_tag(actual);
    let requested = language_tag(requested);
    actual == requested
        || (!requested.contains('-')
            && actual
                .strip_prefix(&requested)
                .is_some_and(|suffix| suffix.starts_with('-')))
}

fn track_key(format: &Format) -> String {
    if let Some(id) = format.audio_track_id.as_deref().filter(|id| !id.is_empty()) {
        format!("id:{id}")
    } else if let Some(language) = format
        .audio_language
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        format!("language:{}", language_tag(language))
    } else {
        "unidentified".into()
    }
}

fn audio_score(format: &Format, preferred: Option<&str>) -> (bool, bool, u64) {
    (
        preferred == Some(format.id.as_str()),
        !format.has_video(),
        format.bitrate.unwrap_or_default(),
    )
}

fn best_audio<'a>(
    formats: impl Iterator<Item = &'a Format>,
    preferred: Option<&str>,
) -> Option<&'a Format> {
    formats.max_by_key(|format| audio_score(format, preferred))
}

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
                    "vp8" => value.starts_with("vp08") || value.starts_with("vp8"),
                    "aac" => value.starts_with("mp4a") || value.starts_with("aac"),
                    other => value.starts_with(other),
                }
            })
    };
    let mut video = None;
    if settings.media_type == "audio"
        && settings.audio_tracks.is_empty()
        && let Some(id) = &settings.format_id
    {
        let audio = formats
            .iter()
            .find(|f| &f.id == id && f.has_audio())
            .ok_or_else(|| anyhow::anyhow!("Выбранный аудиоформат недоступен"))?;
        return Ok(vec![audio.clone()]);
    }
    if settings.media_type == "video" {
        let selected = if let Some(id) = &settings.format_id {
            formats
                .iter()
                .find(|f| &f.id == id && f.has_video() && settings.vr_mode.matches(f, settings.vr))
        } else {
            formats
                .iter()
                .filter(|f| {
                    f.has_video()
                        && settings.vr_mode.matches(f, settings.vr)
                        && (settings.quality == 0
                            || f.video_quality().is_none_or(|height| height <= settings.quality))
                        && (settings.fps == 0
                            || f.fps.is_none_or(|fps| fps <= f64::from(settings.fps)))
                        && matches_codec(&f.video_codec, &settings.video_codec)
                })
                .max_by_key(|f| {
                    (
                        f.video_quality().unwrap_or_default(),
                        (f.fps.unwrap_or_default() * 100.0) as u64,
                        f.bitrate.unwrap_or_default(),
                    )
                })
        };
        video =
            Some(selected.ok_or_else(|| {
                anyhow::anyhow!("Нет видео с выбранными фильтрами качества/кодека")
            })?);
    }
    let preferred_id = if settings.media_type == "audio" {
        settings.format_id.as_deref()
    } else {
        None
    };
    let preferred = preferred_id
        .map(|id| {
            formats
                .iter()
                .find(|format| format.id == id && format.has_audio())
                .ok_or_else(|| anyhow::anyhow!("Выбранный аудиоформат недоступен"))
        })
        .transpose()?;
    let candidates: Vec<_> = formats
        .iter()
        .filter(|f| f.has_audio() && matches_codec(&f.audio_codec, &settings.audio_codec))
        .collect();
    ensure!(!candidates.is_empty(), "Нет подходящей аудиодорожки");
    let has_default_metadata = formats
        .iter()
        .any(|format| format.has_audio() && format.audio_default.is_some());
    let mut audio = Vec::new();
    if settings
        .audio_tracks
        .iter()
        .any(|value| value.eq_ignore_ascii_case("all"))
    {
        let streams: Vec<_> = formats.iter().filter(|format| format.has_audio()).collect();
        let identified = streams
            .iter()
            .any(|format| track_key(format) != "unidentified");
        let mut tracks: BTreeMap<String, Option<&Format>> = BTreeMap::new();
        for candidate in &streams {
            let key = track_key(candidate);
            if identified && key == "unidentified" {
                continue;
            }
            if candidate.audio_track_id.is_none()
                && candidate.audio_language.as_ref().is_some_and(|language| {
                    streams.iter().any(|other| {
                        other.audio_track_id.is_some()
                            && other
                                .audio_language
                                .as_ref()
                                .is_some_and(|other| language_tag(language) == language_tag(other))
                    })
                })
            {
                continue;
            }
            let current = tracks.entry(key).or_default();
            if matches_codec(&candidate.audio_codec, &settings.audio_codec)
                && current.is_none_or(|current| {
                    audio_score(candidate, preferred_id) > audio_score(current, preferred_id)
                })
            {
                *current = Some(candidate);
            }
        }
        for selected in tracks.into_values() {
            audio.push(selected.ok_or_else(|| {
                anyhow::anyhow!("Одна из аудиодорожек недоступна с выбранным кодеком")
            })?);
        }
    } else {
        let requested = if settings.audio_tracks.is_empty() {
            vec!["default"]
        } else {
            settings.audio_tracks.iter().map(String::as_str).collect()
        };
        for requested in requested {
            let selected = if requested == "-" || requested.eq_ignore_ascii_case("default") {
                if has_default_metadata {
                    best_audio(
                        candidates
                            .iter()
                            .copied()
                            .filter(|format| format.audio_default == Some(true)),
                        preferred_id,
                    )
                } else {
                    video
                        .filter(|format| {
                            format.has_audio()
                                && matches_codec(&format.audio_codec, &settings.audio_codec)
                        })
                        .or_else(|| {
                            best_audio(
                                candidates
                                    .iter()
                                    .copied()
                                    .filter(|format| format.audio_language.is_none()),
                                preferred_id,
                            )
                        })
                        .or_else(|| best_audio(candidates.iter().copied(), preferred_id))
                }
            } else if requested.eq_ignore_ascii_case("original") {
                best_audio(
                    candidates
                        .iter()
                        .copied()
                        .filter(|format| format.audio_original == Some(true)),
                    preferred_id,
                )
            } else {
                best_audio(
                    candidates.iter().copied().filter(|format| {
                        format.audio_track_id.as_deref() == Some(requested)
                            || format.id == requested
                            || format
                                .audio_language
                                .as_deref()
                                .is_some_and(|language| matches_language(language, requested))
                    }),
                    preferred_id,
                )
            }
            .ok_or_else(|| anyhow::anyhow!("Выбранная аудиодорожка недоступна: {requested}"))?;
            if !audio
                .iter()
                .any(|format| track_key(format) == track_key(selected))
            {
                audio.push(selected);
            }
        }
    }
    ensure!(
        preferred.is_none_or(|preferred| audio.iter().any(|selected| selected.id == preferred.id)),
        "Выбранный аудиоформат не соответствует запрошенной дорожке"
    );
    let mut chosen = Vec::new();
    if let Some(video) = video {
        let mut selected = video.clone();
        if !audio.iter().any(|audio| audio.id == video.id) {
            // Explicit audio replaces the muxed track; the video input remains available.
            selected.audio_codec = None;
        }
        chosen.push(selected);
    }
    for selected in audio {
        if video.is_none_or(|video| selected.id != video.id) {
            chosen.push(selected.clone());
        }
    }
    ensure!(!chosen.is_empty(), "Нет доступных медиапотоков");
    Ok(chosen)
}
