use std::collections::BTreeMap;
use videodock_media::scoped_headers;

#[test]
fn private_headers_never_follow_another_origin() {
    let headers = BTreeMap::from([
        ("Cookie".into(), "private=value".into()),
        ("Authorization".into(), "Bearer private".into()),
        ("User-Agent".into(), "VideoDock".into()),
        (
            "Referer".into(),
            "https://example.com/watch?private=value".into(),
        ),
    ]);
    let result = scoped_headers(
        &headers,
        "https://example.com/a.m3u8",
        "https://cdn.example.com/a.ts",
    )
    .unwrap();
    assert!(!result.contains_key("Cookie") && !result.contains_key("Authorization"));
    assert_eq!(result["User-Agent"], "VideoDock");
    assert_eq!(result["Referer"], "https://example.com/");
    assert!(
        scoped_headers(&headers, "https://example.com/a", "https://example.com/b")
            .unwrap()
            .contains_key("Cookie")
    );
    assert!(
        !scoped_headers(&headers, "https://example.com/a", "http://example.com/a")
            .unwrap()
            .contains_key("Referer")
    );
}

#[test]
fn typed_cookie_recomputed_for_each_path_domain_and_expiry() {
    use videodock_media::{ScopedCookie, request_headers};
    let cookies = vec![ScopedCookie {
        name: "private".into(),
        value: "synthetic".into(),
        domain: ".example.com".into(),
        path: "/private".into(),
        secure: true,
        expires: None,
    }];
    let source = "https://cdn.example.com/private/list.m3u8";
    let allowed = request_headers(
        &BTreeMap::new(),
        source,
        "https://media.example.com/private/segment",
        &cookies,
    )
    .unwrap();
    assert!(allowed.contains_key("Cookie"));
    for target in [
        "https://media.example.com/private-other/segment",
        "https://example.com.evil.invalid/private/segment",
        "http://cdn.example.com/private/segment",
    ] {
        assert!(
            !request_headers(&BTreeMap::new(), source, target, &cookies)
                .unwrap()
                .contains_key("Cookie")
        );
    }
    let mut expired = cookies;
    expired[0].expires = Some(1);
    assert!(
        !request_headers(&BTreeMap::new(), source, source, &expired)
            .unwrap()
            .contains_key("Cookie")
    );
}
