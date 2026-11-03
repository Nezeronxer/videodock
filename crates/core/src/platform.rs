use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub fn media_tool(name: &str) -> PathBuf {
    #[cfg(windows)]
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            let path = parent.join(format!("{name}.exe"));
            if path.is_file() {
                return path;
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            let path = directory.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_owned()
            });
            if path.is_file() {
                return path;
            }
        }
    }
    #[cfg(target_os = "macos")]
    for directory in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let path = Path::new(directory).join(name);
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from(name)
}

fn spawn(program: &str, args: &[&std::ffi::OsStr]) -> Result<()> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Не удалось запустить системную программу {program}"))?;
    Ok(())
}

pub fn open_path(path: &Path) -> Result<()> {
    ensure!(path.exists(), "Файл или папка не найдены");
    #[cfg(target_os = "linux")]
    {
        spawn("xdg-open", &[path.as_os_str()])?;
    }
    #[cfg(target_os = "macos")]
    {
        spawn("open", &[path.as_os_str()])?;
    }
    #[cfg(windows)]
    {
        shell_open(path.as_os_str())?;
    }
    Ok(())
}

pub fn reveal(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        return spawn("open", &[std::ffi::OsStr::new("-R"), path.as_os_str()]);
    }
    #[cfg(not(target_os = "macos"))]
    {
        open_path(path.parent().context("Не удалось определить каталог")?)
    }
}

pub fn open_url(value: &str) -> Result<()> {
    let url = videodock_media::validate_url(value)?;
    #[cfg(target_os = "linux")]
    {
        spawn("xdg-open", &[std::ffi::OsStr::new(url.as_str())])?;
    }
    #[cfg(target_os = "macos")]
    {
        spawn("open", &[std::ffi::OsStr::new(url.as_str())])?;
    }
    #[cfg(windows)]
    {
        shell_open(std::ffi::OsStr::new(url.as_str()))?;
    }
    Ok(())
}

#[cfg(windows)]
fn shell_open(path: &std::ffi::OsStr) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            window: *mut std::ffi::c_void,
            verb: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    let path = path.encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let verb = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    ensure!(result > 32, "Windows не смог открыть файл/ссылку");
    Ok(())
}

fn home() -> Result<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .context("Не найден домашний каталог")
}

#[cfg(any(target_os = "macos", test))]
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    let executable = std::env::current_exe()?.canonicalize()?;
    #[cfg(target_os = "linux")]
    {
        let file = home()?.join(".config/autostart/videodock.desktop");
        if enabled {
            std::fs::create_dir_all(file.parent().unwrap())?;
            let path = executable
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('%', "%%");
            std::fs::write(
                file,
                format!(
                    "[Desktop Entry]\nType=Application\nName=VideoDock\nExec=\"{path}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
                ),
            )?;
        } else if file.exists() {
            std::fs::remove_file(file)?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let file = home()?.join("Library/LaunchAgents/com.nezeronxer.videodock.plist");
        if enabled {
            std::fs::create_dir_all(file.parent().unwrap())?;
            std::fs::write(
                file,
                format!(
                    "<?xml version=\"1.0\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>com.nezeronxer.videodock</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/></dict></plist>",
                    xml(&executable.to_string_lossy())
                ),
            )?;
        } else if file.exists() {
            std::fs::remove_file(file)?;
        }
    }
    #[cfg(windows)]
    {
        windows_autostart(enabled, &executable)?;
    }
    Ok(())
}

#[cfg(windows)]
fn windows_autostart(enabled: bool, path: &Path) -> Result<()> {
    use std::ffi::c_void;
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            root: *mut c_void,
            subkey: *const u16,
            reserved: u32,
            class: *mut u16,
            options: u32,
            access: u32,
            security: *const c_void,
            result: *mut *mut c_void,
            disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: *mut c_void,
            name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            size: u32,
        ) -> i32;
        fn RegDeleteValueW(key: *mut c_void, name: *const u16) -> i32;
        fn RegCloseKey(key: *mut c_void) -> i32;
    }
    let subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let name = "VideoDock\0".encode_utf16().collect::<Vec<_>>();
    let mut key = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            (-2147483647isize) as *mut c_void,
            subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            0x0002,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    ensure!(
        status == 0,
        "Не удалось открыть автозапуск Windows ({status})"
    );
    let status = if enabled {
        use std::os::windows::ffi::OsStrExt;
        let mut value = vec![b'"' as u16];
        value.extend(path.as_os_str().encode_wide());
        value.extend([b'"' as u16, 0]);
        unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                1,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, name.as_ptr()) }
    };
    unsafe {
        RegCloseKey(key);
    }
    ensure!(
        status == 0 || (!enabled && status == 2),
        "Не удалось изменить автозапуск Windows ({status})"
    );
    Ok(())
}

pub struct SleepGuard {
    #[cfg(unix)]
    child: Child,
    #[cfg(windows)]
    stop: Option<std::sync::mpsc::Sender<()>>,
    #[cfg(windows)]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SleepGuard {
    pub fn new() -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            #[cfg(target_os = "linux")]
            let mut command = {
                let mut c = Command::new("systemd-inhibit");
                c.args([
                    "--what=sleep:idle",
                    "--mode=block",
                    "--who=VideoDock",
                    "--why=Video download in progress",
                    "sleep",
                    "infinity",
                ]);
                c
            };
            #[cfg(target_os = "macos")]
            let mut command = {
                let mut c = Command::new("caffeinate");
                c.args(["-imsu", "-w", &std::process::id().to_string()]);
                c
            };
            let child = command
                .process_group(0)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Не удалось запретить переход в сон")?;
            Ok(Self { child })
        }
        #[cfg(windows)]
        {
            let (send, receive) = std::sync::mpsc::channel();
            let thread = std::thread::spawn(move || {
                #[link(name = "kernel32")]
                unsafe extern "system" {
                    fn SetThreadExecutionState(flags: u32) -> u32;
                }
                unsafe {
                    SetThreadExecutionState(0x80000001);
                }
                receive.recv().ok();
                unsafe {
                    SetThreadExecutionState(0x80000000);
                }
            });
            Ok(Self {
                stop: Some(send),
                thread: Some(thread),
            })
        }
    }
}

impl Drop for SleepGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            unsafe extern "C" {
                fn kill(pid: i32, signal: i32) -> i32;
            }
            // The inhibitor owns a process group, so its sleep child cannot survive cleanup.
            if self.child.try_wait().ok().flatten().is_none() {
                unsafe {
                    kill(-(self.child.id() as i32), 9);
                }
                self.child.wait().ok();
            }
        }
        #[cfg(windows)]
        {
            if let Some(stop) = self.stop.take() {
                stop.send(()).ok();
            }
            if let Some(thread) = self.thread.take() {
                thread.join().ok();
            }
        }
    }
}

pub async fn check_updates(client: &Client) -> Result<Option<Value>> {
    let Some(url) = option_env!("VIDEODOCK_UPDATE_URL") else {
        bail!("Для этой локальной сборки не задан источник обновлений")
    };
    let url = videodock_media::validate_url(url)?;
    ensure!(
        url.scheme() == "https",
        "Источник обновлений должен использовать HTTPS"
    );
    let mut response = client.get(url).send().await?.error_for_status()?;
    ensure!(
        response
            .content_length()
            .is_none_or(|length| length <= 1024 * 1024),
        "Слишком большой ответ обновлений"
    );
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            body.len() + chunk.len() <= 1024 * 1024,
            "Слишком большой ответ обновлений"
        );
        body.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&body)?;
    let version = value["version"]
        .as_str()
        .context("Не указан номер версии обновления")?;
    Ok((version != env!("CARGO_PKG_VERSION")).then_some(value))
}

#[cfg(test)]
mod tests {
    #[test]
    fn plist_values_are_escaped() {
        assert_eq!(super::xml("a&<\"'"), "a&amp;&lt;&quot;&apos;");
    }
}
