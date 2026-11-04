#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bridge;
mod proof;
use cxx_qt_lib::{QQmlApplicationEngine, QQuickStyle, QString, QUrl};
fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--youtube-proof")) {
        proof::run();
        return;
    }
    if let Ok(executable) = std::env::current_exe() {
        let _ = videodock_media::youtube::register_proof_worker(executable);
    }
    bridge::initialize_webengine();
    let mut application = bridge::create_application();
    if bridge::prepare_profile().is_err() {
        eprintln!("Не удалось открыть защищённые данные VideoDock");
        std::process::exit(1);
    }
    QQuickStyle::set_style(&QString::from("Basic"));
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        engine
            .as_mut()
            .on_object_creation_failed(|_, _| {
                std::process::exit(1);
            })
            .release();
        engine.load(&QUrl::from("qrc:/qt/qml/VideoDock/qml/Main.qml"));
    }
    if let Some(application) = application.as_mut() {
        application.exec();
    }
}
