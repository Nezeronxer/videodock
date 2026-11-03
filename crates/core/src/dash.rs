//! Static MPEG-DASH VOD expansion and HTTP segment transfer. No external downloader.
use crate::transfer::{Limiter, Progress};
use anyhow::{Context, Result, bail, ensure};
use reqwest::{Client, StatusCode};
use roxmltree::{Document, Node, ParsingOptions};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};
use tokio_util::sync::CancellationToken;
use url::Url;
use videodock_media::{Format, Media, Protocol, validate_url};

const MAX_MPD: usize = 8 * 1024 * 1024;
const MAX_SEGMENTS: usize = 100_000;
const MAX_URL_BYTES: usize = 32 * 1024 * 1024;
const MAX_SEGMENT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_OUTPUT_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const SELECTOR: &str = "videodock_rep";

#[derive(Clone, Debug, serde::Serialize)]
struct Segment {
    url: String,
    range: Option<(u64, u64)>,
}
struct Track {
    key: String,
    format: Format,
    segments: Vec<Segment>,
    period_lengths: Vec<usize>,
    period_durations: Vec<f64>,
    compatibility: String,
}

fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}
fn attr(chain: &[Node<'_, '_>], name: &str) -> Option<String> {
    chain
        .iter()
        .rev()
        .find_map(|n| n.attribute(name).map(str::to_owned))
}
fn uint(value: Option<String>, default: u64, what: &str) -> Result<u64> {
    value
        .map(|v| v.parse().with_context(|| format!("Неверное {what} в MPD")))
        .unwrap_or(Ok(default))
}
fn duration(value: &str) -> Result<f64> {
    ensure!(
        value.starts_with('P') && value.len() < 128 && !value.ends_with('T'),
        "Неверная длительность MPD"
    );
    let mut time = false;
    let mut number = String::new();
    let mut total = 0.0;
    let mut any = false;
    let mut previous = 0;
    for c in value[1..].chars() {
        if c == 'T' {
            ensure!(!time && number.is_empty(), "Неверная длительность MPD");
            time = true;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            number.push(c);
            continue;
        }
        let (order, multiplier) = match (time, c) {
            (false, 'D') => (1, 86400.0),
            (true, 'H') => (2, 3600.0),
            (true, 'M') => (3, 60.0),
            (true, 'S') => (4, 1.0),
            _ => bail!("Неподдерживаемая длительность MPD"),
        };
        ensure!(
            order > previous && !number.is_empty(),
            "Неверная длительность MPD"
        );
        previous = order;
        let n: f64 = number.parse().context("Неверная длительность MPD")?;
        total += n * multiplier;
        number.clear();
        any = true;
    }
    ensure!(
        any && number.is_empty() && total.is_finite() && (0.0..=366.0 * 86400.0).contains(&total),
        "Длительность MPD вне допустимых границ"
    );
    Ok(total)
}
fn ticks(seconds: f64, scale: u64) -> Result<u64> {
    let value = seconds * scale as f64;
    ensure!(
        value.is_finite() && value >= 0.0 && value < u64::MAX as f64,
        "Переполнение времени MPD"
    );
    Ok(value.ceil() as u64)
}
fn join(base: &Url, value: &str) -> Result<Url> {
    ensure!(
        !value.chars().any(char::is_control) && value.len() <= 16384,
        "Недопустимый URL MPD"
    );
    let joined = base
        .join(value.trim())
        .context("Неверный URL сегмента MPD")?;
    validate_url(joined.as_str())?;
    ensure!(
        joined.fragment().is_none(),
        "Fragment в URL сегмента MPD не поддерживается"
    );
    Ok(joined)
}
fn byte_range(value: &str) -> Result<(u64, u64)> {
    let (start, end) = value.split_once('-').context("Неверный byte range MPD")?;
    let start: u64 = start.parse()?;
    let end: u64 = end.parse()?;
    let len = end
        .checked_sub(start)
        .and_then(|n| n.checked_add(1))
        .context("Неверные границы byte range MPD")?;
    ensure!(
        len > 0 && len <= MAX_SEGMENT_BYTES,
        "Диапазон MPD превышает допустимый размер"
    );
    Ok((start, len))
}
fn substitute(template: &str, id: &str, bandwidth: u64, number: u64, time: u64) -> Result<String> {
    let mut result = String::new();
    let mut rest = template;
    while let Some(index) = rest.find('$') {
        result.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        if let Some(next) = rest.strip_prefix('$') {
            result.push('$');
            rest = next;
            continue;
        }
        let end = rest
            .find('$')
            .context("Незакрытая подстановка SegmentTemplate")?;
        let token = &rest[..end];
        rest = &rest[end + 1..];
        let (name, width) = if let Some((name, fmt)) = token.split_once('%') {
            ensure!(
                fmt.starts_with('0') && fmt.ends_with('d'),
                "Неподдерживаемый формат SegmentTemplate"
            );
            let width: usize = fmt[1..fmt.len() - 1]
                .parse()
                .context("Неверная ширина SegmentTemplate")?;
            ensure!(
                (1..=20).contains(&width),
                "Слишком широкая подстановка SegmentTemplate"
            );
            (name, Some(width))
        } else {
            (token, None)
        };
        if name == "RepresentationID" {
            ensure!(
                width.is_none(),
                "Форматирование RepresentationID не поддерживается"
            );
            result.push_str(id);
        } else {
            let value = match name {
                "Bandwidth" => bandwidth,
                "Number" => number,
                "Time" => time,
                _ => bail!("Неизвестная подстановка SegmentTemplate"),
            };
            if let Some(width) = width {
                result.push_str(&format!("{value:0width$}"));
            } else {
                result.push_str(&value.to_string());
            }
        }
        ensure!(result.len() <= 16384, "Слишком длинный URL SegmentTemplate");
    }
    result.push_str(rest);
    ensure!(result.len() <= 16384, "Слишком длинный URL SegmentTemplate");
    Ok(result)
}
fn push_segment(
    segments: &mut Vec<Segment>,
    budget: &mut usize,
    base: &Url,
    uri: &str,
    range: Option<(u64, u64)>,
) -> Result<()> {
    ensure!(
        segments.len() < MAX_SEGMENTS,
        "Слишком много DASH-сегментов"
    );
    let url = join(base, uri)?.to_string();
    *budget = budget
        .checked_add(url.len())
        .context("Слишком много URL MPD")?;
    ensure!(
        *budget <= MAX_URL_BYTES,
        "URL сегментов MPD превышают допустимый размер"
    );
    segments.push(Segment { url, range });
    Ok(())
}
fn addressing<'a, 'i>(chain: &[Node<'a, 'i>], name: &str) -> Vec<Node<'a, 'i>> {
    chain.iter().filter_map(|n| child(*n, name)).collect()
}
fn segments(
    chain: &[Node<'_, '_>],
    base: &Url,
    id: &str,
    bandwidth: u64,
    period: Option<f64>,
) -> Result<Vec<Segment>> {
    let templates = addressing(chain, "SegmentTemplate");
    let lists = addressing(chain, "SegmentList");
    let nearest = chain.iter().rev().find_map(|n| {
        n.children().find(|c| {
            c.is_element()
                && matches!(
                    c.tag_name().name(),
                    "SegmentTemplate" | "SegmentList" | "SegmentBase"
                )
        })
    });
    let mut output = Vec::new();
    let mut budget = 0usize;
    match nearest.map(|n| n.tag_name().name()) {
        Some("SegmentTemplate") => {
            let media = attr(&templates, "media").context("SegmentTemplate без media")?;
            let scale = uint(attr(&templates, "timescale"), 1, "timescale")?;
            ensure!(scale > 0, "Нулевой timescale MPD");
            let start = uint(attr(&templates, "startNumber"), 1, "startNumber")?;
            let offset = uint(
                attr(&templates, "presentationTimeOffset"),
                0,
                "presentationTimeOffset",
            )?;
            if let Some(init) = attr(&templates, "initialization") {
                push_segment(
                    &mut output,
                    &mut budget,
                    base,
                    &substitute(&init, id, bandwidth, start, 0)?,
                    None,
                )?;
            }
            let timeline = templates
                .iter()
                .rev()
                .find_map(|n| child(*n, "SegmentTimeline"));
            let mut values = Vec::new();
            if let Some(timeline) = timeline {
                let entries: Vec<_> = timeline
                    .children()
                    .filter(|n| n.has_tag_name("S"))
                    .collect();
                ensure!(!entries.is_empty(), "Пустой SegmentTimeline");
                let mut next = 0u64;
                for (index, entry) in entries.iter().enumerate() {
                    let time = uint(entry.attribute("t").map(str::to_owned), next, "timeline t")?;
                    ensure!(
                        index == 0 || time >= next,
                        "Перекрывающиеся временные сегменты MPD"
                    );
                    let d = uint(entry.attribute("d").map(str::to_owned), 0, "timeline d")?;
                    ensure!(d > 0, "Нулевая длительность сегмента MPD");
                    let repeat: i64 = entry
                        .attribute("r")
                        .unwrap_or("0")
                        .parse()
                        .context("Неверный repeat MPD")?;
                    ensure!(repeat >= -1, "Отрицательный repeat MPD");
                    let count = if repeat == -1 {
                        let next_t = entries
                            .get(index + 1)
                            .and_then(|n| n.attribute("t"))
                            .map(str::parse::<u64>)
                            .transpose()?;
                        ensure!(
                            entries.get(index + 1).is_none() || next_t.is_some(),
                            "После repeat=-1 следующий S должен задавать t"
                        );
                        let bound = if let Some(t) = next_t {
                            t
                        } else {
                            ticks(
                                period.context(
                                    "Неограниченный SegmentTimeline: нет длительности VOD",
                                )?,
                                scale,
                            )?
                            .checked_add(offset)
                            .context("Переполнение timeline")?
                        };
                        ensure!(bound > time, "Неверная граница repeat MPD");
                        if next_t.is_some() {
                            ensure!(
                                (bound - time) % d == 0,
                                "Повтор MPD перекрывает следующий S"
                            );
                        }
                        (bound - time).div_ceil(d)
                    } else {
                        (repeat as u64)
                            .checked_add(1)
                            .context("Repeat MPD переполнен")?
                    };
                    ensure!(
                        count <= MAX_SEGMENTS as u64
                            && values
                                .len()
                                .checked_add(count as usize)
                                .is_some_and(|n| n <= MAX_SEGMENTS),
                        "Слишком много DASH-сегментов"
                    );
                    for i in 0..count {
                        values.push(
                            time.checked_add(d.checked_mul(i).context("Переполнение timeline")?)
                                .context("Переполнение timeline")?,
                        );
                    }
                    next = time
                        .checked_add(d.checked_mul(count).context("Переполнение timeline")?)
                        .context("Переполнение timeline")?;
                }
            } else {
                let d = uint(attr(&templates, "duration"), 0, "SegmentTemplate duration")?;
                ensure!(d > 0, "SegmentTemplate без duration/timeline");
                let count = if let Some(end) = attr(&templates, "endNumber") {
                    let end: u64 = end.parse()?;
                    end.checked_sub(start)
                        .and_then(|n| n.checked_add(1))
                        .context("Неверный endNumber MPD")?
                } else {
                    ticks(period.context("Нет конечной длительности DASH VOD")?, scale)?.div_ceil(d)
                };
                ensure!(
                    count > 0 && count <= MAX_SEGMENTS as u64,
                    "Число сегментов DASH вне границ"
                );
                for i in 0..count {
                    values.push(d.checked_mul(i).context("Переполнение времени DASH")?);
                }
            }
            ensure!(!values.is_empty(), "Нет медиасегментов DASH");
            for (index, time) in values.into_iter().enumerate() {
                let number = start
                    .checked_add(index as u64)
                    .context("Переполнение номера сегмента")?;
                push_segment(
                    &mut output,
                    &mut budget,
                    base,
                    &substitute(&media, id, bandwidth, number, time)?,
                    None,
                )?;
            }
        }
        Some("SegmentList") => {
            if let Some(init) = lists.iter().rev().find_map(|n| child(*n, "Initialization")) {
                push_segment(
                    &mut output,
                    &mut budget,
                    base,
                    init.attribute("sourceURL").unwrap_or(""),
                    init.attribute("range").map(byte_range).transpose()?,
                )?;
            }
            let list = lists
                .iter()
                .rev()
                .find(|n| n.children().any(|c| c.has_tag_name("SegmentURL")))
                .context("Пустой SegmentList")?;
            for segment in list.children().filter(|n| n.has_tag_name("SegmentURL")) {
                push_segment(
                    &mut output,
                    &mut budget,
                    base,
                    segment.attribute("media").unwrap_or(""),
                    segment
                        .attribute("mediaRange")
                        .map(byte_range)
                        .transpose()?,
                )?;
            }
            ensure!(!output.is_empty(), "Нет медиасегментов DASH");
        }
        _ => {
            ensure!(
                !base.path().ends_with('/') && !base.path().to_ascii_lowercase().ends_with(".mpd"),
                "DASH representation без конечного media URL/сегментов"
            );
        }
    }
    Ok(output)
}
fn selector(url: &Url) -> Result<Option<String>> {
    let Some(fragment) = url.fragment() else {
        return Ok(None);
    };
    let pairs: Vec<_> = url::form_urlencoded::parse(fragment.as_bytes()).collect();
    ensure!(
        pairs.len() == 1 && pairs[0].0 == SELECTOR && pairs[0].1.len() <= 2048,
        "Неверный DASH representation selector"
    );
    Ok(Some(pairs[0].1.to_string()))
}
fn parse(xml: &str, manifest: &Url, source: &Format) -> Result<Vec<Track>> {
    ensure!(xml.len() <= MAX_MPD, "Слишком большой MPD");
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
        },
    )
    .context("Неверный XML MPD")?;
    let root = document.root_element();
    ensure!(root.has_tag_name("MPD"), "Ответ не является MPD");
    ensure!(
        root.attribute("type").unwrap_or("static") == "static",
        "Live/dynamic DASH пока не поддерживается: полнота сегментов неизвестна"
    );
    ensure!(
        !root
            .descendants()
            .any(|n| n.is_element() && n.tag_name().name() == "ContentProtection"),
        "DRM/ContentProtection DASH не поддерживается"
    );
    ensure!(
        !root.descendants().any(|n| n
            .attributes()
            .any(|a| a.namespace() == Some("http://www.w3.org/1999/xlink") && a.name() == "href")),
        "Внешние xlink MPD не поддерживаются"
    );
    ensure!(
        root.descendants()
            .all(|n| n.ancestors().take(65).count() <= 64),
        "Слишком глубокий XML MPD"
    );
    let periods: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("Period"))
        .collect();
    ensure!(
        !periods.is_empty() && periods.len() <= 128,
        "Число периодов MPD вне границ"
    );
    let total = root
        .attribute("mediaPresentationDuration")
        .map(duration)
        .transpose()?;
    let mut starts = periods
        .iter()
        .map(|p| p.attribute("start").map(duration).transpose())
        .collect::<Result<Vec<_>>>()?;
    let mut durations = periods
        .iter()
        .map(|p| p.attribute("duration").map(duration).transpose())
        .collect::<Result<Vec<_>>>()?;
    if starts[0].is_none() {
        starts[0] = Some(0.0);
    }
    for _ in 0..periods.len() + 1 {
        let mut changed = false;
        for index in 0..periods.len() {
            let end = if index + 1 < periods.len() {
                starts[index + 1]
            } else {
                total
            };
            match (starts[index], durations[index], end) {
                (Some(start), None, Some(end)) => {
                    durations[index] = Some(end - start);
                    changed = true;
                }
                (None, Some(length), Some(end)) => {
                    starts[index] = Some(end - length);
                    changed = true;
                }
                (Some(start), Some(length), None) if index + 1 < periods.len() => {
                    starts[index + 1] = Some(start + length);
                    changed = true;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }
    let multiple = periods.len() > 1;
    let mut cursor = 0.0;
    let mut all = Vec::new();
    let mut count = 0usize;
    let mut bytes = 0usize;
    for (index, period) in periods.into_iter().enumerate() {
        ensure!(
            durations[index].is_none_or(|d| d.is_finite() && d > 0.0),
            "Неверная длительность периода MPD"
        );
        if multiple {
            let start = starts[index].context("Неизвестная временная шкала multi-period MPD")?;
            let length = durations[index].context("Неизвестная длительность multi-period MPD")?;
            ensure!(
                (start - cursor).abs() < 0.000001,
                "Пробелы/перекрытия периодов DASH не поддерживаются"
            );
            cursor = start + length;
        }
        let mut tracks = parse_period(root, period, durations[index], manifest, source)?;
        if multiple {
            for track in &mut tracks {
                ensure!(
                    [
                        track.format.video_codec.as_deref(),
                        track.format.audio_codec.as_deref()
                    ]
                    .into_iter()
                    .flatten()
                    .all(|c| c != "unknown" && c != "none"),
                    "Неизвестный codec multi-period DASH"
                );
                if track.segments.is_empty() {
                    track.segments.push(Segment {
                        url: track.format.url.clone(),
                        range: None,
                    });
                    track.period_lengths[0] = 1;
                }
            }
        }
        for track in &tracks {
            count = count
                .checked_add(track.segments.len())
                .context("Слишком много DASH-сегментов")?;
            bytes = bytes
                .checked_add(track.segments.iter().map(|s| s.url.len()).sum::<usize>())
                .context("Слишком много URL MPD")?;
        }
        ensure!(
            count <= MAX_SEGMENTS && bytes <= MAX_URL_BYTES,
            "Multi-period MPD превышает предел сегментов/URL"
        );
        all.push(tracks);
    }
    if multiple {
        ensure!(
            total.is_none_or(|total| (cursor - total).abs() < 0.000001),
            "Длительность периодов не совпадает с полной длительностью MPD"
        );
    }
    let mut grouped = all.remove(0);
    for mut period in all {
        ensure!(
            period.len() == grouped.len(),
            "Набор аудио/видеодорожек изменяется между периодами DASH"
        );
        for track in &mut grouped {
            let id = track.key.split_once(':').map(|v| v.1).unwrap_or("");
            let compatible: Vec<_> = period
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.compatibility == track.compatibility)
                .map(|(i, _)| i)
                .collect();
            let exact: Vec<_> = compatible
                .iter()
                .copied()
                .filter(|i| period[*i].key.split_once(':').map(|v| v.1) == Some(id))
                .collect();
            let index = if exact.len() == 1 {
                exact[0]
            } else {
                ensure!(
                    compatible.len() == 1,
                    "Несовместимые или неоднозначные codec/resolution/language между периодами DASH"
                );
                compatible[0]
            };
            let mut next = period.remove(index);
            track.period_lengths.append(&mut next.period_lengths);
            track.period_durations.append(&mut next.period_durations);
            track.segments.append(&mut next.segments);
        }
        ensure!(period.is_empty(), "Не все дорожки DASH сопоставлены");
    }
    let requested = selector(&validate_url(&source.url)?)?;
    if multiple {
        for track in &mut grouped {
            track.format.protocol = Protocol::Dash;
            let mut url = manifest.clone();
            url.set_fragment(Some(
                &url::form_urlencoded::Serializer::new(String::new())
                    .append_pair(SELECTOR, &track.key)
                    .finish(),
            ));
            track.format.url = url.to_string();
        }
    }
    if let Some(wanted) = requested {
        grouped.retain(|track| track.key == wanted);
        for track in &mut grouped {
            track.format.id = source.id.clone();
        }
    }
    for track in &mut grouped {
        track.format.headers =
            videodock_media::scoped_headers(&source.headers, &source.url, &track.format.url)?;
    }
    ensure!(
        !grouped.is_empty(),
        "Выбранное представление DASH изменилось"
    );
    Ok(grouped)
}
fn parse_period(
    root: Node<'_, '_>,
    period: Node<'_, '_>,
    period_duration: Option<f64>,
    manifest: &Url,
    source: &Format,
) -> Result<Vec<Track>> {
    let requested: Option<String> = None;
    let mut output = Vec::new();
    let mut count = 0usize;
    let mut url_bytes = 0usize;
    for (a, adaptation) in period
        .children()
        .filter(|n| n.has_tag_name("AdaptationSet"))
        .enumerate()
    {
        for (r, representation) in adaptation
            .children()
            .filter(|n| n.has_tag_name("Representation"))
            .enumerate()
        {
            let id = representation.attribute("id").unwrap_or("");
            ensure!(id.len() <= 1024, "Слишком длинный id representation");
            let key = format!("0.{a}.{r}:{id}");
            if requested.as_ref().is_some_and(|wanted| wanted != &key) {
                continue;
            }
            let chain = [root, period, adaptation, representation];
            let mut base = manifest.clone();
            for node in chain {
                if let Some(value) =
                    node.attribute(("http://www.w3.org/XML/1998/namespace", "base"))
                {
                    base = join(&base, value)?;
                }
                if let Some(value) = child(node, "BaseURL") {
                    base = join(&base, value.text().context("Пустой BaseURL MPD")?)?;
                }
            }
            let mime = attr(&chain, "mimeType").unwrap_or_default();
            let content = attr(&chain, "contentType").unwrap_or_default();
            if mime.starts_with("text/")
                || content == "text"
                || content == "image"
                || mime.starts_with("image/")
            {
                continue;
            }
            let bandwidth = uint(attr(&chain, "bandwidth"), 0, "bandwidth")?;
            let plan = segments(&chain, &base, id, bandwidth, period_duration)?;
            count = count
                .checked_add(plan.len())
                .context("Слишком много сегментов")?;
            url_bytes = url_bytes
                .checked_add(plan.iter().map(|s| s.url.len()).sum::<usize>())
                .context("Слишком много URL MPD")?;
            ensure!(
                count <= MAX_SEGMENTS && url_bytes <= MAX_URL_BYTES && output.len() < 2048,
                "MPD превышает пределы representations/сегментов"
            );
            let codecs = attr(&chain, "codecs").unwrap_or_default();
            let codecs: Vec<_> = codecs
                .split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .collect();
            let audio = codecs
                .iter()
                .find(|c| {
                    ["mp4a", "opus", "vorbis", "ac-3", "ec-3", "flac", "aac"]
                        .iter()
                        .any(|p| c.starts_with(p))
                })
                .map(|c| c.to_string());
            let video = codecs
                .iter()
                .find(|c| {
                    [
                        "avc", "hvc", "hev", "vp8", "vp9", "vp0", "av01", "dvhe", "dvh1",
                    ]
                    .iter()
                    .any(|p| c.starts_with(p))
                })
                .map(|c| c.to_string());
            let is_audio = content == "audio"
                || mime.starts_with("audio/")
                || (audio.is_some() && video.is_none() && !mime.starts_with("video/"));
            let mut format = source.clone();
            if requested.is_none() {
                format.id = format!("{}-dash-{key}", source.id);
            }
            format.video_codec = if is_audio {
                None
            } else {
                video.or_else(|| Some("unknown".into()))
            };
            format.audio_codec = audio.or_else(|| is_audio.then(|| "unknown".into()));
            format.audio_language = attr(&chain, "lang").or_else(|| {
                chain.iter().rev().find_map(|n| {
                    n.attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
                        .map(str::to_owned)
                })
            });
            format.audio_title = chain
                .iter()
                .rev()
                .find_map(|n| child(*n, "Label").and_then(|l| l.text()).map(str::to_owned));
            format.bitrate = (bandwidth > 0).then_some(bandwidth);
            format.size = None;
            format.width = attr(&chain, "width").map(|v| v.parse()).transpose()?;
            format.height = attr(&chain, "height").map(|v| v.parse()).transpose()?;
            ensure!(
                [format.width, format.height]
                    .iter()
                    .all(|v| v.is_none_or(|n| (1..=32768).contains(&n))),
                "Размеры кадра MPD вне границ"
            );
            format.fps = attr(&chain, "frameRate")
                .map(|v| {
                    let (n, d) = v.split_once('/').unwrap_or((&v, "1"));
                    let n: f64 = n.parse()?;
                    let d: f64 = d.parse()?;
                    ensure!(
                        n.is_finite() && d.is_finite() && n > 0.0 && d > 0.0 && n / d <= 1000.0,
                        "Неверный frameRate MPD"
                    );
                    Ok::<_, anyhow::Error>(n / d)
                })
                .transpose()?;
            format.ext = if mime.contains("webm") {
                "webm"
            } else if is_audio {
                "m4a"
            } else {
                "mp4"
            }
            .into();
            if plan.is_empty() {
                format.protocol = Protocol::Http;
                format.url = base.to_string();
            } else {
                format.protocol = Protocol::Dash;
                let mut url = manifest.clone();
                url.set_fragment(Some(
                    &url::form_urlencoded::Serializer::new(String::new())
                        .append_pair(SELECTOR, &key)
                        .finish(),
                ));
                format.url = url.to_string();
            }
            let channels = chain
                .iter()
                .rev()
                .find_map(|n| child(*n, "AudioChannelConfiguration"))
                .map(|n| {
                    (
                        n.attribute("schemeIdUri").unwrap_or("").to_owned(),
                        n.attribute("value").unwrap_or("").to_owned(),
                    )
                });
            let compatibility = format!(
                "{:?}",
                (
                    &format.ext,
                    &format.video_codec,
                    &format.audio_codec,
                    &format.audio_language,
                    format.width,
                    format.height,
                    format.fps,
                    attr(&chain, "audioSamplingRate"),
                    channels
                )
            );
            output.push(Track {
                key,
                format,
                period_lengths: vec![plan.len()],
                period_durations: vec![period_duration.unwrap_or(0.0)],
                compatibility,
                segments: plan,
            });
        }
    }
    ensure!(
        !output.is_empty(),
        "В MPD нет выбранных аудио/видеопредставлений"
    );
    Ok(output)
}
async fn manifest(
    client: &Client,
    url: &Url,
    headers: &BTreeMap<String, String>,
    cookies: &[videodock_media::ScopedCookie],
    cancel: &CancellationToken,
) -> Result<(Url, String)> {
    let mut clean = url.clone();
    clean.set_fragment(None);
    validate_url(clean.as_str())?;
    let mut response = crate::transfer::get_response_with_cookies(
        client,
        clean.as_str(),
        headers,
        cookies,
        cancel,
    )
    .await?;
    ensure!(
        response.status().is_success(),
        "Сервер MPD вернул HTTP {}",
        response.status().as_u16()
    );
    let final_url = validate_url(response.url().as_str())?;
    let mut bytes = Vec::new();
    ensure!(
        response
            .content_length()
            .is_none_or(|n| n <= MAX_MPD as u64),
        "Слишком большой MPD"
    );
    loop {
        let chunk = tokio::select! {biased;_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),r=response.chunk()=>r.map_err(|_|anyhow::anyhow!("Поток MPD прерван"))?};
        let Some(chunk) = chunk else { break };
        ensure!(bytes.len() + chunk.len() <= MAX_MPD, "Слишком большой MPD");
        bytes.extend_from_slice(&chunk);
    }
    Ok((
        final_url,
        String::from_utf8(bytes).context("MPD должен содержать UTF-8 XML")?,
    ))
}
/// Atomically replaces manifest placeholders only after every MPD has been validated.
pub async fn expand(client: &Client, media: &mut Media, cancel: &CancellationToken) -> Result<()> {
    let mut formats = Vec::new();
    for original in &media.formats {
        if original.protocol != Protocol::Dash {
            formats.push(original.clone());
            continue;
        }
        let mut source = original.clone();
        if source.cookies.is_empty() {
            source.cookies = media.cookies.clone();
        }
        let url = validate_url(&source.url)?;
        let (base, xml) = manifest(client, &url, &source.headers, &source.cookies, cancel).await?;
        formats.extend(
            parse(&xml, &base, &source)?
                .into_iter()
                .map(|track| track.format),
        );
    }
    media.formats = formats;
    Ok(())
}
fn safe_existing(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => ensure!(
            m.is_file() && !m.file_type().is_symlink(),
            "Недопустимый staging-файл DASH"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    };
    Ok(())
}
fn range_header(value: &str) -> Result<(u64, u64, u64)> {
    let (span, total) = value
        .strip_prefix("bytes ")
        .and_then(|v| v.split_once('/'))
        .context("Неверный Content-Range DASH")?;
    let (start, end) = span
        .split_once('-')
        .context("Неверный Content-Range DASH")?;
    let (start, end, total): (u64, u64, u64) = (start.parse()?, end.parse()?, total.parse()?);
    ensure!(start <= end && end < total, "Неверный Content-Range DASH");
    Ok((start, end, total))
}
async fn small_text(path: &Path, max: u64) -> Option<String> {
    if tokio::fs::metadata(path).await.ok()?.len() > max {
        return None;
    }
    tokio::fs::read_to_string(path).await.ok()
}
fn response_validator(response: &reqwest::Response) -> Option<&str> {
    response
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.starts_with("W/"))
        .or_else(|| {
            response
                .headers()
                .get("Last-Modified")
                .and_then(|v| v.to_str().ok())
        })
        .filter(|v| !v.is_empty() && v.len() <= 4096 && !v.chars().any(char::is_control))
}
async fn part(
    client: &Client,
    segment: &Segment,
    format: &Format,
    path: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<u64> {
    let headers = videodock_media::scoped_headers(&format.headers, &format.url, &segment.url)?;
    safe_existing(path)?;
    let validator_path = path.with_extension("validator");
    safe_existing(&validator_path)?;
    let validator = small_text(&validator_path, 4096)
        .await
        .filter(|v| !v.is_empty() && !v.chars().any(char::is_control));
    let local = tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let mut offset =
        if validator.is_some() && local < segment.range.map(|r| r.1).unwrap_or(MAX_SEGMENT_BYTES) {
            local
        } else {
            0
        };
    let mut request_headers = BTreeMap::new();
    for (name, value) in &headers {
        if !["range", "if-range", "content-length", "accept-encoding"]
            .iter()
            .any(|v| name.eq_ignore_ascii_case(v))
        {
            request_headers.insert(name.clone(), value.clone());
        }
    }
    request_headers.insert("Accept-Encoding".into(), "identity".into());
    let requested = if let Some((start, len)) = segment.range {
        let first = start.checked_add(offset).context("Переполнение range")?;
        let end = start.checked_add(len - 1).context("Переполнение range")?;
        request_headers.insert("Range".into(), format!("bytes={first}-{end}"));
        Some((first, Some(end)))
    } else if offset > 0 {
        request_headers.insert("Range".into(), format!("bytes={offset}-"));
        Some((offset, None))
    } else {
        None
    };
    if offset > 0 {
        request_headers.insert("If-Range".into(), validator.as_ref().unwrap().clone());
    }
    let mut response = crate::transfer::get_response_with_cookies(
        client,
        &segment.url,
        &request_headers,
        &format.cookies,
        cancel,
    )
    .await?;
    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE
        && segment.range.is_none()
        && offset > 0
    {
        let remote = response
            .headers()
            .get("Content-Range")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("bytes */"))
            .and_then(|v| v.parse::<u64>().ok());
        ensure!(
            remote == Some(offset) && response_validator(&response) == validator.as_deref(),
            "Сервер не подтвердил полный частичный DASH-файл"
        );
        return Ok(offset);
    }
    let mut total = response.content_length();
    if offset > 0
        && (response.status() == StatusCode::PARTIAL_CONTENT
            && response_validator(&response) != validator.as_deref()
            || response.status() == StatusCode::OK && segment.range.is_some())
    {
        // A changed validator invalidates partial resume; the next retry starts at the range origin.
        tokio::fs::remove_file(&validator_path).await.ok();
        bail!("DASH-сегмент изменился; повторите загрузку с начала сегмента");
    }
    if let Some((start, end)) = requested {
        if response.status() == StatusCode::OK && segment.range.is_none() {
            offset = 0;
        } else {
            ensure!(
                response.status() == StatusCode::PARTIAL_CONTENT,
                "Сервер не подтвердил диапазон DASH"
            );
            if offset > 0 {
                ensure!(
                    response_validator(&response) == validator.as_deref(),
                    "DASH-сегмент изменился во время возобновления"
                );
            }
            let value = response
                .headers()
                .get("Content-Range")
                .and_then(|h| h.to_str().ok())
                .context("Ответ DASH без Content-Range")?;
            let (actual_start, actual_end, remote_total) = range_header(value)?;
            ensure!(
                actual_start == start && end.is_none_or(|end| actual_end == end),
                "Сервер вернул другой диапазон DASH"
            );
            ensure!(
                response
                    .content_length()
                    .is_none_or(|n| n == actual_end - actual_start + 1),
                "Неверная длина диапазона DASH"
            );
            total = Some(segment.range.map(|r| r.1).unwrap_or(remote_total));
        }
    } else {
        ensure!(
            response.status() == StatusCode::OK,
            "Сервер DASH вернул HTTP {}",
            response.status().as_u16()
        );
    }
    ensure!(
        total.is_none_or(|n| n > 0 && n <= MAX_SEGMENT_BYTES),
        "Размер DASH-сегмента вне границ"
    );
    let new_validator = response_validator(&response);
    if let Some(value) = new_validator {
        tokio::fs::write(&validator_path, value).await?;
    } else if validator_path.exists() {
        tokio::fs::remove_file(&validator_path).await?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(offset > 0)
        .truncate(offset == 0)
        .open(path)
        .await?;
    let original = offset;
    let started = Instant::now();
    loop {
        let chunk = tokio::select! {biased;_ = cancel.cancelled()=>{file.flush().await?;bail!("Загрузка приостановлена")},r=response.chunk()=>match r{Ok(chunk)=>chunk,Err(_)=>{file.flush().await?;bail!("DASH-сегмент прерван")}}};
        let Some(chunk) = chunk else { break };
        ensure!(
            offset
                .checked_add(chunk.len() as u64)
                .is_some_and(|n| n <= MAX_SEGMENT_BYTES && total.is_none_or(|t| n <= t)),
            "DASH-сегмент превышает заявленный размер"
        );
        limiter.wait(chunk.len() as u64, cancel).await?;
        file.write_all(&chunk).await?;
        offset += chunk.len() as u64;
        progress(Progress {
            downloaded: offset,
            total,
            speed: (offset - original) as f64 / started.elapsed().as_secs_f64().max(0.001),
        });
    }
    file.flush().await?;
    ensure!(
        offset > 0 && total.is_none_or(|n| n == offset),
        "Неполный DASH-сегмент"
    );
    Ok(offset)
}
/// Returns one local media track. Separate audio/video are muxed by the caller.
pub async fn download_dash(
    client: &Client,
    format: &Format,
    folder: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    on_progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<PathBuf> {
    ensure!(format.protocol == Protocol::Dash, "Формат не является DASH");
    let url = validate_url(&format.url)?;
    let wanted = selector(&url)?.context("MPD необходимо раскрыть до загрузки")?;
    let (base, xml) = manifest(client, &url, &format.headers, &format.cookies, cancel).await?;
    let track = parse(&xml, &base, format)?
        .into_iter()
        .find(|t| t.key == wanted)
        .context("Выбранное представление DASH изменилось")?;
    ensure!(
        !track.segments.is_empty(),
        "Представление стало progressive HTTP; обновите сведения"
    );
    ensure!(
        format.height.is_none_or(|v| track.format.height == Some(v))
            && format
                .bitrate
                .is_none_or(|v| track.format.bitrate == Some(v)),
        "Параметры DASH representation изменились; обновите сведения"
    );
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            &track.segments,
            &track.period_lengths,
            &track.period_durations
        ))?)
    );
    tokio::fs::create_dir_all(folder).await?;
    let staging = folder.join(format!("dash-{fingerprint}"));
    if let Ok(meta) = std::fs::symlink_metadata(&staging) {
        ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Недопустимый staging-каталог DASH"
        );
    }
    tokio::fs::create_dir_all(&staging).await?;
    let lock_path = staging.join("transfer.lock");
    safe_existing(&lock_path)?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    lock.try_lock()
        .map_err(|_| anyhow::anyhow!("Эта DASH-дорожка уже загружается"))?;
    let mut completed = 0u64;
    let mut paths = Vec::new();
    let total = track
        .segments
        .iter()
        .try_fold(0u64, |sum, s| s.range.and_then(|r| sum.checked_add(r.1)));
    for (index, segment) in track.segments.iter().enumerate() {
        ensure!(!cancel.is_cancelled(), "Загрузка приостановлена");
        let done = staging.join(format!("{index:06}.segment"));
        let partial = staging.join(format!("{index:06}.part"));
        let receipt = staging.join(format!("{index:06}.size"));
        safe_existing(&done)?;
        safe_existing(&receipt)?;
        let saved = small_text(&receipt, 32)
            .await
            .and_then(|v| v.parse::<u64>().ok());
        let len = tokio::fs::metadata(&done).await.ok().map(|m| m.len());
        let size = if saved.is_some()
            && saved == len
            && len.is_some_and(|n| {
                n > 0 && n <= MAX_SEGMENT_BYTES && segment.range.is_none_or(|r| n == r.1)
            }) {
            len.unwrap()
        } else {
            let callback = on_progress.clone();
            let offset = completed;
            let size = part(
                client,
                segment,
                &track.format,
                &partial,
                cancel,
                limiter,
                Arc::new(move |p| {
                    callback(Progress {
                        downloaded: offset + p.downloaded,
                        total,
                        speed: p.speed,
                    })
                }),
            )
            .await?;
            tokio::fs::rename(&partial, &done).await?;
            tokio::fs::write(&receipt, size.to_string()).await?;
            size
        };
        completed = completed
            .checked_add(size)
            .context("Переполнение размера DASH")?;
        ensure!(
            completed <= MAX_OUTPUT_BYTES,
            "DASH превышает допустимый размер"
        );
        paths.push(done);
        on_progress(Progress {
            downloaded: completed,
            total,
            speed: 0.0,
        });
    }
    ensure!(!paths.is_empty(), "Пустая DASH-дорожка");
    let ext = if track.format.ext == "webm" {
        "webm"
    } else {
        "mp4"
    };
    let multiple = track.period_lengths.len() > 1;
    let mut position = 0usize;
    let mut concat = String::from("ffconcat version 1.0\n");
    let mut result = None;
    for (period, length) in track.period_lengths.iter().copied().enumerate() {
        ensure!(
            length > 0 && position + length <= paths.len(),
            "Неверные границы локального периода DASH"
        );
        let filename = if multiple {
            format!("period-{period:04}.{ext}")
        } else {
            format!("assembled.{ext}")
        };
        let output = staging.join(&filename);
        let temporary = staging.join(format!("period-{period:04}.part"));
        safe_existing(&temporary)?;
        safe_existing(&output)?;
        let mut file = File::create(&temporary).await?;
        for path in &paths[position..position + length] {
            let mut input = File::open(path).await?;
            tokio::select! {biased;_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),r=tokio::io::copy(&mut input,&mut file)=>{r?;}};
        }
        file.flush().await?;
        drop(file);
        tokio::fs::rename(&temporary, &output).await?;
        position += length;
        if multiple {
            concat.push_str(&format!(
                "file '{filename}'\nduration {:.9}\n",
                track.period_durations[period]
            ));
        } else {
            result = Some(output);
        }
    }
    ensure!(
        position == paths.len(),
        "Остались несопоставленные сегменты DASH"
    );
    if multiple {
        let output = staging.join("assembled.ffconcat");
        safe_existing(&output)?;
        let temporary = staging.join("concat.part");
        safe_existing(&temporary)?;
        tokio::fs::write(&temporary, concat).await?;
        tokio::fs::rename(&temporary, &output).await?;
        result = Some(output);
    }
    result.context("Пустой локальный результат DASH")
}
