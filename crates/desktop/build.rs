use cxx_qt_build::{CxxQtBuilder, QmlModule};
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // Deployment rewrites library names and adds Homebrew framework search paths.
        println!("cargo:rustc-link-arg=-Wl,-headerpad_max_install_names");
    }
    let mut module = QmlModule::new("VideoDock");
    for file in [
        "Main",
        "UiTheme",
        "SmartModeBar",
        "ControlPanel",
        "AppDialog",
        "SettingsDialog",
        "ManualDialog",
        "BrowserPane",
        "JobList",
        "TransferDialog",
        "DetailsDialog",
        "OnboardingDialog",
    ] {
        module = module.qml_file(format!("qml/{file}.qml"));
    }
    let builder = CxxQtBuilder::new_qml_module(module)
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("WebEngineQuick")
        .qt_module("WebEngineCore")
        .qt_module("WebEngineWidgets")
        .qt_module("Widgets")
        .qrc("qml/translations.qrc")
        .qrc("qml/icons.qrc")
        .qrc("attestation/proof.qrc")
        .include_dir("include");
    // Only add Qt glue; preserve all compiler flags selected by CXX-Qt.
    unsafe {
        builder.cc_builder(|cc| {
            cc.file("src/platform.cpp");
            cc.file("src/attestation.cpp");
        })
    }
    .files(["src/bridge.rs"])
    .build();
}
