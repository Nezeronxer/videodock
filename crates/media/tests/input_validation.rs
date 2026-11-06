use videodock_media::validate_url;

#[test]
fn malformed_links_have_actionable_errors_without_echoing_input() {
    for value in ["", "private-fixture-marker", "https://"] {
        let error = validate_url(value).unwrap_err().to_string();
        assert!(error.contains("HTTP"), "No actionable link guidance: {error}");
        assert!(!error.contains("private-fixture-marker"));
    }
}
