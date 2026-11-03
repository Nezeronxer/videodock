//! Explicit integration run downloads only public official model/demo assets.
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use videodock_core::processing;
fn data_dir() -> PathBuf {
    std::env::var_os("VIDEODOCK_PROCESSING_TEST_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".local/share/videodock")
        })
}
async fn fixture(name: &str, url: &str, size: u64, expected: &str) -> PathBuf {
    let path = data_dir().join("models/test-fixtures").join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    if let Ok(bytes) = std::fs::read(&path)
        && bytes.len() as u64 == size
        && format!("{:x}", Sha256::digest(&bytes)) == expected
    {
        return path;
    }
    let mut response = reqwest::Client::new().get(url).send().await.unwrap();
    assert!(response.status().is_success());
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.unwrap() {
        assert!(bytes.len() + chunk.len() <= size as usize);
        bytes.extend_from_slice(&chunk);
    }
    assert_eq!(bytes.len() as u64, size);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
    std::fs::write(&path, bytes).unwrap();
    path
}
fn samples(path: &Path) -> (hound::WavSpec, Vec<f32>) {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    let samples = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().map(Result::unwrap).collect()
    } else {
        let scale = (1u64 << (spec.bits_per_sample - 1)) as f32;
        reader
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / scale)
            .collect()
    };
    (spec, samples)
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
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[tokio::test]
#[ignore = "downloads verified official CPU models and requires FFmpeg"]
async fn official_demo_has_real_two_stems_and_speech_enhancement() {
    let music=fixture("piao-yang.wav","https://github.com/k2-fsa/sherpa-onnx/releases/download/source-separation-models/piao-yang-guo-hai-lai-kan-ni.wav",7_224_586,"f6efce84c25d443c32c23a3722da57f77bc295aa77762aa4c5f50927126dc77f").await;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("official-demo.wav");
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-v", "error", "-nostdin", "-y", "-i"])
        .arg(music)
        .args(["-t", "2", "-ac", "2", "-ar", "44100", "-c:a", "pcm_f32le"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let progress = Arc::new(Mutex::new(Vec::new()));
    let log = progress.clone();
    let started = Instant::now();
    let vocals = processing::process(
        input.clone(),
        "extract_vocals",
        dir.path().join("out"),
        data_dir(),
        CancellationToken::new(),
        Arc::new(move |p| {
            eprintln!("processing progress {p:.3}");
            log.lock().unwrap().push(p)
        }),
    )
    .await
    .unwrap();
    eprintln!(
        "vocals processing (cached assets if present) {:.3}s",
        started.elapsed().as_secs_f64()
    );
    {
        let numbers = progress.lock().unwrap();
        assert!(numbers.windows(2).all(|s| s[0] <= s[1]));
        assert_eq!(numbers.last().copied(), Some(100.0));
    }
    let instrumental = processing::process(
        input.clone(),
        "remove_vocals",
        dir.path().join("out"),
        data_dir(),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    let (spec, original) = samples(&input);
    let (vs, v) = samples(&vocals);
    let (aspec, a) = samples(&instrumental);
    eprintln!(
        "stem fixture: {} interleaved samples, {} channels, {}Hz",
        original.len(),
        spec.channels,
        spec.sample_rate
    );
    assert_eq!(spec.sample_rate, vs.sample_rate);
    assert_eq!(spec.channels, vs.channels);
    assert_eq!(spec.sample_rate, aspec.sample_rate);
    assert_eq!(original.len(), v.len());
    assert_eq!(original.len(), a.len());
    let energy = original.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
    let error = original
        .iter()
        .zip(v.iter().zip(&a))
        .map(|(x, (v, a))| f64::from(x - v - a).powi(2))
        .sum::<f64>();
    let change = v
        .iter()
        .zip(&original)
        .map(|(v, x)| f64::from(v - x).powi(2))
        .sum::<f64>();
    eprintln!(
        "stem reconstruction error/energy {:.8}; vocals change {:.8}",
        error / energy,
        change / energy
    );
    assert!(
        error / energy < 0.10,
        "stems must approximately reconstruct actual input"
    );
    assert!(change / energy > 0.001);
    assert!(v.iter().any(|x| x.abs() > 0.0001) && a.iter().any(|x| x.abs() > 0.0001));
    assert_eq!(probe(&vocals)["streams"][0]["codec_name"], "pcm_f32le");
    let speech=fixture("speech_with_noise.wav","https://github.com/k2-fsa/sherpa-onnx/releases/download/speech-enhancement-models/speech_with_noise.wav",76_770,"9354345a6297f4522e690d337546aa9a686a7e61eefcd935478a2141b924db8f").await;
    let voice = processing::process(
        speech.clone(),
        "extract_voice",
        dir.path().join("out"),
        data_dir(),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    let (_, input_speech) = samples(&speech);
    let (vspec, voice) = samples(&voice);
    eprintln!(
        "speech fixture/output: {} / {} samples, {} channels, {}Hz",
        input_speech.len(),
        voice.len(),
        vspec.channels,
        vspec.sample_rate
    );
    assert_eq!(vspec.sample_rate, 16000);
    assert_eq!(voice.len(), input_speech.len());
    assert!(
        voice
            .iter()
            .zip(&input_speech)
            .any(|(a, b)| (a - b).abs() > 0.001)
    );
    assert!(voice.iter().all(|v| v.is_finite()));
}
