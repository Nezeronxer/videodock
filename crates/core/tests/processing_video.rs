//! Explicit end-to-end processing of local video; no remote/user audio.
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
use videodock_core::processing;
fn data_dir() -> PathBuf {
    std::env::var_os("VIDEODOCK_PROCESSING_TEST_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share/videodock")
        })
}
fn probe(path: &Path) -> serde_json::Value {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}
fn frame_hashes(path: &Path) -> Vec<String> {
    let out = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe,crypto,data",
            "-i",
        ])
        .arg(path)
        .args(["-map", "0:v:0", "-f", "framemd5", "-"])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|line| line.rsplit(',').next().unwrap().trim().to_owned())
        .collect()
}
fn stream_offset(probe: &serde_json::Value) -> f64 {
    let streams = probe["streams"].as_array().unwrap();
    let start = |kind: &str| {
        streams.iter().find(|s| s["codec_type"] == kind).unwrap()["start_time"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    start("audio") - start("video")
}

#[tokio::test]
#[ignore = "real Spleeter/WPE and FFmpeg integration"]
async fn remove_effects_preserve_video_frames_and_delayed_audio_sync() {
    let dir = tempfile::tempdir().unwrap();
    let sound = dir.path().join("room.wav");
    let rate = 48000usize;
    let mut signal = vec![0f32; rate * 2];
    let mut rng = 13u32;
    for i in 0..signal.len() {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let phase = i % (rate / 4);
        if phase < rate / 16 {
            signal[i] = ((rng >> 8) as f32 / (1u32 << 24) as f32 - 0.5)
                * (std::f32::consts::PI * phase as f32 / (rate / 16) as f32)
                    .sin()
                    .powi(2);
        }
        if i >= rate * 80 / 1000 {
            signal[i] += 0.6 * signal[i - rate * 80 / 1000];
        }
    }
    let mut writer = hound::WavWriter::create(
        &sound,
        hound::WavSpec {
            channels: 2,
            sample_rate: rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for sample in signal {
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample * 0.9).unwrap();
    }
    writer.finalize().unwrap();
    let input = dir.path().join("source.mp4");
    let out = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-nostdin",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x96:rate=10:duration=2",
            "-itsoffset",
            "0.25",
            "-i",
        ])
        .arg(&sound)
        .args([
            "-map", "0:v:0", "-map", "1:a:0", "-c:v", "libx264", "-threads", "1", "-c:a", "aac",
            "-b:a", "128k",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let before = probe(&input);
    let original_hashes = frame_hashes(&input);
    assert_eq!(original_hashes.len(), 20);
    for effect in ["remove_vocals", "remove_echo"] {
        let result = processing::process(
            input.clone(),
            effect,
            dir.path().join("out"),
            data_dir(),
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
        assert_eq!(result.extension().unwrap(), "mkv");
        let after = probe(&result);
        assert_eq!(
            frame_hashes(&result),
            original_hashes,
            "video must be stream-copy, not re-encoded"
        );
        assert!(
            (stream_offset(&after) - stream_offset(&before)).abs() < 0.08,
            "audio/video sync changed"
        );
        let duration = |v: &serde_json::Value| {
            v["format"]["duration"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };
        eprintln!(
            "{effect}: 20 identical video frames; AV offset {:.6}->{:.6}s; duration {:.6}->{:.6}s",
            stream_offset(&before),
            stream_offset(&after),
            duration(&before),
            duration(&after)
        );
        assert!((duration(&after) - duration(&before)).abs() < 0.10);
    }
}

#[tokio::test]
#[ignore = "real model cancellation and workspace privacy integration"]
async fn cancel_during_native_model_work_publishes_nothing_and_cleans_private_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.wav");
    let mut writer = hound::WavWriter::create(
        &input,
        hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for i in 0..44100 * 2 {
        let sample = ((i as f32 * 0.03).sin() * 5000.0) as i16;
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    let before = std::fs::read(&input).unwrap();
    let output = dir.path().join("out");
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = fired.clone();
    let result = processing::process(
        input.clone(),
        "extract_vocals",
        output.clone(),
        data_dir(),
        cancel,
        Arc::new(move |p| {
            if (30.0..90.0).contains(&p) && !flag.swap(true, std::sync::atomic::Ordering::Relaxed) {
                let token = token.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    token.cancel();
                });
            }
        }),
    )
    .await;
    assert!(fired.load(std::sync::atomic::Ordering::Relaxed));
    assert!(result.is_err());
    assert!(!output.exists() || std::fs::read_dir(output).unwrap().next().is_none());
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert!(
        std::fs::read_dir(data_dir().join("models"))
            .unwrap()
            .all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".work-"))
    );
}
