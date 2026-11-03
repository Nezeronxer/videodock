use anyhow::{Context, Result, bail, ensure};
use bzip2::read::BzDecoder;
use reqwest::Client;
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::ffi::CString;
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

pub(super) const VERSION: &str = "2.0.1";
#[derive(Clone, Copy)]
pub(super) enum Family {
    Separation,
    Speech,
    Echo,
}
#[derive(Clone)]
pub(super) struct Installed {
    pub root: PathBuf,
    pub capi: PathBuf,
    pub ort: PathBuf,
    pub vocals: PathBuf,
    pub accompaniment: PathBuf,
    pub speech: PathBuf,
}
#[derive(Clone, Copy)]
struct Asset {
    url: &'static str,
    hash: &'static str,
    size: u64,
    name: &'static str,
}
const VOCALS: Asset = Asset {
    url: "https://huggingface.co/csukuangfj/sherpa-onnx-spleeter-2stems-fp16/resolve/93ba771920ade509f8cbd6825b1a90856c797e08/vocals.fp16.onnx",
    hash: "24cef84aedcd1fe87c0b743ef3370ad34dc1fabf6c9014d6128a75a538c7b668",
    size: 19_681_017,
    name: "vocals.fp16.onnx",
};
const ACCOMPANIMENT: Asset = Asset {
    url: "https://huggingface.co/csukuangfj/sherpa-onnx-spleeter-2stems-fp16/resolve/93ba771920ade509f8cbd6825b1a90856c797e08/accompaniment.fp16.onnx",
    hash: "d14cea55793cc531a5875f5f4da08207d1c5ab9292e8e0099a104eecb014fcc0",
    size: 19_681_024,
    name: "accompaniment.fp16.onnx",
};
const SPEECH: Asset = Asset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speech-enhancement-models/dpdfnet2.onnx",
    hash: "ce35d6025fc71df0ef10d1540e1b7916837bbfe5f6896deb744508d2cad487a9",
    size: 10_249_356,
    name: "dpdfnet2.onnx",
};
fn runtime() -> Option<Asset> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Asset {
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v2.0.1/sherpa-onnx-v2.0.1-linux-x64-shared-lib.tar.bz2",
            hash: "532c45bbd41036d83cd70606797529aa8086c2053395a58967736c727625b120",
            size: 9_710_131,
            name: "sherpa-runtime-linux-2.0.1.tar.bz2",
        },
        ("macos", "aarch64") => Asset {
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v2.0.1/sherpa-onnx-v2.0.1-osx-arm64-shared-lib.tar.bz2",
            hash: "e2d57ad9c83803d6c48f259caa66e44879bd4c4724c61f0d51a277b411bffb1d",
            size: 8_730_600,
            name: "sherpa-runtime-macos-arm64-2.0.1.tar.bz2",
        },
        ("macos", "x86_64") => Asset {
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v2.0.1/sherpa-onnx-v2.0.1-osx-x64-shared-lib.tar.bz2",
            hash: "be9042971c5d1b14da76a960534d2a8cdb6f0e5e02f5e1dc8bddff8576359bdd",
            size: 9_956_644,
            name: "sherpa-runtime-macos-x64-2.0.1.tar.bz2",
        },
        ("windows", "x86_64") => Asset {
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v2.0.1/sherpa-onnx-v2.0.1-win-x64-shared-MT-Release-lib.tar.bz2",
            hash: "afa4de3ca4ab0afd53bb4aab40d28ee15608364b549ffc0fe0caf6489bfc39a5",
            size: 7_899_712,
            name: "sherpa-runtime-windows-2.0.1.tar.bz2",
        },
        _ => return None,
    })
}
pub(super) fn supported() -> bool {
    runtime().is_some()
}
pub(super) fn root(data_dir: &Path) -> Result<PathBuf> {
    ensure!(
        data_dir.is_absolute() && data_dir.file_name().is_some(),
        "Нужна абсолютная отдельная папка данных MediaTools"
    );
    ensure!(
        data_dir
            .components()
            .all(|c| !matches!(c, Component::ParentDir | Component::CurDir)),
        "Недопустимый путь данных MediaTools"
    );
    let text = data_dir
        .to_str()
        .context("Путь моделей должен быть UTF-8")?;
    ensure!(
        !text.to_lowercase().contains("dropbox"),
        "Модели MediaTools должны храниться вне Dropbox"
    );
    let ancestor = data_dir
        .ancestors()
        .find(|p| p.exists())
        .context("Папка данных недоступна")?;
    ensure!(ancestor.is_dir(), "Путь данных проходит через файл");
    let canonical = ancestor
        .canonicalize()?
        .join(data_dir.strip_prefix(ancestor)?);
    ensure!(
        !canonical
            .to_string_lossy()
            .to_lowercase()
            .contains("dropbox"),
        "Модели MediaTools должны храниться вне Dropbox"
    );
    let root = canonical.join("models");
    safe_directory(&root)?;
    Ok(root)
}
fn safe_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => ensure!(
            m.is_dir() && !m.file_type().is_symlink(),
            "Недопустимая папка моделей MediaTools"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    };
    Ok(())
}
fn safe_file(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => ensure!(
            m.is_file() && !m.file_type().is_symlink(),
            "Недопустимый файл модели MediaTools"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    };
    Ok(())
}
// Darwin statvfs block counters are u32, while Linux counters and fragment sizes are u64.
#[allow(clippy::unnecessary_cast)]
pub(super) fn free_space(path: &Path) -> Result<u64> {
    let path = path
        .ancestors()
        .find(|p| p.exists())
        .context("Нельзя проверить свободное место")?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = CString::new(path.as_os_str().as_bytes())?;
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: name is terminated and stat points to valid writable storage.
        ensure!(
            unsafe { libc::statvfs(name.as_ptr(), stat.as_mut_ptr()) } == 0,
            "Не удалось проверить свободное место"
        );
        let stat = unsafe { stat.assume_init() };
        (stat.f_bavail as u64)
            .checked_mul(stat.f_frsize as u64)
            .context("Переполнение размера диска")
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                path: *const u16,
                available: *mut u64,
                total: *mut u64,
                free: *mut u64,
            ) -> i32;
        }
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut available = 0u64;
        ensure!(
            unsafe {
                GetDiskFreeSpaceExW(
                    name.as_ptr(),
                    &mut available,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } != 0,
            "Не удалось проверить свободное место"
        );
        Ok(available)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        bail!("Платформа не поддерживается");
    }
}
fn file_hash(path: &Path) -> Result<String> {
    safe_file(path)?;
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn matches(path: &Path, asset: Asset) -> Result<bool> {
    safe_file(path)?;
    Ok(fs::metadata(path).is_ok_and(|m| m.len() == asset.size) && file_hash(path)? == asset.hash)
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
async fn download(
    client: &Client,
    asset: Asset,
    destination: &Path,
    cancel: &CancellationToken,
    progress: Arc<dyn Fn(f64) + Send + Sync>,
) -> Result<()> {
    let path = destination.to_owned();
    if tokio::task::spawn_blocking(move || matches(&path, asset)).await?? {
        return Ok(());
    }
    ensure!(
        free_space(destination)? > asset.size + 64 * 1024 * 1024,
        "Недостаточно места для модели MediaTools"
    );
    let temporary =
        Temporary(destination.with_file_name(format!(".download-{}.part", uuid::Uuid::new_v4())));
    let mut response = tokio::select! {biased;_ = cancel.cancelled()=>bail!("Обработка отменена"),r=client.get(asset.url).header("Accept-Encoding","identity").send()=>r.map_err(|_|anyhow::anyhow!("Не удалось скачать модель MediaTools"))?};
    ensure!(
        response.status().is_success() && response.url().scheme() == "https",
        "Сервер моделей вернул недопустимый ответ"
    );
    ensure!(
        response.content_length().is_none_or(|n| n == asset.size),
        "Размер модели не совпал с опубликованным"
    );
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary.0)
        .await?;
    let mut received = 0u64;
    let mut hash = Sha256::new();
    loop {
        let chunk = tokio::select! {biased;_ = cancel.cancelled()=>{file.flush().await?;bail!("Обработка отменена")},r=response.chunk()=>match r{Ok(chunk)=>chunk,Err(_)=>{file.flush().await?;bail!("Скачивание модели прервано")}}};
        let Some(chunk) = chunk else { break };
        received = received
            .checked_add(chunk.len() as u64)
            .context("Слишком большая модель")?;
        ensure!(
            received <= asset.size,
            "Модель превышает опубликованный размер"
        );
        hash.update(&chunk);
        file.write_all(&chunk).await?;
        progress(received as f64 / asset.size as f64);
    }
    file.flush().await?;
    drop(file);
    ensure!(
        received == asset.size && format!("{:x}", hash.finalize()) == asset.hash,
        "SHA256 модели не совпал с опубликованным; установка отклонена"
    );
    safe_file(destination)?;
    tokio::fs::rename(&temporary.0, destination).await?;
    Ok(())
}
fn library_names() -> (&'static str, &'static str) {
    match std::env::consts::OS {
        "windows" => ("sherpa-onnx-c-api.dll", "onnxruntime.dll"),
        "macos" => ("libsherpa-onnx-c-api.dylib", "libonnxruntime.dylib"),
        _ => ("libsherpa-onnx-c-api.so", "libonnxruntime.so"),
    }
}
fn extract_runtime(
    archive: &Path,
    asset: Asset,
    destination: &Path,
    cancel: &CancellationToken,
) -> Result<()> {
    ensure!(
        matches(archive, asset)?,
        "Runtime SHA256 не подтверждён до распаковки"
    );
    safe_directory(destination)?;
    fs::create_dir_all(destination)?;
    let mut tar = tar::Archive::new(BzDecoder::new(fs::File::open(archive)?));
    let mut total = 0u64;
    let mut seen = HashSet::new();
    let names = library_names();
    for entry in tar.entries()? {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        let entry = entry?;
        let path = entry.path()?.into_owned();
        ensure!(
            path.components().all(|c| matches!(c, Component::Normal(_)))
                && !path.to_string_lossy().contains(['\\', ':'])
                && path.as_os_str().len() < 4096,
            "Небезопасный путь в архиве runtime"
        );
        if entry.header().entry_type().is_dir() {
            continue;
        }
        ensure!(
            entry.header().entry_type().is_file(),
            "Ссылки/специальные файлы в runtime-архиве запрещены"
        );
        let size = entry.size();
        total = total
            .checked_add(size)
            .context("Слишком большой runtime-архив")?;
        ensure!(
            size <= 128 * 1024 * 1024 && total <= 256 * 1024 * 1024,
            "Размер распакованного runtime превышает предел"
        );
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Неверное имя runtime")?;
        if !filename.ends_with(".dll")
            && !filename.ends_with(".so")
            && !filename.ends_with(".dylib")
        {
            continue;
        }
        ensure!(
            seen.insert(filename.to_owned()),
            "Повторяющаяся библиотека в runtime"
        );
        let mut bytes = Vec::with_capacity(size as usize);
        entry.take(size + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 == size, "Повреждённый runtime-архив");
        let expected = format!("{:x}", Sha256::digest(&bytes));
        let target = destination.join(filename);
        safe_file(&target)?;
        if target.exists() && file_hash(&target)? == expected {
            continue;
        }
        let temporary =
            Temporary(destination.join(format!(".library-{}.part", uuid::Uuid::new_v4())));
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary.0)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary.0, &target)?;
    }
    ensure!(
        seen.contains(names.0) && seen.contains(names.1),
        "В runtime нет необходимых C API/ONNX библиотек"
    );
    Ok(())
}
const NOTICE: &str = "MediaTools third-party provenance\nSherpa ONNX 2.0.1: Apache-2.0, https://github.com/k2-fsa/sherpa-onnx/tree/v2.0.1\nONNX Runtime 1.28.2: MIT, https://github.com/microsoft/onnxruntime/tree/v1.28.2\nSpleeter: Deezer MIT-licensed project, https://github.com/deezer/spleeter; converted pretrained weights published by official sherpa export workflow to csukuangfj/sherpa-onnx-spleeter-2stems-fp16, pinned revision93ba771920ade509f8cbd6825b1a90856c797e08. The converted model repository has no separate model-card license declaration.\nDPDFNet2: Apache-2.0 model card https://huggingface.co/Ceva-IP/DPDFNet, code https://github.com/ceva-ip/DPDFNet\nRust WPE algorithm follows published long-term weighted prediction equations; reference NARA-WPE (MIT), Yoshioka/Nakatani2012 and Drude et al.2018, https://github.com/fgnt/nara_wpe\nNo user audio is uploaded.\n";
pub(super) async fn ensure_installed(
    data_dir: &Path,
    family: Family,
    cancel: &CancellationToken,
    progress: Arc<dyn Fn(f64) + Send + Sync>,
) -> Result<Installed> {
    let root = root(data_dir)?;
    ensure!(supported(), "Нет CPU runtime MediaTools для этой платформы");
    ensure!(!cancel.is_cancelled(), "Обработка отменена");
    ensure!(
        free_space(&root)? > 256 * 1024 * 1024,
        "Недостаточно места для моделей MediaTools"
    );
    tokio::fs::create_dir_all(&root).await?;
    let archive_dir = root.join(".archives");
    safe_directory(&archive_dir)?;
    tokio::fs::create_dir_all(&archive_dir).await?;
    let lock_path = root.join("install.lock");
    safe_file(&lock_path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
        }
        tokio::select! {biased;_ = cancel.cancelled()=>bail!("Обработка отменена"),_ = tokio::time::sleep(Duration::from_millis(150))=>{}}
    }
    let runtime_root = root.join(format!(
        "sherpa-{VERSION}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    safe_directory(&runtime_root)?;
    let names = library_names();
    let installed = Installed {
        root: root.clone(),
        capi: runtime_root.join(names.0),
        ort: runtime_root.join(names.1),
        vocals: root.join(VOCALS.name),
        accompaniment: root.join(ACCOMPANIMENT.name),
        speech: root.join(SPEECH.name),
    };
    if !matches!(family, Family::Echo) {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(600))
            .redirect(reqwest::redirect::Policy::limited(8))
            .build()?;
        let runtime = runtime().context("Платформа не поддерживается")?;
        let archive = archive_dir.join(runtime.name);
        let models = if matches!(family, Family::Separation) {
            vec![VOCALS, ACCOMPANIMENT]
        } else {
            vec![SPEECH]
        };
        let total_bytes = runtime.size + models.iter().map(|m| m.size).sum::<u64>();
        let callback = {
            let progress = progress.clone();
            Arc::new(move |p| progress(p * runtime.size as f64 / total_bytes as f64))
                as Arc<dyn Fn(f64) + Send + Sync>
        };
        download(&client, runtime, &archive, cancel, callback).await?;
        let asset_path = archive.clone();
        let target = runtime_root.clone();
        let token = cancel.clone();
        tokio::task::spawn_blocking(move || extract_runtime(&asset_path, runtime, &target, &token))
            .await??;
        let mut complete = runtime.size;
        progress(complete as f64 / total_bytes as f64);
        for model in &models {
            let callback = {
                let progress = progress.clone();
                let size = model.size;
                Arc::new(move |p| {
                    progress((complete as f64 + p * size as f64) / total_bytes as f64)
                }) as Arc<dyn Fn(f64) + Send + Sync>
            };
            download(&client, *model, &root.join(model.name), cancel, callback).await?;
            complete += model.size;
            progress(complete as f64 / total_bytes as f64);
        }
        let pins = models
            .iter()
            .chain(std::iter::once(&runtime))
            .map(|a| serde_json::json!({"url":a.url,"sha256":a.hash,"bytes":a.size,"name":a.name}))
            .collect::<Vec<_>>();
        let path = root.join(match family {
            Family::Separation => "source-separation-pins.json",
            _ => "speech-pins.json",
        });
        safe_file(&path)?;
        tokio::fs::write(path, serde_json::to_vec_pretty(&pins)?).await?;
    }
    for (name, content) in [
        ("THIRD_PARTY.md", NOTICE),
        (
            "Sherpa-DPDFNet-Apache-2.0.txt",
            include_str!("notices/Apache-2.0.txt"),
        ),
        (
            "ONNXRuntime-LICENSE.txt",
            include_str!("notices/ONNXRuntime-LICENSE.txt"),
        ),
        (
            "Spleeter-LICENSE.txt",
            include_str!("notices/Spleeter-LICENSE.txt"),
        ),
        (
            "NaraWPE-LICENSE.txt",
            include_str!("notices/NaraWPE-LICENSE.txt"),
        ),
    ] {
        let path = root.join(name);
        safe_file(&path)?;
        tokio::fs::write(path, content).await?;
    }
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(path: &Path, unsafe_path: bool, link: bool) -> Asset {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_path("runtime/lib/fixture.so").unwrap();
        header.set_size(if link { 0 } else { 3 });
        header.set_entry_type(if link {
            tar::EntryType::Symlink
        } else {
            tar::EntryType::Regular
        });
        if link {
            header.set_link_name("../outside").unwrap();
        }
        if unsafe_path {
            let name = b"../outside";
            header.as_mut_bytes()[..100].fill(0);
            header.as_mut_bytes()[..name.len()].copy_from_slice(name);
        }
        header.set_cksum();
        let file = fs::File::create(path).unwrap();
        let mut output = bzip2::write::BzEncoder::new(file, bzip2::Compression::fast());
        output.write_all(header.as_bytes()).unwrap();
        if !link {
            output.write_all(b"abc").unwrap();
            output.write_all(&[0; 509]).unwrap();
        }
        output.write_all(&[0; 1024]).unwrap();
        output.finish().unwrap();
        Asset {
            url: "https://example.invalid/fixture",
            hash: Box::leak(file_hash(path).unwrap().into_boxed_str()),
            size: fs::metadata(path).unwrap().len(),
            name: "fixture.tar.bz2",
        }
    }
    #[test]
    fn published_digest_mismatch_prevents_any_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("runtime.bz2");
        let mut asset = archive(&path, false, false);
        asset.hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let destination = dir.path().join("installed");
        assert!(extract_runtime(&path, asset, &destination, &CancellationToken::new()).is_err());
        assert!(!destination.exists());
    }
    #[test]
    fn tar_traversal_and_links_are_rejected_after_digest_verification() {
        for (path_attack, link) in [(true, false), (false, true)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("runtime.bz2");
            let asset = archive(&path, path_attack, link);
            let destination = dir.path().join("installed");
            assert!(
                extract_runtime(&path, asset, &destination, &CancellationToken::new()).is_err()
            );
            assert!(!dir.path().join("outside").exists());
        }
    }
    #[tokio::test]
    async fn waiting_for_another_installer_is_cancellable_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let models = dir.path().join("models");
        fs::create_dir(&models).unwrap();
        let hold = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(models.join("install.lock"))
            .unwrap();
        hold.try_lock().unwrap();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            token.cancel();
        });
        let started = std::time::Instant::now();
        assert!(
            ensure_installed(dir.path(), Family::Echo, &cancel, Arc::new(|_| {}))
                .await
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    #[cfg(unix)]
    #[test]
    fn symlinked_model_roots_and_model_files_cannot_redirect_writes() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        fs::create_dir(&data).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, data.join("models")).unwrap();
        assert!(root(&data).is_err());
        let target = outside.join("protected");
        fs::write(&target, b"unchanged").unwrap();
        let alias = dir.path().join("model.onnx");
        symlink(&target, &alias).unwrap();
        assert!(safe_file(&alias).is_err());
        assert_eq!(fs::read(target).unwrap(), b"unchanged");
    }
}
