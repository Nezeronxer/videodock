use serde_json::json;
use videodock_media::{Format, Selection, choose_formats};

fn format(id: &str, projection: Option<&str>, vr: bool, height: u32) -> Format {
    serde_json::from_value(json!({"id":id,"url":"https://media.example/fixture","protocol":"http","ext":"mp4","height":height,"video_codec":"h264","audio_codec":"aac","headers":{},"vr":vr,"projection":projection})).unwrap()
}
fn selection(mode: &str) -> Selection {
    serde_json::from_value(json!({"quality":0,"vr_mode":mode})).unwrap()
}

#[test]
fn exact_projection_does_not_fall_back_to_a_different_vr_shape() {
    let formats = vec![
        format("stereo180", Some("3D/180°"), true, 720),
        format("stereo360", Some("3D/360°"), true, 1080),
        format("mono360", Some("2D/360°"), true, 2160),
        format("unknown-vr", None, true, 4320),
    ];
    for (mode, id) in [
        ("2D/360°", "mono360"),
        ("3D/180°", "stereo180"),
        ("3D/360°", "stereo360"),
    ] {
        assert_eq!(
            choose_formats(&formats, &selection(mode)).unwrap()[0].id,
            id
        );
    }
    let mut exact = selection("3D/180°");
    exact.format_id = Some("stereo360".into());
    assert!(choose_formats(&formats, &exact).is_err());
    assert!(
        choose_formats(
            &[format("unknown", None, true, 1080)],
            &selection("3D/180°")
        )
        .is_err()
    );
}

#[test]
fn none_excludes_vr_any_accepts_it_and_legacy_true_remains_any_vr() {
    let flat = format("flat", Some("None"), false, 720);
    let unknown = format("legacy-vr", None, true, 2160);
    let formats = [flat.clone(), unknown.clone()];
    assert_eq!(
        choose_formats(&formats, &selection("None")).unwrap()[0].id,
        "flat"
    );
    assert!(choose_formats(std::slice::from_ref(&unknown), &selection("None")).is_err());
    assert_eq!(
        choose_formats(&formats, &selection("Any")).unwrap()[0].id,
        "legacy-vr"
    );
    let legacy: Selection = serde_json::from_value(json!({"vr":true,"quality":0})).unwrap();
    assert_eq!(
        choose_formats(&formats, &legacy).unwrap()[0].id,
        "legacy-vr"
    );
    assert!(choose_formats(&[flat], &legacy).is_err());
}
