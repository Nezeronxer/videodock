// Native cookie metadata crosses Qt together without exposing it to QML.
#![allow(clippy::too_many_arguments)]
#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;
        include!("platform.h");
        fn initialize_webengine();
        fn create_application() -> UniquePtr<QGuiApplication>;
        fn create_proof_application(resolver_rules: &QString) -> UniquePtr<QGuiApplication>;
        fn application_data_directory() -> QString;
        fn prepare_profile(directory: &QString);
        fn configure_browser_proxy(address: &QString) -> bool;
        fn watch_cookies(backend: Pin<&mut QObject>);
        fn clear_browser_cookies();
        fn install_translation(language: &QString);
        fn reference_asset(name: &QString) -> QString;
        fn initialize_proof_runtime();
        fn run_youtube_proof(
            video_id: &QString,
            proxy: &QString,
            google_address: &QString,
        ) -> QString;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, snapshot)]
        type Backend = super::BackendRust;
        #[qinvokable]
        fn dispatch(self: Pin<&mut Self>, action: &QString) -> QString;
        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "initializeProfile"]
        fn initialize_profile(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "referenceAsset"]
        fn reference_asset(self: &Self, name: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "cookieEvent"]
        fn cookie_event(
            self: Pin<&mut Self>,
            name: &QString,
            value: &QString,
            domain: &QString,
            path: &QString,
            secure: bool,
            expires: i64,
            removed: bool,
        );
    }
}
use cxx_qt::casting::Upcast;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    pin::Pin,
    sync::{Arc, OnceLock},
};
use videodock_core::Engine;

pub struct BackendRust {
    snapshot: QString,
    engine: Option<Arc<Engine>>,
    error: Option<String>,
    profile_connected: bool,
    browser_proxy_supported: bool,
    browser_proxy_restart_required: bool,
}
static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();
static BROWSER_PROXY: OnceLock<(Option<String>, bool)> = OnceLock::new();
static STARTUP_NETWORK_ERROR: OnceLock<String> = OnceLock::new();
fn data_directory() -> PathBuf {
    std::env::var_os("VIDEODOCK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(ffi::application_data_directory().to_string()))
}
pub fn initialize_webengine() {
    ffi::initialize_webengine();
}
pub fn create_application() -> cxx::UniquePtr<cxx_qt_lib::QGuiApplication> {
    ffi::create_application()
}
pub fn create_proof_application(
    resolver_rules: &str,
) -> cxx::UniquePtr<cxx_qt_lib::QGuiApplication> {
    ffi::create_proof_application(&QString::from(resolver_rules))
}
pub fn initialize_proof_runtime() {
    ffi::initialize_proof_runtime();
}
pub fn run_youtube_proof(video_id: &str, proxy: &str, google_address: &str) -> String {
    ffi::run_youtube_proof(
        &QString::from(video_id),
        &QString::from(proxy),
        &QString::from(google_address),
    )
    .to_string()
}
pub fn prepare_profile() -> anyhow::Result<()> {
    let data = data_directory();
    anyhow::ensure!(
        data.is_absolute(),
        "Нужна абсолютная папка данных VideoDock"
    );
    let engine = Arc::new(Engine::new(data.clone())?);
    let directory = data.join("browser");
    std::fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let (proxy, supported) = match engine.network_proxy() {
        Ok(proxy) => {
            let supported =
                ffi::configure_browser_proxy(&QString::from(proxy.clone().unwrap_or_default()));
            (proxy, supported)
        }
        Err(_) => {
            // Keep settings reachable, but leave both browser and Rust networking fail-closed.
            ffi::configure_browser_proxy(&QString::from("unsupported://invalid"));
            let _ = STARTUP_NETWORK_ERROR.set(
                "Не удалось определить системный прокси. Укажите прокси в настройках VideoDock."
                    .into(),
            );
            (None, false)
        }
    };
    let _ = BROWSER_PROXY.set((proxy, supported));
    ffi::prepare_profile(&QString::from(directory.to_string_lossy().as_ref()));
    let state = engine.snapshot();
    ffi::install_translation(&QString::from(
        state["settings"]["language"].as_str().unwrap_or("en"),
    ));
    let _ = ENGINE.set(engine);
    Ok(())
}
impl Default for BackendRust {
    fn default() -> Self {
        let engine = ENGINE.get().cloned();
        let browser_proxy_supported = BROWSER_PROXY.get().is_some_and(|(_, supported)| *supported);
        let error = if engine.is_none() {
            Some("Движок не инициализирован".to_owned())
        } else {
            STARTUP_NETWORK_ERROR.get().cloned()
        };
        Self {
            snapshot: QString::from("{}"),
            engine,
            error,
            profile_connected: false,
            browser_proxy_supported,
            browser_proxy_restart_required: false,
        }
    }
}
impl ffi::Backend {
    pub fn reference_asset(&self, name: &QString) -> QString {
        ffi::reference_asset(name)
    }
    pub fn initialize_profile(mut self: Pin<&mut Self>) {
        if self.rust().profile_connected {
            return;
        }
        self.as_mut().rust_mut().profile_connected = true;
        ffi::watch_cookies(self.upcast_pin());
    }
    pub fn cookie_event(
        self: Pin<&mut Self>,
        name: &QString,
        value: &QString,
        domain: &QString,
        path: &QString,
        secure: bool,
        expires: i64,
        removed: bool,
    ) {
        if let Some(engine) = &self.rust().engine {
            // Cookie values travel directly from Qt to Rust, never through QML properties or logs.
            let _ = engine.dispatch(json!({"action":"account", "command":"cookie", "name":name.to_string(), "value":value.to_string(), "domain":domain.to_string(), "path":path.to_string(), "secure":secure, "expires":u64::try_from(expires).ok(), "removed":removed}));
        }
    }
    pub fn refresh(mut self: Pin<&mut Self>) {
        let mut state = self
            .rust()
            .engine
            .as_ref()
            .map(|engine| engine.snapshot())
            .unwrap_or_else(|| json!({}));
        if let Some(error) = &self.rust().error {
            state["notice"] = Value::String(error.clone());
        }
        state["browserProxySupported"] = json!(self.rust().browser_proxy_supported);
        state["browserProxyRestartRequired"] = json!(self.rust().browser_proxy_restart_required);
        let next = QString::from(state.to_string());
        if self.snapshot() != &next {
            self.as_mut().set_snapshot(next);
        }
    }
    pub fn dispatch(mut self: Pin<&mut Self>, action: &QString) -> QString {
        let result = serde_json::from_str::<Value>(&action.to_string())
            .map_err(anyhow::Error::from)
            .and_then(|action| {
                let logout = action["action"] == "account" && action["command"] == "logout";
                let settings = action["action"] == "settings";
                let response = self
                    .rust()
                    .engine
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Движок недоступен"))?
                    .dispatch(action)?;
                if settings {
                    let proxy = self.rust().engine.as_ref().unwrap().network_proxy()?;
                    let original = BROWSER_PROXY.get();
                    let changed = original.is_none_or(|(initial, _)| initial != &proxy);
                    self.as_mut().rust_mut().browser_proxy_restart_required = changed;
                    self.as_mut().rust_mut().browser_proxy_supported =
                        !changed && original.is_some_and(|(_, supported)| *supported);
                }
                if logout {
                    ffi::clear_browser_cookies();
                }
                Ok(response)
            });
        let response = match result {
            Ok(response) => {
                self.as_mut().rust_mut().error = None;
                response
            }
            Err(error) => {
                let error = error.to_string();
                self.as_mut().rust_mut().error = Some(error.clone());
                json!({"error": error})
            }
        };
        self.as_mut().refresh();
        QString::from(response.to_string())
    }
}
