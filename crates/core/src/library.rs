use anyhow::{Context, Result, ensure};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};
use videodock_media::MediaKind;

#[derive(Clone, Debug)]
pub struct LibraryFile {
    pub path: PathBuf,
    pub title: String,
    pub kind: MediaKind,
    pub size: u64,
    pub duration: Option<f64>,
}

type Cache = HashMap<PathBuf, (u64, Option<SystemTime>, LibraryFile)>;
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

pub async fn scan(root: &Path) -> Result<Vec<LibraryFile>> {
    ensure!(root.is_absolute(), "Нужна абсолютная папка библиотеки");
    if !root.exists() {
        return Ok(vec![]);
    }
    let root = tokio::fs::canonicalize(root).await?;
    let mut directories = VecDeque::from([(root.clone(), 0usize)]);
    let mut output = Vec::new();
    let mut examined = 0;
    while let Some((directory, depth)) = directories.pop_front() {
        let mut entries = tokio::fs::read_dir(directory)
            .await
            .context("Не удалось прочитать папку библиотеки")?;
        while let Some(entry) = entries.next_entry().await? {
            examined += 1;
            ensure!(
                examined <= 250_000,
                "Слишком много записей в папке библиотеки"
            );
            let kind = entry.file_type().await?;
            if kind.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if kind.is_dir() {
                if depth < 24 {
                    directories.push_back((entry.path(), depth + 1));
                }
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ![
                "mp4", "mkv", "webm", "mov", "avi", "flv", "mp3", "m4a", "ogg", "opus", "flac",
                "wav", "aac",
            ]
            .contains(&ext.as_str())
            {
                continue;
            }
            let metadata = entry.metadata().await?;
            let size = metadata.len();
            if size == 0 {
                continue;
            }
            let modified = metadata.modified().ok();
            if modified
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .is_some_and(|age| age < Duration::from_secs(5))
            {
                continue;
            }
            let path = tokio::fs::canonicalize(&path).await?;
            if !path.starts_with(&root) {
                continue;
            }
            let cached = {
                CACHE
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .unwrap()
                    .get(&path)
                    .filter(|(old_size, old_time, _)| *old_size == size && *old_time == modified)
                    .map(|(_, _, file)| file.clone())
            };
            if let Some(file) = cached {
                output.push(file);
                continue;
            }
            let Ok(info) = crate::pipeline::probe(&path).await else {
                continue;
            };
            let Some(streams) = info["streams"].as_array() else {
                continue;
            };
            let kind = if streams.iter().any(|stream| stream["codec_type"] == "video") {
                MediaKind::Video
            } else if streams.iter().any(|stream| stream["codec_type"] == "audio") {
                MediaKind::Audio
            } else {
                continue;
            };
            let duration = info["format"]["duration"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|n| n.is_finite() && *n >= 0.0);
            let file = LibraryFile {
                title: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                path: path.clone(),
                kind,
                size,
                duration,
            };
            CACHE
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .insert(path, (size, modified, file.clone()));
            output.push(file);
        }
    }
    output.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(output)
}
