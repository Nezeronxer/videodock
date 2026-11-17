//! Stable GitHub release updates; download verification never depends on a filename alone.
use anyhow::{Context, Result, ensure};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[cfg(any(target_os = "macos", all(test, unix)))]
pub mod macos;

#[derive(Clone, Copy)]
enum Target {
    Windows,
    MacosX64,
    MacosArm64,
}
impl Target {
    fn current() -> Self {
        if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                Self::MacosArm64
            } else {
                Self::MacosX64
            }
        } else {
            // Keep Linux's existing manual release checks; automatic installation is disabled there.
            Self::Windows
        }
    }
    fn asset(self, version: &str) -> String {
        let suffix = match self {
            Self::Windows => "windows-x64-Setup.exe",
            Self::MacosX64 => "macos-x64.zip",
            Self::MacosArm64 => "macos-arm64.zip",
        };
        format!("VideoDock-{version}-{suffix}")
    }
    fn cache_path(self, directory: &Path, number: &str) -> Result<PathBuf> {
        version(number)?;
        let name = match self {
            Self::Windows => format!("VideoDock-{number}-Setup.exe"),
            _ => self.asset(number),
        };
        Ok(directory.join("updates").join(name))
    }
}
pub(crate) fn automatic_updates_supported() -> bool {
    cfg!(windows)
        || cfg!(all(
            target_os = "macos",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))
}
pub fn cache_path(directory: &Path, number: &str) -> Result<PathBuf> {
    Target::current().cache_path(directory, number)
}

const RELEASES: &str = "https://api.github.com/repos/Nezeronxer/videodock/releases/latest";
const MAX_INSTALLER: u64 = 512 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct Update {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
fn version(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .collect();
    ensure!(parts.len() == 3, "Некорректная версия обновления");
    let mut result = [0; 3];
    for (index, part) in parts.into_iter().enumerate() {
        ensure!(
            !part.is_empty() && part.len() <= 10 && part.bytes().all(|b| b.is_ascii_digit()),
            "Некорректная версия обновления"
        );
        result[index] = part.parse()?;
    }
    Ok(result)
}
fn release(data: &Value, current: &str) -> Result<Option<Update>> {
    release_for(data, current, Target::current())
}
fn release_for(data: &Value, current: &str, target: Target) -> Result<Option<Update>> {
    let tag = data["tag_name"]
        .as_str()
        .context("Не указана версия обновления")?;
    if data["draft"] == true || data["prerelease"] == true {
        return Ok(None);
    }
    if version(tag)? <= version(current)? {
        return Ok(None);
    }
    let number = tag.strip_prefix('v').unwrap_or(tag);
    let expected = target.asset(number);
    let assets = data["assets"].as_array().context("Нет файлов обновления")?;
    let matching: Vec<_> = assets
        .iter()
        .filter(|asset| asset["name"] == expected)
        .collect();
    ensure!(
        matching.len() == 1,
        "Не найден единственный пакет обновления для платформы"
    );
    let asset = matching[0];
    let url = asset["browser_download_url"]
        .as_str()
        .context("Нет адреса установщика")?;
    ensure!(
        url == format!(
            "https://github.com/Nezeronxer/videodock/releases/download/{tag}/{expected}"
        ),
        "Неверный источник установщика"
    );
    let digest = asset["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .context("Нет контрольной суммы установщика")?;
    ensure!(
        digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
        "Некорректная контрольная сумма"
    );
    let bytes = asset["size"].as_u64().context("Нет размера установщика")?;
    ensure!(
        bytes > 0 && bytes <= MAX_INSTALLER,
        "Недопустимый размер установщика"
    );
    Ok(Some(Update {
        version: number.into(),
        url: url.into(),
        sha256: digest.to_ascii_lowercase(),
        bytes,
    }))
}
fn trusted_url(value: &str) -> Result<url::Url> {
    let url = url::Url::parse(value)?;
    ensure!(
        url.scheme() == "https"
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
            && matches!(
                url.host_str(),
                Some(
                    "api.github.com"
                        | "github.com"
                        | "release-assets.githubusercontent.com"
                        | "objects.githubusercontent.com"
                        | "github-releases.githubusercontent.com"
                )
            ),
        "Неверный сервер обновлений"
    );
    Ok(url)
}
async fn get(client: &Client, value: &str) -> Result<Response> {
    let mut url = trusted_url(value)?;
    for _ in 0..6 {
        let response = client
            .get(url.clone())
            .header(
                "User-Agent",
                concat!("VideoDock/", env!("CARGO_PKG_VERSION")),
            )
            .timeout(Duration::from_secs(180))
            .send()
            .await?;
        if response.status().is_redirection() {
            let target = response
                .headers()
                .get(reqwest::header::LOCATION)
                .context("Нет адреса перенаправления")?
                .to_str()?;
            url = trusted_url(url.join(target)?.as_str())?;
        } else {
            return Ok(response.error_for_status()?);
        }
    }
    anyhow::bail!("Слишком много перенаправлений обновления")
}
async fn body(mut response: Response, maximum: usize) -> Result<Vec<u8>> {
    ensure!(
        response
            .content_length()
            .is_none_or(|size| size <= maximum as u64),
        "Ответ обновлений слишком велик"
    );
    let mut result = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            result.len().saturating_add(chunk.len()) <= maximum,
            "Ответ обновлений слишком велик"
        );
        result.extend_from_slice(&chunk);
    }
    Ok(result)
}
pub async fn check(client: &Client) -> Result<Option<Update>> {
    let data: Value =
        serde_json::from_slice(&body(get(client, RELEASES).await?, 1024 * 1024).await?)?;
    release(&data, env!("CARGO_PKG_VERSION"))
}
async fn verified_body(
    mut response: Response,
    update: &Update,
    file: &mut tokio::fs::File,
) -> Result<()> {
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    while let Some(chunk) = response.chunk().await? {
        bytes = bytes
            .checked_add(chunk.len() as u64)
            .context("Размер обновления переполнен")?;
        ensure!(
            bytes <= update.bytes && bytes <= MAX_INSTALLER,
            "Размер установщика не совпал"
        );
        hash.update(&chunk);
        file.write_all(&chunk).await?;
    }
    ensure!(
        bytes == update.bytes && format!("{:x}", hash.finalize()) == update.sha256,
        "Проверка установщика не пройдена"
    );
    file.sync_all().await?;
    Ok(())
}
pub async fn download(
    client: &Client,
    update: &Update,
    directory: &Path,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    let destination = cache_path(directory, &update.version)?;
    let directory = directory.join("updates");
    tokio::fs::create_dir_all(&directory).await?;
    // A private temporary file is removed on cancellation or error; only verified bytes become an installer.
    let temporary = tempfile::NamedTempFile::new_in(&directory)?;
    let mut file = tokio::fs::File::from_std(temporary.reopen()?);
    tokio::select! {
        _ = cancel.cancelled() => anyhow::bail!("Обновление отменено"),
        result = async { verified_body(get(client, &update.url).await?, update, &mut file).await } => result?,
    }
    drop(file);
    temporary.persist(&destination).map_err(|e| e.error)?;
    Ok(destination)
}
pub fn verify_file(path: &Path, update: &Update) -> Result<()> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.len() == update.bytes,
        "Размер установщика изменился"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == update.sha256,
        "Контрольная сумма установщика изменилась"
    );
    Ok(())
}
pub(crate) fn cached(directory: &Path, value: &Value) -> Option<Update> {
    cached_for(directory, value, Target::current())
}
fn cached_for(directory: &Path, value: &Value, target: Target) -> Option<Update> {
    if value["status"] != "ready" {
        return None;
    }
    let update: Update = serde_json::from_value(value.clone()).ok()?;
    if version(&update.version).ok()? <= version(env!("CARGO_PKG_VERSION")).ok()? {
        return None;
    }
    let expected = target.asset(&update.version);
    if ![
        format!(
            "https://github.com/Nezeronxer/videodock/releases/download/v{}/{}",
            update.version, expected
        ),
        format!(
            "https://github.com/Nezeronxer/videodock/releases/download/{}/{}",
            update.version, expected
        ),
    ]
    .contains(&update.url)
        || update.bytes == 0
        || update.bytes > MAX_INSTALLER
    {
        return None;
    }
    let path = target.cache_path(directory, &update.version).ok()?;
    verify_file(&path, &update).ok()?;
    Some(update)
}
#[cfg(windows)]
pub fn install_on_exit(path: &Path, update: &Update) -> Result<()> {
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    verify_file(path, update)?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let directory = executable.parent().context("Нет папки программы")?;
    let quote = |p: &Path| {
        format!(
            "'{}'",
            p.to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('\'', "''")
        )
    };
    let script = format!(
        "$ErrorActionPreference='Stop'; Write-Output 'Waiting for application exit'; Wait-Process -Id {} -Timeout 120 -ErrorAction SilentlyContinue; if(Get-Process -Id {} -ErrorAction SilentlyContinue){{throw 'Application did not exit'}}; Write-Output 'Starting installer'; $p=Start-Process -FilePath {} -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/CLOSEAPPLICATIONS',('/LOG='+[char]34+{}+[char]34),('/DIR='+[char]34+{}+[char]34)) -Wait -PassThru; Write-Output ('Installer exit code: '+$p.ExitCode); if($p.ExitCode -ne 0){{throw 'Installer failed'}}; Start-Process -FilePath {}; Write-Output 'Application restart requested'",
        std::process::id(),
        std::process::id(),
        quote(path),
        quote(&path.with_extension("setup.log")),
        quote(directory),
        quote(&executable)
    );
    let system = std::env::var_os("SystemRoot").context("Нет системной папки Windows")?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.with_extension("install.log"))?;
    Command::new(PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .creation_flags(0x08000000)
        .spawn()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_for(target: Target) -> Value {
        let name = target.asset("0.3.0");
        serde_json::json!({"tag_name":"v0.3.0","draft":false,"prerelease":false,"assets":[{"name":name,"browser_download_url":format!("https://github.com/Nezeronxer/videodock/releases/download/v0.3.0/{name}"),"digest":format!("sha256:{}", "ab".repeat(32)),"size":123}]})
    }
    fn fixture() -> Value {
        fixture_for(Target::current())
    }
    #[test]
    fn macos_release_assets_are_selected_and_verified_for_both_architectures() {
        for target in [Target::Windows, Target::MacosX64, Target::MacosArm64] {
            let data = fixture_for(target);
            let update = release_for(&data, "0.2.9", target).unwrap().unwrap();
            assert_eq!(update.url, data["assets"][0]["browser_download_url"]);
            assert_eq!(update.version, "0.3.0");
            assert_eq!(update.bytes, 123);
            assert_eq!(update.sha256, "ab".repeat(32));
            for other in [Target::Windows, Target::MacosX64, Target::MacosArm64] {
                if target.asset("0.3.0") != other.asset("0.3.0") {
                    assert!(release_for(&data, "0.2.9", other).is_err());
                }
            }
            for (key, value) in [
                (
                    "browser_download_url",
                    serde_json::json!(
                        "https://github.com/Nezeronxer/videodock/releases/download/v0.3.0/other.zip"
                    ),
                ),
                ("digest", serde_json::json!("sha256:invalid")),
                ("size", serde_json::json!(0)),
                ("size", serde_json::json!(MAX_INSTALLER + 1)),
            ] {
                let mut bad = data.clone();
                bad["assets"][0][key] = value;
                assert!(release_for(&bad, "0.2.9", target).is_err());
            }
        }
    }
    #[test]
    fn platform_cache_paths_preserve_windows_name_and_isolate_mac_architectures() {
        let directory = Path::new("profile");
        for (target, name) in [
            (Target::Windows, "VideoDock-0.3.0-Setup.exe"),
            (Target::MacosX64, "VideoDock-0.3.0-macos-x64.zip"),
            (Target::MacosArm64, "VideoDock-0.3.0-macos-arm64.zip"),
        ] {
            assert_eq!(
                target.cache_path(directory, "0.3.0").unwrap(),
                directory.join("updates").join(name)
            );
            assert!(target.cache_path(directory, "../bad").is_err());
        }
    }
    #[test]
    fn each_platform_recovers_only_its_own_verified_cache() {
        for target in [Target::Windows, Target::MacosX64, Target::MacosArm64] {
            let directory = tempfile::tempdir().unwrap();
            std::fs::create_dir(directory.path().join("updates")).unwrap();
            let path = target.cache_path(directory.path(), "999.0.0").unwrap();
            std::fs::write(&path, b"fixture").unwrap();
            let name = target.asset("999.0.0");
            let good = serde_json::json!({"status":"ready","version":"999.0.0","url":format!("https://github.com/Nezeronxer/videodock/releases/download/v999.0.0/{name}"),"bytes":7,"sha256":format!("{:x}", Sha256::digest(b"fixture"))});
            assert!(cached_for(directory.path(), &good, target).is_some());
            let mut unprefixed = good.clone();
            unprefixed["url"] = serde_json::json!(format!(
                "https://github.com/Nezeronxer/videodock/releases/download/999.0.0/{name}"
            ));
            assert!(cached_for(directory.path(), &unprefixed, target).is_some());
            for other in [Target::Windows, Target::MacosX64, Target::MacosArm64] {
                if name != other.asset("999.0.0") {
                    assert!(cached_for(directory.path(), &good, other).is_none());
                }
            }
            for (key, value) in [
                ("sha256", serde_json::json!("ab".repeat(32))),
                ("bytes", serde_json::json!(8)),
                ("status", serde_json::json!("available")),
                ("version", serde_json::json!("0.0.0")),
                ("version", serde_json::json!("../999")),
            ] {
                let mut bad = good.clone();
                bad[key] = value;
                assert!(cached_for(directory.path(), &bad, target).is_none());
            }
            std::fs::write(path, b"changed").unwrap();
            assert!(cached_for(directory.path(), &good, target).is_none());
        }
    }
    #[test]
    fn stable_updates_require_newer_version_exact_asset_and_digest() {
        assert!(release(&fixture(), "0.2.9").unwrap().is_some());
        for current in ["0.3.0", "0.4.0", "1.0.0"] {
            assert!(release(&fixture(), current).unwrap().is_none());
        }
        for (key, value) in [
            ("draft", Value::Bool(true)),
            ("prerelease", Value::Bool(true)),
        ] {
            let mut data = fixture();
            data[key] = value;
            assert!(release(&data, "0.2.0").unwrap().is_none());
        }
        for (key, value) in [
            ("digest", Value::Null),
            ("digest", serde_json::json!("sha256:invalid")),
            ("size", serde_json::json!(0)),
            ("size", serde_json::json!(MAX_INSTALLER + 1)),
            (
                "browser_download_url",
                serde_json::json!("https://example.org/setup.exe"),
            ),
        ] {
            let mut data = fixture();
            data["assets"][0][key] = value;
            assert!(release(&data, "0.2.0").is_err());
        }
        let mut duplicate = fixture();
        let asset = duplicate["assets"][0].clone();
        duplicate["assets"].as_array_mut().unwrap().push(asset);
        assert!(release(&duplicate, "0.2.0").is_err());
        for bad in ["0.3", "0.3.0-beta", "../0.3.0", "0.3.0.1"] {
            assert!(version(bad).is_err());
        }
        for bad in [
            "http://github.com/",
            "https://github.com.evil.test/",
            "https://user@github.com/",
            "https://github.com:444/",
        ] {
            assert!(trusted_url(bad).is_err());
        }
    }
    #[test]
    fn installer_is_rechecked_before_launch() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"fixture").unwrap();
        let update = Update {
            version: "0.3.0".into(),
            url: String::new(),
            sha256: format!("{:x}", Sha256::digest(b"fixture")),
            bytes: 7,
        };
        verify_file(temp.path(), &update).unwrap();
        std::fs::write(temp.path(), b"changed").unwrap();
        assert!(verify_file(temp.path(), &update).is_err());
    }

    #[test]
    fn cached_ready_update_is_recovered_only_after_version_source_and_hash_checks() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("updates")).unwrap();
        let path = cache_path(directory.path(), "999.0.0").unwrap();
        std::fs::write(&path, b"fixture").unwrap();
        let name = Target::current().asset("999.0.0");
        let good = serde_json::json!({"status":"ready","version":"999.0.0","url":format!("https://github.com/Nezeronxer/videodock/releases/download/v999.0.0/{name}"),"bytes":7,"sha256":format!("{:x}", Sha256::digest(b"fixture"))});
        assert!(cached(directory.path(), &good).is_some());
        for (key, value) in [
            ("version", serde_json::json!("../999")),
            ("version", serde_json::json!("0.0.0")),
            (
                "url",
                serde_json::json!("https://untrusted.invalid/installer"),
            ),
            ("status", serde_json::json!("downloading")),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(cached(directory.path(), &bad).is_none());
        }
        std::fs::write(path, b"changed").unwrap();
        assert!(cached(directory.path(), &good).is_none());
    }

    #[tokio::test]
    async fn corrupt_truncated_and_oversized_bodies_are_rejected() {
        use tokio::{io::AsyncWriteExt, net::TcpListener};
        for (bytes, digest, valid) in [
            (7, "fixture", true),
            (8, "fixture", false),
            (6, "fixture", false),
            (7, "changed", false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                assert!(socket.read(&mut request).await.unwrap() > 0);
                socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nfixture",
                    )
                    .await
                    .unwrap();
            });
            let response = Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(format!("http://{address}/"))
                .send()
                .await
                .unwrap();
            let update = Update {
                version: "0.3.0".into(),
                url: String::new(),
                sha256: format!("{:x}", Sha256::digest(digest.as_bytes())),
                bytes,
            };
            let temporary = tempfile::NamedTempFile::new().unwrap();
            let mut file = tokio::fs::File::from_std(temporary.reopen().unwrap());
            assert_eq!(
                verified_body(response, &update, &mut file).await.is_ok(),
                valid
            );
            server.await.unwrap();
        }
        let directory = tempfile::tempdir().unwrap();
        let update = Update {
            version: "0.3.0".into(),
            url: "https://untrusted.invalid/setup.exe".into(),
            sha256: "ab".repeat(32),
            bytes: 7,
        };
        assert!(
            download(
                &Client::new(),
                &update,
                directory.path(),
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("updates"))
                .unwrap()
                .count(),
            0,
            "failed download published an installer or leaked its temporary file"
        );
    }
}
