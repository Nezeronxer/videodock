use super::*;

pub(super) fn start(inner: &Arc<Inner>) -> Result<()> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        let weak = Arc::downgrade(inner);
        let cancel = inner.shutdown.clone();
        runtime()?.spawn(async move {
            let mut delay = Duration::from_secs(20);
            loop {
                tokio::select! { _ = cancel.cancelled() => break, _ = tokio::time::sleep(delay) => {} }
                let Some(inner) = weak.upgrade() else { break };
                if lock(&inner.state)["settings"]["autoUpdate"] == true { let _ = check(&inner, false); }
                delay = Duration::from_secs(6 * 60 * 60);
            }
        });
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = inner;
    Ok(())
}
pub(super) fn check(inner: &Arc<Inner>, manual: bool) -> Result<()> {
    if inner.updating.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let inner = inner.clone();
    runtime()?.spawn(async move {
        let previous = lock(&inner.state)["update"].clone();
        let result = async {
            let client = queue::catalog(&inner)?.client();
            let Some(update) = crate::updater::check(&client).await? else { return Ok(None) };
            let mut value = serde_json::to_value(&update)?;
            if crate::updater::automatic_updates_supported() && lock(&inner.state)["settings"]["autoUpdate"] == true {
                if crate::updater::cached(&inner.directory, &previous).is_none_or(|cached| cached.version != update.version || cached.sha256 != update.sha256 || cached.bytes != update.bytes || cached.url != update.url) {
                    change(&inner, |state| { if previous["status"] != "ready" { state["update"] = value.clone(); state["update"]["status"] = json!("downloading"); } Ok(()) })?;
                    crate::updater::download(&client, &update, &inner.directory, &inner.shutdown).await?;
                }
                value["status"] = json!("ready");
            } else { value["status"] = json!("available"); }
            Ok::<_, anyhow::Error>(Some(value))
        };
        let result = tokio::select! { _ = inner.shutdown.cancelled() => { inner.updating.store(false, Ordering::SeqCst); return; }, result = result => result };
        inner.updating.store(false, Ordering::SeqCst);
        match result {
            Ok(Some(value)) => { let _ = change(&inner, |state| { state["notice"] = json!(if value["status"] == "ready" { "Обновление загружено и проверено. Оно установится после закрытия VideoDock." } else if crate::updater::automatic_updates_supported() { "Доступна новая версия VideoDock" } else { "Доступна новая версия VideoDock. Обновите приложение вручную." }); state["update"] = value; Ok(()) }); }
            Ok(None) if manual => notice(&inner, "Установлена актуальная версия"),
            Err(_) => { let _ = change(&inner, |state| { state["update"] = if crate::updater::cached(&inner.directory, &previous).is_some() { previous } else { Value::Null }; if manual { state["notice"] = json!("Не удалось получить обновление. Проверьте подключение и повторите проверку."); } Ok(()) }); }
            _ => {}
        }
    });
    Ok(())
}
