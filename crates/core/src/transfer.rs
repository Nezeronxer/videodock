use anyhow::{Context, Result, ensure};
use reqwest::{
    Client, Response, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_RANGE, ETAG},
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::Mutex, time::sleep};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: f64,
}

pub struct Limiter {
    limit: AtomicU64,
    next: Mutex<Instant>,
}
impl Limiter {
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            limit: AtomicU64::new(bytes_per_second),
            next: Mutex::new(Instant::now()),
        }
    }
    pub fn set(&self, bytes_per_second: u64) {
        self.limit.store(bytes_per_second, Ordering::Relaxed);
    }
    pub(crate) async fn wait(&self, bytes: u64, cancel: &CancellationToken) -> Result<()> {
        let limit = self.limit.load(Ordering::Relaxed);
        if limit == 0 {
            return Ok(());
        }
        let delay = {
            let mut next = self.next.lock().await;
            let now = Instant::now();
            let start = (*next).max(now);
            *next = start + Duration::from_secs_f64(bytes as f64 / limit as f64);
            start.saturating_duration_since(now)
        };
        tokio::select! {_ = cancel.cancelled()=>anyhow::bail!("Загрузка приостановлена"),_ = sleep(delay)=>Ok(())}
    }
}

pub async fn get_response(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> Result<Response> {
    get_response_with_cookies(client, url, headers, &[], cancel).await
}

pub async fn get_response_with_cookies(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cookies: &[videodock_media::ScopedCookie],
    cancel: &CancellationToken,
) -> Result<Response> {
    let source = videodock_media::validate_url(url)?;
    let mut current = source.clone();
    for _ in 0..10 {
        let scoped =
            videodock_media::request_headers(headers, source.as_str(), current.as_str(), cookies)?;
        let mut request = client.get(current.clone());
        for (name, value) in scoped {
            request = request.header(name, value);
        }
        let response = tokio::select! {_ = cancel.cancelled()=>anyhow::bail!("Загрузка приостановлена"),
        result=request.send()=>result.map_err(|error|anyhow::anyhow!("Ошибка соединения с медиасервером: {}",if error.is_timeout(){"таймаут"}else{"соединение прервано"}))?};
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .context("Переадресация без Location")?;
            current = videodock_media::validate_url(response.url().join(location)?.as_str())?;
        } else {
            return Ok(response);
        }
    }
    anyhow::bail!("Слишком много перенаправлений медиасервера")
}

pub async fn download(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    path: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    on_progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<()> {
    download_with_cookies(
        client,
        url,
        headers,
        &[],
        path,
        cancel,
        limiter,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)] // Request scope and cancellation stay explicit at this boundary.
pub async fn download_with_cookies(
    client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cookies: &[videodock_media::ScopedCookie],
    path: &Path,
    cancel: &CancellationToken,
    limiter: &Limiter,
    on_progress: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<()> {
    videodock_media::validate_url(url)?;
    let mut offset = tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or_default();
    let metadata_path = path.with_extension("http.json");
    let saved_etag = tokio::fs::read(&metadata_path)
        .await
        .ok()
        .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
        .and_then(|data| data["etag"].as_str().map(str::to_owned));
    let mut headers = headers.clone();
    headers.insert("Accept-Encoding".into(), "identity".into());
    if offset > 0 {
        headers.insert("Range".into(), format!("bytes={offset}-"));
        if let Some(etag) = &saved_etag {
            headers.insert("If-Range".into(), etag.clone());
        }
    }
    let mut response = get_response_with_cookies(client, url, &headers, cookies, cancel).await?;
    let status = response.status();
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        let remote = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("bytes */"))
            .and_then(|s| s.parse::<u64>().ok());
        ensure!(
            remote == Some(offset) && offset > 0,
            "Сервер не подтвердил размер частичного файла"
        );
        return Ok(());
    }
    ensure!(
        status.is_success(),
        "Медиасервер вернул HTTP {}",
        status.as_u16()
    );
    let append = offset > 0 && status == StatusCode::PARTIAL_CONTENT;
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|header| header.to_str().ok())
        .map(str::to_owned);
    if append {
        ensure!(
            saved_etag
                .as_ref()
                .zip(etag.as_ref())
                .is_none_or(|(old, new)| old == new),
            "Сервер изменил содержимое частичного файла"
        );
    }
    let total = if status == StatusCode::PARTIAL_CONTENT {
        let range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|h| h.to_str().ok())
            .context("Ответ 206 без Content-Range")?;
        let (span, total) = range
            .strip_prefix("bytes ")
            .and_then(|s| s.split_once('/'))
            .context("Неверный Content-Range")?;
        let (start, _end) = span.split_once('-').context("Неверный диапазон")?;
        ensure!(
            start.parse::<u64>()? == offset,
            "Сервер прислал другой диапазон, частичный файл сохранён"
        );
        total.parse::<u64>().ok()
    } else {
        offset = 0;
        response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse().ok())
    };
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(
        &metadata_path,
        serde_json::to_vec(&serde_json::json!({"etag":etag}))?,
    )
    .await?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .await?;
    let started = Instant::now();
    let original = offset;
    let mut last = Instant::now() - Duration::from_secs(1);
    loop {
        let chunk = tokio::select! {_ = cancel.cancelled()=>{file.flush().await?;anyhow::bail!("Загрузка приостановлена")},
        next=response.chunk()=>next.map_err(|_|anyhow::anyhow!("Поток медиа прерван"))?};
        let Some(chunk) = chunk else { break };
        limiter.wait(chunk.len() as u64, cancel).await?;
        file.write_all(&chunk).await?;
        offset += chunk.len() as u64;
        if last.elapsed() >= Duration::from_millis(200) {
            on_progress(Progress {
                downloaded: offset,
                total,
                speed: (offset - original) as f64 / started.elapsed().as_secs_f64().max(0.001),
            });
            last = Instant::now();
        }
    }
    file.flush().await?;
    ensure!(
        total.is_none_or(|total| total == offset),
        "Медиапоток завершился до получения всего файла"
    );
    on_progress(Progress {
        downloaded: offset,
        total,
        speed: (offset - original) as f64 / started.elapsed().as_secs_f64().max(0.001),
    });
    Ok(())
}
