//! Transient SABR data is never serialized to exports, queue state or the UI.
use anyhow::{Result, ensure};
use std::sync::Arc;
use url::Url;

pub struct Session {
    pub url: String,
    pub video_id: String,
    pub config: Vec<u8>,
    pub content_po: Vec<u8>,
    pub user_agent: String,
    pub client_version: String,
    client: reqwest::Client,
}
impl Session {
    pub(crate) fn new(
        url: String,
        video_id: String,
        config: Vec<u8>,
        content_po: Vec<u8>,
        user_agent: String,
        client_version: String,
        client: reqwest::Client,
    ) -> Result<Self> {
        validate_url(&url)?;
        ensure!(
            !config.is_empty()
                && config.len() <= 64 * 1024
                && !content_po.is_empty()
                && content_po.len() <= 16 * 1024,
            "Неверные параметры SABR"
        );
        ensure!(
            video_id.len() == 11
                && video_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
            "Неверный идентификатор SABR"
        );
        ensure!(
            !user_agent.is_empty()
                && user_agent.len() <= 2048
                && user_agent.bytes().all(|b| (32..=126).contains(&b)),
            "Некорректный User-Agent SABR"
        );
        ensure!(
            !client_version.is_empty()
                && client_version.len() <= 128
                && client_version
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)),
            "Неверная версия клиента SABR"
        );
        Ok(Self {
            url,
            video_id,
            config,
            content_po,
            user_agent,
            client_version,
            client,
        })
    }
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SabrSession").finish_non_exhaustive()
    }
}
#[derive(Clone)]
pub struct Delivery {
    pub session: Arc<Session>,
    pub itag: u32,
    pub last_modified: u64,
    pub xtags: Option<String>,
    pub duration_ms: u64,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u64,
    pub channels: u32,
}
impl std::fmt::Debug for Delivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SabrDelivery")
            .field("itag", &self.itag)
            .finish_non_exhaustive()
    }
}

pub fn validate_url(value: &str) -> Result<Url> {
    ensure!(
        value.len() <= 16 * 1024 && !value.chars().any(char::is_control),
        "Некорректный адрес SABR"
    );
    let url = Url::parse(value).map_err(|_| anyhow::anyhow!("Некорректный адрес SABR"))?;
    ensure!(
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|h| h == "googlevideo.com" || h.ends_with(".googlevideo.com"))
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && url.path() == "/videoplayback",
        "Недопустимый сервер SABR"
    );
    Ok(url)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirect_targets_cannot_receive_proof_outside_the_exact_delivery_origin_class() {
        for bad in [
            "http://r.googlevideo.com/videoplayback",
            "https://googlevideo.com.evil.test/videoplayback",
            "https://evilgooglevideo.com/videoplayback",
            "https://r.googlevideo.com:444/videoplayback",
            "https://user:secret@r.googlevideo.com/videoplayback",
            "https://r.googlevideo.com/videoplayback#fragment",
            "https://r.googlevideo.com/other-path",
        ] {
            assert!(
                validate_url(bad).is_err(),
                "accepted invalid delivery target"
            );
        }
        assert!(
            validate_url("https://r1---fixture.googlevideo.com/videoplayback?expire=1").is_ok()
        );
    }
}
