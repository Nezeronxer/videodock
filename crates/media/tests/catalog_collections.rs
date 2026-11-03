use videodock_media::{Auth, MediaKind, catalog::Catalog};
#[tokio::test]
#[ignore = "public organizational Twitch channel pagination"]
async fn twitch_public_channel_vods() {
    let result = Catalog::new(None)
        .unwrap()
        .inspect("https://www.twitch.tv/twitch/videos", &Auth::default())
        .await;
    let media = match result {
        Ok(media) => media,
        Err(error) => {
            assert!(
                error
                    .to_string()
                    .contains("полный список сейчас недоступен")
            );
            println!(
                "Twitch channel: continuation integrity gate reported, no partial list returned"
            );
            return;
        }
    };
    assert_eq!(media.kind, MediaKind::Channel);
    assert!(!media.entries.is_empty());
    assert!(media.continuation.is_none());
    assert!(
        media
            .entries
            .iter()
            .all(|entry| entry.url.starts_with("https://www.twitch.tv/videos/"))
    );
    println!(
        "Twitch organizational channel entries={}",
        media.entries.len()
    );
}
#[tokio::test]
#[ignore = "public organizational SoundCloud channel pagination"]
async fn soundcloud_public_channel_tracks() {
    let media = Catalog::new(None)
        .unwrap()
        .inspect("https://soundcloud.com/nasa", &Auth::default())
        .await
        .unwrap();
    assert_eq!(media.kind, MediaKind::Channel);
    assert!(!media.entries.is_empty());
    assert!(media.continuation.is_none());
    assert!(
        media
            .entries
            .iter()
            .all(|entry| entry.url.starts_with("https://soundcloud.com/"))
    );
    println!(
        "SoundCloud organizational channel entries={}",
        media.entries.len()
    );
}
