//! Minimal C ABI declarations checked against sherpa-onnx v2.0.1 c-api.h.
use super::assets::{Installed, VERSION};
use anyhow::{Context, Result, ensure};
use libloading::Library;
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    sync::Arc,
};
#[repr(C)]
struct Spleeter {
    vocals: *const c_char,
    accompaniment: *const c_char,
}
#[repr(C)]
struct Uvr {
    model: *const c_char,
}
#[repr(C)]
struct SeparationModel {
    spleeter: Spleeter,
    uvr: Uvr,
    num_threads: i32,
    debug: i32,
    provider: *const c_char,
}
#[repr(C)]
struct SeparationConfig {
    model: SeparationModel,
}
#[repr(C)]
struct Stem {
    samples: *const *const f32,
    num_channels: i32,
    n: i32,
}
#[repr(C)]
struct SeparationOutput {
    stems: *const Stem,
    num_stems: i32,
    sample_rate: i32,
}
#[repr(C)]
struct Gtcrn {
    model: *const c_char,
}
#[repr(C)]
struct Dpdf {
    model: *const c_char,
    attenuation_limit_db: f32,
}
#[repr(C)]
struct VoxLock {
    enroll_model: *const c_char,
    chunk_model: *const c_char,
    voice_sample: *const c_char,
}
#[repr(C)]
struct DenoiserModel {
    gtcrn: Gtcrn,
    num_threads: i32,
    debug: i32,
    provider: *const c_char,
    dpdfnet: Dpdf,
    voxlock: VoxLock,
}
#[repr(C)]
struct DenoiserConfig {
    model: DenoiserModel,
}
#[repr(C)]
struct DenoisedAudio {
    samples: *const f32,
    n: i32,
    sample_rate: i32,
}
type CreateSeparation = unsafe extern "C" fn(*const SeparationConfig) -> *const c_void;
type RunSeparation = unsafe extern "C" fn(
    *const c_void,
    *const *const f32,
    i32,
    i32,
    i32,
) -> *const SeparationOutput;
type DestroySeparation = unsafe extern "C" fn(*const c_void);
type DestroyStems = unsafe extern "C" fn(*const SeparationOutput);
type CreateDenoiser = unsafe extern "C" fn(*const DenoiserConfig) -> *const c_void;
type RunDenoiser =
    unsafe extern "C" fn(*const c_void, *const f32, i32, i32) -> *const DenoisedAudio;
type DestroyDenoiser = unsafe extern "C" fn(*const c_void);
type DestroyDenoised = unsafe extern "C" fn(*const DenoisedAudio);
type GetRate = unsafe extern "C" fn(*const c_void) -> i32;
struct Api {
    _libraries: Vec<Library>,
    create_separation: CreateSeparation,
    run_separation: RunSeparation,
    destroy_separation: DestroySeparation,
    destroy_stems: DestroyStems,
    separation_rate: GetRate,
    create_denoiser: CreateDenoiser,
    run_denoiser: RunDenoiser,
    destroy_denoiser: DestroyDenoiser,
    destroy_denoised: DestroyDenoised,
    denoiser_rate: GetRate,
}
fn load(path: &Path) -> Result<Library> {
    // SAFETY: callers verify archive and installed bytes before this absolute library path is used.
    #[cfg(windows)]
    {
        let library =
            unsafe { libloading::os::windows::Library::load_with_flags(path, 0x100 | 0x1000) }
                .context("Не удалось открыть CPU runtime MediaTools")?;
        return Ok(library.into());
    }
    #[cfg(not(windows))]
    {
        unsafe { Library::new(path) }.context("Не удалось открыть CPU runtime MediaTools")
    }
}
impl Api {
    fn open(installed: &Installed) -> Result<Arc<Self>> {
        #[cfg(windows)]
        let providers = load(
            &installed
                .capi
                .parent()
                .context("Нет каталога CPU runtime")?
                .join("onnxruntime_providers_shared.dll"),
        )?;
        let ort = load(&installed.ort)?;
        let library = load(&installed.capi)?;
        // SAFETY: symbol signatures/layouts exactly follow the pinned official v2.0.1 header.
        unsafe {
            let version = library
                .get::<unsafe extern "C" fn() -> *const c_char>(b"SherpaOnnxGetVersionStr\0")?;
            let value = version();
            ensure!(
                !value.is_null() && CStr::from_ptr(value).to_str()? == VERSION,
                "Неверная версия C API MediaTools"
            );
            Ok(Arc::new(Self {
                create_separation: *library.get(b"SherpaOnnxCreateOfflineSourceSeparation\0")?,
                run_separation: *library.get(b"SherpaOnnxOfflineSourceSeparationProcess\0")?,
                destroy_separation: *library.get(b"SherpaOnnxDestroyOfflineSourceSeparation\0")?,
                destroy_stems: *library.get(b"SherpaOnnxDestroySourceSeparationOutput\0")?,
                separation_rate: *library
                    .get(b"SherpaOnnxOfflineSourceSeparationGetOutputSampleRate\0")?,
                create_denoiser: *library.get(b"SherpaOnnxCreateOfflineSpeechDenoiser\0")?,
                run_denoiser: *library.get(b"SherpaOnnxOfflineSpeechDenoiserRun\0")?,
                destroy_denoiser: *library.get(b"SherpaOnnxDestroyOfflineSpeechDenoiser\0")?,
                destroy_denoised: *library.get(b"SherpaOnnxDestroyDenoisedAudio\0")?,
                denoiser_rate: *library.get(b"SherpaOnnxOfflineSpeechDenoiserGetSampleRate\0")?,
                _libraries: {
                    #[cfg(windows)]
                    {
                        vec![library, ort, providers]
                    }
                    #[cfg(not(windows))]
                    {
                        vec![library, ort]
                    }
                },
            }))
        }
    }
}
fn name(path: &Path) -> Result<CString> {
    CString::new(path.to_str().context("Путь модели должен быть UTF-8")?)
        .context("Недопустимый путь модели")
}
pub(super) struct Engine {
    api: Arc<Api>,
    handle: *const c_void,
    separation: bool,
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if self.separation {
                (self.api.destroy_separation)(self.handle)
            } else {
                (self.api.destroy_denoiser)(self.handle)
            }
        }
    }
}
struct StemsGuard<'a> {
    api: &'a Api,
    output: *const SeparationOutput,
}
impl Drop for StemsGuard<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.destroy_stems)(self.output) }
    }
}
struct SpeechGuard<'a> {
    api: &'a Api,
    output: *const DenoisedAudio,
}
impl Drop for SpeechGuard<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.destroy_denoised)(self.output) }
    }
}
impl Engine {
    pub fn create(installed: &Installed, separation: bool) -> Result<Self> {
        let api = Api::open(installed)?;
        let cpu = CString::new("cpu")?;
        let null = std::ptr::null();
        let handle = if separation {
            let vocals = name(&installed.vocals)?;
            let accompaniment = name(&installed.accompaniment)?;
            let config = SeparationConfig {
                model: SeparationModel {
                    spleeter: Spleeter {
                        vocals: vocals.as_ptr(),
                        accompaniment: accompaniment.as_ptr(),
                    },
                    uvr: Uvr { model: null },
                    num_threads: 2,
                    debug: 0,
                    provider: cpu.as_ptr(),
                },
            };
            unsafe { (api.create_separation)(&config) }
        } else {
            let model = name(&installed.speech)?;
            let config = DenoiserConfig {
                model: DenoiserModel {
                    gtcrn: Gtcrn { model: null },
                    num_threads: 2,
                    debug: 0,
                    provider: cpu.as_ptr(),
                    dpdfnet: Dpdf {
                        model: model.as_ptr(),
                        attenuation_limit_db: 0.0,
                    },
                    voxlock: VoxLock {
                        enroll_model: null,
                        chunk_model: null,
                        voice_sample: null,
                    },
                },
            };
            unsafe { (api.create_denoiser)(&config) }
        };
        ensure!(!handle.is_null(), "CPU модель MediaTools не загрузилась");
        let engine = Self {
            api,
            handle,
            separation,
        };
        let rate = unsafe {
            if separation {
                (engine.api.separation_rate)(handle)
            } else {
                (engine.api.denoiser_rate)(handle)
            }
        };
        ensure!(
            rate == if separation { 44100 } else { 16000 },
            "Модель MediaTools имеет неожиданную частоту"
        );
        Ok(engine)
    }
    pub fn run(&self, input: &[f32], channels: usize, rate: u32, vocals: bool) -> Result<Vec<f32>> {
        ensure!(
            channels > 0
                && channels <= 2
                && input.len().is_multiple_of(channels)
                && input.len() / channels <= i32::MAX as usize,
            "Недопустимый блок PCM"
        );
        ensure!(
            input.iter().all(|v| v.is_finite()),
            "PCM содержит нечисловые значения"
        );
        let n = input.len() / channels;
        if self.separation {
            ensure!(
                channels == 2 && rate == 44100,
                "Spleeter требует stereo44100Hz"
            );
            // Pinned Spleeter has center=false; full-window aligned padding prevents ISTFT edge gain.
            let pad = 4096usize;
            let padded = n.checked_add(pad * 2).context("Переполнение блока PCM")?;
            ensure!(padded <= i32::MAX as usize, "Слишком большой блок PCM");
            let planes: Vec<Vec<f32>> = (0..channels)
                .map(|c| {
                    let mut samples = vec![0.0; padded];
                    for (i, value) in input.iter().skip(c).step_by(channels).enumerate() {
                        samples[pad + i] = *value;
                    }
                    samples
                })
                .collect();
            let pointers: Vec<_> = planes.iter().map(|v| v.as_ptr()).collect();
            let output = unsafe {
                (self.api.run_separation)(
                    self.handle,
                    pointers.as_ptr(),
                    channels as i32,
                    padded as i32,
                    rate as i32,
                )
            };
            ensure!(!output.is_null(), "Модель не вернула разделённые источники");
            let guard = StemsGuard {
                api: &self.api,
                output,
            };
            let output = unsafe { &*guard.output };
            ensure!(
                output.num_stems == 2
                    && !output.stems.is_null()
                    && output.sample_rate == rate as i32,
                "Неверный результат Spleeter"
            );
            let stem = unsafe { &*output.stems.add(if vocals { 0 } else { 1 }) };
            ensure!(
                stem.num_channels == channels as i32
                    && stem.n > 0
                    && stem.n as usize >= pad + n
                    && (stem.n as usize).abs_diff(padded) <= rate as usize
                    && !stem.samples.is_null(),
                "Неверные размеры stem Spleeter"
            );
            let mut result = vec![0.0f32; input.len()];
            for c in 0..channels {
                let pointer = unsafe { *stem.samples.add(c) };
                ensure!(!pointer.is_null(), "Пустой stem Spleeter");
                let samples = unsafe { std::slice::from_raw_parts(pointer, stem.n as usize) };
                ensure!(
                    samples.iter().all(|s| s.is_finite()),
                    "Модель вернула нечисловой PCM"
                );
                for (i, value) in samples[pad..pad + n].iter().enumerate() {
                    result[i * channels + c] = *value;
                }
            }
            Ok(result)
        } else {
            ensure!(
                channels == 1 && rate == 16000,
                "DPDFNet2 требует mono16000Hz"
            );
            // Pinned DPDFNet2 metadata window=320; upstream shifts640 samples. Supply tail context.
            let mut padded = input.to_vec();
            padded.resize(n + 640, 0.0);
            ensure!(
                padded.len() <= i32::MAX as usize,
                "Слишком большой блок PCM"
            );
            let output = unsafe {
                (self.api.run_denoiser)(
                    self.handle,
                    padded.as_ptr(),
                    padded.len() as i32,
                    rate as i32,
                )
            };
            ensure!(!output.is_null(), "Модель не вернула улучшенную речь");
            let guard = SpeechGuard {
                api: &self.api,
                output,
            };
            let output = unsafe { &*guard.output };
            ensure!(
                !output.samples.is_null()
                    && output.n > 0
                    && output.n as usize >= n
                    && (output.n as usize).abs_diff(padded.len()) <= rate as usize / 2
                    && output.sample_rate == rate as i32,
                "Неверный размер PCM DPDFNet"
            );
            // sherpa's pinned DPDFNet implementation already shifts by 2*window_length; do not shift twice.
            let samples = unsafe { std::slice::from_raw_parts(output.samples, output.n as usize) };
            ensure!(
                samples.iter().all(|s| s.is_finite()),
                "Модель вернула нечисловую речь"
            );
            Ok(samples[..n].to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_64bit_c_api_layout_matches_v2_header() {
        if std::mem::size_of::<usize>() == 8 {
            assert_eq!(std::mem::size_of::<SeparationConfig>(), 40);
            assert_eq!(std::mem::size_of::<DenoiserConfig>(), 64);
            assert_eq!(std::mem::size_of::<Stem>(), 16);
            assert_eq!(std::mem::size_of::<SeparationOutput>(), 16);
            assert_eq!(std::mem::size_of::<DenoisedAudio>(), 16);
        }
    }
}
