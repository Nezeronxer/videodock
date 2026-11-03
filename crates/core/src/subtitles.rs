use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use videodock_media::Subtitle;

pub fn select(tracks: &[Subtitle], requested: &str) -> Result<(Subtitle, String)> {
    let parts = requested.split(':').collect::<Vec<_>>();
    let language = parts[0];
    ensure!(
        !language.is_empty()
            && language.len() <= 32
            && language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "Недопустимый код языка субтитров"
    );
    let kind = parts.get(1).copied().unwrap_or("");
    let exact = match kind {
        "auto" => tracks
            .iter()
            .find(|t| t.language == language && t.automatic),
        "authored" => tracks
            .iter()
            .find(|t| t.language == language && !t.automatic),
        "translated" => None,
        "" => tracks
            .iter()
            .find(|t| t.language == language && !t.automatic)
            .or_else(|| tracks.iter().find(|t| t.language == language)),
        _ => bail!("Неизвестный тип субтитров"),
    };
    if let Some(track) = exact {
        return Ok((track.clone(), requested.replace(':', ".")));
    }
    ensure!(
        kind.is_empty() || kind == "translated",
        "Субтитры выбранного типа недоступны"
    );
    let source = parts.get(2).copied();
    let mut track = tracks
        .iter()
        .find(|t| t.translatable && source.is_none_or(|s| t.language == s))
        .context("Перевод выбранных субтитров недоступен")?
        .clone();
    let mut url = videodock_media::validate_url(&track.url)?;
    url.query_pairs_mut().append_pair("tlang", language);
    track.url = url.to_string();
    track.language = language.into();
    track.automatic = true;
    Ok((
        track,
        if kind == "translated" {
            requested.replace(':', ".")
        } else {
            format!("{language}.translated")
        },
    ))
}

pub fn language_tag(code: &str) -> &'static str {
    let code = code.split(['-', '_']).next().unwrap_or("");
    let code = match code {
        "iw" => "he",
        "in" => "id",
        "ji" => "yi",
        other => other,
    };
    isolang::Language::from_639_1(code)
        .or_else(|| isolang::Language::from_639_3(code))
        .map(|l| l.to_639_3())
        .unwrap_or("und")
}

#[derive(Debug)]
struct Cue {
    start: f64,
    end: f64,
    text: String,
}
fn timestamp(seconds: f64) -> Result<String> {
    ensure!(
        seconds.is_finite() && (0.0..=604800.0).contains(&seconds),
        "Некорректное время субтитров"
    );
    let ms = (seconds * 1000.0).round() as u64;
    Ok(format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    ))
}
fn vtt(cues: Vec<Cue>) -> Result<Vec<u8>> {
    let mut output = String::from("WEBVTT\n\n");
    for cue in cues {
        if cue.text.trim().is_empty() {
            continue;
        }
        ensure!(
            cue.end > cue.start,
            "Конец субтитра должен быть после начала"
        );
        output.push_str(&format!(
            "{} --> {}\n{}\n\n",
            timestamp(cue.start)?,
            timestamp(cue.end)?,
            cue.text.trim()
        ));
    }
    ensure!(output.len() > 8, "Пустые субтитры");
    Ok(output.into_bytes())
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}
fn time(value: &str, frame_rate: f64, tick_rate: Option<f64>) -> Result<f64> {
    let value = value.trim();
    for (suffix, scale) in [
        ("ms", 0.001),
        ("s", 1.0),
        ("m", 60.0),
        ("h", 3600.0),
        ("f", 1.0 / frame_rate),
    ] {
        if let Some(value) = value.strip_suffix(suffix) {
            return Ok(value.parse::<f64>()? * scale);
        }
    }
    if let Some(value) = value.strip_suffix('t') {
        return Ok(value.parse::<f64>()? / tick_rate.context("Не указан tickRate")?);
    }
    let parts = value
        .split(':')
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(match parts.as_slice() {
        [s] => *s,
        [h, m, s] => h * 3600.0 + m * 60.0 + s,
        [h, m, s, frame] => h * 3600.0 + m * 60.0 + s + frame / frame_rate,
        _ => bail!("Неизвестный формат времени субтитров"),
    })
}

pub fn normalize(data: &[u8], extension: &str) -> Result<(Vec<u8>, &'static str)> {
    ensure!(
        data.len() <= 16 * 1024 * 1024,
        "Слишком большой файл субтитров"
    );
    let text = if data.starts_with(&[0xff, 0xfe]) || data.starts_with(&[0xfe, 0xff]) {
        ensure!(data.len().is_multiple_of(2), "Повреждённые UTF-16 субтитры");
        let little = data[0] == 0xff;
        String::from_utf16(
            &data[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|v| {
                    if little {
                        u16::from_le_bytes([v[0], v[1]])
                    } else {
                        u16::from_be_bytes([v[0], v[1]])
                    }
                })
                .collect::<Vec<_>>(),
        )?
    } else {
        std::str::from_utf8(data)?
            .trim_start_matches('\u{feff}')
            .to_owned()
    };
    let trimmed = text.trim_start();
    if trimmed.starts_with("WEBVTT") {
        return Ok((text.into_bytes(), "vtt"));
    }
    if trimmed
        .lines()
        .next()
        .is_some_and(|line| line.trim().parse::<usize>().is_ok())
        && text.contains("-->")
    {
        return Ok((text.into_bytes(), "srt"));
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let json: Value = serde_json::from_str(&text)?;
        let mut cues = Vec::new();
        if let Some(body) = json.get("body").and_then(Value::as_array) {
            for item in body {
                cues.push(Cue {
                    start: number(&item["from"]).context("Нет начала субтитра")?,
                    end: number(&item["to"]).context("Нет конца субтитра")?,
                    text: item["content"].as_str().unwrap_or_default().into(),
                });
            }
        } else if let Some(events) = json.get("events").and_then(Value::as_array) {
            for (index, event) in events.iter().enumerate() {
                let Some(start) = number(&event["tStartMs"]) else {
                    continue;
                };
                let duration = number(&event["dDurationMs"])
                    .or_else(|| {
                        events
                            .get(index + 1)
                            .and_then(|next| number(&next["tStartMs"]))
                            .map(|next| next - start)
                    })
                    .unwrap_or(3000.0);
                let body = event["segs"]
                    .as_array()
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|part| part["utf8"].as_str())
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                cues.push(Cue {
                    start: start / 1000.0,
                    end: (start + duration) / 1000.0,
                    text: body,
                });
            }
        } else {
            bail!("Неизвестная JSON-схема субтитров")
        }
        return Ok((vtt(cues)?, "vtt"));
    }
    if trimmed.starts_with('<') {
        let document = roxmltree::Document::parse_with_options(
            &text,
            roxmltree::ParsingOptions {
                allow_dtd: false,
                nodes_limit: 500_000,
            },
        )?;
        let root = document.root_element();
        let attribute = |name: &str| {
            root.attributes()
                .find(|attribute| attribute.name() == name)
                .map(|attribute| attribute.value())
        };
        let mut frame_rate = attribute("frameRate")
            .map(str::parse::<f64>)
            .transpose()?
            .unwrap_or(30.0);
        if let Some(multiplier) = attribute("frameRateMultiplier") {
            let parts = multiplier
                .split_whitespace()
                .map(str::parse::<f64>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(
                parts.len() == 2 && parts[1] > 0.0,
                "Неверный frameRateMultiplier"
            );
            frame_rate *= parts[0] / parts[1];
        }
        ensure!(
            frame_rate.is_finite() && frame_rate > 0.0 && frame_rate <= 1000.0,
            "Неверный frameRate"
        );
        let tick_rate = attribute("tickRate").map(str::parse::<f64>).transpose()?;
        ensure!(
            tick_rate.is_none_or(|rate| rate.is_finite() && rate > 0.0),
            "Неверный tickRate"
        );
        let is_cue = |node: roxmltree::Node<'_, '_>| {
            node.is_element() && matches!(node.tag_name().name(), "p" | "text")
        };
        let mut cues = Vec::new();
        for node in root
            .descendants()
            .filter(|node| is_cue(*node) && !node.ancestors().skip(1).any(is_cue))
        {
            let start = node
                .attribute("begin")
                .or_else(|| node.attribute("start"))
                .map(|value| time(value, frame_rate, tick_rate))
                .transpose()?
                .unwrap_or_default();
            let end = if let Some(end) = node.attribute("end") {
                time(end, frame_rate, tick_rate)?
            } else {
                start
                    + node
                        .attribute("dur")
                        .map(|value| time(value, frame_rate, tick_rate))
                        .transpose()?
                        .unwrap_or(3.0)
            };
            let mut text = String::new();
            for child in node.descendants() {
                if child.is_text() {
                    text.push_str(child.text().unwrap_or_default());
                } else if child.is_element() && child.tag_name().name() == "br" {
                    text.push('\n');
                }
            }
            cues.push(Cue { start, end, text });
        }
        return Ok((vtt(cues)?, "vtt"));
    }
    bail!(
        "Неизвестный формат субтитров: {}",
        extension.chars().take(16).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_auto_and_translated_remain_distinct() {
        let tracks = vec![
            Subtitle {
                language: "en".into(),
                url: "https://example.com/manual".into(),
                translatable: true,
                ..Default::default()
            },
            Subtitle {
                language: "en".into(),
                url: "https://example.com/auto".into(),
                automatic: true,
                ..Default::default()
            },
        ];
        assert!(select(&tracks, "en").unwrap().0.url.ends_with("manual"));
        assert!(select(&tracks, "en:auto").unwrap().0.url.ends_with("auto"));
        let translated = select(&tracks, "ru:translated:en").unwrap();
        assert!(translated.0.url.contains("tlang=ru"));
        assert_eq!(language_tag("ru-RU"), "rus");
        assert_eq!(language_tag("en"), "eng");
    }
    #[test]
    fn json_and_ttml_become_timed_webvtt() {
        let (json, _) =
            normalize(br#"{"body":[{"from":1,"to":2,"content":"Hello"}]}"#, "json").unwrap();
        assert!(
            String::from_utf8(json)
                .unwrap()
                .contains("00:00:01.000 --> 00:00:02.000\nHello")
        );
        let (xml, _) = normalize(
            br#"<tt><body><p begin="1s" dur="1s">Hello &amp; world<br/>next</p></body></tt>"#,
            "ttml",
        )
        .unwrap();
        let text = String::from_utf8(xml).unwrap();
        assert!(text.contains("Hello & world\nnext"));
        assert!(normalize(br#"<!DOCTYPE tt SYSTEM "file:///etc/passwd"><tt/>"#, "ttml").is_err());
    }
}
