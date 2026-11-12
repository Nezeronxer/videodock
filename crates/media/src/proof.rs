//! Private IPC to the explicitly registered native Google attestation worker.
use anyhow::{Result, ensure};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static WORKER: OnceLock<PathBuf> = OnceLock::new();

pub fn register_proof_worker(path: PathBuf) -> Result<()> {
    ensure!(
        path.is_absolute(),
        "Нужен абсолютный путь модуля проверки Google"
    );
    let path = path
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("Модуль проверки Google недоступен"))?;
    ensure!(path.is_file(), "Модуль проверки Google недоступен");
    if let Some(existing) = WORKER.get() {
        ensure!(
            existing == &path,
            "Модуль проверки Google уже зарегистрирован"
        );
        return Ok(());
    }
    WORKER
        .set(path)
        .map_err(|_| anyhow::anyhow!("Модуль проверки Google уже зарегистрирован"))?;
    Ok(())
}

pub(crate) fn registered_worker() -> Option<&'static Path> {
    WORKER.get().map(PathBuf::as_path)
}

use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
};

const IPC_LIMIT: usize = 32 * 1024;
#[derive(Debug)]
pub enum AttestationRetryable {
    Failed,
    Timeout,
}
impl std::fmt::Display for AttestationRetryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Failed => "Google не подтвердил анонимную проверку; повторите загрузку",
            Self::Timeout => "Проверка Google превысила время ожидания",
        })
    }
}
impl std::error::Error for AttestationRetryable {}
#[derive(Serialize)]
struct Bootstrap {
    host: String,
    address: String,
}
#[derive(Serialize)]
struct Request<'a> {
    version: u8,
    request_id: &'a str,
    video_id: &'a str,
    proxy: Option<&'a str>,
    dns_bootstrap: Vec<Bootstrap>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    version: u8,
    request_id: String,
    ok: bool,
    content_po: Option<String>,
    session_po: Option<String>,
    visitor_data: Option<String>,
    user_agent: Option<String>,
    stage: Option<Stage>,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Stage {
    Validation,
    Network,
    Homepage,
    Challenge,
    Interpreter,
    Snapshot,
    Integrity,
    Mint,
    Timeout,
    Internal,
}

pub(crate) struct Proof {
    pub content_po: String,
    pub visitor_data: String,
    pub user_agent: String,
}
impl std::fmt::Debug for Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleProof").finish_non_exhaustive()
    }
}

fn token(value: Option<String>) -> Result<String> {
    let value = value.ok_or_else(|| anyhow::anyhow!("Проверка Google не вернула PO"))?;
    ensure!(
        !value.is_empty()
            && value.len() <= 16 * 1024
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_=".contains(&b)),
        "Некорректный PO проверки Google"
    );
    ensure!(
        data_encoding::BASE64URL_NOPAD
            .decode(value.trim_end_matches('=').as_bytes())
            .is_ok(),
        "Некорректная кодировка PO"
    );
    Ok(value)
}
fn response(bytes: &[u8], id: &str) -> Result<Proof> {
    ensure!(
        bytes.len() <= IPC_LIMIT,
        "Ответ проверки Google слишком велик"
    );
    let line = bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| anyhow::anyhow!("Оборванный ответ проверки Google"))?;
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    ensure!(
        !line.contains(&b'\n') && !line.contains(&b'\r'),
        "Лишний вывод модуля проверки Google"
    );
    let data: Response = serde_json::from_slice(line)
        .map_err(|_| anyhow::anyhow!("Некорректный ответ проверки Google"))?;
    ensure!(
        data.version == 1 && data.request_id == id,
        "Ответ проверки Google относится к другому запросу"
    );
    if !data.ok {
        ensure!(
            data.content_po.is_none()
                && data.session_po.is_none()
                && data.visitor_data.is_none()
                && data.user_agent.is_none(),
            "Некорректный отказ проверки Google"
        );
        let stage = data
            .stage
            .ok_or_else(|| anyhow::anyhow!("Нет причины отказа проверки Google"))?;
        if matches!(
            &stage,
            Stage::Homepage
                | Stage::Challenge
                | Stage::Interpreter
                | Stage::Snapshot
                | Stage::Integrity
                | Stage::Mint
        ) {
            return Err(AttestationRetryable::Failed.into());
        }
        if matches!(&stage, Stage::Timeout) {
            return Err(AttestationRetryable::Timeout.into());
        }
        let message = match stage {
            Stage::Validation => {
                "Проверка Google поддерживает HTTP (включая вход) и SOCKS5 без логина; HTTPS и SOCKS4 не поддерживаются"
            }
            Stage::Network => "Проверке Google недоступна сеть через выбранный маршрут",
            _ => "Google не подтвердил анонимную проверку; повторите загрузку",
        };
        anyhow::bail!(message)
    }
    ensure!(data.stage.is_none(), "Противоречивый ответ проверки Google");
    let content_po = token(data.content_po)?;
    let _session_po = token(data.session_po)?;
    let visitor_data = data
        .visitor_data
        .ok_or_else(|| anyhow::anyhow!("Нет visitor data проверки Google"))?;
    let user_agent = data
        .user_agent
        .ok_or_else(|| anyhow::anyhow!("Нет User-Agent проверки Google"))?;
    ensure!(
        !visitor_data.is_empty()
            && visitor_data.len() <= 8192
            && visitor_data.bytes().all(|b| b.is_ascii_graphic()),
        "Некорректный visitor data проверки Google"
    );
    ensure!(
        !user_agent.is_empty()
            && user_agent.len() <= 2048
            && user_agent.bytes().all(|b| (32..=126).contains(&b)),
        "Некорректный User-Agent проверки Google"
    );
    Ok(Proof {
        content_po,
        visitor_data,
        user_agent,
    })
}

pub(crate) async fn request(path: &Path, video_id: &str, proxy: Option<&str>) -> Result<Proof> {
    ensure!(
        video_id.len() == 11
            && video_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
        "Неверный идентификатор YouTube"
    );
    tokio::time::timeout(Duration::from_secs(90), async {
        let bootstrap = crate::anonymous_dns_bootstrap(proxy.is_some())
            .await?
            .into_iter()
            .map(|(host, address)| Bootstrap {
                host,
                address: address.to_string(),
            })
            .collect();
        request_inner(path, video_id, proxy, bootstrap).await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Проверка Google превысила время ожидания"))?
}
async fn request_inner(
    path: &Path,
    video_id: &str,
    proxy: Option<&str>,
    dns_bootstrap: Vec<Bootstrap>,
) -> Result<Proof> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut input = serde_json::to_vec(&Request {
        version: 1,
        request_id: &id,
        video_id,
        proxy,
        dns_bootstrap,
    })?;
    input.push(b'\n');
    ensure!(
        input.len() <= IPC_LIMIT,
        "Запрос проверки Google слишком велик"
    );
    let mut command = Command::new(path);
    command
        .arg("--youtube-proof")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .env_clear();
    // Preserve only the native GUI/runtime loader environment, never account keys or implicit proxy routes.
    for name in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "SystemRoot",
        "TEMP",
        "TMP",
        "TMPDIR",
        "LOCALAPPDATA",
        "APPDATA",
        "XDG_RUNTIME_DIR",
        "XDG_SESSION_TYPE",
        "XDG_CURRENT_DESKTOP",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "QT_QPA_PLATFORM",
        "QT_PLUGIN_PATH",
        "QML2_IMPORT_PATH",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "QTWEBENGINE_RESOURCES_PATH",
        "QTWEBENGINE_LOCALES_PATH",
        "FONTCONFIG_FILE",
        "FONTCONFIG_PATH",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    command.creation_flags(0x00000004 | 0x08000000);
    let child = command
        .spawn()
        .map_err(|_| anyhow::anyhow!("Не удалось запустить модуль проверки Google"))?;
    let mut process = Process::new(child)?;
    let result = async {
        let mut stdin = process
            .child
            .as_mut()
            .and_then(|c| c.stdin.take())
            .ok_or_else(|| anyhow::anyhow!("Нет приватного канала проверки Google"))?;
        let stdout = process
            .child
            .as_mut()
            .and_then(|c| c.stdout.take())
            .ok_or_else(|| anyhow::anyhow!("Нет ответа проверки Google"))?;
        stdin
            .write_all(&input)
            .await
            .map_err(|_| anyhow::anyhow!("Модуль проверки Google закрыл запрос"))?;
        stdin.shutdown().await.ok();
        drop(stdin);
        drop(input);
        let mut bytes = Vec::new();
        stdout
            .take((IPC_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| anyhow::anyhow!("Ответ проверки Google прерван"))?;
        let proof = response(&bytes, &id)?;
        // Tree cleanup precedes reaping, so no descendant or reused process group can outlive this request.
        let mut exited = false;
        for _ in 0..300 {
            if worker_exited(process.child.as_ref().unwrap()) {
                exited = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        ensure!(exited, "Модуль проверки Google не завершил запрос");
        process.tree.kill();
        let status = process
            .child
            .as_mut()
            .unwrap()
            .wait()
            .await
            .map_err(|_| anyhow::anyhow!("Модуль проверки Google завершился с ошибкой"))?;
        ensure!(
            status.success(),
            "Модуль проверки Google завершился с ошибкой"
        );
        Ok(proof)
    }
    .await;
    process.stop().await;
    result
}

struct Process {
    child: Option<Child>,
    tree: Tree,
}
impl Process {
    fn new(child: Child) -> Result<Self> {
        let tree = Tree::new(&child)?;
        Ok(Self {
            child: Some(child),
            tree,
        })
    }
    async fn stop(&mut self) {
        self.tree.kill();
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.tree.kill();
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}

#[cfg(unix)]
fn worker_exited(child: &Child) -> bool {
    let Some(id) = child.id() else { return true };
    // Observe the exit without reaping: the PID still protects the process-group identity during cleanup.
    unsafe {
        let mut info: libc::siginfo_t = std::mem::zeroed();
        let okay = libc::waitid(
            libc::P_PID,
            id,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        ) == 0;
        #[cfg(target_os = "macos")]
        let observed = info.si_pid;
        #[cfg(not(target_os = "macos"))]
        let observed = info.si_pid();
        okay && observed == id as i32
    }
}
#[cfg(windows)]
fn worker_exited(child: &Child) -> bool {
    use windows_sys::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
    child
        .raw_handle()
        .is_none_or(|handle| unsafe { WaitForSingleObject(handle, 0) == WAIT_OBJECT_0 })
}

#[cfg(unix)]
struct Tree {
    group: i32,
}
#[cfg(unix)]
impl Tree {
    fn new(child: &Child) -> Result<Self> {
        let group = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .ok_or_else(|| anyhow::anyhow!("Неизвестен процесс проверки Google"))?;
        Ok(Self { group })
    }
    fn kill(&mut self) {
        if self.group > 0 {
            unsafe {
                libc::kill(-self.group, libc::SIGKILL);
            }
            self.group = 0;
        }
    }
}
#[cfg(windows)]
#[path = "proof_windows.rs"]
mod windows;
#[cfg(windows)]
use windows::Tree;

#[cfg(test)]
mod tests {
    #[test]
    fn transient_worker_failures_require_fresh_attestation_but_invalid_requests_do_not() {
        for (stage, retryable) in [
            ("homepage", true),
            ("challenge", true),
            ("interpreter", true),
            ("snapshot", true),
            ("integrity", true),
            ("mint", true),
            ("timeout", true),
            ("validation", false),
            ("network", false),
            ("internal", false),
        ] {
            let mut data = serde_json::to_vec(&serde_json::json!({
                "version":1,"request_id":"fixture","ok":false,"stage":stage
            }))
            .unwrap();
            data.push(b'\n');
            let error = super::response(&data, "fixture").unwrap_err();
            assert_eq!(
                error.is::<super::AttestationRetryable>(),
                retryable,
                "stage {stage}"
            );
        }
        let mut data = serde_json::to_vec(&serde_json::json!({
            "version":1,"request_id":"fixture","ok":false,"stage":"mint",
            "content_po":"synthetic-private-value"
        }))
        .unwrap();
        data.push(b'\n');
        let error = super::response(&data, "fixture").unwrap_err();
        assert!(!error.is::<super::AttestationRetryable>());
        assert!(!error.to_string().contains("synthetic-private-value"));
    }
    use super::*;
    fn good() -> serde_json::Value {
        serde_json::json!({"version":1,"request_id":"fixture","ok":true,"content_po":"YWJj","session_po":"ZGVm","visitor_data":"fixture-visitor","user_agent":"Fixture Agent"})
    }
    fn bytes(v: serde_json::Value) -> Vec<u8> {
        let mut b = serde_json::to_vec(&v).unwrap();
        b.push(b'\n');
        b
    }
    #[test]
    fn schema_and_identity_are_exact() {
        let good = good();
        assert!(response(&bytes(good.clone()), "fixture").is_ok());
        assert!(response(&bytes(good.clone()), "other").is_err());
        let mut extra = good.clone();
        extra["url"] = serde_json::json!("fixture-value");
        assert!(response(&bytes(extra), "fixture").is_err());
        let mut bad = good;
        bad["version"] = serde_json::json!(2);
        assert!(response(&bytes(bad), "fixture").is_err());
    }
    #[test]
    fn framing_and_header_injection_fail_without_leaks() {
        let b = bytes(good());
        assert!(response(&b[..b.len() - 1], "fixture").is_err());
        let mut noisy = b.clone();
        noisy.extend_from_slice(&b);
        assert!(response(&noisy, "fixture").is_err());
        let mut bad = good();
        bad["user_agent"] = serde_json::json!("synthetic-secret\r\nCookie: leak");
        let error = response(&bytes(bad), "fixture").unwrap_err().to_string();
        assert!(!error.contains("synthetic-secret"));
        let oversized = vec![b'x'; IPC_LIMIT + 1];
        assert!(response(&oversized, "fixture").is_err());
    }
    #[test]
    fn proof_debug_is_opaque() {
        assert!(!format!("{:?}", response(&bytes(good()), "fixture").unwrap()).contains("YWJj"));
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_terminates_and_reaps_the_worker_group() {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("videodock-proof-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let script = dir.join("worker");
        let pidfile = dir.join("pid");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho $$ > '{}'\ncat >/dev/null\nsleep 60 &\nwait\n",
                pidfile.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let owned = script.clone();
        let task =
            tokio::spawn(async move { request_inner(&owned, "fixture0001", None, vec![]).await });
        let mut pid = None;
        for _ in 0..100 {
            if let Ok(p) = std::fs::read_to_string(&pidfile) {
                pid = p.trim().parse::<i32>().ok();
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let pid = pid.expect("worker did not start");
        task.abort();
        let _ = task.await;
        for _ in 0..100 {
            if unsafe { libc::kill(pid, 0) } != 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_ne!(
            unsafe { libc::kill(pid, 0) },
            0,
            "worker survived cancellation"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
