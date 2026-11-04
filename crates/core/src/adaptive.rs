use crate::transfer::{self, Limiter, Progress};
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use anyhow::{Context, Result, bail, ensure};
use m3u8_rs::{AlternativeMediaType, KeyMethod, Playlist};
use reqwest::Client;
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};
use tokio_util::sync::CancellationToken;
use videodock_media::{Format, Media, MediaKind, Protocol, validate_url};

async fn bytes_response(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    limit: usize,
    range: Option<(u64, u64)>,
    cancel: &CancellationToken,
    cookies: &[videodock_media::ScopedCookie],
) -> Result<(Vec<u8>, String)> {
    validate_url(url)?;
    let mut headers = headers.clone();
    headers.insert("Accept-Encoding".into(), "identity".into());
    if let Some((start, len)) = range {
        ensure!(len > 0, "Пустой диапазон HLS");
        headers.insert(
            "Range".into(),
            format!(
                "bytes={start}-{}",
                start
                    .checked_add(len - 1)
                    .context("Слишком большой диапазон")?
            ),
        );
    }
    let mut response =
        transfer::get_response_with_cookies(client, url, &headers, cookies, cancel).await?;
    let final_url = response.url().to_string();
    ensure!(
        response.status().is_success(),
        "Медиасервер вернул HTTP {}",
        response.status().as_u16()
    );
    if let Some((start, _)) = range {
        ensure!(
            response.status() == reqwest::StatusCode::PARTIAL_CONTENT,
            "Сервер проигнорировал диапазон HLS"
        );
        ensure!(
            response
                .headers()
                .get("Content-Range")
                .and_then(|s| s.to_str().ok())
                .is_some_and(|s| s.starts_with(&format!("bytes {start}-"))),
            "Неверный диапазон HLS"
        );
    }
    let mut data = Vec::new();
    loop {
        let chunk = tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),chunk=response.chunk()=>chunk.map_err(|_|anyhow::anyhow!("Медиапоток прерван"))?};
        let Some(chunk) = chunk else { break };
        ensure!(
            data.len() + chunk.len() <= limit,
            "Ответ манифеста/сегмента превышает допустимый размер"
        );
        data.extend_from_slice(&chunk);
    }
    if let Some((_, len)) = range {
        ensure!(data.len() as u64 == len, "Неполный диапазон HLS");
    }
    Ok((data, final_url))
}

async fn bytes(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    limit: usize,
    range: Option<(u64, u64)>,
    cancel: &CancellationToken,
    cookies: &[videodock_media::ScopedCookie],
) -> Result<Vec<u8>> {
    Ok(
        bytes_response(client, url, headers, limit, range, cancel, cookies)
            .await?
            .0,
    )
}

fn playlist(data: &[u8]) -> Result<Playlist> {
    let text = std::str::from_utf8(data)?.trim_start_matches('\u{feff}');
    // m3u8-rs 6.0.1 rejects METHOD=NONE without IV; an unused IV preserves key clearing.
    let mut text = text
        .lines()
        .map(|line| {
            if line.trim() == "#EXT-X-KEY:METHOD=NONE" {
                "#EXT-X-KEY:METHOD=NONE,IV=0x0"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    let mut parsed = m3u8_rs::parse_playlist_res(text.as_bytes())
        .map_err(|_| anyhow::anyhow!("Неверный HLS-манифест"))?;
    if let Playlist::MediaPlaylist(media) = &mut parsed {
        // m3u8-rs reports KEY declarations only; HLS keys persist until replaced or METHOD=NONE.
        let mut key = None;
        for segment in &mut media.segments {
            if segment.key.is_some() {
                key = segment.key.clone();
            } else {
                segment.key = key.clone();
            }
        }
    }
    Ok(parsed)
}

pub async fn expand(client: &Client, media: &mut Media, cancel: &CancellationToken) -> Result<()> {
    let mut result = Vec::new();
    for format in &media.formats {
        if format.protocol != Protocol::Hls {
            result.push(format.clone());
            continue;
        }
        let (data, final_url) = bytes_response(
            client,
            &format.url,
            &format.headers,
            8 * 1024 * 1024,
            None,
            cancel,
            &format.cookies,
        )
        .await?;
        let base = validate_url(&final_url)?;
        match playlist(&data)? {
            Playlist::MasterPlaylist(master) => {
                for (index, variant) in master.variants.iter().filter(|v| !v.is_i_frame).enumerate()
                {
                    let mut entry = format.clone();
                    entry.id = format!("{}-{index}", format.id);
                    entry.url = base.join(&variant.uri)?.to_string();
                    entry.headers =
                        videodock_media::scoped_headers(&format.headers, &format.url, &entry.url)?;
                    entry.bitrate = Some(variant.bandwidth);
                    entry.height = variant
                        .resolution
                        .as_ref()
                        .and_then(|r| r.height.try_into().ok());
                    entry.width = variant
                        .resolution
                        .as_ref()
                        .and_then(|r| r.width.try_into().ok());
                    entry.fps = variant.frame_rate;
                    let codecs = variant
                        .codecs
                        .as_deref()
                        .unwrap_or("")
                        .split(',')
                        .map(str::trim)
                        .collect::<Vec<_>>();
                    entry.video_codec = codecs
                        .iter()
                        .find(|c| {
                            c.starts_with("avc")
                                || c.starts_with("hev")
                                || c.starts_with("hvc")
                                || c.starts_with("vp")
                                || c.starts_with("av01")
                        })
                        .map(|s| s.to_string());
                    if entry.video_codec.is_none() && media.kind != MediaKind::Audio {
                        entry.video_codec = Some("unknown".into());
                    }
                    let separate_audio = variant.audio.as_ref().is_some_and(|group| {
                        master
                            .alternatives
                            .iter()
                            .any(|a| &a.group_id == group && a.uri.is_some())
                    });
                    entry.audio_codec = if separate_audio {
                        None
                    } else {
                        codecs
                            .iter()
                            .find(|c| {
                                c.starts_with("mp4a")
                                    || c.starts_with("opus")
                                    || c.starts_with("ac-3")
                            })
                            .map(|s| s.to_string())
                            .or_else(|| Some("unknown".into()))
                    };
                    entry.ext = "ts".into();
                    result.push(entry);
                }
                for track in master
                    .alternatives
                    .iter()
                    .filter(|a| a.media_type == AlternativeMediaType::Audio)
                {
                    if let Some(uri) = &track.uri {
                        let mut entry = format.clone();
                        entry.id = format!("{}-audio-{}", format.id, track.name);
                        entry.url = base.join(uri)?.to_string();
                        entry.headers = videodock_media::scoped_headers(
                            &format.headers,
                            &format.url,
                            &entry.url,
                        )?;
                        entry.video_codec = None;
                        entry.audio_codec = Some("unknown".into());
                        entry.audio_language = track.language.clone();
                        entry.audio_title = Some(track.name.clone());
                        entry.ext = "ts".into();
                        result.push(entry);
                    }
                }
            }
            Playlist::MediaPlaylist(_) => {
                let mut entry = format.clone();
                if !entry.has_video() && !entry.has_audio() {
                    if media.kind != MediaKind::Audio {
                        entry.video_codec = Some("unknown".into());
                    }
                    entry.audio_codec = Some("unknown".into());
                }
                result.push(entry);
            }
        }
    }
    media.formats = result;
    Ok(())
}

fn iv(value: Option<&str>, sequence: u64) -> Result<[u8; 16]> {
    let Some(value) = value else {
        return Ok(u128::from(sequence).to_be_bytes());
    };
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    ensure!(!value.is_empty() && value.len() <= 32, "Неверный IV HLS");
    Ok(u128::from_str_radix(value, 16)?.to_be_bytes())
}

async fn decrypt(source: &Path, target: &Path, key: &[u8], iv: &[u8]) -> Result<()> {
    ensure!(
        key.len() == 16 && iv.len() == 16,
        "Неверный размер AES-128 key/IV"
    );
    let mut cipher = cbc::Decryptor::<aes::Aes128>::new_from_slices(key, iv)
        .map_err(|_| anyhow::anyhow!("Неверный AES-128 key/IV"))?;
    let mut input = BufReader::new(File::open(source).await?);
    let mut output = BufWriter::new(File::create(target).await?);
    let mut pending = None;
    loop {
        let mut block = [0u8; 16];
        let mut filled = 0;
        while filled < 16 {
            let n = input.read(&mut block[filled..]).await?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled == 0 {
            break;
        }
        ensure!(filled == 16, "Повреждённый блок AES-128");
        cipher.decrypt_block_mut((&mut block).into());
        if let Some(previous) = pending.replace(block) {
            output.write_all(&previous).await?;
        }
    }
    let last = pending.context("Пустой шифрованный сегмент")?;
    let padding = last[15] as usize;
    ensure!(
        (1..=16).contains(&padding) && last[16 - padding..].iter().all(|b| *b as usize == padding),
        "Неверный padding AES-128"
    );
    output.write_all(&last[..16 - padding]).await?;
    output.flush().await?;
    Ok(())
}

pub async fn download_hls(
    client: &Client,
    format: &Format,
    folder: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    on_progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(folder).await?;
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    let mut maps = HashSet::new();
    let mut previous_range_end = 0;
    let started = Instant::now();
    let mut downloaded = 0u64;
    loop {
        let (data, final_url) = bytes_response(
            client,
            &format.url,
            &format.headers,
            8 * 1024 * 1024,
            None,
            cancel,
            &format.cookies,
        )
        .await?;
        let base = validate_url(&final_url)?;
        let Playlist::MediaPlaylist(playlist) = playlist(&data)? else {
            bail!("HLS master должен быть раскрыт до начала загрузки")
        };
        let total_segments = playlist.segments.len();
        for (index, segment) in playlist.segments.iter().enumerate() {
            let sequence = playlist.media_sequence + index as u64;
            if !seen.insert(sequence) {
                continue;
            }
            if let Some(map) = &segment.map {
                let uri = base.join(&map.uri)?.to_string();
                if maps.insert(uri.clone()) {
                    let path = folder.join(format!("init-{}.bin", maps.len()));
                    let range = map
                        .byte_range
                        .as_ref()
                        .map(|r| (r.offset.unwrap_or(0), r.length));
                    let headers =
                        videodock_media::scoped_headers(&format.headers, &format.url, &uri)?;
                    let data = bytes(
                        client,
                        &uri,
                        &headers,
                        64 * 1024 * 1024,
                        range,
                        cancel,
                        &format.cookies,
                    )
                    .await?;
                    tokio::fs::write(&path, data).await?;
                    paths.push(path);
                }
            }
            let path = folder.join(format!("segment-{sequence}.bin"));
            if !path.is_file() {
                let part = folder.join(format!("segment-{sequence}.part"));
                let uri = base.join(&segment.uri)?.to_string();
                let headers = videodock_media::scoped_headers(&format.headers, &format.url, &uri)?;
                if let Some(range) = &segment.byte_range {
                    let start = range.offset.unwrap_or(previous_range_end);
                    previous_range_end = start
                        .checked_add(range.length)
                        .context("Диапазон HLS переполнен")?;
                    let data = bytes(
                        client,
                        &uri,
                        &headers,
                        256 * 1024 * 1024,
                        Some((start, range.length)),
                        cancel,
                        &format.cookies,
                    )
                    .await?;
                    limiter.wait(data.len() as u64, cancel).await?;
                    tokio::fs::write(&part, data).await?;
                } else {
                    transfer::download_with_cookies(
                        client,
                        &uri,
                        &headers,
                        &format.cookies,
                        &part,
                        cancel,
                        limiter,
                        Arc::new(|_| {}),
                    )
                    .await?;
                }
                match &segment.key {
                    Some(key) if key.method != KeyMethod::None => {
                        ensure!(
                            key.method == KeyMethod::AES128
                                && key.keyformat.as_deref().is_none_or(|v| v == "identity"),
                            "Защищённый/неподдерживаемый метод шифрования HLS"
                        );
                        let key_url =
                            base.join(key.uri.as_deref().context("Не указан ключ HLS")?)?;
                        let headers = videodock_media::scoped_headers(
                            &format.headers,
                            &format.url,
                            key_url.as_str(),
                        )?;
                        let key_bytes = bytes(
                            client,
                            key_url.as_str(),
                            &headers,
                            16,
                            None,
                            cancel,
                            &format.cookies,
                        )
                        .await?;
                        let decrypted = folder.join(format!("segment-{sequence}.decrypted.part"));
                        decrypt(
                            &part,
                            &decrypted,
                            &key_bytes,
                            &iv(key.iv.as_deref(), sequence)?,
                        )
                        .await?;
                        tokio::fs::rename(decrypted, &path).await?;
                    }
                    _ => {
                        tokio::fs::rename(&part, &path).await?;
                    }
                }
            } else if let Some(range) = &segment.byte_range {
                previous_range_end = range
                    .offset
                    .unwrap_or(previous_range_end)
                    .checked_add(range.length)
                    .context("Диапазон HLS переполнен")?;
            }
            downloaded += tokio::fs::metadata(&path).await?.len();
            paths.push(path);
            on_progress(Progress {
                downloaded,
                total: if playlist.end_list {
                    Some(downloaded.saturating_mul(total_segments as u64) / (index as u64 + 1))
                } else {
                    None
                },
                speed: downloaded as f64 / started.elapsed().as_secs_f64().max(0.001),
            });
        }
        if playlist.end_list {
            break;
        }
        tokio::select! {_ = cancel.cancelled()=>bail!("Загрузка приостановлена"),_ = tokio::time::sleep(std::time::Duration::from_secs((playlist.target_duration/2).max(1)))=>{}}
    }
    ensure!(!paths.is_empty(), "В HLS нет медиасегментов");
    let output = folder.join("assembled.ts");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&output)
        .await?;
    for path in paths {
        let mut input = File::open(path).await?;
        tokio::io::copy(&mut input, &mut file).await?;
    }
    file.flush().await?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hls_key_clear_and_sequence_iv_are_parsed() {
        let data=b"#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-KEY:METHOD=NONE\n#EXTINF:1,\na.ts\n#EXT-X-ENDLIST\n";
        assert!(matches!(
            playlist(data).unwrap(),
            Playlist::MediaPlaylist(_)
        ));
        assert_eq!(iv(None, 259).unwrap(), 259u128.to_be_bytes());
        assert!(iv(Some("ZZ"), 0).is_err());
    }
    #[test]
    fn hls_key_applies_to_following_segments_until_cleared_or_rotated() {
        let data = b"#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-KEY:METHOD=AES-128,URI=\"first.key\"\n#EXTINF:1,\na.ts\n#EXTINF:1,\nb.ts\n#EXT-X-KEY:METHOD=NONE\n#EXTINF:1,\nc.ts\n#EXTINF:1,\nd.ts\n#EXT-X-KEY:METHOD=AES-128,URI=\"second.key\"\n#EXTINF:1,\ne.ts\n#EXTINF:1,\nf.ts\n#EXT-X-ENDLIST\n";
        let Playlist::MediaPlaylist(media) = playlist(data).unwrap() else {
            panic!("not media")
        };
        for (segment, expected) in media.segments.iter().zip([
            Some("first.key"),
            Some("first.key"),
            None,
            None,
            Some("second.key"),
            Some("second.key"),
        ]) {
            assert!(segment.key.is_some(), "HLS key state must be inherited");
            let key = segment.key.as_ref().unwrap();
            assert_eq!(key.uri.as_deref(), expected);
            assert_eq!(
                key.method,
                if expected.is_some() {
                    KeyMethod::AES128
                } else {
                    KeyMethod::None
                }
            );
        }
    }
    #[tokio::test]
    async fn aes_cbc_decrypts_each_padding_length() {
        use aes::cipher::BlockEncryptMut;
        let dir = tempfile::tempdir().unwrap();
        let key = [7u8; 16];
        let iv = [3u8; 16];
        for length in 32..48 {
            let plain = vec![42u8; length];
            let cipher = cbc::Encryptor::<aes::Aes128>::new((&key).into(), (&iv).into())
                .encrypt_padded_vec_mut::<cbc::cipher::block_padding::Pkcs7>(&plain);
            let source = dir.path().join("source");
            let target = dir.path().join("target");
            tokio::fs::write(&source, cipher).await.unwrap();
            decrypt(&source, &target, &key, &iv).await.unwrap();
            assert_eq!(tokio::fs::read(target).await.unwrap(), plain);
        }
    }
    #[tokio::test]
    async fn aes_cbc_decrypts_a_large_fragment_without_truncation() {
        use aes::cipher::BlockEncryptMut;
        let dir = tempfile::tempdir().unwrap();
        let key = [7u8; 16];
        let iv = [3u8; 16];
        let plain = vec![42u8; 1024 * 1024 + 7];
        let cipher = cbc::Encryptor::<aes::Aes128>::new((&key).into(), (&iv).into())
            .encrypt_padded_vec_mut::<cbc::cipher::block_padding::Pkcs7>(&plain);
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        tokio::fs::write(&source, cipher).await.unwrap();
        let started = Instant::now();
        decrypt(&source, &target, &key, &iv).await.unwrap();
        println!("AES_1MiB_ms={}", started.elapsed().as_millis());
        assert_eq!(tokio::fs::read(target).await.unwrap(), plain);
    }
}
