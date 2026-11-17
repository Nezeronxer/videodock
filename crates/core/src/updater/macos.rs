//! Signed, same-volume bundle replacement with a startup-confirmed rollback.
use super::{Update, verify_file};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::CommandExt,
    },
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const STAGE_PREFIX: &str = ".videodock-update-";
const ACK_ENV: &str = "VIDEODOCK_UPDATE_STAGE";
// Leaf certificate of the published VideoDock Intel and Apple Silicon bundles.
const SIGNATURE_REQUIREMENT: &str = r#"=identifier "com.nezeronxer.videodock" and certificate leaf = H"3ffd8c9fcb44cd392aa2e22f98db38e1b4256a3c""#;
const PROFILE_FILES: [&str; 4] = [
    "state.sqlite",
    "state.sqlite-wal",
    "state.sqlite-shm",
    "secrets.json",
];
#[derive(Serialize, Deserialize)]
struct Handoff {
    current: PathBuf,
    profile: PathBuf,
    update: Update,
    parent: u32,
}
fn output(command: &mut Command) -> Result<String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .context("Нет вывода проверки")?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("Слишком большой вывод проверки обновления");
    }
    ensure!(
        child.wait()?.success(),
        "Не пройдена системная проверка обновления"
    );
    Ok(String::from_utf8(bytes)?)
}
fn bundle() -> Result<PathBuf> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let app = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .context("Приложение запущено вне .app")?;
    ensure!(
        app.extension().is_some_and(|s| s == "app")
            && executable == app.join("Contents/MacOS/VideoDock"),
        "Неизвестный пакет приложения"
    );
    Ok(app.to_owned())
}
fn plist(app: &Path, key: &str) -> Result<String> {
    Ok(output(
        Command::new("/usr/bin/plutil")
            .args(["-extract", key, "raw", "-o", "-"])
            .arg(app.join("Contents/Info.plist")),
    )?
    .trim()
    .into())
}
fn verify_bundle(app: &Path, expected: &str) -> Result<()> {
    output(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict", "-R"])
            .arg(SIGNATURE_REQUIREMENT)
            .arg(app),
    )
    .context("Не совпала подпись VideoDock или сертификат издателя")?;
    ensure!(
        plist(app, "CFBundleIdentifier")? == "com.nezeronxer.videodock"
            && plist(app, "CFBundleExecutable")? == "VideoDock"
            && plist(app, "CFBundleShortVersionString")? == expected,
        "Пакет или версия обновления не совпали"
    );
    let cpu_type = if cfg!(target_arch = "aarch64") {
        0x0100_000c
    } else {
        0x0100_0007
    };
    verify_executable_architecture(&app.join("Contents/MacOS/VideoDock"), cpu_type)
}
fn verify_executable_architecture(executable: &Path, cpu_type: u32) -> Result<()> {
    // Release packages contain a thin, little-endian Mach-O executable.
    let mut header = [0; 32];
    fs::File::open(executable)?.read_exact(&mut header)?;
    ensure!(
        header[..4] == 0xfeed_facfu32.to_le_bytes()
            && header[4..8] == cpu_type.to_le_bytes()
            && header[12..16] == 2u32.to_le_bytes(),
        "Архитектура приложения не совпала"
    );
    Ok(())
}
fn valid_member(member: &str) -> bool {
    let path = Path::new(member);
    let mut components = path.components();
    matches!(components.next(), Some(Component::Normal(root)) if root == "VideoDock.app" || root == "__MACOSX")
        && components.all(|part| matches!(part, Component::Normal(_)))
        && !member.contains(['\\', '\r', '\n'])
        && !member.split('/').any(|part| part == ".." || part == ".")
}
fn validate_tree(path: &Path, root: &Path, count: &mut usize, bytes: &mut u64) -> Result<()> {
    *count += 1;
    ensure!(*count <= 100_000, "Слишком много файлов в обновлении");
    let info = fs::symlink_metadata(path)?;
    if info.file_type().is_symlink() {
        ensure!(
            path.canonicalize()?.starts_with(root),
            "Ссылка выходит за пределы пакета"
        );
    } else if info.is_dir() {
        for entry in fs::read_dir(path)? {
            validate_tree(&entry?.path(), root, count, bytes)?;
        }
    } else {
        ensure!(
            info.is_file() && info.nlink() == 1,
            "Недопустимый тип файла обновления"
        );
        *bytes = bytes
            .checked_add(info.len())
            .context("Размер пакета переполнен")?;
        ensure!(
            *bytes <= 3 * 1024 * 1024 * 1024,
            "Распакованный пакет слишком велик"
        );
    }
    Ok(())
}
fn private_stage(path: &Path) -> Result<()> {
    let info = fs::symlink_metadata(path)?;
    ensure!(
        path.is_absolute()
            && path.canonicalize()? == path
            && info.is_dir()
            && info.mode() & 0o777 == 0o700
            && info.uid() == unsafe { libc::geteuid() }
            && path
                .file_name()
                .is_some_and(|s| s.to_string_lossy().starts_with(STAGE_PREFIX)),
        "Небезопасная папка обновления"
    );
    Ok(())
}
fn read_handoff(stage: &Path) -> Result<Handoff> {
    private_stage(stage)?;
    let path = stage.join("handoff.json");
    let info = fs::symlink_metadata(&path)?;
    ensure!(
        info.is_file() && !info.file_type().is_symlink() && info.len() < 65536,
        "Некорректный журнал обновления"
    );
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
pub fn install_on_exit(archive: &Path, update: &Update, profile: &Path) -> Result<()> {
    stage_and_handoff(&bundle()?, archive, update, profile, std::process::id())
}
fn stage_and_handoff(
    current: &Path,
    archive: &Path,
    update: &Update,
    profile: &Path,
    parent: u32,
) -> Result<()> {
    verify_file(archive, update)?;
    let current = current.canonicalize()?;
    let profile = profile.canonicalize()?;
    ensure!(
        !profile.starts_with(&current),
        "Профиль должен находиться вне приложения"
    );
    verify_bundle(&current, &plist(&current, "CFBundleShortVersionString")?)?;
    let stage = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempdir_in(current.parent().context("Нет папки установки")?)?;
    fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o700))?;
    // Preflight expanded size before extraction; libarchive also refuses traversal via links.
    let listing = output(Command::new("/usr/bin/zipinfo").arg("-l").arg(archive))?;
    let summary = listing.lines().last().context("Пустой ZIP")?;
    let fields: Vec<_> = summary.split_whitespace().collect();
    ensure!(
        fields.len() >= 4
            && fields[1].starts_with("file")
            && fields[3] == "bytes"
            && fields[0].parse::<usize>()? <= 100_000
            && fields[2].parse::<u64>()? <= 3 * 1024 * 1024 * 1024,
        "Недопустимый размер ZIP"
    );
    let names = output(Command::new("/usr/bin/tar").arg("-tf").arg(archive))?;
    ensure!(
        !names.is_empty() && names.lines().all(valid_member),
        "Недопустимый путь в ZIP"
    );
    output(
        Command::new("/usr/bin/tar")
            .args(["-xf"])
            .arg(archive)
            .arg("-C")
            .arg(stage.path())
            .arg("--no-same-owner"),
    )?;
    let candidate = stage.path().join("VideoDock.app");
    ensure!(
        fs::symlink_metadata(&candidate)?.is_dir(),
        "Пакет приложения отсутствует"
    );
    validate_tree(&candidate, &candidate, &mut 0, &mut 0)?;
    if stage.path().join("__MACOSX").exists() {
        validate_tree(
            &stage.path().join("__MACOSX"),
            &stage.path().join("__MACOSX"),
            &mut 0,
            &mut 0,
        )?;
    }
    verify_bundle(&candidate, &update.version)?;
    let handoff = Handoff {
        current: current.clone(),
        profile,
        update: update.clone(),
        parent,
    };
    fs::write(
        stage.path().join("handoff.json"),
        serde_json::to_vec(&handoff)?,
    )?;
    // Keep recovery files if spawning the helper fails; never remove the verified cache.
    let stage = stage.keep();
    let log = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(stage.join("update.log"))?;
    let mut command = Command::new(current.join("Contents/MacOS/VideoDock"));
    command
        .arg("--apply-macos-update")
        .arg(&stage)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
        .spawn()
        .context("Не удалось запустить установку обновления")?;
    Ok(())
}
fn alive(pid: u32) -> bool {
    unsafe {
        libc::kill(pid as libc::pid_t, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}
fn transient_browser_artifact(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        matches!(
            name.to_str(),
            Some(
                "SingletonLock"
                    | "SingletonSocket"
                    | "SingletonCookie"
                    | "DevToolsActivePort"
                    | "LOCK"
            )
        )
    })
}
fn copy_browser(
    source: &Path,
    destination: &Path,
    root: &Path,
    count: &mut usize,
    bytes: &mut u64,
    depth: usize,
) -> Result<()> {
    if transient_browser_artifact(source) {
        return Ok(());
    }
    *count += 1;
    ensure!(
        *count <= 100_000 && depth <= 128,
        "Слишком большой профиль браузера"
    );
    let info = fs::symlink_metadata(source)?;
    ensure!(
        info.uid() == unsafe { libc::geteuid() },
        "Профиль браузера принадлежит другому пользователю"
    );
    if info.file_type().is_symlink() {
        let target = source.canonicalize()?;
        ensure!(
            target.starts_with(root),
            "Ссылка выходит за пределы профиля браузера"
        );
        // Relative links still resolve inside the snapshot when the install volume differs.
        let mut relative = PathBuf::new();
        for _ in source
            .parent()
            .context("Нет папки ссылки")?
            .strip_prefix(root)?
            .components()
        {
            relative.push("..");
        }
        relative.push(target.strip_prefix(root)?);
        if relative.as_os_str().is_empty() {
            relative.push(".");
        }
        std::os::unix::fs::symlink(relative, destination)?;
    } else if info.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_browser(
                &entry.path(),
                &destination.join(entry.file_name()),
                root,
                count,
                bytes,
                depth + 1,
            )?;
        }
        fs::set_permissions(destination, info.permissions())?;
    } else {
        ensure!(
            info.is_file(),
            "Неизвестный специальный файл профиля браузера"
        );
        *bytes = bytes
            .checked_add(info.len())
            .context("Размер профиля переполнен")?;
        ensure!(
            *bytes <= 16 * 1024 * 1024 * 1024,
            "Профиль браузера превышает 16 GiB"
        );
        fs::copy(source, destination)?;
    }
    Ok(())
}
fn snapshot_profile(stage: &Path, profile: &Path) -> Result<()> {
    fs::create_dir(stage.join("profile"))?;
    for name in PROFILE_FILES {
        let source = profile.join(name);
        match fs::symlink_metadata(&source) {
            Ok(info) => {
                ensure!(
                    info.is_file()
                        && !info.file_type().is_symlink()
                        && info.uid() == unsafe { libc::geteuid() },
                    "Небезопасный файл профиля"
                );
                fs::copy(source, stage.join("profile").join(name))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let browser = profile.join("browser");
    match fs::symlink_metadata(&browser) {
        Ok(info) => {
            ensure!(
                info.is_dir() && !info.file_type().is_symlink(),
                "Небезопасная папка браузера"
            );
            copy_browser(
                &browser,
                &stage.join("profile/browser"),
                &browser,
                &mut 0,
                &mut 0,
                0,
            )?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
fn profile_entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn restore_profile(stage: &Path, profile: &Path) -> Result<()> {
    // Keep migrated browser data on its own filesystem; never traverse download paths.
    let recovery = tempfile::Builder::new()
        .prefix(".videodock-profile-recovery-")
        .tempdir_in(profile)?
        .keep();
    let browser = stage.join("profile/browser");
    if browser.exists() {
        copy_browser(
            &browser,
            &recovery.join("restored-browser"),
            &browser,
            &mut 0,
            &mut 0,
            0,
        )?;
    }
    if profile_entry_exists(&profile.join("browser"))? {
        fs::rename(profile.join("browser"), recovery.join("migrated-browser"))?;
    }
    if browser.exists()
        && let Err(error) = fs::rename(recovery.join("restored-browser"), profile.join("browser"))
    {
        if recovery.join("migrated-browser").exists() {
            fs::rename(recovery.join("migrated-browser"), profile.join("browser"))?;
        }
        return Err(error.into());
    }
    for name in PROFILE_FILES {
        let destination = profile.join(name);
        let source = stage.join("profile").join(name);
        if source.exists() {
            let restored = recovery.join(name);
            fs::copy(source, &restored)?;
            fs::rename(restored, destination)?;
        } else if profile_entry_exists(&destination)? {
            fs::rename(destination, recovery.join(format!("migrated-{name}")))?;
        }
    }
    Ok(())
}
fn apply(stage: &Path, handoff: &Handoff) -> Result<()> {
    ensure!(
        handoff.parent > 1 && handoff.parent <= i32::MAX as u32,
        "Некорректный родительский процесс"
    );
    println!("Waiting for the previous application to exit");
    let deadline = Instant::now() + Duration::from_secs(120);
    while alive(handoff.parent) {
        ensure!(
            Instant::now() < deadline,
            "Приложение не завершилось вовремя"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    let current = &handoff.current;
    ensure!(
        stage.parent() == current.parent() && !handoff.profile.starts_with(current),
        "Папки установки изменились"
    );
    // Reauthenticate both bundles after the old process exits and before any rename.
    verify_bundle(current, &plist(current, "CFBundleShortVersionString")?)?;
    let candidate = stage.join("VideoDock.app");
    validate_tree(&candidate, &candidate, &mut 0, &mut 0)?;
    verify_bundle(&candidate, &handoff.update.version)?;
    let profile_lease = crate::engine::lock_profile(&handoff.profile)?;
    snapshot_profile(stage, &handoff.profile)?;
    println!("Verified staged bundle; replacing application");
    let backup = stage.join("previous.app");
    fs::rename(current, &backup).context("Нет прав на замену приложения")?;
    if let Err(error) = fs::rename(&candidate, current) {
        fs::rename(&backup, current).context("Не удалось вернуть исходный пакет")?;
        return Err(error.into());
    }
    drop(profile_lease);
    let mut child = None;
    let startup = (|| -> Result<()> {
        child = Some(
            Command::new(current.join("Contents/MacOS/VideoDock"))
                .env("VIDEODOCK_DATA_DIR", &handoff.profile)
                .env(ACK_ENV, stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let child = child.as_mut().unwrap();
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            ensure!(
                child.try_wait()?.is_none(),
                "Обновлённое приложение завершилось при запуске"
            );
            if stage.join("ready").try_exists()? {
                let info = fs::symlink_metadata(stage.join("ready"))?;
                ensure!(
                    info.is_file()
                        && !info.file_type().is_symlink()
                        && fs::read_to_string(stage.join("ready"))? == child.id().to_string(),
                    "Неверное подтверждение запуска"
                );
                std::thread::sleep(Duration::from_secs(2));
                ensure!(
                    child.try_wait()?.is_none(),
                    "Приложение завершилось после загрузки интерфейса"
                );
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Нет подтверждения запуска интерфейса"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    })();
    if let Err(error) = startup {
        if let Some(child) = child.as_mut() {
            let _ = child.kill();
            child
                .wait()
                .context("Не удалось остановить обновлённый процесс")?;
        }
        fs::rename(current, &candidate).context("Не удалось убрать неработающий пакет")?;
        fs::rename(&backup, current).context("Не удалось вернуть исходное приложение")?;
        let _profile_lease = crate::engine::lock_profile(&handoff.profile).context(
            "Исходное приложение возвращено, резервная копия занятого профиля сохранена",
        )?;
        restore_profile(stage, &handoff.profile)?;
        return Err(error.context("Обновление отменено; исходный пакет и профиль восстановлены"));
    }
    println!("Updated GUI acknowledged startup");
    fs::remove_dir_all(stage).context("Обновление установлено; папка восстановления оставлена")?;
    Ok(())
}
pub fn apply_update_helper(args: &[OsString]) -> Result<()> {
    ensure!(args.len() == 1, "Некорректный вызов установщика");
    let stage = PathBuf::from(&args[0]);
    let handoff = read_handoff(&stage)?;
    ensure!(
        bundle()? == handoff.current,
        "Установщик вызван из другого приложения"
    );
    apply(&stage, &handoff)
}
pub fn acknowledge_launch() -> Result<()> {
    let Some(stage) = std::env::var_os(ACK_ENV) else {
        return Ok(());
    };
    let stage = PathBuf::from(stage);
    let handoff = read_handoff(&stage)?;
    ensure!(
        bundle()? == handoff.current && stage.parent() == handoff.current.parent(),
        "Подтверждение относится к другому приложению"
    );
    let mut marker = tempfile::NamedTempFile::new_in(&stage)?;
    marker.write_all(std::process::id().to_string().as_bytes())?;
    marker.as_file().sync_all()?;
    marker.persist_noclobber(stage.join("ready"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_executable_matches_package_architecture() {
        let file = tempfile::NamedTempFile::new().unwrap();
        for cpu_type in [0x0100_0007u32, 0x0100_000c] {
            let mut header = [0; 32];
            header[..4].copy_from_slice(&0xfeed_facfu32.to_le_bytes());
            header[4..8].copy_from_slice(&cpu_type.to_le_bytes());
            header[12..16].copy_from_slice(&2u32.to_le_bytes());
            fs::write(file.path(), header).unwrap();
            assert!(verify_executable_architecture(file.path(), cpu_type).is_ok());
            assert!(verify_executable_architecture(file.path(), cpu_type ^ 1).is_err());
            fs::write(file.path(), &header[..8]).unwrap();
            assert!(verify_executable_architecture(file.path(), cpu_type).is_err());
        }
    }
    #[test]
    fn archive_paths_are_confined() {
        for good in [
            "VideoDock.app/Contents/Info.plist",
            "__MACOSX/._VideoDock.app",
            "VideoDock.app/",
        ] {
            assert!(valid_member(good));
        }
        for bad in [
            "/VideoDock.app/x",
            "VideoDock.app/../outside",
            "VideoDock.app/./x",
            "other.app/x",
            "VideoDock.app/a\nb",
            "VideoDock.app/a\\b",
        ] {
            assert!(!valid_member(bad));
        }
    }
    #[test]
    fn links_cannot_escape_bundle() {
        let directory = tempfile::tempdir().unwrap();
        let app = directory
            .path()
            .canonicalize()
            .unwrap()
            .join("VideoDock.app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("real"), "data").unwrap();
        std::os::unix::fs::symlink("real", app.join("link")).unwrap();
        assert!(validate_tree(&app, &app, &mut 0, &mut 0).is_ok());
        fs::remove_file(app.join("link")).unwrap();
        std::os::unix::fs::symlink("../", app.join("link")).unwrap();
        assert!(validate_tree(&app, &app, &mut 0, &mut 0).is_err());
    }
    #[test]
    fn rollback_restores_database_browser_and_secrets_without_touching_downloads() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let stage = root.join("stage");
        let profile = root.join("profile");
        fs::create_dir(&stage).unwrap();
        fs::create_dir(&profile).unwrap();
        fs::write(profile.join("state.sqlite"), "old history").unwrap();
        fs::write(profile.join("download.mp4"), "keep download").unwrap();
        let browser = profile.join("browser");
        fs::create_dir_all(browser.join("Default/Local Storage/leveldb")).unwrap();
        fs::write(browser.join("Default/Cookies"), "old cookies").unwrap();
        fs::write(
            browser.join("Default/Local Storage/leveldb/site.ldb"),
            "old site data",
        )
        .unwrap();
        fs::write(profile.join("secrets.json"), "old secrets").unwrap();
        fs::set_permissions(
            profile.join("secrets.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        fs::set_permissions(&browser, fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(
            "/unavailable/session/socket",
            browser.join("SingletonSocket"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            browser.join("Default/Cookies"),
            browser.join("cookie-alias"),
        )
        .unwrap();
        snapshot_profile(&stage, &profile).unwrap();
        fs::write(profile.join("state.sqlite"), "incompatible migration").unwrap();
        fs::write(profile.join("state.sqlite-wal"), "new WAL").unwrap();
        fs::write(
            browser.join("Default/Cookies"),
            "incompatible cookie migration",
        )
        .unwrap();
        fs::write(
            browser.join("Default/Local Storage/leveldb/site.ldb"),
            "migrated site data",
        )
        .unwrap();
        fs::write(profile.join("secrets.json"), "migrated secrets").unwrap();
        restore_profile(&stage, &profile).unwrap();
        assert_eq!(
            fs::read(profile.join("state.sqlite")).unwrap(),
            b"old history"
        );
        assert!(!profile.join("state.sqlite-wal").exists());
        assert_eq!(
            fs::read(browser.join("Default/Cookies")).unwrap(),
            b"old cookies"
        );
        assert_eq!(
            fs::read(browser.join("cookie-alias")).unwrap(),
            b"old cookies"
        );
        assert_eq!(
            fs::read(browser.join("Default/Local Storage/leveldb/site.ldb")).unwrap(),
            b"old site data"
        );
        assert_eq!(
            fs::read(profile.join("secrets.json")).unwrap(),
            b"old secrets"
        );
        assert_eq!(
            fs::metadata(profile.join("secrets.json")).unwrap().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::metadata(&browser).unwrap().mode() & 0o777, 0o700);
        assert!(fs::symlink_metadata(browser.join("SingletonSocket")).is_err());
        assert_eq!(
            fs::read(profile.join("download.mp4")).unwrap(),
            b"keep download"
        );
    }
    #[test]
    fn browser_backup_rejects_external_links_and_is_bounded() {
        let directory = tempfile::tempdir().unwrap();
        let browser = directory.path().join("browser");
        fs::create_dir(&browser).unwrap();
        let external = directory.path().join("download.mp4");
        fs::write(&external, "do not copy").unwrap();
        std::os::unix::fs::symlink(&external, browser.join("external")).unwrap();
        assert!(
            copy_browser(
                &browser,
                &directory.path().join("snapshot"),
                &browser,
                &mut 0,
                &mut 0,
                0
            )
            .is_err()
        );
        assert_eq!(fs::read(&external).unwrap(), b"do not copy");
        fs::remove_file(browser.join("external")).unwrap();
        assert!(
            copy_browser(
                &browser,
                &directory.path().join("bounded"),
                &browser,
                &mut 100_000,
                &mut 0,
                0
            )
            .is_err()
        );
    }
    #[test]
    fn staging_must_be_private_and_not_a_symlink() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().canonicalize().unwrap();
        let stage = tempfile::Builder::new()
            .prefix(STAGE_PREFIX)
            .tempdir_in(&root)
            .unwrap();
        fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(private_stage(stage.path()).is_ok());
        fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private_stage(stage.path()).is_err());
        fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let alias = root.join(".videodock-update-alias");
        std::os::unix::fs::symlink(stage.path(), &alias).unwrap();
        assert!(private_stage(&alias).is_err());
    }
    #[test]
    #[ignore = "native signed GUI fixture; scripts/rust/check-macos-update.py"]
    fn native_handoff() {
        let current = PathBuf::from(std::env::var_os("VIDEODOCK_FIXTURE_CURRENT").unwrap());
        let archive = PathBuf::from(std::env::var_os("VIDEODOCK_FIXTURE_ARCHIVE").unwrap());
        let profile = PathBuf::from(std::env::var_os("VIDEODOCK_FIXTURE_PROFILE").unwrap());
        let update: Update =
            serde_json::from_str(&std::env::var("VIDEODOCK_FIXTURE_UPDATE").unwrap()).unwrap();
        let result = stage_and_handoff(&current, &archive, &update, &profile, std::process::id());
        if std::env::var_os("VIDEODOCK_FIXTURE_REJECT").is_some() {
            assert!(result.is_err(), "Unsafe archive was accepted");
        } else {
            result.unwrap();
        }
    }
}
