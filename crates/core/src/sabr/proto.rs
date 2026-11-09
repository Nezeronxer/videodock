// SABR wire field numbers from LuanRT/googlevideo (MIT); see LICENSE.googlevideo.
// Only the messages used by this single-track VOD downloader are represented.
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct FormatId {
    #[prost(int32, optional, tag = "1")]
    pub itag: Option<i32>,
    #[prost(uint64, optional, tag = "2")]
    pub last_modified: Option<u64>,
    #[prost(string, optional, tag = "3")]
    pub xtags: Option<String>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct TimeRange {
    #[prost(int64, optional, tag = "1")]
    pub start_ticks: Option<i64>,
    #[prost(int64, optional, tag = "2")]
    pub duration_ticks: Option<i64>,
    #[prost(int32, optional, tag = "3")]
    pub timescale: Option<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct BufferedRange {
    #[prost(message, required, tag = "1")]
    pub format_id: FormatId,
    #[prost(int64, required, tag = "2")]
    pub start_time_ms: i64,
    #[prost(int64, required, tag = "3")]
    pub duration_ms: i64,
    #[prost(int32, required, tag = "4")]
    pub start_segment_index: i32,
    #[prost(int32, required, tag = "5")]
    pub end_segment_index: i32,
    #[prost(message, optional, tag = "6")]
    pub time_range: Option<TimeRange>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct AbrState {
    #[prost(message, optional, tag = "38")]
    pub media_capabilities: Option<MediaCapabilities>,
    #[prost(int64, optional, tag = "23")]
    pub bandwidth_estimate: Option<i64>,
    #[prost(int64, optional, tag = "28")]
    pub player_time_ms: Option<i64>,
    #[prost(int32, optional, tag = "34")]
    pub visibility: Option<i32>,
    #[prost(float, optional, tag = "35")]
    pub playback_rate: Option<f32>,
    #[prost(int64, optional, tag = "36")]
    pub elapsed_wall_time_ms: Option<i64>,
    #[prost(int32, optional, tag = "40")]
    pub enabled_track_types_bitfield: Option<i32>,
    #[prost(bool, optional, tag = "46")]
    pub drc_enabled: Option<bool>,
    #[prost(string, optional, tag = "69")]
    pub audio_track_id: Option<String>,
    #[prost(bool, optional, tag = "76")]
    pub enable_voice_boost: Option<bool>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct ClientInfo {
    #[prost(int32, optional, tag = "16")]
    pub client_name: Option<i32>,
    #[prost(string, optional, tag = "17")]
    pub client_version: Option<String>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct SabrContext {
    #[prost(int32, optional, tag = "1")]
    pub r#type: Option<i32>,
    #[prost(bytes, optional, tag = "2")]
    pub value: Option<Vec<u8>>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct StreamerContext {
    #[prost(message, optional, tag = "1")]
    pub client_info: Option<ClientInfo>,
    #[prost(bytes, optional, tag = "2")]
    pub po_token: Option<Vec<u8>>,
    #[prost(bytes, optional, tag = "3")]
    pub playback_cookie: Option<Vec<u8>>,
    #[prost(message, repeated, tag = "5")]
    pub sabr_contexts: Vec<SabrContext>,
    #[prost(int32, repeated, tag = "6")]
    pub unsent_sabr_contexts: Vec<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct PlaybackRequest {
    #[prost(message, optional, tag = "1")]
    pub client_abr_state: Option<AbrState>,
    #[prost(message, repeated, tag = "2")]
    pub initialization_format_ids: Vec<FormatId>,
    #[prost(message, repeated, tag = "3")]
    pub buffered_ranges: Vec<BufferedRange>,
    #[prost(bytes, optional, tag = "5")]
    pub video_playback_ustreamer_config: Option<Vec<u8>>,
    #[prost(message, repeated, tag = "16")]
    pub selected_audio_format_ids: Vec<FormatId>,
    #[prost(message, repeated, tag = "17")]
    pub selected_video_format_ids: Vec<FormatId>,
    #[prost(message, optional, tag = "19")]
    pub streamer_context: Option<StreamerContext>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct FormatInit {
    #[prost(string, optional, tag = "1")]
    pub video_id: Option<String>,
    #[prost(message, optional, tag = "2")]
    pub format_id: Option<FormatId>,
    #[prost(int64, optional, tag = "3")]
    pub end_time_ms: Option<i64>,
    #[prost(int64, optional, tag = "4")]
    pub end_segment_number: Option<i64>,
    #[prost(string, optional, tag = "5")]
    pub mime_type: Option<String>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct MediaHeader {
    #[prost(uint32, optional, tag = "1")]
    pub header_id: Option<u32>,
    #[prost(string, optional, tag = "2")]
    pub video_id: Option<String>,
    #[prost(int32, optional, tag = "3")]
    pub itag: Option<i32>,
    #[prost(uint64, optional, tag = "4")]
    pub lmt: Option<u64>,
    #[prost(string, optional, tag = "5")]
    pub xtags: Option<String>,
    #[prost(int32, optional, tag = "7")]
    pub compression_algorithm: Option<i32>,
    #[prost(bool, optional, tag = "8")]
    pub is_init_seg: Option<bool>,
    #[prost(int32, optional, tag = "9")]
    pub sequence_number: Option<i32>,
    #[prost(int64, optional, tag = "11")]
    pub start_ms: Option<i64>,
    #[prost(int64, optional, tag = "12")]
    pub duration_ms: Option<i64>,
    #[prost(message, optional, tag = "13")]
    pub format_id: Option<FormatId>,
    #[prost(int64, optional, tag = "14")]
    pub content_length: Option<i64>,
    #[prost(message, optional, tag = "15")]
    pub time_range: Option<TimeRange>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct NextPolicy {
    #[prost(int32, optional, tag = "4")]
    pub backoff_time_ms: Option<i32>,
    #[prost(bytes, optional, tag = "7")]
    pub playback_cookie: Option<Vec<u8>>,
    #[prost(string, optional, tag = "8")]
    pub video_id: Option<String>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct Redirect {
    #[prost(string, optional, tag = "1")]
    pub url: Option<String>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct SabrError {
    #[prost(string, optional, tag = "1")]
    pub r#type: Option<String>,
    #[prost(int32, optional, tag = "2")]
    pub code: Option<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct ContextUpdate {
    #[prost(int32, optional, tag = "1")]
    pub r#type: Option<i32>,
    #[prost(bytes, optional, tag = "3")]
    pub value: Option<Vec<u8>>,
    #[prost(bool, optional, tag = "4")]
    pub send_by_default: Option<bool>,
    #[prost(int32, optional, tag = "5")]
    pub write_policy: Option<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct ContextPolicy {
    #[prost(int32, repeated, tag = "1")]
    pub start_policy: Vec<i32>,
    #[prost(int32, repeated, tag = "2")]
    pub stop_policy: Vec<i32>,
    #[prost(int32, repeated, tag = "3")]
    pub discard_policy: Vec<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct Protection {
    #[prost(int32, optional, tag = "1")]
    pub status: Option<i32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct MediaCapabilities {
    #[prost(message, repeated, tag = "1")]
    pub video: Vec<VideoCapability>,
    #[prost(message, repeated, tag = "2")]
    pub audio: Vec<AudioCapability>,
    #[prost(int32, optional, tag = "5")]
    pub hdr_mode_bitmask: Option<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct VideoCapability {
    #[prost(int32, optional, tag = "1")]
    pub codec: Option<i32>,
    #[prost(int32, optional, tag = "3")]
    pub height: Option<i32>,
    #[prost(int32, optional, tag = "4")]
    pub width: Option<i32>,
    #[prost(int32, optional, tag = "11")]
    pub fps: Option<i32>,
    #[prost(int32, optional, tag = "12")]
    pub bitrate: Option<i32>,
    #[prost(bool, optional, tag = "15")]
    pub ten_bit: Option<bool>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub(super) struct AudioCapability {
    #[prost(int32, optional, tag = "1")]
    pub codec: Option<i32>,
    #[prost(int32, optional, tag = "2")]
    pub channels: Option<i32>,
    #[prost(int32, optional, tag = "6")]
    pub spatial: Option<i32>,
}
