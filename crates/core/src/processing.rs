//! Local MediaTools. Neural stems/speech plus classical WPE dereverberation.
//! Native calls run off the UI thread; cancellation is checked between <=9s audio blocks.
mod assets;
mod native;
mod wpe;
use anyhow::{Context, Result, bail, ensure};
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;
const MAX_PCM_BYTES: u64 = 2 * 1024 * 1024 * 1024;
static CAPACITY: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

/// Offline capability detection. Verified models are installed only by process().
pub fn available(data_dir: &Path) -> bool {
    assets::supported() && assets::root(data_dir).is_ok()
}
#[derive(Clone, Copy)]
enum Effect {
    RemoveVocals,
    ExtractVocals,
    ExtractVoice,
    RemoveEcho,
}
impl Effect {
    fn parse(name: &str) -> Result<Self> {
        Ok(match name {
            "remove_vocals" => Self::RemoveVocals,
            "extract_vocals" => Self::ExtractVocals,
            "extract_voice" => Self::ExtractVoice,
            "remove_echo" => Self::RemoveEcho,
            _ => bail!("Неизвестный эффект обработки"),
        })
    }
    fn family(self) -> assets::Family {
        match self {
            Self::RemoveVocals | Self::ExtractVocals => assets::Family::Separation,
            Self::ExtractVoice => assets::Family::Speech,
            Self::RemoveEcho => assets::Family::Echo,
        }
    }
    fn profile(self) -> (u32, usize) {
        match self {
            Self::RemoveVocals | Self::ExtractVocals => (44100, 2),
            Self::ExtractVoice => (16000, 1),
            Self::RemoveEcho => (48000, 2),
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Self::RemoveVocals => "instrumental",
            Self::ExtractVocals => "vocals",
            Self::ExtractVoice => "voice",
            Self::RemoveEcho => "dereverberated",
        }
    }
    fn removes(self) -> bool {
        matches!(self, Self::RemoveVocals | Self::RemoveEcho)
    }
}
struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn source_file(input: &Path) -> Result<PathBuf> {
    ensure!(
        input.is_absolute(),
        "Нужен абсолютный путь к исходному медиафайлу"
    );
    let metadata = std::fs::symlink_metadata(input)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() > 0,
        "Исходник должен быть обычным непустым медиафайлом"
    );
    let mut file = std::fs::File::open(input)?;
    let mut bytes = [0u8; 4096];
    let n = file.read(&mut bytes)?;
    let b = &bytes[..n];
    let container = b.starts_with(b"RIFF")
        || b.starts_with(b"FORM")
        || b.starts_with(b"ID3")
        || b.starts_with(b"OggS")
        || b.starts_with(b"fLaC")
        || b.starts_with(b"wvpk")
        || b.starts_with(b"MAC ")
        || b.starts_with(b".snd")
        || b.starts_with(b"#!AMR")
        || b.starts_with(&[0x1a, 0x45, 0xdf, 0xa3])
        || b.starts_with(&[0x30, 0x26, 0xb2, 0x75])
        || b.starts_with(&[0, 0, 1])
        || b.first() == Some(&0x47)
        || b.len() > 1 && b[0] == 0xff && b[1] & 0xe0 == 0xe0
        || b.len() >= 8
            && [
                b"ftyp".as_slice(),
                b"styp",
                b"moov",
                b"mdat",
                b"wide",
                b"free",
            ]
            .contains(&&b[4..8]);
    ensure!(
        container,
        "Неизвестный медиа-контейнер; текстовые плейлисты/URL не принимаются"
    );
    input.canonicalize().context("Исходник недоступен")
}
async fn local(mut command: Command, cancel: &CancellationToken) -> Result<Vec<u8>> {
    ensure!(!cancel.is_cancelled(), "Обработка отменена");
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("FFmpeg/FFprobe не установлен")?;
    let stdout = child.stdout.take().context("Нет stdout медиапроцесса")?;
    let mut stderr = child.stderr.take().context("Нет stderr медиапроцесса")?;
    let mut output = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.take(1024 * 1024 + 1).read_to_end(&mut bytes).await?;
        ensure!(
            bytes.len() <= 1024 * 1024,
            "Слишком большой отчёт медиапроцесса"
        );
        Ok::<_, anyhow::Error>(bytes)
    });
    let error = tokio::spawn(async move {
        let mut tail = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = stderr.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&chunk[..n]);
            if tail.len() > 8192 {
                tail.drain(..tail.len() - 8192);
            }
        }
        Ok::<_, std::io::Error>(tail)
    });
    let bytes = tokio::select! {biased;_ = cancel.cancelled()=>{child.kill().await.ok();child.wait().await.ok();output.abort();error.abort();bail!("Обработка отменена")},r=&mut output=>r.map_err(anyhow::Error::from).and_then(|v|v)};
    let bytes = match bytes {
        Ok(v) => v,
        Err(e) => {
            child.kill().await.ok();
            child.wait().await.ok();
            error.abort();
            return Err(e);
        }
    };
    let status = tokio::select! {biased;_ = cancel.cancelled()=>{child.kill().await.ok();child.wait().await.ok();error.abort();bail!("Обработка отменена")},s=child.wait()=>s?};
    let errors = error
        .await
        .unwrap_or_else(|_| Ok(Vec::new()))
        .unwrap_or_default();
    ensure!(
        status.success(),
        "Локальный медиапроцесс не завершился: {}",
        String::from_utf8_lossy(&errors)
            .chars()
            .take(700)
            .collect::<String>()
    );
    Ok(bytes)
}
struct Probe {
    duration: Option<f64>,
    video: Option<u64>,
    audio_offset: f64,
}
async fn probe(input: &Path, cancel: &CancellationToken) -> Result<Probe> {
    let mut command = Command::new(crate::platform::media_tool("ffprobe"));
    command.args(["-v","error","-protocol_whitelist","file,pipe,crypto,data","-show_entries","format=duration,start_time:stream=index,codec_type,sample_rate,start_time:stream_disposition=attached_pic","-of","json"]).arg(input);
    let json: serde_json::Value = serde_json::from_slice(&local(command, cancel).await?)?;
    let streams = json["streams"]
        .as_array()
        .context("В исходнике нет медиапотоков")?;
    let audio = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .context("В исходнике нет аудиодорожки")?;
    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n > 0.0);
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        .and_then(|s| s["index"].as_u64());
    let format_start = json["format"]["start_time"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(0.0);
    let start = audio["start_time"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(format_start);
    let offset = start - format_start;
    ensure!(offset.abs() < 86400.0, "Слишком большое смещение аудио");
    Ok(Probe {
        duration,
        video,
        audio_offset: offset,
    })
}
fn chunks(
    installed: assets::Installed,
    effect: Effect,
    input: &Path,
    output: &Path,
    cancel: &CancellationToken,
    progress: Arc<dyn Fn(f64) + Send + Sync>,
) -> Result<()> {
    let (rate, channels) = effect.profile();
    let frame_bytes = channels * 4;
    let len = std::fs::metadata(input)?.len();
    ensure!(
        len > 0 && len < MAX_PCM_BYTES && len % frame_bytes as u64 == 0,
        "Неверная длина декодированного PCM"
    );
    let frames = len as usize / frame_bytes;
    if matches!(effect, Effect::RemoveEcho) {
        ensure!(
            frames >= rate as usize / 2,
            "Для dereverberation требуется хотя бы0.5секунды"
        );
    }
    ensure!(!cancel.is_cancelled(), "Обработка отменена");
    let engine = if matches!(effect, Effect::RemoveEcho) {
        None
    } else {
        Some(native::Engine::create(
            &installed,
            matches!(effect, Effect::RemoveVocals | Effect::ExtractVocals),
        )?)
    };
    let mut source = std::fs::File::open(input)?;
    let mut wave = hound::WavWriter::create(
        output,
        hound::WavSpec {
            channels: channels as u16,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    let core = rate as usize * 8;
    let context = rate as usize / 2;
    let mut begin = 0usize;
    while begin < frames {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        let end = (begin + core).min(frames);
        let read_start = begin.saturating_sub(context);
        let read_end = (end + context).min(frames);
        source.seek(SeekFrom::Start((read_start * frame_bytes) as u64))?;
        let mut bytes = vec![0u8; (read_end - read_start) * frame_bytes];
        source.read_exact(&mut bytes)?;
        let pcm: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(f32::from_le_bytes)
            .collect();
        ensure!(
            pcm.iter().all(|v| v.is_finite()),
            "Исходник содержит нечисловой PCM"
        );
        let processed = if let Some(engine) = &engine {
            engine.run(
                &pcm,
                channels,
                rate,
                matches!(effect, Effect::ExtractVocals),
            )?
        } else {
            let mut interleaved = vec![0f32; pcm.len()];
            for channel in 0..channels {
                let mono: Vec<_> = pcm
                    .iter()
                    .skip(channel)
                    .step_by(channels)
                    .copied()
                    .collect();
                let output = wpe::dereverb(&mono, rate, cancel)?;
                for (i, sample) in output.into_iter().enumerate() {
                    interleaved[i * channels + channel] = sample;
                }
            }
            interleaved
        };
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        ensure!(
            processed.len() == pcm.len(),
            "Обработка изменила временную шкалу PCM"
        );
        ensure!(
            processed.iter().all(|v| v.is_finite() && v.abs() <= 8.0),
            "Обработка вернула неустойчивый уровень аудио"
        );
        let offset = (begin - read_start) * channels;
        let count = (end - begin) * channels;
        for value in &processed[offset..offset + count] {
            wave.write_sample(*value)?;
        }
        begin = end;
        progress(begin as f64 / frames as f64);
    }
    wave.finalize()?;
    Ok(())
}
/// Four real effects; progress is percent0..100. Native-call cancellation latency depends on CPU.
pub async fn process(
    input: PathBuf,
    effect: &str,
    output_dir: PathBuf,
    data_dir: PathBuf,
    cancel: CancellationToken,
    progress: Arc<dyn Fn(f64) + Send + Sync>,
) -> Result<PathBuf> {
    let cancel = cancel.child_token();
    let _cancel_on_drop = cancel.clone().drop_guard();
    let effect = Effect::parse(effect)?;
    ensure!(!cancel.is_cancelled(), "Обработка отменена");
    ensure!(
        available(&data_dir),
        "MediaTools недоступны для платформы/папки данных"
    );
    ensure!(
        output_dir.is_absolute()
            && output_dir
                .components()
                .all(|c| !matches!(c, Component::ParentDir)),
        "Нужна абсолютная папка результата обработки"
    );
    let input = source_file(&input)?;
    let capacity = CAPACITY
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone();
    let permit = Arc::new(
        tokio::select! {biased;_ = cancel.cancelled()=>bail!("Обработка отменена"),p=capacity.acquire_owned()=>p.context("Очередь MediaTools закрыта")?},
    );
    let high = Arc::new(AtomicU64::new(0));
    let emitted = AtomicU64::new(0);
    let emitted_ms = AtomicU64::new(0);
    let started = std::time::Instant::now();
    let report = {
        let progress = progress.clone();
        Arc::new(move |value: f64| {
            if !value.is_finite() {
                return;
            }
            let bits = value.clamp(0.0, 1.0).to_bits();
            let old = high.fetch_max(bits, Ordering::Relaxed);
            let current = bits.max(old);
            let value = f64::from_bits(current);
            let now = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            if value < 1.0
                && value - f64::from_bits(emitted.load(Ordering::Relaxed)) < 0.005
                && now.saturating_sub(emitted_ms.load(Ordering::Relaxed)) < 150
            {
                return;
            }
            emitted.store(current, Ordering::Relaxed);
            emitted_ms.store(now, Ordering::Relaxed);
            progress(value * 100.0);
        }) as Arc<dyn Fn(f64) + Send + Sync>
    };
    let info = probe(&input, &cancel).await?;
    report(0.02);
    let (rate, channels) = effect.profile();
    let estimated = info
        .duration
        .map(|d| d * rate as f64 * channels as f64 * 4.0);
    ensure!(
        estimated.is_none_or(|n| n.is_finite() && n < MAX_PCM_BYTES as f64),
        "Медиа слишком длинное для безопасной локальной обработки"
    );
    let expected = estimated.map(|n| n.ceil() as u64).unwrap_or(MAX_PCM_BYTES);
    let reserve = expected
        .checked_mul(3)
        .and_then(|n| {
            n.checked_add(if info.video.is_some() && effect.removes() {
                std::fs::metadata(&input).ok()?.len()
            } else {
                0
            })
        })
        .context("Размер обработки переполнен")?;
    let root = assets::root(&data_dir)?;
    ensure!(
        assets::free_space(&root)? > reserve + 256 * 1024 * 1024,
        "Недостаточно места для локальной обработки"
    );
    ensure!(
        assets::free_space(&output_dir)? > expected + 64 * 1024 * 1024,
        "Недостаточно места в папке результата"
    );
    let callback = {
        let report = report.clone();
        Arc::new(move |p| report(0.02 + 0.23 * p)) as Arc<dyn Fn(f64) + Send + Sync>
    };
    let installed = assets::ensure_installed(&data_dir, effect.family(), &cancel, callback).await?;
    report(0.25);
    let job = Arc::new(Workspace(
        installed
            .root
            .join(format!(".work-{}", uuid::Uuid::new_v4())),
    ));
    std::fs::create_dir(&job.0)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&job.0, std::fs::Permissions::from_mode(0o700))?;
    }
    let decoded = job.0.join("decoded.f32");
    let enhanced = job.0.join("processed.wav");
    let mut decode = Command::new(crate::platform::media_tool("ffmpeg"));
    decode
        .args([
            "-hide_banner",
            "-nostdin",
            "-y",
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe,crypto,data",
            "-i",
        ])
        .arg(&input)
        .args(["-map", "0:a:0", "-vn", "-ac"])
        .arg(channels.to_string())
        .arg("-ar")
        .arg(rate.to_string())
        .args(["-c:a", "pcm_f32le", "-f", "f32le", "-fs"])
        .arg((MAX_PCM_BYTES + 4).to_string())
        .arg(&decoded);
    local(decode, &cancel).await?;
    report(0.30);
    ensure!(
        tokio::fs::metadata(&decoded).await?.len() < MAX_PCM_BYTES,
        "Декодированный PCM превышает допустимый размер"
    );
    let callback = {
        let report = report.clone();
        Arc::new(move |p| report(0.30 + 0.60 * p)) as Arc<dyn Fn(f64) + Send + Sync>
    };
    let token = cancel.clone();
    let source = decoded.clone();
    let target = enhanced.clone();
    // Await the worker even on cancellation so no native handle outlives its model/work files.
    let workspace = job.clone();
    let worker_permit = permit.clone();
    tokio::task::spawn_blocking(move || {
        let _workspace = workspace;
        let _capacity = worker_permit;
        chunks(installed, effect, &source, &target, &token, callback)
    })
    .await??;
    ensure!(!cancel.is_cancelled(), "Обработка отменена");
    report(0.90);
    let video = info.video.filter(|_| effect.removes());
    let ready = if let Some(index) = video {
        let target = job.0.join("processed.mkv");
        let mut encode = Command::new(crate::platform::media_tool("ffmpeg"));
        encode
            .args([
                "-hide_banner",
                "-nostdin",
                "-y",
                "-v",
                "error",
                "-protocol_whitelist",
                "file,pipe,crypto,data",
                "-i",
            ])
            .arg(&input);
        encode
            .arg("-itsoffset")
            .arg(format!("{:.9}", info.audio_offset))
            .args(["-protocol_whitelist", "file,pipe,crypto,data", "-i"])
            .arg(&enhanced)
            .arg("-map")
            .arg(format!("0:{index}"))
            .args([
                "-map",
                "1:a:0",
                "-c:v",
                "copy",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-map_metadata",
                "-1",
            ])
            .arg(&target);
        local(encode, &cancel).await?;
        target
    } else {
        enhanced
    };
    report(0.97);
    let final_info = probe(&ready, &cancel).await?;
    ensure!(
        video.is_none() || final_info.video.is_some(),
        "Обработанный файл потерял видео"
    );
    let expected_duration = if video.is_some() {
        info.duration
    } else {
        Some(
            tokio::fs::metadata(&decoded).await?.len() as f64
                / (rate as f64 * channels as f64 * 4.0),
        )
    };
    if let Some(duration) = expected_duration {
        ensure!(
            final_info
                .duration
                .is_some_and(|d| (d - duration).abs() < 0.25),
            "Обработанный файл не сохранил длительность"
        );
    }
    tokio::fs::create_dir_all(&output_dir).await?;
    let stem = crate::pipeline::safe_name(
        input
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref(),
    );
    let ext = if video.is_some() { "mkv" } else { "wav" };
    let mut index = 0u64;
    let result = loop {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(" ({index})")
        };
        let destination = output_dir.join(format!("{stem} - {}{suffix}.{ext}", effect.suffix()));
        let published = tokio::select! {biased;_ = cancel.cancelled()=>bail!("Обработка отменена"),result=crate::pipeline::publish(&ready,&destination)=>result};
        match published {
            Ok(()) => break destination,
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) =>
            {
                index = index
                    .checked_add(1)
                    .context("Слишком много файлов результата")?;
            }
            Err(error) => return Err(error),
        }
    };
    report(1.0);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn queued_processing_is_cancellable_before_models_or_children_start() {
        if !assets::supported() {
            return;
        }
        let capacity = CAPACITY
            .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
            .clone();
        let hold = capacity.acquire_owned().await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("queued.wav");
        let data = dir.path().join("data");
        let output = dir.path().join("out");
        let mut wave = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..800 {
            wave.write_sample(0i16).unwrap();
        }
        wave.finalize().unwrap();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let data_copy = data.clone();
        let output_copy = output.clone();
        let job = tokio::spawn(process(
            input,
            "remove_echo",
            output_copy,
            data_copy,
            token,
            Arc::new(|_| {}),
        ));
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        assert!(!job.is_finished());
        assert!(!data.exists() && !output.exists());
        cancel.cancel();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), job)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        drop(hold);
    }
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn cancellation_kills_and_reaps_the_owned_ffmpeg_process() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("owned.pid");
        let ffmpeg = crate::platform::media_tool("ffmpeg");
        let mut command = Command::new("sh");
        command.arg("-c").arg("printf '%s' $$ > \"$1\"; exec \"$2\" -hide_banner -v error -nostdin -re -f lavfi -i anullsrc=r=8000:cl=mono -t 30 -f null -").arg("fixture").arg(&pidfile).arg(ffmpeg);
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            token.cancel();
        });
        let started = std::time::Instant::now();
        assert!(local(command, &cancel).await.is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(3));
        let pid = std::fs::read_to_string(pidfile).unwrap();
        assert!(
            !Path::new("/proc").join(pid).exists(),
            "owned FFmpeg must be reaped"
        );
    }
}
