use serde_json::json;
use videodock_media::{Format, Selection, choose_formats};

fn audio(
    id: &str,
    track: Option<&str>,
    language: Option<&str>,
    default: Option<bool>,
    original: Option<bool>,
    bitrate: u64,
) -> Format {
    let mut value = serde_json::to_value(Format {
        id: id.into(),
        audio_codec: Some("aac".into()),
        audio_language: language.map(str::to_owned),
        bitrate: Some(bitrate),
        ..Default::default()
    })
    .unwrap();
    value["audio_track_id"] = json!(track);
    value["audio_default"] = json!(default);
    value["audio_original"] = json!(original);
    serde_json::from_value(value).unwrap()
}

fn selection(tracks: &[&str]) -> Selection {
    Selection {
        media_type: "audio".into(),
        audio_tracks: tracks.iter().map(|v| (*v).into()).collect(),
        ..Default::default()
    }
}
fn ids(formats: &[Format], tracks: &[&str]) -> Vec<String> {
    choose_formats(formats, &selection(tracks))
        .unwrap()
        .iter()
        .map(|f| f.id.clone())
        .collect()
}

#[test]
fn original_is_explicit_metadata_not_the_loudest_or_default_language() {
    let formats = [
        audio(
            "source-en",
            Some("en.0"),
            Some("en-US"),
            Some(false),
            Some(true),
            128,
        ),
        audio(
            "dubbed",
            Some("ru.0"),
            Some("ru"),
            Some(true),
            Some(false),
            256,
        ),
    ];
    assert_eq!(ids(&formats, &["original"]), ["source-en"]);
}

#[test]
fn original_does_not_guess_when_multiple_track_types_are_unknown() {
    let formats = [
        audio("en", Some("en.0"), Some("en"), None, None, 128),
        audio("ru", Some("ru.0"), Some("ru"), None, None, 256),
    ];
    assert!(choose_formats(&formats, &selection(&["original"])).is_err());
}

#[test]
fn default_aliases_and_implicit_default_use_declared_default() {
    let formats = [
        audio(
            "original",
            Some("en.0"),
            Some("en-US"),
            Some(false),
            Some(true),
            256,
        ),
        audio(
            "localized-default",
            Some("ru.0"),
            Some("ru"),
            Some(true),
            Some(false),
            128,
        ),
    ];
    for requested in [vec![], vec!["default"], vec!["-"]] {
        assert_eq!(ids(&formats, &requested), ["localized-default"]);
    }
}

#[test]
fn default_falls_back_only_without_declared_metadata_and_preserves_codec_errors() {
    let legacy = [
        audio("legacy-low", None, None, None, None, 64),
        audio("legacy-high", None, None, None, None, 128),
    ];
    assert_eq!(ids(&legacy, &["-"]), ["legacy-high"]);
    let declared = [audio(
        "not-default",
        Some("en.0"),
        Some("en"),
        Some(false),
        Some(true),
        128,
    )];
    assert!(choose_formats(&declared, &selection(&["default"])).is_err());
    let mut codec = selection(&["default"]);
    codec.audio_codec = "opus".into();
    assert!(choose_formats(&legacy, &codec).is_err());
}

#[test]
fn unavailable_default_codec_does_not_substitute_another_track() {
    let mut default = audio(
        "default-opus",
        Some("ru.0"),
        Some("ru"),
        Some(true),
        Some(false),
        128,
    );
    default.audio_codec = Some("opus".into());
    let other = audio(
        "other-aac",
        Some("en.0"),
        Some("en"),
        Some(false),
        Some(true),
        256,
    );
    let mut wanted = selection(&["default"]);
    wanted.audio_codec = "aac".into();
    assert!(choose_formats(&[default, other], &wanted).is_err());
}

#[test]
fn all_errors_when_one_track_has_no_requested_codec_instead_of_omitting_it() {
    let mut english = audio(
        "en-opus",
        Some("en.0"),
        Some("en"),
        Some(true),
        Some(true),
        128,
    );
    english.audio_codec = Some("opus".into());
    let russian = audio(
        "ru-aac",
        Some("ru.0"),
        Some("ru"),
        Some(false),
        Some(false),
        128,
    );
    let mut wanted = selection(&["all"]);
    wanted.audio_codec = "opus".into();
    assert!(choose_formats(&[english, russian], &wanted).is_err());
}

#[test]
fn all_deduplicates_formats_per_actual_track_and_keeps_same_language_variants() {
    let formats = [
        audio(
            "en-low",
            Some("en.0"),
            Some("en-US"),
            Some(true),
            Some(true),
            96,
        ),
        audio(
            "en-high",
            Some("en.0"),
            Some("en-US"),
            Some(true),
            Some(true),
            192,
        ),
        audio(
            "en-description",
            Some("en.1"),
            Some("en-US"),
            Some(false),
            Some(false),
            96,
        ),
        audio(
            "ru-low",
            Some("ru.0"),
            Some("ru"),
            Some(false),
            Some(false),
            64,
        ),
        audio(
            "ru-high",
            Some("ru.0"),
            Some("ru"),
            Some(false),
            Some(false),
            128,
        ),
    ];
    let selected = ids(&formats, &["all", "original", "en"]);
    assert_eq!(selected.len(), 3);
    for id in ["en-high", "en-description", "ru-high"] {
        assert!(selected.iter().any(|v| v == id));
    }
}

#[test]
fn all_does_not_add_the_unidentified_muxed_copy_or_drop_video() {
    let mut muxed = audio("video", None, None, None, None, 1_000_000);
    muxed.video_codec = Some("avc1".into());
    muxed.height = Some(720);
    let formats = [
        muxed,
        audio("en", Some("en.0"), Some("en"), Some(true), Some(true), 128),
        audio(
            "ru",
            Some("ru.0"),
            Some("ru"),
            Some(false),
            Some(false),
            128,
        ),
    ];
    let selected = choose_formats(
        &formats,
        &Selection {
            audio_tracks: vec!["all".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].id, "video");
    assert!(
        !selected[0].has_audio(),
        "Muxed audio would be mapped alongside the explicit tracks"
    );
    assert!(selected[1..].iter().all(|f| f.has_audio()));
}

#[test]
fn legacy_default_reuses_the_selected_muxed_stream() {
    let mut muxed = audio("video", None, None, None, None, 1_000_000);
    muxed.video_codec = Some("avc1".into());
    muxed.height = Some(720);
    let result = choose_formats(
        &[muxed],
        &Selection {
            audio_tracks: vec!["-".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].has_audio());
}

#[test]
fn declared_default_and_original_replace_unknown_muxed_audio_without_mutating_source() {
    let mut video = audio("video", None, None, None, None, 1_000_000);
    video.video_codec = Some("avc1".into());
    video.height = Some(720);
    let formats = [
        video,
        audio(
            "source-en",
            Some("en.0"),
            Some("en"),
            Some(false),
            Some(true),
            128,
        ),
        audio(
            "default-ru",
            Some("ru.0"),
            Some("ru"),
            Some(true),
            Some(false),
            192,
        ),
    ];
    for (token, expected) in [
        ("original", "source-en"),
        ("default", "default-ru"),
        ("-", "default-ru"),
    ] {
        let result = choose_formats(
            &formats,
            &Selection {
                format_id: Some("video".into()),
                audio_tracks: vec![token.into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            result
                .iter()
                .map(|format| format.id.as_str())
                .collect::<Vec<_>>(),
            ["video", expected]
        );
        assert!(!result[0].has_audio());
        assert!(
            formats[0].has_audio(),
            "Planning mutated the source inventory"
        );
    }
}

#[test]
fn selected_muxed_track_is_retained_when_another_audio_track_is_added() {
    let mut video = audio(
        "video",
        Some("en.0"),
        Some("en"),
        Some(true),
        Some(true),
        1_000_000,
    );
    video.video_codec = Some("avc1".into());
    video.height = Some(720);
    let formats = [
        video,
        audio(
            "ru",
            Some("ru.0"),
            Some("ru"),
            Some(false),
            Some(false),
            128,
        ),
    ];
    for requested in [vec!["all"], vec!["-", "ru"], vec!["original", "ru"]] {
        let result = choose_formats(
            &formats,
            &Selection {
                audio_tracks: requested.iter().map(|value| (*value).into()).collect(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            result
                .iter()
                .map(|format| format.id.as_str())
                .collect::<Vec<_>>(),
            ["video", "ru"]
        );
        assert!(
            result[0].has_audio(),
            "The selected embedded track was lost from the plan"
        );
    }
}

#[test]
fn language_bases_match_regions_but_explicit_regions_remain_distinct() {
    let formats = [audio(
        "en-US",
        Some("english-track"),
        Some("en-US"),
        None,
        None,
        128,
    )];
    assert_eq!(ids(&formats, &["EN"]), ["en-US"]);
    assert_eq!(ids(&formats, &["en_us"]), ["en-US"]);
    assert!(choose_formats(&formats, &selection(&["en-GB"])).is_err());
}

#[test]
fn legacy_language_aliases_match_at_the_language_boundary() {
    for (actual, requested) in [
        ("iw-IL", "he"),
        ("he-IL", "iw"),
        ("fil-PH", "fl"),
        ("tl-PH", "fil"),
        ("fil-PH", "tl"),
    ] {
        assert_eq!(
            ids(
                &[audio("track", None, Some(actual), None, None, 128)],
                &[requested]
            ),
            ["track"]
        );
    }
    assert!(
        choose_formats(
            &[audio("track", None, Some("hebrew"), None, None, 128)],
            &selection(&["he"])
        )
        .is_err()
    );
}

#[test]
fn explicit_track_id_selects_its_best_format() {
    let formats = [
        audio("low", Some("en.1"), Some("en"), None, None, 64),
        audio("high", Some("en.1"), Some("en"), None, None, 128),
    ];
    assert_eq!(ids(&formats, &["en.1"]), ["high"]);
}

#[test]
fn explicit_audio_format_id_remains_authoritative_with_track_selection() {
    let formats = [
        audio("low", Some("en.0"), Some("en"), Some(true), Some(true), 64),
        audio(
            "high",
            Some("en.0"),
            Some("en"),
            Some(true),
            Some(true),
            192,
        ),
        audio(
            "ru",
            Some("ru.0"),
            Some("ru"),
            Some(false),
            Some(false),
            128,
        ),
    ];
    let mut wanted = selection(&["en"]);
    wanted.format_id = Some("low".into());
    assert_eq!(choose_formats(&formats, &wanted).unwrap()[0].id, "low");
    wanted.audio_tracks = vec!["ru".into()];
    assert!(
        choose_formats(&formats, &wanted).is_err(),
        "Conflicting track selection silently replaced the explicit format"
    );
    wanted.audio_tracks = vec!["all".into()];
    let chosen = choose_formats(&formats, &wanted).unwrap();
    assert!(chosen.iter().any(|format| format.id == "low"));
    assert!(chosen.iter().all(|format| format.id != "high"));
    wanted.format_id = Some("missing".into());
    assert!(choose_formats(&formats, &wanted).is_err());
}

#[test]
fn legacy_format_json_defaults_metadata_and_debug_does_not_expose_it() {
    let original = audio(
        "id",
        Some("private-track-id"),
        Some("en"),
        Some(true),
        Some(true),
        128,
    );
    let value = serde_json::to_value(&original).unwrap();
    assert_eq!(value["audio_track_id"], "private-track-id");
    assert_eq!(value["audio_default"], true);
    assert_eq!(value["audio_original"], true);
    let mut legacy = value;
    for key in ["audio_track_id", "audio_default", "audio_original"] {
        legacy.as_object_mut().unwrap().remove(key);
    }
    let parsed: Format = serde_json::from_value(legacy).unwrap();
    let legacy = serde_json::to_value(parsed).unwrap();
    assert!(legacy["audio_default"].is_null());
    assert!(legacy["audio_original"].is_null());
    assert!(!format!("{original:?}").contains("private-track-id"));
}
