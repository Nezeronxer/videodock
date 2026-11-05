mod exchange;
mod queue;
mod secrets;
mod settings;

use crate::{store::Store, transfer::Limiter};
use anyhow::{Context, Result, anyhow, ensure};
use secrets::Secrets;
use serde_json::{Value, json};
use settings::{defaults, selection};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{runtime::Runtime, sync::Notify};
use tokio_util::sync::CancellationToken;
use videodock_media::{Media, Selection};

#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
    _owner: Arc<Owner>,
}

struct Owner {
    cancel: CancellationToken,
    inner: Weak<Inner>,
    _lease: std::fs::File,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(inner) = self.inner.upgrade() {
            drop(lock(&inner.state));
        }
        // A concurrent fork may retain the file description until its child execs.
        let _ = self._lease.unlock();
    }
}

struct Inner {
    dispatch_gate: Mutex<()>,
    state: Mutex<Value>,
    store: Mutex<Store>,
    secrets: Mutex<Secrets>,
    directory: PathBuf,
    shutdown: CancellationToken,
    wake: Arc<Notify>,
    active: Mutex<HashMap<String, CancellationToken>>,
    checking_subscriptions: Mutex<HashSet<String>>,
    manual: Mutex<Option<Media>>,
    parse_cancel: Mutex<Option<CancellationToken>>,
    parse_paused: AtomicBool,
    parse_wake: Arc<Notify>,
    generation: AtomicU64,
    auth_generation: AtomicU64,
    limiter: Arc<Limiter>,
    library_last: AtomicU64,
    library_scanning: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn runtime() -> Result<&'static Runtime> {
    static RUNTIME: OnceLock<std::result::Result<Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .thread_name("videodock")
                .build()
                .map_err(|_| "Не удалось запустить рабочие потоки".to_owned())
        })
        .as_ref()
        .map_err(|error| anyhow!(error.clone()))
}

impl Engine {
    /// Native browser integration only; never include this credential-bearing value in snapshots.
    pub fn network_proxy(&self) -> Result<Option<String>> {
        secrets::proxy(&self.inner)
    }

    pub fn new(directory: PathBuf) -> Result<Self> {
        ensure!(
            directory.is_absolute() && directory.file_name().is_some(),
            "Нужна отдельная абсолютная папка данных VideoDock"
        );
        std::fs::create_dir_all(&directory)?;
        let canonical = directory.canonicalize()?;
        ensure!(
            !canonical
                .to_string_lossy()
                .to_lowercase()
                .contains("dropbox"),
            "SQLite и защищённые данные должны находиться в локальной папке вне Dropbox"
        );
        ensure!(
            Some(canonical.as_os_str())
                != std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).as_deref()
                && canonical != std::env::temp_dir(),
            "Нужна отдельная папка приложения, а не домашний или временный каталог целиком"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lease = options.open(directory.join("state.lock"))?;
        lease
            .try_lock()
            .map_err(|_| anyhow!("Этот каталог данных уже открыт другим экземпляром VideoDock"))?;
        let store = Store::open(&directory.join("state.sqlite"))?;
        let secrets = secrets::load(&directory)?;
        let mut state = store.read()?.unwrap_or_else(initial_state);
        ensure!(state.is_object(), "Повреждено состояние приложения");
        let retired_media_tools = retire_media_tools(&mut state);
        let mut settings = defaults();
        if let Some(saved) = state["settings"].as_object() {
            for (key, value) in saved {
                if settings.get(key).is_some()
                    && key != "proxyPassword"
                    && key != "proxyPasswordSet"
                {
                    settings[key] = settings::normalize(key, value)
                        .with_context(|| format!("Повреждена настройка {key}"))?;
                }
            }
        }
        settings["speedLimit"] = json!(0);
        #[cfg(target_os = "linux")]
        if crate::platform::flatpak() {
            settings["autoStart"] = json!(false);
            settings["preventSleepEnabled"] = json!(false);
        }
        settings["proxyPasswordSet"] = json!(!secrets.proxy_password.is_empty());
        state["settings"] = settings;
        for key in ["jobs", "subscriptions"] {
            if !state[key].is_array() {
                state[key] = json!([]);
            }
            ensure!(
                state[key].as_array().unwrap().len() <= 250_000,
                "Слишком много сохранённых элементов"
            );
            for item in state[key].as_array_mut().unwrap() {
                ensure!(
                    item.is_object()
                        && item["id"]
                            .as_str()
                            .is_some_and(|id| !id.is_empty() && id.len() <= 128),
                    "Повреждён сохранённый элемент"
                );
                if matches!(
                    item["state"].as_str(),
                    Some("downloading" | "parsing" | "processing" | "removing")
                ) {
                    item["state"] = json!("paused");
                    item["error"] = json!(
                        "Работа прервана. Нажмите Продолжить для получения свежих ссылок и возобновления."
                    );
                }
            }
        }
        state["manual"] = Value::Null;
        state["account"] = json!({"logged_in":false,"user_name":"","avatar_url":""});
        state["capabilities"] = json!({"browser":true,"tray":true,"auto_start":true});
        #[cfg(target_os = "linux")]
        if crate::platform::flatpak() {
            state["capabilities"]["auto_start"] = json!(false);
            state["capabilities"]["prevent_sleep"] = json!(false);
        }
        if retired_media_tools {
            store.save(&persisted(&state))?;
        }
        let shutdown = CancellationToken::new();
        let inner = Arc::new(Inner {
            dispatch_gate: Mutex::new(()),
            state: Mutex::new(state),
            store: Mutex::new(store),
            secrets: Mutex::new(secrets),
            directory,
            shutdown: shutdown.clone(),
            wake: Arc::new(Notify::new()),
            active: Mutex::new(HashMap::new()),
            checking_subscriptions: Mutex::new(HashSet::new()),
            manual: Mutex::new(None),
            parse_cancel: Mutex::new(None),
            parse_paused: AtomicBool::new(false),
            parse_wake: Arc::new(Notify::new()),
            generation: AtomicU64::new(0),
            auth_generation: AtomicU64::new(0),
            limiter: Arc::new(Limiter::new(0)),
            library_last: AtomicU64::new(0),
            library_scanning: AtomicBool::new(false),
        });
        runtime()?;
        let engine = Self {
            inner: inner.clone(),
            _owner: Arc::new(Owner {
                cancel: shutdown,
                inner: Arc::downgrade(&inner),
                _lease: lease,
            }),
        };
        queue::start(&inner)?;
        secrets::verify_debounced(&inner);
        Ok(engine)
    }

    pub fn snapshot(&self) -> Value {
        queue::public_state(&lock(&self.inner.state))
    }

    pub fn dispatch(&self, action: Value) -> Result<Value> {
        let _dispatch = lock(&self.inner.dispatch_gate);
        ensure!(action.is_object(), "Действие должно быть объектом JSON");
        let kind = action["action"].as_str().context("Не указано действие")?;
        match kind {
            "settings" => {
                settings::update(&self.inner, &action["values"])?;
                Ok(json!({}))
            }
            "parse" => queue::parse(&self.inner, &action),
            "preview_output" => queue::preview_output(&self.inner, &action),
            "download" => queue::download(&self.inner, &action),
            "manual_pause" | "manual_resume" => queue::parsing_control(&self.inner, &action),
            "jobs" => queue::jobs(&self.inner, &action),
            "subscriptions" => queue::subscriptions(&self.inner, &action),
            "process" => anyhow::bail!("Медиаинструменты удалены из VideoDock"),
            "account" => secrets::account(&self.inner, &action),
            "import" => {
                exchange::import(&self.inner, &action)?;
                Ok(json!({}))
            }
            "export" => exchange::export(&self.inner, &action),
            "manual_close" => {
                self.inner.generation.fetch_add(1, Ordering::SeqCst);
                if let Some(cancel) = lock(&self.inner.parse_cancel).take() {
                    cancel.cancel();
                }
                change(&self.inner, |state| {
                    *lock(&self.inner.manual) = None;
                    state["manual"] = Value::Null;
                    Ok(())
                })?;
                Ok(json!({}))
            }
            "app" => app_action(&self.inner, &action),
            _ => anyhow::bail!("Неизвестное действие: {kind}"),
        }
    }
}

// Preserve former output/history records privately; they must never become executable download jobs.
fn retire_media_tools(state: &mut Value) -> bool {
    let object = state.as_object_mut().expect("validated state object");
    let Some(legacy) = object.remove("processing") else {
        return false;
    };
    if legacy.as_array().is_some_and(Vec::is_empty) {
        return true;
    }
    match object.entry("_legacy_processing") {
        serde_json::map::Entry::Vacant(entry) => {
            entry.insert(legacy);
        }
        serde_json::map::Entry::Occupied(mut entry) => {
            let archive = entry.get_mut();
            match legacy {
                Value::Array(mut rows) if archive.is_array() => {
                    archive.as_array_mut().unwrap().append(&mut rows)
                }
                value => {
                    let previous = archive.take();
                    *archive = json!([previous, value]);
                }
            }
        }
    }
    true
}

fn initial_state() -> Value {
    json!({"settings":defaults(),"jobs":[],"subscriptions":[],"manual":null,
        "account":{"logged_in":false,"user_name":"","avatar_url":""},"notice":"","notification":null,"onboarding_completed":false})
}
fn persisted(state: &Value) -> Value {
    let mut state = state.clone();
    state["manual"] = Value::Null;
    state["account"] = json!({"logged_in":false,"user_name":"","avatar_url":""});
    if let Some(settings) = state["settings"].as_object_mut() {
        settings.remove("proxyPassword");
    }
    state
}
fn change<T>(inner: &Inner, operation: impl FnOnce(&mut Value) -> Result<T>) -> Result<T> {
    let mut state = lock(&inner.state);
    ensure!(!inner.shutdown.is_cancelled(), "Операция отменена");
    let mut next = state.clone();
    let result = operation(&mut next)?;
    secrets::seal_sources(inner, &mut next)?;
    lock(&inner.store).save(&persisted(&next))?;
    *state = next;
    inner.wake.notify_one();
    Ok(result)
}
fn notice(inner: &Inner, error: impl std::fmt::Display) {
    let message = error.to_string().chars().take(1500).collect::<String>();
    let saved = change(inner, |state| {
        state["notice"] = json!(message);
        Ok(())
    });
    if saved.is_err() && !inner.shutdown.is_cancelled() {
        lock(&inner.state)["notice"] = json!(message);
    }
}
fn notify(state: &mut Value, title: &str, body: &str) {
    state["notification"] =
        json!({"id":uuid::Uuid::new_v4().to_string(),"title":title,"body":body});
}
fn app_action(inner: &Arc<Inner>, action: &Value) -> Result<Value> {
    match action["command"]
        .as_str()
        .context("Не указано действие приложения")?
    {
        "clear_notice" => change(inner, |state| {
            state["notice"] = json!("");
            Ok(())
        })?,
        "onboarding_complete" => change(inner, |state| {
            state["onboarding_completed"] = json!(true);
            Ok(())
        })?,
        "open_directory" => {
            let path = PathBuf::from(
                lock(&inner.state)["settings"]["path"]
                    .as_str()
                    .context("Нет папки загрузок")?,
            );
            std::fs::create_dir_all(&path)?;
            crate::platform::open_path(&path)?;
        }
        "open_logs" => {
            let path = inner.directory.join("logs");
            std::fs::create_dir_all(&path)?;
            crate::platform::open_path(&path)?;
        }
        "check_updates" => {
            let inner = inner.clone();
            runtime()?.spawn(async move {
                let result = async {
                    let catalog = queue::catalog(&inner)?;
                    crate::platform::check_updates(&catalog.client()).await
                }
                .await;
                match result {
                    Ok(Some(update)) => {
                        let _ = change(&inner, |state| {
                            state["update"] = update;
                            state["notice"] = json!("Доступна новая версия");
                            Ok(())
                        });
                    }
                    Ok(None) => notice(&inner, "Установлена актуальная версия"),
                    Err(error) => notice(&inner, error),
                }
            });
        }
        _ => anyhow::bail!("Неизвестное действие приложения"),
    }
    Ok(json!({}))
}

#[cfg(all(test, unix))]
mod lease_tests {
    use super::*;

    #[test]
    fn last_owner_releases_profile_even_if_a_child_inherited_the_descriptor() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::new(directory.path().to_owned()).unwrap();
        let clone = engine.clone();
        // A forked child retains this open file description until exec closes it.
        let inherited = engine._owner._lease.try_clone().unwrap();
        drop(engine);
        assert!(Engine::new(directory.path().to_owned()).is_err());
        drop(clone);
        let reopened = Engine::new(directory.path().to_owned()).unwrap();
        assert!(Engine::new(directory.path().to_owned()).is_err());
        drop(reopened);
        drop(inherited);
    }
}
