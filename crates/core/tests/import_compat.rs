use serde_json::{Value, json};
use std::{fs, net::TcpListener, path::Path, time::Duration};
use tempfile::TempDir;
use videodock_core::Engine;

const VIDEO: &str =
    "URL,Title,Type,Format,AudioBitrate,VideoResolution,FrameRate,Subtitles,OutputDir,Version 1";
const AUDIO: &str = "URL,Title,Type,Format,AudioBitrate,OutputDir,Version 1";
const URL: &str = "http://127.0.0.1:9/legacy-fixture";

fn isolated_engine(data: &TempDir) -> Engine {
    let engine = Engine::new(data.path().join("state")).unwrap();
    engine
        .dispatch(json!({"action":"settings","values":{"path":data.path().join("output")}}))
        .unwrap();
    engine
}

fn import(engine: &Engine, path: &Path, output: &Path, use_file: bool) -> anyhow::Result<Value> {
    engine.dispatch(
        json!({"action":"import","path":path,"output_dir":output,"preferences_from_file":use_file}),
    )
}

fn exported(engine: &Engine, data: &Path, name: &str) -> Value {
    let path = data.join(name);
    engine
        .dispatch(json!({"action":"export","path":path}))
        .unwrap();
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn legacy_video_preserves_quoted_unicode_title_and_file_preferences() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    let path = data.path().join("video.csv");
    fs::write(
        &path,
        format!(
            "{VIDEO}\n{URL},\"Пример, \"\"один\"\"\",Video,mKv,192000,720p,60FPS,fr-en-fr,/unused,\n"
        ),
    )
    .unwrap();
    import(&engine, &path, data.path(), true).unwrap();
    let document = exported(&engine, data.path(), "video.json");
    let item = &document["items"][0];
    assert_eq!(item["playlistTitle"], "Пример, \"один\"");
    assert_eq!(item["isSubscription"], false);
    assert_eq!(item["downloadPreferences"]["outputDir"], json!(data.path()));
    assert_eq!(item["downloadPreferences"]["format"]["container"], "MKV");
    assert_eq!(
        item["downloadPreferences"]["format"]["video"]["resolution"],
        "720p"
    );
    assert_eq!(
        item["downloadPreferences"]["format"]["video"]["fps"],
        "60fps"
    );
    // The original video CSV decoder supplies Any audio bitrate, regardless of this column.
    assert_eq!(
        item["downloadPreferences"]["format"]["audio"]["bitrate"],
        -1
    );
    assert_eq!(
        item["downloadPreferences"]["subtitles"],
        json!([{"lang":"en"},{"lang":"fr"}])
    );
}

#[test]
fn legacy_audio_supports_semicolon_and_qt_invalid_integer_default() {
    for (bitrate, expected) in [
        ("128000", 128000),
        ("garbage", -1),
        ("2147483648", -1),
        ("", -1),
    ] {
        let data = TempDir::new().unwrap();
        let engine = isolated_engine(&data);
        let path = data.path().join("audio.csv");
        fs::write(
            &path,
            format!(
                "{}\r\n{URL};Example;Audio;mP3;{bitrate};/unused;\r\n",
                AUDIO.replace(',', ";")
            ),
        )
        .unwrap();
        import(&engine, &path, data.path(), true).unwrap();
        let document = exported(&engine, data.path(), "audio.json");
        let format = &document["items"][0]["downloadPreferences"]["format"];
        assert_eq!(format["container"], "MP3");
        assert!(format.get("video").is_none());
        assert_eq!(format["audio"]["bitrate"], expected);
    }
}

#[test]
fn legacy_profile_is_recognized_in_csv_txt_and_4kv_carriers() {
    for extension in ["csv", "CSV", "txt", "4kv"] {
        let data = TempDir::new().unwrap();
        let engine = isolated_engine(&data);
        let path = data.path().join(format!("legacy.{extension}"));
        fs::write(&path, format!("\u{feff}{AUDIO}\r\n{URL},Example,Audio,MP3,192000,/unused,\r\n{URL},Duplicate,Audio,MP3,192000,/unused,\r\n")).unwrap();
        import(&engine, &path, data.path(), true).unwrap();
        assert_eq!(engine.snapshot()["jobs"].as_array().unwrap().len(), 1);
        assert!(
            engine.snapshot()["subscriptions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn smart_mode_overrides_legacy_file_preferences_and_output() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    engine.dispatch(json!({"action":"settings","values":{"container":"mp4","quality":1080,"path":data.path()}})).unwrap();
    let path = data.path().join("audio.txt");
    fs::write(
        &path,
        format!("{AUDIO}\n{URL},Example,Audio,MP3,128000,relative-unused,\n"),
    )
    .unwrap();
    engine
        .dispatch(json!({"action":"import","path":path,"preferences_from_file":false}))
        .unwrap();
    let document = exported(&engine, data.path(), "smart.json");
    let preferences = &document["items"][0]["downloadPreferences"];
    assert_eq!(preferences["format"]["container"], "MP4");
    assert_eq!(preferences["format"]["video"]["resolution"], "1080p");
    assert_eq!(preferences["outputDir"], json!(data.path()));
}

#[test]
fn legacy_uses_file_output_when_no_explicit_override() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    let path = data.path().join("audio.csv");
    let output = data.path().join("выгрузка");
    fs::write(
        &path,
        format!(
            "{AUDIO}\n{URL},Example,Audio,OGG,128000,{},\n",
            output.display()
        ),
    )
    .unwrap();
    engine
        .dispatch(json!({"action":"import","path":path,"preferences_from_file":true}))
        .unwrap();
    let document = exported(&engine, data.path(), "output.json");
    assert_eq!(
        document["items"][0]["downloadPreferences"]["outputDir"],
        json!(output)
    );
}

#[test]
fn legacy_resolution_aliases_and_dimension_do_not_inherit_smart_mode() {
    for (resolution, expected, fps, expected_fps) in [
        ("QCIF", "144p", "25", "25fps"),
        ("2k", "1440p", "120", "120fps"),
        ("", "Original", "Any", "maximum fps"),
        ("Original", "Original", "max", "maximum fps"),
    ] {
        let data = TempDir::new().unwrap();
        let engine = isolated_engine(&data);
        engine
            .dispatch(json!({"action":"settings","values":{"videoVR":true,"quality":1080}}))
            .unwrap();
        let path = data.path().join("alias.csv");
        fs::write(
            &path,
            format!("{VIDEO}\n{URL},Example,Video,MP4,,{resolution},{fps},,/unused,\n"),
        )
        .unwrap();
        import(&engine, &path, data.path(), true).unwrap();
        let document = exported(&engine, data.path(), "alias.json");
        let video = &document["items"][0]["downloadPreferences"]["format"]["video"];
        assert_eq!(video["resolution"], expected);
        assert_eq!(video["fps"], expected_fps);
        assert_eq!(video["dimension"], "2D");
    }
}

#[test]
fn invalid_legacy_is_atomic_before_any_source_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/must-not-request", listener.local_addr().unwrap());
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    let path = data.path().join("bad.csv");
    let good = format!("{url},First,Audio,MP3,128000,/unused,");
    for bad in [
        "file:///tmp/forbidden,Second,Audio,MP3,128000,/unused,",
        "http://user:password@127.0.0.1/,Second,Audio,MP3,128000,/unused,",
        "https://example.invalid/,Second,audio,MP3,128000,/unused,",
        "https://example.invalid/,Second,Audio,unknown,128000,/unused,",
        "https://example.invalid/,Second,Audio,MP3,1,/unused,",
        "https://example.invalid/,Second,Audio,MP3,128000,/unused",
        "https://example.invalid/,Second,Audio,MP3,128000,/unused,,extra",
        "https://example.invalid/,\"unterminated,Audio,MP3,128000,/unused,",
    ] {
        fs::write(&path, format!("{AUDIO}\n{good}\n{bad}\n")).unwrap();
        assert!(
            import(&engine, &path, data.path(), true).is_err(),
            "accepted {bad}"
        );
        assert!(engine.snapshot()["jobs"].as_array().unwrap().is_empty());
        assert!(
            engine.snapshot()["subscriptions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn unknown_layouts_are_not_guessed_from_extension() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    for (name, content) in [
        ("plain.txt", URL.to_owned()),
        ("plain.4kv", URL.to_owned()),
        ("url.csv", format!("URL\n{URL}\n")),
        (
            "reordered.csv",
            format!(
                "Title,URL,Type,Format,AudioBitrate,OutputDir,Version 1\nTitle,{URL},Audio,MP3,128000,/unused,\n"
            ),
        ),
        (
            "json.4kv",
            "{\"json_schema_version\":\"1.0.0\",\"items\":[]}".to_owned(),
        ),
    ] {
        let path = data.path().join(name);
        fs::write(&path, content).unwrap();
        assert!(
            import(&engine, &path, data.path(), true).is_err(),
            "accepted {name}"
        );
    }
    assert!(engine.snapshot()["jobs"].as_array().unwrap().is_empty());
}

#[test]
fn legacy_limits_and_absolute_paths_are_enforced_atomically() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    let path = data.path().join("large.csv");
    fs::write(&path, vec![b'x'; 16 * 1024 * 1024 + 1]).unwrap();
    assert!(import(&engine, &path, data.path(), true).is_err());
    fs::write(
        &path,
        format!(
            "{AUDIO}\n{}",
            format!("{URL},Example,Audio,MP3,128000,/unused,\n").repeat(10001)
        ),
    )
    .unwrap();
    assert!(import(&engine, &path, data.path(), true).is_err());
    fs::write(
        &path,
        format!("{AUDIO}\n{URL},Example,Audio,MP3,128000,relative,\n"),
    )
    .unwrap();
    assert!(
        engine
            .dispatch(json!({"action":"import","path":path,"preferences_from_file":true}))
            .is_err()
    );
    assert!(import(&engine, Path::new("relative.csv"), data.path(), true).is_err());
    assert!(engine.snapshot()["jobs"].as_array().unwrap().is_empty());
}

#[test]
fn legacy_private_sources_keep_existing_export_exclusion() {
    let data = TempDir::new().unwrap();
    let engine = isolated_engine(&data);
    let path = data.path().join("private.csv");
    let secret = "synthetic-import-secret";
    fs::write(
        &path,
        format!("{AUDIO}\n{URL}?token={secret},Example,Audio,MP3,128000,/unused,\n"),
    )
    .unwrap();
    import(&engine, &path, data.path(), true).unwrap();
    assert!(!engine.snapshot().to_string().contains(secret));
    let document = exported(&engine, data.path(), "private.json");
    assert!(document["items"].as_array().unwrap().is_empty());
    assert!(!document.to_string().contains(secret));
}

#[test]
fn legacy_bom_encodings_match_qtextstream_unicode_detection() {
    let text = format!("{AUDIO}\r\n{URL},Пример,Audio,MP3,128000,/unused,\r\n");
    for (bom, width, little_endian) in [
        (&b"\xff\xfe"[..], 2, true),
        (&b"\xfe\xff"[..], 2, false),
        (&b"\xff\xfe\0\0"[..], 4, true),
        (&b"\0\0\xfe\xff"[..], 4, false),
    ] {
        let data = TempDir::new().unwrap();
        let engine = isolated_engine(&data);
        let path = data.path().join("unicode.4kv");
        let mut bytes = bom.to_vec();
        if width == 2 {
            for unit in text.encode_utf16() {
                bytes.extend(if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
        } else {
            for unit in text.chars().map(u32::from) {
                bytes.extend(if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
        }
        fs::write(&path, bytes).unwrap();
        import(&engine, &path, data.path(), true).unwrap();
        assert_eq!(engine.snapshot()["jobs"][0]["title"], "Пример");
    }
}
