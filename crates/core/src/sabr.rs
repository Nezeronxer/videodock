//! Native single-track SABR/UMP transport. Protocol definitions follow
//! LuanRT/googlevideo and mthwJsmith/sabr-rs (MIT; licenses in sabr/).
mod proto;
mod track;
mod ump;
use crate::transfer::{Limiter, Progress};
use anyhow::{Result, bail, ensure};
use prost::Message;
use proto::*;
use std::{
    collections::BTreeMap,
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
};
use tokio_util::sync::CancellationToken;
use track::Track;
use videodock_media::sabr::{Delivery, validate_url};

const META_LIMIT: usize = 64 * 1024;
const CONTEXT_LIMIT: usize = 64;
const REDIRECT_LIMIT: u8 = 8;
const ATTESTATION_WAIT: Duration = Duration::from_secs(30);
#[derive(Debug)]
pub(crate) struct PendingAttestation;
impl std::fmt::Display for PendingAttestation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YouTube SABR не подтвердил проверку Google вовремя")
    }
}
impl std::error::Error for PendingAttestation {}
#[derive(Debug)]
pub(crate) struct Retryable(pub(crate) &'static str);
impl std::fmt::Display for Retryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Retryable {}
fn server_error(error: SabrError) -> anyhow::Error {
    // A fresh, identical request can succeed after this backend format refusal.
    if error.code == Some(2) && error.r#type.as_deref() == Some("sabr.no_available_formats") {
        return Retryable("YouTube временно не предоставил медиапоток; повторите загрузку").into();
    }
    anyhow::anyhow!("YouTube сообщил об ошибке SABR")
}
fn decode<T: Message + Default>(data: &[u8]) -> Result<T> {
    ensure!(
        data.len() <= META_LIMIT,
        "Слишком большой служебный кадр SABR"
    );
    T::decode(data).map_err(|_| anyhow::anyhow!("Некорректный служебный кадр SABR"))
}

struct Context {
    value: Vec<u8>,
    send: bool,
}
struct State {
    url: url::Url,
    number: u32,
    redirects: u8,
    cookie: Option<Vec<u8>>,
    contexts: BTreeMap<i32, Context>,
    backoff: u64,
    attested: bool,
    last_protection: i32,
    stalls: u8,
    pending_since: Option<Instant>,
}
impl State {
    fn new(delivery: &Delivery) -> Result<Self> {
        Ok(Self {
            url: validate_url(&delivery.session.url)?,
            number: 0,
            redirects: 0,
            cookie: None,
            contexts: BTreeMap::new(),
            backoff: 0,
            attested: false,
            last_protection: 0,
            stalls: 0,
            pending_since: None,
        })
    }
    fn redirect(&mut self, value: &str) -> Result<()> {
        let url = validate_url(value)?;
        ensure!(
            self.redirects < REDIRECT_LIMIT,
            "Слишком много переадресаций SABR"
        );
        self.redirects += 1;
        self.url = url;
        Ok(())
    }
    fn fallback(&mut self) -> Result<bool> {
        let params: BTreeMap<_, _> = self
            .url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let Some(fvip) = params
            .get("fvip")
            .filter(|v| !v.is_empty() && v.len() <= 3 && v.bytes().all(|b| b.is_ascii_digit()))
        else {
            return Ok(false);
        };
        let count = match params.get("fallback_count") {
            Some(value) => match value.parse::<usize>() {
                Ok(n) if n < usize::from(REDIRECT_LIMIT) => n,
                _ => return Ok(false),
            },
            None => 0,
        };
        let Some(node) = params.get("mn").and_then(|v| v.split(',').nth(count + 1)) else {
            return Ok(false);
        };
        if !node.starts_with("sn-")
            || node.len() > 96
            || !node
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Ok(false);
        }
        let prefix = if self.url.host_str().is_some_and(|h| h.starts_with("rr")) {
            "rr"
        } else {
            "r"
        };
        let host = format!("{prefix}{fvip}---{node}.googlevideo.com");
        if self.url.host_str() == Some(&host) {
            return Ok(false);
        }
        let mut next = self.url.clone();
        next.set_host(Some(&host))
            .map_err(|_| anyhow::anyhow!("Некорректный резервный узел YouTube"))?;
        next.query_pairs_mut()
            .clear()
            .extend_pairs(
                params
                    .iter()
                    .filter(|(k, _)| k.as_str() != "fallback_count"),
            )
            .append_pair("fallback_count", &(count + 1).to_string());
        self.redirect(next.as_str())?;
        Ok(true)
    }
    fn progress(&mut self, before: (u64, u64, bool), after: (u64, u64, bool)) -> Result<()> {
        // A paired response may advance only companion audio or finish attestation.
        if after != before {
            self.stalls = 0;
            self.pending_since = None;
        } else {
            self.stalls = self.stalls.saturating_add(1);
            if self.last_protection == 2 && !self.attested {
                let since = self.pending_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= ATTESTATION_WAIT {
                    return Err(PendingAttestation.into());
                }
                self.backoff = self.backoff.max(250);
                return Ok(());
            }
        }
        ensure!(
            self.stalls < 3,
            "YouTube SABR не передаёт новые сегменты; повторите проверку Google"
        );
        Ok(())
    }
    fn finished(&self, track: &Track) -> Result<bool> {
        if !track.covered() {
            return Ok(false);
        }
        ensure!(
            track.complete(),
            "Размер SABR-потока не совпадает с метаданными"
        );
        // Complete media still needs a positive verification; pending polling has a time limit.
        Ok(self.attested)
    }
    fn body(
        &self,
        delivery: &Delivery,
        track: &Track,
        video: bool,
        audio: &Delivery,
        companion: Option<&Track>,
    ) -> Result<Vec<u8>> {
        let format = track.format.clone();
        let audio_format = if video {
            FormatId {
                itag: Some(
                    i32::try_from(audio.itag)
                        .map_err(|_| anyhow::anyhow!("Неверный аудиоформат SABR"))?,
                ),
                last_modified: Some(audio.last_modified),
                xtags: audio.xtags.clone(),
            }
        } else {
            format.clone()
        };
        let mut media_capabilities = capabilities(delivery, video);
        media_capabilities.audio = capabilities(audio, false).audio;
        let mut ranges = Vec::new();
        for track in [Some(track), companion]
            .into_iter()
            .flatten()
            .filter(|t| t.last_sequence > 0)
        {
            ranges.push(BufferedRange {
                format_id: track.format.clone(),
                start_time_ms: 0,
                duration_ms: track.end_ms,
                start_segment_index: 1,
                end_segment_index: track.last_sequence as i32,
                time_range: Some(TimeRange {
                    start_ticks: Some(0),
                    duration_ticks: Some(track.end_ms),
                    timescale: Some(1000),
                }),
            });
        }
        Ok(PlaybackRequest {
            client_abr_state: Some(AbrState {
                media_capabilities: Some(media_capabilities),
                bandwidth_estimate: Some(100_000_000),
                player_time_ms: Some(track.end_ms),
                visibility: Some(2),
                playback_rate: Some(1.0),
                elapsed_wall_time_ms: Some(track.end_ms),
                // SABR video requests require a companion audio format even when we save video separately.
                enabled_track_types_bitfield: Some(if video { 0 } else { 1 }),
                audio_track_id: audio.audio_track_id.clone(),
                drc_enabled: Some(audio.drc_enabled),
                enable_voice_boost: Some(audio.voice_boost),
            }),
            initialization_format_ids: [Some(track), companion]
                .into_iter()
                .flatten()
                .filter(|t| t.initialized())
                .map(|t| t.format.clone())
                .collect(),
            buffered_ranges: ranges,
            video_playback_ustreamer_config: Some(delivery.session.config.clone()),
            selected_audio_format_ids: vec![audio_format],
            selected_video_format_ids: if video { vec![format] } else { vec![] },
            streamer_context: Some(StreamerContext {
                client_info: Some(ClientInfo {
                    client_name: Some(1),
                    client_version: Some(delivery.session.client_version.clone()),
                }),
                po_token: Some(delivery.session.content_po.clone()),
                playback_cookie: self.cookie.clone(),
                sabr_contexts: self
                    .contexts
                    .iter()
                    .filter(|(_, c)| c.send)
                    .map(|(k, c)| SabrContext {
                        r#type: Some(*k),
                        value: Some(c.value.clone()),
                    })
                    .collect(),
                unsent_sabr_contexts: self
                    .contexts
                    .iter()
                    .filter_map(|(k, c)| (!c.send).then_some(*k))
                    .collect(),
            }),
        }
        .encode_to_vec())
    }
    fn update(&mut self, update: ContextUpdate) -> Result<()> {
        let key = update
            .r#type
            .ok_or_else(|| anyhow::anyhow!("Нет типа контекста SABR"))?;
        ensure!(key >= 0, "Неверный тип контекста SABR");
        if update.write_policy == Some(2) && self.contexts.contains_key(&key) {
            return Ok(());
        }
        ensure!(
            update.write_policy.is_none_or(|v| (0..=2).contains(&v)),
            "Неизвестная политика контекста SABR"
        );
        let value = update.value.unwrap_or_default();
        ensure!(value.len() <= 16 * 1024, "Контекст SABR слишком велик");
        ensure!(
            self.contexts.contains_key(&key) || self.contexts.len() < CONTEXT_LIMIT,
            "Слишком много контекстов SABR"
        );
        let total = self
            .contexts
            .iter()
            .filter(|(k, _)| **k != key)
            .map(|(_, v)| v.value.len())
            .sum::<usize>();
        ensure!(
            total + value.len() <= META_LIMIT,
            "Контексты SABR превышают предел"
        );
        self.contexts.insert(
            key,
            Context {
                value,
                send: update.send_by_default.unwrap_or(false),
            },
        );
        Ok(())
    }
    fn policy(&mut self, p: ContextPolicy) -> Result<()> {
        ensure!(
            p.start_policy.len() <= CONTEXT_LIMIT
                && p.stop_policy.len() <= CONTEXT_LIMIT
                && p.discard_policy.len() <= CONTEXT_LIMIT,
            "Слишком большая политика SABR"
        );
        for key in p.start_policy {
            if let Some(c) = self.contexts.get_mut(&key) {
                c.send = true;
            }
        }
        for key in p.stop_policy {
            if let Some(c) = self.contexts.get_mut(&key) {
                c.send = false;
            }
        }
        for key in p.discard_policy {
            self.contexts.remove(&key);
        }
        Ok(())
    }
}

// Spatial capability values match googlevideo/mediaCapabilityUtils, not channel-count guesses.
fn audio_spatial(itag: u32, channels: u32) -> i32 {
    match itag {
        327 | 338 => 4,
        258 | 261 | 380 | 381 | 328 | 329 => 1,
        _ if channels > 2 => 1,
        _ => 0,
    }
}

fn capabilities(d: &Delivery, video: bool) -> MediaCapabilities {
    let codec = d.mime_type.split('"').nth(1).unwrap_or("");
    let video_codec =
        if codec.starts_with("avc") || codec.starts_with("hev") || codec.starts_with("hvc") {
            Some(2)
        } else if codec.starts_with("vp") {
            Some(4)
        } else if codec.starts_with("av01") {
            Some(8)
        } else {
            None
        };
    let audio_codec = if codec.starts_with("mp4a") {
        Some(1)
    } else if codec.starts_with("opus") {
        Some(3)
    } else if codec.starts_with("ac-3") {
        Some(7)
    } else if codec.starts_with("ec-3") {
        Some(5)
    } else {
        None
    };
    MediaCapabilities {
        video: if video {
            vec![VideoCapability {
                codec: video_codec,
                height: d.height.try_into().ok(),
                width: d.width.try_into().ok(),
                fps: d.fps.try_into().ok(),
                bitrate: d.bitrate.try_into().ok(),
                ten_bit: Some(false),
            }]
        } else {
            vec![]
        },
        audio: if video {
            vec![]
        } else {
            vec![AudioCapability {
                codec: audio_codec,
                channels: d.channels.try_into().ok(),
                spatial: Some(audio_spatial(d.itag, d.channels)),
            }]
        },
        hdr_mode_bitmask: Some(3),
    }
}
struct Ignored {
    length: u64,
    expected: Option<u64>,
    selected: bool,
}
#[derive(Default)]
struct OtherTracks {
    pending: BTreeMap<u8, Ignored>,
    companion: Option<Track>,
}
impl OtherTracks {
    fn initialize(&mut self, meta: &FormatInit) -> Result<()> {
        if let Some(track) = &mut self.companion {
            initialize(track, meta.clone())?;
        }
        Ok(())
    }
    fn rollback(&mut self) {
        self.pending.clear();
        if let Some(track) = &mut self.companion {
            track.rollback();
        }
    }
    fn header(&mut self, h: MediaHeader, video_id: &str) -> Result<()> {
        if let Some(id) = &h.video_id {
            ensure!(id == video_id, "SABR вернул другое видео");
        }
        let id = u8::try_from(
            h.header_id
                .ok_or_else(|| anyhow::anyhow!("Нет номера заголовка SABR"))?,
        )
        .map_err(|_| anyhow::anyhow!("Неверный номер заголовка SABR"))?;
        ensure!(
            self.pending.len() < 16 && !self.pending.contains_key(&id),
            "SABR смешал дополнительные сегменты"
        );
        let expected = h
            .content_length
            .map(u64::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("Неверная длина дополнительного сегмента"))?;
        ensure!(
            expected.is_none_or(|n| n > 0 && n <= track::SEGMENT_LIMIT),
            "Дополнительный сегмент SABR слишком велик"
        );
        let selected = self.companion.as_ref().is_some_and(|track| {
            h.itag.or_else(|| h.format_id.as_ref().and_then(|f| f.itag)) == track.format.itag
        });
        if let Some(track) = self.companion.as_mut().filter(|_| selected) {
            track.header(h)?;
        }
        self.pending.insert(
            id,
            Ignored {
                length: 0,
                expected,
                selected,
            },
        );
        Ok(())
    }
    fn data(&mut self, id: u8, data: &[u8]) -> Result<bool> {
        let Some(part) = self.pending.get_mut(&id) else {
            return Ok(false);
        };
        part.length = part
            .length
            .checked_add(data.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("Переполнение дополнительного сегмента"))?;
        ensure!(
            part.length <= track::SEGMENT_LIMIT && part.expected.is_none_or(|n| part.length <= n),
            "Размер дополнительного сегмента SABR не совпадает"
        );
        if let Some(track) = self.companion.as_mut().filter(|_| part.selected) {
            track.data(id, data)?;
        }
        Ok(true)
    }
    fn finish(&mut self, id: u8) -> Result<bool> {
        let Some(part) = self.pending.remove(&id) else {
            return Ok(false);
        };
        ensure!(
            part.length > 0 && part.expected.is_none_or(|n| part.length == n),
            "Оборван дополнительный сегмент SABR"
        );
        if let Some(track) = self.companion.as_mut().filter(|_| part.selected) {
            let segment = track.finish_segment(id)?;
            track.commit(segment)?;
        }
        Ok(true)
    }
}

fn initialize(track: &mut Track, meta: FormatInit) -> Result<()> {
    let itag = meta
        .format_id
        .as_ref()
        .and_then(|id| id.itag)
        .ok_or_else(|| anyhow::anyhow!("Нет идентификатора описания SABR"))?;
    // YouTube also announces unselected adaptive formats; only the requested identity may alter this track.
    if Some(itag) != track.format.itag {
        return Ok(());
    }
    track.initialize(meta)
}

async fn response(
    state: &mut State,
    delivery: &Delivery,
    body: Vec<u8>,
    cancel: &CancellationToken,
) -> Result<reqwest::Response> {
    loop {
        validate_url(state.url.as_str())?;
        let mut url = state.url.clone();
        let pairs: Vec<_> = url
            .query_pairs()
            .filter(|(key, _)| key != "rn")
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        url.query_pairs_mut()
            .clear()
            .extend_pairs(pairs)
            .append_pair("rn", &state.number.to_string());
        state.number = state
            .number
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Слишком много запросов SABR"))?;
        let request = delivery
            .session
            .client()
            .post(url)
            .header("User-Agent", &delivery.session.user_agent)
            .header("Content-Type", "application/x-protobuf")
            .header("Accept", "application/vnd.yt-ump")
            .header("Accept-Encoding", "identity")
            .body(body.clone());
        let result = tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),r=request.send()=>r.map_err(|_|anyhow::Error::new(Retryable("Соединение SABR прервано")))?};
        // The dedicated client has redirects disabled; no proof-bearing body is sent before this validation.
        if result.status().is_redirection() {
            let location = result
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow::anyhow!("Переадресация SABR без адреса"))?;
            ensure!(
                location.len() <= 16 * 1024,
                "Адрес переадресации SABR слишком велик"
            );
            let next = state
                .url
                .join(location)
                .map_err(|_| anyhow::anyhow!("Некорректная переадресация SABR"))?;
            state.redirect(next.as_str())?;
            continue;
        }
        if matches!(result.status().as_u16(), 429 | 500 | 502 | 503 | 504) {
            return Err(Retryable("YouTube временно отклонил SABR-запрос").into());
        }
        ensure!(
            result.status().is_success(),
            "Сервер YouTube SABR вернул HTTP {}",
            result.status().as_u16()
        );
        return Ok(result);
    }
}

struct PendingFile {
    file: File,
    _path: tempfile::TempPath,
}
async fn new_file(directory: &Path) -> Result<(File, tempfile::TempPath)> {
    let (file, path) = tempfile::NamedTempFile::new_in(directory)
        .map_err(|_| anyhow::anyhow!("Не удалось создать временный файл SABR"))?
        .into_parts();
    Ok((File::from_std(file), path))
}
async fn commit(
    track: &mut Track,
    pending: &mut Option<PendingFile>,
    output: &mut File,
    id: u8,
    cancel: &CancellationToken,
) -> Result<()> {
    let segment = track.finish_segment(id)?;
    let mut part = pending
        .take()
        .ok_or_else(|| anyhow::anyhow!("Нет временного сегмента SABR"))?;
    if !segment.duplicate {
        part.file.flush().await?;
        part.file.seek(SeekFrom::Start(0)).await?;
        let old = track.bytes;
        let result = tokio::select! {biased;_ = cancel.cancelled()=>Err(anyhow::anyhow!("Загрузка приостановлена")),r=tokio::io::copy(&mut part.file,&mut *output)=>r.map_err(|_|anyhow::anyhow!("Не удалось записать сегмент SABR"))};
        let result = match result {
            Ok(n) if n == segment.length => output
                .flush()
                .await
                .map_err(|_| anyhow::anyhow!("Не удалось зафиксировать сегмент SABR")),
            Ok(_) => Err(anyhow::anyhow!("Временный сегмент SABR оборван")),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let _ = output.flush().await;
            output.set_len(old).await?;
            output.seek(SeekFrom::Start(old)).await?;
            return Err(error);
        }
    }
    track.commit(segment)?;
    Ok(())
}
struct Receiver<'a> {
    delivery: &'a Delivery,
    audio: &'a Delivery,
    video: bool,
    directory: &'a Path,
    cancel: &'a CancellationToken,
    limiter: &'a Limiter,
    progress: &'a Arc<dyn Fn(Progress) + Send + Sync>,
    started: Instant,
}
impl Receiver<'_> {
    async fn cycle(
        &self,
        state: &mut State,
        track: &mut Track,
        output: &mut File,
        pending: &mut Option<PendingFile>,
        other: &mut OtherTracks,
    ) -> Result<()> {
        let (delivery, video, directory, cancel, limiter, progress, started) = (
            self.delivery,
            self.video,
            self.directory,
            self.cancel,
            self.limiter,
            self.progress,
            self.started,
        );
        let body = state.body(delivery, track, video, self.audio, other.companion.as_ref())?;
        ensure!(body.len() <= 256 * 1024, "SABR-запрос слишком велик");
        let mut response = response(state, delivery, body, cancel).await?;
        let mut reader = ump::Reader::default();
        let mut received = 0u64;
        loop {
            let chunk = tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),r=response.chunk()=>r.map_err(|_|anyhow::Error::new(Retryable("Оборван ответ SABR")))?};
            let Some(chunk) = chunk else { break };
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("Переполнение ответа SABR"))?;
            ensure!(received <= 512 * 1024 * 1024, "Ответ SABR превышает предел");
            limiter.wait(chunk.len() as u64, cancel).await?;
            reader.push(&chunk)?;
            while let Some((kind, data)) = reader.next()? {
                match kind {
                    42 => {
                        let meta: FormatInit = decode(&data)?;
                        other.initialize(&meta)?;
                        initialize(track, meta)?;
                    }
                    20 => {
                        let h: MediaHeader = decode(&data)?;
                        let itag = h
                            .itag
                            .or_else(|| h.format_id.as_ref().and_then(|f| f.itag))
                            .ok_or_else(|| anyhow::anyhow!("Нет формата сегмента SABR"))?;
                        if Some(itag) != track.format.itag {
                            other.header(h, &delivery.session.video_id)?;
                        } else {
                            track.header(h)?;
                            let (file, path) = new_file(directory).await?;
                            *pending = Some(PendingFile { file, _path: path });
                        }
                    }
                    21 => {
                        let (&id, bytes) = data
                            .split_first()
                            .ok_or_else(|| anyhow::anyhow!("Пустой медиакaдр SABR"))?;
                        if other.data(id, bytes)? {
                            continue;
                        }
                        track.data(id, bytes)?;
                        pending
                            .as_mut()
                            .ok_or_else(|| anyhow::anyhow!("Нет сегмента SABR"))?
                            .file
                            .write_all(bytes)
                            .await?;
                    }
                    22 => {
                        ensure!(data.len() == 1, "Неверное завершение UMP");
                        if other.finish(data[0])? {
                            continue;
                        }
                        commit(track, pending, output, data[0], cancel).await?;
                        progress(Progress {
                            downloaded: track.bytes,
                            total: Some(track.expected_bytes),
                            speed: track.bytes as f64 / started.elapsed().as_secs_f64().max(0.001),
                        });
                    }
                    35 => {
                        let policy: NextPolicy = decode(&data)?;
                        if let Some(id) = policy.video_id {
                            ensure!(
                                id == delivery.session.video_id,
                                "SABR вернул другую политику видео"
                            );
                        }
                        let backoff = policy.backoff_time_ms.unwrap_or(0);
                        ensure!((0..=30_000).contains(&backoff), "Неверная задержка SABR");
                        state.backoff = backoff as u64;
                        ensure!(
                            policy
                                .playback_cookie
                                .as_ref()
                                .is_none_or(|c| c.len() <= META_LIMIT),
                            "Cookie SABR слишком велик"
                        );
                        state.cookie = policy.playback_cookie;
                    }
                    43 => {
                        let redirect: Redirect = decode(&data)?;
                        state.redirect(
                            redirect
                                .url
                                .as_deref()
                                .ok_or_else(|| anyhow::anyhow!("Нет адреса SABR"))?,
                        )?;
                    }
                    44 => {
                        return Err(server_error(decode(&data)?));
                    }
                    57 => state.update(decode(&data)?)?,
                    58 => {
                        let status: Protection = decode(&data)?;
                        state.last_protection = status.status.unwrap_or(0);
                        match state.last_protection {
                            0 | 2 => {}
                            1 => state.attested = true,
                            3 => bail!("Google отклонил PO; повторите проверку"),
                            _ => bail!("Неизвестное состояние проверки Google"),
                        }
                    }
                    59 => state.policy(decode(&data)?)?,
                    _ => ensure!(
                        data.len() <= META_LIMIT,
                        "Служебный кадр SABR слишком велик"
                    ),
                }
            }
        }
        reader
            .finish()
            .map_err(|_| anyhow::Error::new(Retryable("Оборван UMP-кадр SABR")))?;
        if track.pending() || !other.pending.is_empty() {
            return Err(Retryable("Ответ SABR закончился внутри сегмента").into());
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "SABR track={} bytes={} sequence={} end_ms={} protection={}",
            delivery.itag, track.bytes, track.last_sequence, track.end_ms, state.last_protection
        );
        Ok(())
    }
}

pub async fn download(
    delivery: &Delivery,
    audio: Option<(&Delivery, u64)>,
    expected_bytes: u64,
    directory: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<PathBuf> {
    validate_url(&delivery.session.url)?;
    let video = delivery.mime_type.starts_with("video/");
    let (audio, audio_bytes) = if video {
        let (audio, bytes) =
            audio.ok_or_else(|| anyhow::anyhow!("Для видео YouTube не выбрана аудиодорожка"))?;
        ensure!(
            Arc::ptr_eq(&delivery.session, &audio.session) && audio.mime_type.starts_with("audio/"),
            "Аудиодорожка относится к другому потоку YouTube"
        );
        (audio, bytes)
    } else {
        (delivery, expected_bytes)
    };
    tokio::fs::create_dir_all(directory).await?;
    let format = FormatId {
        itag: Some(
            i32::try_from(delivery.itag).map_err(|_| anyhow::anyhow!("Неверный формат SABR"))?,
        ),
        last_modified: Some(delivery.last_modified),
        xtags: delivery.xtags.clone(),
    };
    let mut track = Track::new(
        format,
        delivery.session.video_id.clone(),
        expected_bytes,
        i64::try_from(delivery.duration_ms)
            .map_err(|_| anyhow::anyhow!("Неверная длительность SABR"))?,
    )?;
    let mut state = State::new(delivery)?;
    // A paired request must acknowledge verified audio too, even though its bytes are saved by the audio transfer.
    let mut other = OtherTracks {
        companion: if video {
            Some(Track::new(
                FormatId {
                    itag: Some(i32::try_from(audio.itag)?),
                    last_modified: Some(audio.last_modified),
                    xtags: audio.xtags.clone(),
                },
                audio.session.video_id.clone(),
                audio_bytes,
                i64::try_from(audio.duration_ms)?,
            )?)
        } else {
            None
        },
        ..Default::default()
    };
    let (mut output, path) = new_file(directory).await?;
    let mut pending = None;
    let started = Instant::now();
    let mut retries = 0u8;
    let receiver = Receiver {
        delivery,
        audio,
        video,
        directory,
        cancel,
        limiter,
        progress: &progress,
        started,
    };
    loop {
        let before = (
            track.bytes,
            other.companion.as_ref().map_or(0, |track| track.bytes),
            state.attested,
        );
        match receiver
            .cycle(
                &mut state,
                &mut track,
                &mut output,
                &mut pending,
                &mut other,
            )
            .await
        {
            Ok(()) => {
                retries = 0;
                if state.finished(&track)? {
                    break;
                }
                state.progress(
                    before,
                    (
                        track.bytes,
                        other.companion.as_ref().map_or(0, |track| track.bytes),
                        state.attested,
                    ),
                )?;
            }
            Err(error) => {
                track.rollback();
                other.rollback();
                pending = None;
                if !error.is::<Retryable>() || retries >= 2 {
                    return Err(error);
                }
                state.fallback()?;
                retries += 1;
                output.flush().await?;
                output.set_len(track.bytes).await?;
                output.seek(SeekFrom::Start(track.bytes)).await?;
                tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),_ = tokio::time::sleep(Duration::from_millis(250u64<<retries))=>{}}
                continue;
            }
        }
        if state.backoff > 0 {
            tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),_ = tokio::time::sleep(Duration::from_millis(state.backoff))=>{}}
        }
    }
    ensure!(track.complete(), "SABR-поток получен не полностью");
    output.flush().await?;
    output.sync_all().await?;
    ensure!(
        output.metadata().await?.len() == expected_bytes,
        "Размер готового SABR-потока не совпадает"
    );
    drop(output);
    path.keep()
        .map_err(|_| anyhow::anyhow!("Не удалось сохранить временный поток SABR"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_cdn_moves_to_the_next_advertised_googlevideo_node() {
        let mut s = state();
        s.url = validate_url("https://rr3---sn-primary.googlevideo.com/videoplayback?mn=sn-primary,sn-secondary&fvip=3&sig=fixture&rn=7").unwrap();
        assert!(s.fallback().unwrap(), "failed CDN had no recovery path");
        assert_eq!(s.url.host_str(), Some("rr3---sn-secondary.googlevideo.com"));
        assert!(
            s.url
                .query_pairs()
                .any(|(k, v)| k == "sig" && v == "fixture")
        );
        assert!(
            s.url
                .query_pairs()
                .any(|(k, v)| k == "fallback_count" && v == "1")
        );
        assert!(!s.fallback().unwrap(), "exhausted CDN list must not loop");
    }
    #[test]
    fn cdn_fallback_rejects_foreign_hosts_and_invalid_counters() {
        for query in [
            "mn=sn-primary,other.example&fvip=3",
            "mn=sn-primary,sn-secondary&fvip=3&fallback_count=999",
            "mn=sn-primary,sn-secondary&fvip=bad",
            "mn=sn-primary,sn-secondary&fvip=3&fallback_count=bad",
        ] {
            let mut s = state();
            s.url = validate_url(&format!(
                "https://rr3---sn-primary.googlevideo.com/videoplayback?{query}"
            ))
            .unwrap();
            let before = s.url.clone();
            assert!(!s.fallback().unwrap_or(false));
            assert_eq!(s.url, before);
        }
    }
    #[test]
    fn temporary_format_refusal_is_retryable_but_unknown_refusals_fail_closed() {
        let error = server_error(SabrError {
            r#type: Some("sabr.no_available_formats".into()),
            code: Some(2),
        });
        assert!(
            error.is::<Retryable>(),
            "temporary format refusal aborted the download"
        );
        for (kind, code) in [
            ("sabr.no_available_formats", Some(3)),
            ("sabr.unknown", Some(2)),
            ("private-response", None),
        ] {
            let error = server_error(SabrError {
                r#type: Some(kind.into()),
                code,
            });
            assert!(!error.is::<Retryable>(), "unknown refusal must fail closed");
            assert!(
                !error.to_string().contains(kind),
                "server response leaked into error"
            );
        }
    }
    fn state() -> State {
        State {
            url: validate_url("https://r.googlevideo.com/videoplayback").unwrap(),
            number: 0,
            redirects: 0,
            cookie: None,
            contexts: BTreeMap::new(),
            backoff: 0,
            attested: false,
            last_protection: 0,
            stalls: 0,
            pending_since: None,
        }
    }
    #[test]
    fn companion_audio_keeps_verified_ranges_across_responses_and_rollback() {
        let format = FormatId {
            itag: Some(251),
            last_modified: Some(2),
            xtags: None,
        };
        let mut other = OtherTracks {
            companion: Some(Track::new(format.clone(), "fixture0001".into(), 3, 2000).unwrap()),
            ..Default::default()
        };
        other
            .initialize(&FormatInit {
                video_id: Some("fixture0001".into()),
                format_id: Some(format.clone()),
                end_time_ms: Some(2000),
                end_segment_number: Some(2),
                mime_type: None,
            })
            .unwrap();
        let header = |init, sequence, start, duration, size| MediaHeader {
            header_id: Some(9),
            video_id: Some("fixture0001".into()),
            format_id: Some(format.clone()),
            is_init_seg: Some(init),
            sequence_number: sequence,
            start_ms: Some(start),
            duration_ms: Some(duration),
            content_length: Some(size),
            ..Default::default()
        };
        for (h, bytes) in [
            (header(true, None, 0, 0, 1), b"a"),
            (header(false, Some(1), 0, 1000, 1), b"b"),
        ] {
            other.header(h, "fixture0001").unwrap();
            assert!(other.data(9, bytes).unwrap());
            assert!(other.finish(9).unwrap());
        }
        assert!(other.pending.is_empty());
        let audio = other.companion.as_ref().unwrap();
        assert!(audio.initialized());
        assert_eq!(
            (audio.bytes, audio.last_sequence, audio.end_ms),
            (2, 1, 1000)
        );
        // Replayed audio must not advance the buffer twice.
        other
            .header(header(false, Some(1), 0, 1000, 1), "fixture0001")
            .unwrap();
        other.data(9, b"b").unwrap();
        other.finish(9).unwrap();
        assert_eq!(other.companion.as_ref().unwrap().bytes, 2);
        other
            .header(header(false, Some(2), 1000, 1000, 2), "fixture0001")
            .unwrap();
        other.data(9, b"c").unwrap();
        assert!(other.finish(9).is_err());
        other.rollback();
        assert_eq!(other.companion.as_ref().unwrap().end_ms, 1000);
        other
            .header(header(false, Some(2), 1000, 1000, 1), "fixture0001")
            .unwrap();
        other.data(9, b"c").unwrap();
        other.finish(9).unwrap();
        assert!(other.companion.as_ref().unwrap().complete());
    }
    #[test]
    fn extra_format_descriptors_do_not_change_the_selected_track() {
        let id = FormatId {
            itag: Some(133),
            last_modified: Some(1),
            xtags: None,
        };
        let mut track = Track::new(id, "fixture0001".into(), 9, 2000).unwrap();
        let other = FormatInit {
            format_id: Some(FormatId {
                itag: Some(251),
                last_modified: Some(2),
                xtags: None,
            }),
            end_time_ms: Some(2000),
            end_segment_number: Some(1),
            ..Default::default()
        };
        assert!(initialize(&mut track, other).is_ok());
        assert!(!track.initialized());
        assert!(!track.covered());
    }
    #[test]
    fn additional_tracks_are_discarded_with_size_and_identity_validation() {
        let mut other = OtherTracks::default();
        other
            .header(
                MediaHeader {
                    header_id: Some(9),
                    itag: Some(251),
                    video_id: Some("fixture0001".into()),
                    content_length: Some(4),
                    ..Default::default()
                },
                "fixture0001",
            )
            .unwrap();
        assert!(other.data(9, b"abcd").unwrap());
        assert!(other.finish(9).unwrap());
        assert!(!other.data(7, b"selected").unwrap());
        assert!(
            other
                .header(
                    MediaHeader {
                        header_id: Some(9),
                        content_length: Some(1),
                        video_id: Some("other-video".into()),
                        ..Default::default()
                    },
                    "fixture0001"
                )
                .is_err()
        );
    }
    #[test]
    fn surround_aac_advertises_the_required_multichannel_capability() {
        assert_eq!(audio_spatial(258, 6), 1);
        assert_eq!(audio_spatial(140, 2), 0);
        assert_eq!(audio_spatial(327, 4), 4);
    }

    #[test]
    fn stalled_attestation_does_not_become_success_or_spin_forever() {
        let mut a = state();
        a.progress((0, 0, false), (1, 0, false)).unwrap();
        a.progress((1, 0, false), (1, 0, false)).unwrap();
        a.progress((1, 0, false), (1, 0, false)).unwrap();
        assert!(a.progress((1, 0, false), (1, 0, false)).is_err());
        let mut b = state();
        b.progress((0, 0, false), (5, 0, false)).unwrap();
        assert_eq!(b.stalls, 0);
    }
    #[test]
    fn companion_progress_and_new_attestation_do_not_count_as_video_stalls() {
        let mut state = state();
        let mut before = (1024, 0, false);
        for audio in 1..=4 {
            let after = (1024, audio, false);
            state.progress(before, after).unwrap();
            before = after;
        }
        state.progress(before, before).unwrap();
        state.progress(before, before).unwrap();
        let verified = (1024, 4, true);
        state.progress(before, verified).unwrap();
        state.progress(verified, verified).unwrap();
        state.progress(verified, verified).unwrap();
        assert!(state.progress(verified, verified).is_err());
    }
    #[test]
    fn complete_media_waits_for_pending_attestation_before_publication() {
        let id = FormatId {
            itag: Some(140),
            last_modified: Some(1),
            xtags: None,
        };
        let mut track = Track::new(id.clone(), "fixture0001".into(), 2, 1000).unwrap();
        track
            .initialize(FormatInit {
                video_id: Some("fixture0001".into()),
                format_id: Some(id.clone()),
                end_time_ms: Some(1000),
                end_segment_number: Some(1),
                mime_type: None,
            })
            .unwrap();
        for (init, sequence, duration) in [(true, None, 0), (false, Some(1), 1000)] {
            track
                .header(MediaHeader {
                    header_id: Some(7),
                    format_id: Some(id.clone()),
                    is_init_seg: Some(init),
                    sequence_number: sequence,
                    start_ms: Some(0),
                    duration_ms: Some(duration),
                    content_length: Some(1),
                    ..Default::default()
                })
                .unwrap();
            track.data(7, b"x").unwrap();
            let segment = track.finish_segment(7).unwrap();
            track.commit(segment).unwrap();
        }
        assert!(track.complete());
        let mut state = state();
        state.last_protection = 2;
        assert!(
            !state.finished(&track).unwrap(),
            "pending verification is not a rejection or success"
        );
        state.progress((2, 0, false), (2, 0, false)).unwrap();
        // Pending verification may outlive several empty responses after all media arrived.
        for _ in 0..260 {
            state.progress((2, 0, false), (2, 0, false)).unwrap();
            assert!(!state.finished(&track).unwrap());
        }
        assert!(state.backoff >= 250, "pending polls must not spin");
        state.pending_since = Some(Instant::now() - ATTESTATION_WAIT);
        assert!(
            state
                .progress((2, 0, false), (2, 0, false))
                .unwrap_err()
                .is::<PendingAttestation>()
        );
        state.attested = true;
        assert!(state.finished(&track).unwrap());
        track.bytes -= 1;
        assert!(
            state.finished(&track).is_err(),
            "accepted verification cannot publish a truncated track"
        );
    }
    #[test]
    fn all_redirects_are_validated_before_another_request() {
        let mut s = state();
        assert!(s.redirect("https://evil.test/videoplayback").is_err());
        assert_eq!(s.redirects, 0);
        for _ in 0..REDIRECT_LIMIT {
            s.redirect("https://r2.googlevideo.com/videoplayback")
                .unwrap();
        }
        assert!(
            s.redirect("https://r3.googlevideo.com/videoplayback")
                .is_err()
        );
    }
    #[test]
    fn contexts_are_bounded_and_discarded_types_can_be_freshly_replaced() {
        let mut s = state();
        for n in 0..CONTEXT_LIMIT {
            s.update(ContextUpdate {
                r#type: Some(n as i32),
                value: Some(vec![1]),
                send_by_default: Some(true),
                write_policy: None,
            })
            .unwrap();
        }
        assert!(
            s.update(ContextUpdate {
                r#type: Some(999),
                value: Some(vec![1]),
                ..Default::default()
            })
            .is_err()
        );
        s.policy(ContextPolicy {
            discard_policy: vec![0],
            ..Default::default()
        })
        .unwrap();
        s.update(ContextUpdate {
            r#type: Some(0),
            value: Some(vec![2]),
            send_by_default: Some(true),
            ..Default::default()
        })
        .unwrap();
        assert!(s.contexts[&0].send);
    }
}

#[cfg(test)]
mod atomic_tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    fn header(sequence: Option<i32>, start: i64, length: i64) -> MediaHeader {
        MediaHeader {
            header_id: Some(7),
            format_id: Some(FormatId {
                itag: Some(251),
                last_modified: Some(1),
                xtags: None,
            }),
            is_init_seg: Some(sequence.is_none()),
            sequence_number: sequence,
            start_ms: Some(start),
            duration_ms: Some(if sequence.is_some() { 1000 } else { 0 }),
            content_length: Some(length),
            ..Default::default()
        }
    }
    async fn put(
        t: &mut Track,
        p: &mut Option<PendingFile>,
        dir: &Path,
        sequence: Option<i32>,
        start: i64,
        expected: i64,
        bytes: &[u8],
    ) {
        t.header(header(sequence, start, expected)).unwrap();
        let (mut file, path) = new_file(dir).await.unwrap();
        t.data(7, bytes).unwrap();
        file.write_all(bytes).await.unwrap();
        *p = Some(PendingFile { file, _path: path });
    }
    async fn fixture() {
        let dir = tempfile::tempdir().unwrap();
        let (mut output, _outpath) = new_file(dir.path()).await.unwrap();
        let id = FormatId {
            itag: Some(251),
            last_modified: Some(1),
            xtags: None,
        };
        let mut t = Track::new(id.clone(), "fixture0001".into(), 9, 2000).unwrap();
        t.initialize(FormatInit {
            format_id: Some(id),
            end_time_ms: Some(2000),
            end_segment_number: Some(2),
            ..Default::default()
        })
        .unwrap();
        let mut pending = None;
        let cancel = CancellationToken::new();
        put(&mut t, &mut pending, dir.path(), None, 0, 1, b"i").await;
        commit(&mut t, &mut pending, &mut output, 7, &cancel)
            .await
            .unwrap();
        put(&mut t, &mut pending, dir.path(), Some(1), 0, 4, b"abcd").await;
        commit(&mut t, &mut pending, &mut output, 7, &cancel)
            .await
            .unwrap();
        put(&mut t, &mut pending, dir.path(), Some(2), 1000, 4, b"wx").await;
        assert!(
            commit(&mut t, &mut pending, &mut output, 7, &cancel)
                .await
                .is_err()
        );
        t.rollback();
        pending = None;
        assert_eq!(output.metadata().await.unwrap().len(), 5);
        put(&mut t, &mut pending, dir.path(), Some(1), 0, 4, b"abcd").await;
        commit(&mut t, &mut pending, &mut output, 7, &cancel)
            .await
            .unwrap();
        assert_eq!(output.metadata().await.unwrap().len(), 5);
        put(&mut t, &mut pending, dir.path(), Some(2), 1000, 4, b"wxyz").await;
        commit(&mut t, &mut pending, &mut output, 7, &cancel)
            .await
            .unwrap();
        assert!(t.complete());
        output.seek(SeekFrom::Start(0)).await.unwrap();
        let mut bytes = Vec::new();
        output.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"iabcdwxyz");
    }
    #[tokio::test]
    async fn concurrent_jobs_commit_only_complete_segments_and_skip_verified_replays() {
        tokio::join!(fixture(), fixture());
    }
    #[tokio::test]
    async fn cancelled_segment_commit_rolls_back_before_publication() {
        let dir = tempfile::tempdir().unwrap();
        let (mut output, _outpath) = new_file(dir.path()).await.unwrap();
        let id = FormatId {
            itag: Some(251),
            last_modified: Some(1),
            xtags: None,
        };
        let mut t = Track::new(id.clone(), "fixture0001".into(), 1, 1000).unwrap();
        t.initialize(FormatInit {
            format_id: Some(id),
            end_time_ms: Some(1000),
            end_segment_number: Some(1),
            ..Default::default()
        })
        .unwrap();
        let mut pending = None;
        put(&mut t, &mut pending, dir.path(), None, 0, 1, b"i").await;
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(
            commit(&mut t, &mut pending, &mut output, 7, &cancel)
                .await
                .is_err()
        );
        assert_eq!(t.bytes, 0);
        assert_eq!(output.metadata().await.unwrap().len(), 0);
    }
}
