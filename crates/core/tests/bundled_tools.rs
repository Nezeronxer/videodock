#![cfg(any(windows, target_os = "macos"))]

use std::{env, fs, process::Command};

#[test]
fn bundled_tools_take_precedence_over_path() {
    let executable = env::current_exe().unwrap();
    let name = |tool: &str| {
        if cfg!(windows) {
            format!("{tool}.exe")
        } else {
            tool.into()
        }
    };
    if env::var_os("VIDEODOCK_BUNDLED_TOOL_CHILD").is_some() {
        for tool in ["ffmpeg", "ffprobe"] {
            assert_eq!(
                videodock_core::platform::media_tool(tool),
                executable.parent().unwrap().join(name(tool))
            );
        }
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let child = directory.path().join(name("tool-check"));
    fs::copy(executable, &child).unwrap();
    for tool in ["ffmpeg", "ffprobe"] {
        fs::write(
            directory.path().join(name(tool)),
            b"test fixture; never executed",
        )
        .unwrap();
    }
    assert!(
        Command::new(child)
            .args(["--exact", "bundled_tools_take_precedence_over_path"])
            .env("VIDEODOCK_BUNDLED_TOOL_CHILD", "1")
            .env("PATH", "")
            .status()
            .unwrap()
            .success()
    );
}
