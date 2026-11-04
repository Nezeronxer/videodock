use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    #[default]
    Video,
    Audio,
    Playlist,
    Channel,
    Search,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    #[default]
    Http,
    Hls,
    Dash,
    Sabr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoProjection {
    #[serde(rename = "None")]
    Flat,
    #[serde(rename = "2D/360°")]
    Mono360,
    #[serde(rename = "3D/180°")]
    Stereo180,
    #[serde(rename = "3D/360°")]
    Stereo360,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoVrMode {
    #[default]
    Any,
    #[serde(rename = "2D/360°", alias = "2D360")]
    Mono360,
    #[serde(rename = "3D/180°", alias = "3D180")]
    Stereo180,
    #[serde(rename = "3D/360°", alias = "3D360")]
    Stereo360,
    #[serde(rename = "None")]
    Flat,
}

impl VideoVrMode {
    pub fn matches(self, format: &Format, legacy_any_vr: bool) -> bool {
        match self {
            Self::Any => !legacy_any_vr || format.is_vr(),
            Self::Flat => !format.is_vr(),
            Self::Mono360 => format.projection == Some(VideoProjection::Mono360),
            Self::Stereo180 => format.projection == Some(VideoProjection::Stereo180),
            Self::Stereo360 => format.projection == Some(VideoProjection::Stereo360),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Format {
    #[serde(skip)]
    pub sabr: Option<crate::sabr::Delivery>,
    pub id: String,
    pub url: String,
    pub protocol: Protocol,
    pub ext: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    #[serde(default)]
    pub vr: bool,
    #[serde(default)]
    pub projection: Option<VideoProjection>,
    #[serde(default)]
    pub projection_type: Option<String>,
    #[serde(default)]
    pub stereo_layout: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_language: Option<String>,
    pub audio_title: Option<String>,
    #[serde(default)]
    pub audio_track_id: Option<String>,
    #[serde(default)]
    pub audio_default: Option<bool>,
    #[serde(default)]
    pub audio_original: Option<bool>,
    pub bitrate: Option<u64>,
    pub size: Option<u64>,
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub cookies: Vec<ScopedCookie>,
}

impl std::fmt::Debug for Format {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Format")
            .field("id", &self.id)
            .field("protocol", &self.protocol)
            .field("ext", &self.ext)
            .field("height", &self.height)
            .field("size", &self.size)
            .field("url", &"[redacted]")
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Format {
    pub fn is_vr(&self) -> bool {
        self.vr
            || self
                .projection
                .is_some_and(|projection| projection != VideoProjection::Flat)
    }

    pub fn has_video(&self) -> bool {
        self.video_codec
            .as_deref()
            .is_some_and(|v| !v.is_empty() && v != "none")
    }

    pub fn has_audio(&self) -> bool {
        self.audio_codec
            .as_deref()
            .is_some_and(|v| !v.is_empty() && v != "none")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subtitle {
    pub language: String,
    pub title: String,
    pub url: String,
    pub ext: String,
    pub automatic: bool,
    pub translatable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Media {
    pub id: String,
    pub url: String,
    pub title: String,
    pub description: String,
    pub kind: MediaKind,
    pub duration: Option<f64>,
    pub thumbnail: String,
    pub channel_id: String,
    pub channel_title: String,
    pub formats: Vec<Format>,
    pub subtitles: Vec<Subtitle>,
    pub entries: Vec<Entry>,
    pub continuation: Option<String>,
    #[serde(default)]
    pub cookies: Vec<ScopedCookie>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ScopedCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    #[serde(default)]
    pub expires: Option<u64>,
}

impl ScopedCookie {
    pub fn matches_domain(&self, url: &url::Url) -> bool {
        let host = url.host_str().unwrap_or_default();
        let domain = self.domain.trim_start_matches('.');
        !domain.is_empty()
            && (host.eq_ignore_ascii_case(domain)
                || (self.domain.starts_with('.')
                    && host
                        .to_ascii_lowercase()
                        .ends_with(&format!(".{}", domain.to_ascii_lowercase()))))
            && (!self.secure || url.scheme() == "https")
    }
    pub fn applies_to(&self, url: &url::Url) -> bool {
        let path = if self.path.is_empty() {
            "/"
        } else {
            &self.path
        };
        let not_expired = self.expires.is_none_or(|end| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|now| now.as_secs() < end)
                .unwrap_or(false)
        });
        self.matches_domain(url)
            && not_expired
            && url.path().starts_with(path)
            && (path.ends_with('/')
                || url.path().len() == path.len()
                || url.path().as_bytes().get(path.len()) == Some(&b'/'))
    }
}
impl std::fmt::Debug for ScopedCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedCookie")
            .field("value", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct Auth {
    pub cookie_header: String,
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Auth")
            .field("cookie_header", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Selection {
    pub media_type: String,
    pub container: String,
    pub quality: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub audio_bitrate: u32,
    pub fps: u32,
    pub vr: bool,
    pub vr_mode: VideoVrMode,
    pub subtitles: Vec<String>,
    pub audio_tracks: Vec<String>,
    pub format_id: Option<String>,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            media_type: "video".into(),
            container: "mp4".into(),
            quality: 1080,
            video_codec: "any".into(),
            audio_codec: "any".into(),
            audio_bitrate: 192,
            fps: 0,
            vr: false,
            vr_mode: VideoVrMode::Any,
            subtitles: vec![],
            audio_tracks: vec![],
            format_id: None,
        }
    }
}
