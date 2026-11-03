use crate::{
    adaptive,
    transfer::{self, Limiter, Progress},
};
use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;
use videodock_media::{Media, Protocol, Selection, choose_formats};

#[derive(Clone)]
pub struct Options {
    pub selection: Selection,
    pub output_dir: PathBuf,
    pub filename: Option<String>,
    pub embed_subtitles: bool,
    pub tags: bool,
    pub skip_duplicates: bool,
}

#[derive(Clone, Debug)]
pub struct PipelineProgress {
    pub phase: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: f64,
    pub percent: f64,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
    pub size: u64,
    pub info: Value,
}

pub async fn prepare(client: &Client, media: &mut Media, cancel: &CancellationToken) -> Result<()> {
    let mut formats = Vec::new();
    let mut errors = Vec::new();
    ensure!(
        media.formats.len() <= 2048,
        "Слишком много форматов в ответе сервиса"
    );
    for format in media.formats.drain(..) {
        let mut format = format;
        format.cookies.extend(media.cookies.clone());
        let mut candidate = Media {
            kind: media.kind.clone(),
            formats: vec![format.clone()],
            ..Default::default()
        };
        let result = match format.protocol {
            Protocol::Http => Ok(()),
            Protocol::Hls => adaptive::expand(client, &mut candidate, cancel).await,
            Protocol::Dash => crate::dash::expand(client, &mut candidate, cancel).await,
        };
        match result {
            Ok(()) => formats.extend(candidate.formats),
            Err(error) => errors.push(error),
        }
    }
    if formats.is_empty() && !errors.is_empty() {
        return Err(errors.remove(0));
    }
    media.formats = formats;
    Ok(())
}

pub fn safe_name(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        let ch = if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
            '_'
        } else {
            ch
        };
        if result.len() + ch.len_utf8() > 180 {
            break;
        }
        result.push(ch);
    }
    let name = result.trim().trim_matches('.').trim();
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|v| matches!(v, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"));
    if name.is_empty() {
        "media".into()
    } else if reserved {
        format!("_{name}")
    } else {
        name.into()
    }
}

pub async fn probe(path: &Path) -> Result<Value> {
    let result=Command::new(crate::platform::media_tool("ffprobe")).args(["-v","error","-protocol_whitelist","file,pipe,crypto,data","-show_entries","format=duration,size,bit_rate:stream=codec_type,codec_name,width,height,bit_rate,sample_rate,channels,avg_frame_rate,bits_per_raw_sample","-of","json"])
        .arg(path).kill_on_drop(true).output().await.context("FFprobe не установлен")?;
    ensure!(
        result.status.success(),
        "FFprobe не смог прочитать готовый медиафайл"
    );
    ensure!(
        result.stdout.len() <= 8 * 1024 * 1024,
        "Слишком большой отчёт FFprobe"
    );
    Ok(serde_json::from_slice(&result.stdout)?)
}

async fn ffmpeg(
    args: Vec<String>,
    duration: Option<f64>,
    cancel: &CancellationToken,
    progress: Arc<dyn Fn(PipelineProgress) + Send + Sync>,
) -> Result<()> {
    let mut child = Command::new(crate::platform::media_tool("ffmpeg"))
        .args([
            "-hide_banner",
            "-nostdin",
            "-y",
            "-v",
            "error",
            "-progress",
            "pipe:1",
        ])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("FFmpeg не установлен")?;
    let stdout = child.stdout.take().context("Нет канала FFmpeg")?;
    let mut lines = BufReader::new(stdout).lines();
    let mut stderr = child.stderr.take().context("Нет error канала FFmpeg")?;
    let error_task = tokio::spawn(async move {
        let mut tail = Vec::new();
        let mut chunk = [0; 4096];
        while let Ok(n) = stderr.read(&mut chunk).await {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&chunk[..n]);
            if tail.len() > 8192 {
                tail.drain(..tail.len() - 8192);
            }
        }
        tail
    });
    loop {
        let line = tokio::select! {_ = cancel.cancelled()=>{child.kill().await.ok();child.wait().await.ok();error_task.abort();bail!("Обработка приостановлена")},line=lines.next_line()=>line?};
        let Some(line) = line else { break };
        if let Some(seconds) = line
            .strip_prefix("out_time_us=")
            .and_then(|s| s.parse::<f64>().ok())
            .map(|n| n / 1_000_000.0)
        {
            progress(PipelineProgress {
                phase: "processing".into(),
                downloaded: 0,
                total: None,
                speed: 0.0,
                percent: duration
                    .filter(|d| *d > 0.0)
                    .map(|d| 90.0 + 9.0 * (seconds / d).clamp(0.0, 1.0))
                    .unwrap_or(95.0),
            });
        }
    }
    let status = tokio::select! {_ = cancel.cancelled()=>{child.kill().await.ok();child.wait().await.ok();bail!("Обработка приостановлена")},status=child.wait()=>status?};
    let errors = error_task.await.unwrap_or_default();
    if !status.success() {
        let error = String::from_utf8_lossy(&errors);
        bail!(
            "FFmpeg не завершил обработку: {}",
            error.chars().take(1500).collect::<String>()
        )
    }
    Ok(())
}

pub(crate) async fn publish(source: &Path, destination: &Path) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let temp = tempfile::NamedTempFile::new_in(
        destination
            .parent()
            .context("Не указан каталог назначения")?,
    )?;
    let mut input = tokio::fs::File::open(source).await?;
    let mut output = tokio::fs::File::from_std(temp.reopen()?);
    tokio::io::copy(&mut input, &mut output).await?;
    output.flush().await?;
    output.sync_all().await?;
    drop(output);
    match temp.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) => {
            let tempfile::PersistError { error, file: temp } = error;
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(error.into());
            }
            let output = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(destination)?;
            struct OwnedOutput {
                path: PathBuf,
                committed: bool,
            }
            impl Drop for OwnedOutput {
                fn drop(&mut self) {
                    if !self.committed {
                        std::fs::remove_file(&self.path).ok();
                    }
                }
            }
            let mut guard = OwnedOutput {
                path: destination.to_path_buf(),
                committed: false,
            };
            let mut input = tokio::fs::File::from_std(temp.reopen()?);
            let mut output = tokio::fs::File::from_std(output);
            tokio::io::copy(&mut input, &mut output).await?;
            output.flush().await?;
            output.sync_all().await?;
            guard.committed = true;
            Ok(())
        }
    }
}

fn unique_path(folder: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut path = folder.join(format!("{stem}.{ext}"));
    let mut number = 1;
    while path.symlink_metadata().is_ok() {
        path = folder.join(format!("{stem} ({number}).{ext}"));
        number += 1;
    }
    path
}

pub async fn run(
    mut media: Media,
    client: Client,
    options: Options,
    staging_dir: PathBuf,
    cancel: CancellationToken,
    limiter: Arc<Limiter>,
    on_progress: Arc<dyn Fn(PipelineProgress) + Send + Sync>,
) -> Result<Output> {
    ensure!(
        options.output_dir.is_absolute(),
        "Нужен абсолютный путь к папке загрузок"
    );
    let ext = options.selection.container.to_ascii_lowercase();
    ensure!(
        ["mp4", "mkv", "webm", "mp3", "m4a", "ogg", "flac", "wav"].contains(&ext.as_str()),
        "Неизвестный контейнер"
    );
    tokio::fs::create_dir_all(&staging_dir).await?;
    tokio::fs::create_dir_all(&options.output_dir).await?;
    prepare(&client, &mut media, &cancel).await?;
    let selected = choose_formats(&media.formats, &options.selection)?;
    let mut inputs = Vec::new();
    let mut downloaded = 0u64;
    let total = selected.iter().try_fold(0u64, |sum, format| {
        format.size.and_then(|size| sum.checked_add(size))
    });
    for (index, format) in selected.iter().enumerate() {
        let stream_downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let partial = staging_dir.join(format!("stream-{index}-{}.part", safe_name(&format.id)));
        let callback = {
            let callback = on_progress.clone();
            let base = downloaded;
            let count = selected.len();
            let stream_downloaded = stream_downloaded.clone();
            Arc::new(move |p: Progress| {
                stream_downloaded.store(p.downloaded, std::sync::atomic::Ordering::Relaxed);
                let fraction = p
                    .total
                    .filter(|n| *n > 0)
                    .map(|n| p.downloaded as f64 / n as f64)
                    .unwrap_or(0.0);
                let percent = total
                    .filter(|n| *n > 0)
                    .map(|n| 80.0 * (base + p.downloaded) as f64 / n as f64)
                    .unwrap_or(80.0 * (index as f64 + fraction) / count as f64);
                callback(PipelineProgress {
                    phase: "downloading".into(),
                    downloaded: base + p.downloaded,
                    total,
                    speed: p.speed,
                    percent: percent.clamp(0.0, 80.0),
                });
            }) as Arc<dyn Fn(Progress) + Send + Sync>
        };
        let path = match format.protocol {
            Protocol::Http => {
                transfer::download_with_cookies(
                    &client,
                    &format.url,
                    &format.headers,
                    &format.cookies,
                    &partial,
                    &cancel,
                    &limiter,
                    callback,
                )
                .await?;
                partial
            }
            Protocol::Hls => {
                adaptive::download_hls(
                    &client,
                    format,
                    &staging_dir.join(format!("hls-{index}")),
                    &cancel,
                    &limiter,
                    callback,
                )
                .await?
            }
            Protocol::Dash => {
                crate::dash::download_dash(
                    &client,
                    format,
                    &staging_dir.join(format!("dash-{index}")),
                    &cancel,
                    &limiter,
                    callback,
                )
                .await?
            }
        };
        downloaded += stream_downloaded
            .load(std::sync::atomic::Ordering::Relaxed)
            .max(tokio::fs::metadata(&path).await?.len());
        inputs.push(path);
    }
    let mut subtitles = Vec::new();
    for requested in &options.selection.subtitles {
        let (track, id) = crate::subtitles::select(&media.subtitles, requested)?;
        let raw = staging_dir.join(format!("subtitle-{}.raw", safe_name(&id)));
        transfer::download_with_cookies(
            &client,
            &track.url,
            &BTreeMap::new(),
            &media.cookies,
            &raw,
            &cancel,
            &limiter,
            Arc::new(|_| {}),
        )
        .await?;
        ensure!(
            tokio::fs::metadata(&raw).await?.len() <= 16 * 1024 * 1024,
            "Слишком большой файл субтитров"
        );
        let (data, suffix) = crate::subtitles::normalize(&tokio::fs::read(raw).await?, &track.ext)?;
        let path = staging_dir.join(format!("subtitle-{}.{}", safe_name(&id), suffix));
        tokio::fs::write(&path, data).await?;
        subtitles.push((track.language, id, path, suffix));
    }
    let mut args = Vec::new();
    for path in &inputs {
        args.extend([
            "-protocol_whitelist".into(),
            "file,pipe,crypto,data".into(),
            "-i".into(),
            path.to_string_lossy().into_owned(),
        ]);
    }
    if options.embed_subtitles {
        for (_, _, path, _) in &subtitles {
            args.extend([
                "-protocol_whitelist".into(),
                "file,pipe,crypto,data".into(),
                "-i".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
    }
    if options.selection.media_type == "audio" {
        args.extend(["-map".into(), "0:a:0".into(), "-vn".into()]);
        let mut source_codec = selected[0]
            .audio_codec
            .clone()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if options.selection.audio_bitrate == 0
            && (source_codec.is_empty() || source_codec == "unknown")
            && let Ok(info) = probe(&inputs[0]).await
        {
            source_codec = info["streams"]
                .as_array()
                .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "audio"))
                .and_then(|s| s["codec_name"].as_str())
                .unwrap_or("")
                .to_owned();
        }
        let compatible = match ext.as_str() {
            "mp3" => source_codec == "mp3",
            "m4a" | "mp4" => {
                source_codec.starts_with("aac")
                    || source_codec.starts_with("mp4a")
                    || ["alac", "ac3", "eac3"].contains(&source_codec.as_str())
            }
            "ogg" => ["opus", "vorbis", "flac"].contains(&source_codec.as_str()),
            "flac" => source_codec == "flac",
            "wav" => source_codec.starts_with("pcm_"),
            "webm" => ["opus", "vorbis"].contains(&source_codec.as_str()),
            "mkv" => true,
            _ => false,
        };
        let codec = if options.selection.audio_bitrate == 0 && compatible {
            "copy"
        } else {
            match ext.as_str() {
                "mp3" => "libmp3lame",
                "m4a" | "mp4" => "aac",
                "ogg" => "libvorbis",
                "webm" => "libopus",
                "flac" => "flac",
                "wav" => "pcm_s24le",
                _ => "copy",
            }
        };
        args.extend(["-c:a".into(), codec.into()]);
        if codec != "copy" && matches!(ext.as_str(), "mp3" | "m4a" | "mp4" | "ogg" | "webm") {
            let bitrate = options.selection.audio_bitrate;
            if bitrate > 0 {
                ensure!((32..=1536).contains(&bitrate), "Неверный битрейт аудио");
                ensure!(
                    ext != "mp3" || bitrate <= 320,
                    "MP3 поддерживает битрейт до320kbps"
                );
                ensure!(
                    ext != "webm" || bitrate <= 512,
                    "Opus поддерживает битрейт до512kbps"
                );
                args.extend(["-b:a".into(), format!("{bitrate}k")]);
            } else if codec == "libmp3lame" {
                args.extend(["-q:a".into(), "0".into()]);
            } else if codec == "libvorbis" {
                args.extend(["-q:a".into(), "10".into()]);
            } else {
                args.extend(["-b:a".into(), "320k".into()]);
            }
        }
    } else {
        args.extend(["-map".into(), "0:v:0".into()]);
        if inputs.len() == 1 {
            args.extend(["-map".into(), "0:a?".into()]);
        } else {
            if selected[0].has_audio()
                && options
                    .selection
                    .audio_tracks
                    .iter()
                    .any(|language| language == "default")
            {
                args.extend(["-map".into(), "0:a:0".into()]);
            }
            for index in 1..inputs.len() {
                args.extend(["-map".into(), format!("{index}:a:0")]);
            }
        }
        args.extend(["-c:v".into(), "copy".into(), "-c:a".into(), "copy".into()]);
    }
    if options.embed_subtitles {
        for (index, (language, id, _, _)) in subtitles.iter().enumerate() {
            args.extend([
                "-map".into(),
                format!("{}:s:0", inputs.len() + index),
                format!("-metadata:s:s:{index}"),
                format!("language={}", crate::subtitles::language_tag(language)),
                format!("-metadata:s:s:{index}"),
                format!("title={id}"),
            ]);
        }
        args.extend([
            "-c:s".into(),
            if ext == "mp4" || ext == "m4a" {
                "mov_text"
            } else if ext == "webm" {
                "webvtt"
            } else {
                "srt"
            }
            .into(),
        ]);
    }
    if options.tags {
        args.extend([
            "-metadata".into(),
            format!("title={}", media.title),
            "-metadata".into(),
            format!("artist={}", media.channel_title),
        ]);
    } else {
        args.extend(["-map_metadata".into(), "-1".into()]);
    }
    if ext == "mp4" || ext == "m4a" {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
    let assembled = staging_dir.join(format!("result.{ext}"));
    args.push(assembled.to_string_lossy().into_owned());
    on_progress(PipelineProgress {
        phase: "processing".into(),
        downloaded,
        total,
        speed: 0.0,
        percent: 90.0,
    });
    ffmpeg(args, media.duration, &cancel, on_progress.clone()).await?;
    let result = probe(&assembled).await?;
    let streams = result["streams"]
        .as_array()
        .context("В готовом файле нет медиапотоков")?;
    ensure!(
        streams
            .iter()
            .any(|s| s["codec_type"] == options.selection.media_type),
        "Готовый файл не содержит выбранный тип медиа"
    );
    ensure!(!cancel.is_cancelled(), "Загрузка приостановлена");
    let stem = options
        .filename
        .as_deref()
        .map(|f| {
            Path::new(f)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| format!("{} [{}]", media.title, media.id));
    let stem = safe_name(&stem);
    let mut destination = unique_path(&options.output_dir, &stem, &ext);
    #[cfg(windows)]
    ensure!(
        destination.as_os_str().len() < 245,
        "Слишком длинный путь. Выбери более короткую папку"
    );
    loop {
        ensure!(!cancel.is_cancelled(), "Загрузка приостановлена");
        match publish(&assembled, &destination).await {
            Ok(()) => break,
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) =>
            {
                destination = unique_path(&options.output_dir, &stem, &ext)
            }
            Err(error) => return Err(error),
        }
    }
    let mut files = vec![destination.clone()];
    if !options.embed_subtitles {
        for (_, id, path, suffix) in subtitles {
            let sidecar = unique_path(
                &options.output_dir,
                &format!("{stem}.{}", safe_name(&id)),
                suffix,
            );
            publish(&path, &sidecar).await?;
            files.push(sidecar);
        }
    }
    let size = tokio::fs::metadata(&destination).await?.len();
    on_progress(PipelineProgress {
        phase: "complete".into(),
        downloaded,
        total: Some(downloaded),
        speed: 0.0,
        percent: 100.0,
    });
    Ok(Output {
        path: destination,
        files,
        size,
        info: result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn dangling_symlink_is_an_occupied_output_name() {
        let folder = tempfile::tempdir().unwrap();
        let occupied = folder.path().join("clip.mp4");
        std::os::unix::fs::symlink("missing-target", &occupied).unwrap();
        assert_eq!(
            unique_path(folder.path(), "clip", "mp4"),
            folder.path().join("clip (1).mp4")
        );
        assert!(occupied.is_symlink());
    }
    #[test]
    fn path_names_never_escape_or_use_windows_devices() {
        for name in ["../evil", "NUL", "CON.txt", "a/b\\c", ""] {
            let result = safe_name(name);
            assert!(!result.contains(['/', '\\']));
            assert!(!["NUL", "CON.txt"].contains(&result.as_str()));
            assert!(!result.is_empty());
        }
    }
    #[tokio::test]
    async fn concurrent_publish_never_replaces_another_complete_file() {
        let folder = tempfile::tempdir().unwrap();
        let a = folder.path().join("a");
        let b = folder.path().join("b");
        let output = folder.path().join("output");
        tokio::fs::write(&a, b"first complete media").await.unwrap();
        tokio::fs::write(&b, b"second complete media")
            .await
            .unwrap();
        let (left, right) = tokio::join!(publish(&a, &output), publish(&b, &output));
        assert_ne!(left.is_ok(), right.is_ok());
        let bytes = tokio::fs::read(output).await.unwrap();
        assert!(bytes == b"first complete media" || bytes == b"second complete media");
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 3);
    }
}
