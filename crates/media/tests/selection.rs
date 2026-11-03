use videodock_media::{Format, Selection, choose_formats};

fn format(
    id: &str,
    height: u32,
    video: Option<&str>,
    audio: Option<&str>,
    lang: Option<&str>,
) -> Format {
    Format {
        id: id.into(),
        height: Some(height),
        video_codec: video.map(str::to_owned),
        audio_codec: audio.map(str::to_owned),
        audio_language: lang.map(str::to_owned),
        ..Default::default()
    }
}

#[test]
fn ceiling_and_audio_language_are_obeyed() {
    let formats = vec![
        format("720", 720, Some("avc1"), None, None),
        format("4k", 2160, Some("av01"), None, None),
        format("en", 0, None, Some("aac"), Some("en")),
        format("ru", 0, None, Some("aac"), Some("ru")),
    ];
    let selection = Selection {
        quality: 1080,
        audio_tracks: vec!["ru".into()],
        ..Default::default()
    };
    let chosen = choose_formats(&formats, &selection).unwrap();
    assert_eq!(
        chosen.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        vec!["720", "ru"]
    );
}

#[test]
fn requested_missing_codec_is_an_error_not_a_fake_match() {
    let formats = vec![format("720", 720, Some("avc1"), Some("aac"), None)];
    assert!(
        choose_formats(
            &formats,
            &Selection {
                video_codec: "av1".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn audio_mode_never_chooses_video_only() {
    let formats = vec![
        format("video", 2160, Some("av01"), None, None),
        format("audio", 0, None, Some("opus"), None),
    ];
    let result = choose_formats(
        &formats,
        &Selection {
            media_type: "audio".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result[0].id, "audio");
}

#[test]
fn exact_audio_id_and_vr_are_not_silently_ignored() {
    let formats = vec![
        format("audio1", 0, None, Some("aac"), None),
        format("audio2", 0, None, Some("opus"), None),
    ];
    let result = choose_formats(
        &formats,
        &Selection {
            media_type: "audio".into(),
            format_id: Some("audio1".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result[0].id, "audio1");
    let mut regular = format("flat", 1080, Some("avc1"), Some("aac"), None);
    assert!(
        choose_formats(
            &[regular.clone()],
            &Selection {
                vr: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    regular.vr = true;
    assert_eq!(
        choose_formats(
            &[regular],
            &Selection {
                vr: true,
                ..Default::default()
            }
        )
        .unwrap()[0]
            .id,
        "flat"
    );
}
