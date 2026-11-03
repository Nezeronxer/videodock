//! Delayed iterative weighted linear prediction, not a denoising substitute.
//! Equations: Yoshioka/Nakatani 2012; NARA-WPE reference implementation (MIT).
// Indexed loops keep the matrix and frequency/time axes explicit.
#![allow(clippy::needless_range_loop)]
use anyhow::{Result, ensure};
use rustfft::{FftPlanner, num_complex::Complex64};
use tokio_util::sync::CancellationToken;
const TAPS: usize = 16;
const DELAY: usize = 3;
fn solve(
    mut matrix: [[Complex64; TAPS]; TAPS],
    mut vector: [Complex64; TAPS],
) -> Option<[Complex64; TAPS]> {
    for column in 0..TAPS {
        let pivot = (column..TAPS).max_by(|&a, &b| {
            matrix[a][column]
                .norm_sqr()
                .total_cmp(&matrix[b][column].norm_sqr())
        })?;
        if matrix[pivot][column].norm_sqr() < 1e-24 {
            return None;
        }
        matrix.swap(column, pivot);
        vector.swap(column, pivot);
        let divisor = matrix[column][column];
        for j in column..TAPS {
            matrix[column][j] /= divisor;
        }
        vector[column] /= divisor;
        for row in 0..TAPS {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for j in column..TAPS {
                let value = matrix[column][j];
                matrix[row][j] -= factor * value;
            }
            let value = vector[column];
            vector[row] -= factor * value;
        }
    }
    vector
        .iter()
        .all(|v| v.re.is_finite() && v.im.is_finite())
        .then_some(vector)
}
pub(super) fn dereverb(input: &[f32], rate: u32, cancel: &CancellationToken) -> Result<Vec<f32>> {
    ensure!(
        (8000..=96000).contains(&rate)
            && input.len() >= rate as usize / 2
            && input.len() <= rate as usize * 20,
        "WPE требует блок от 0,5 до 20 секунд"
    );
    ensure!(
        input.iter().all(|v| v.is_finite()),
        "Нечисловой PCM для WPE"
    );
    let size = if rate <= 24000 { 512 } else { 1024 };
    let hop = size / 4;
    let pad = size / 2;
    let frames = input.len().div_ceil(hop) + 1;
    let bins = size / 2 + 1;
    let window: Vec<_> = (0..size)
        .map(|i| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / size as f64).cos()).sqrt())
        .collect();
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    let mut buffer = vec![Complex64::new(0.0, 0.0); size];
    let mut spectra = vec![vec![Complex64::new(0.0, 0.0); frames]; bins];
    for frame in 0..frames {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        for j in 0..size {
            let index = frame * hop + j;
            let value = if index >= pad && index - pad < input.len() {
                input[index - pad] as f64
            } else {
                0.0
            };
            buffer[j] = Complex64::new(value * window[j], 0.0);
        }
        fft.process(&mut buffer);
        for frequency in 0..bins {
            spectra[frequency][frame] = buffer[frequency];
        }
    }
    for spectrum in &mut spectra {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        let original = spectrum.clone();
        let maximum = original.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
        if maximum < 1e-16 {
            continue;
        }
        let floor = (maximum * 1e-8).max(1e-12);
        let zero = Complex64::new(0.0, 0.0);
        for _ in 0..3 {
            let mut matrix = [[zero; TAPS]; TAPS];
            let mut vector = [zero; TAPS];
            for time in DELAY + TAPS - 1..frames {
                let weight = 1.0 / spectrum[time].norm_sqr().max(floor);
                let past: [Complex64; TAPS] = std::array::from_fn(|k| original[time - DELAY - k]);
                for row in 0..TAPS {
                    vector[row] += past[row] * original[time].conj() * weight;
                    for column in 0..TAPS {
                        matrix[row][column] += past[row] * past[column].conj() * weight;
                    }
                }
            }
            let trace = (0..TAPS).map(|i| matrix[i][i].re.max(0.0)).sum::<f64>();
            let regularizer = trace / TAPS as f64 * 1e-6 + 1e-10;
            for i in 0..TAPS {
                matrix[i][i].re += regularizer;
            }
            let Some(filter) = solve(matrix, vector) else {
                continue;
            };
            for time in DELAY + TAPS - 1..frames {
                let predicted = (0..TAPS)
                    .map(|k| filter[k].conj() * original[time - DELAY - k])
                    .sum::<Complex64>();
                spectrum[time] = original[time] - predicted;
            }
        }
    }
    let mut output = vec![0.0f64; frames * hop + size];
    let mut normalization = vec![0.0f64; output.len()];
    for frame in 0..frames {
        ensure!(!cancel.is_cancelled(), "Обработка отменена");
        for frequency in 0..bins {
            buffer[frequency] = spectra[frequency][frame];
        }
        for frequency in 1..size / 2 {
            buffer[size - frequency] = buffer[frequency].conj();
        }
        inverse.process(&mut buffer);
        for j in 0..size {
            let index = frame * hop + j;
            output[index] += buffer[j].re / size as f64 * window[j];
            normalization[index] += window[j] * window[j];
        }
    }
    let result: Vec<f32> = (pad..pad + input.len())
        .map(|i| (output[i] / normalization[i].max(1e-12)) as f32)
        .collect();
    ensure!(
        result.iter().all(|v| v.is_finite()),
        "WPE потерял численную устойчивость"
    );
    let input_energy = input.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    let output_energy = result.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    ensure!(
        output_energy <= input_energy * 100.0 + 1e-10,
        "WPE потерял устойчивость усиления"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_prediction_reduces_synthetic_late_echo_without_muting_direct_signal() {
        let rate = 16000usize;
        let mut clean = vec![0f32; rate * 3];
        let mut rng = 7u32;
        for (i, value) in clean.iter_mut().enumerate() {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let frame = i % (rate / 4);
            if frame < rate / 16 {
                let window = (std::f32::consts::PI * frame as f32 / (rate / 16) as f32)
                    .sin()
                    .powi(2);
                *value = ((rng >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * window;
            }
        }
        let delay = rate * 80 / 1000;
        let mut reverberant = clean.clone();
        for i in delay..clean.len() {
            reverberant[i] += 0.65 * reverberant[i - delay];
        }
        let output = dereverb(&reverberant, rate as u32, &CancellationToken::new()).unwrap();
        assert_eq!(output.len(), clean.len());
        assert!(output.iter().all(|x| x.is_finite()));
        let before: f64 = reverberant
            .iter()
            .zip(&clean)
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum();
        let after: f64 = output
            .iter()
            .zip(&clean)
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum();
        let direct: f64 = output
            .iter()
            .zip(&clean)
            .filter(|(_, b)| b.abs() > 0.02)
            .map(|(a, _)| f64::from(*a).powi(2))
            .sum();
        let original_direct: f64 = clean
            .iter()
            .filter(|v| v.abs() > 0.02)
            .map(|v| f64::from(*v).powi(2))
            .sum();
        eprintln!(
            "WPE error before={before:.6} after={after:.6}; direct ratio={:.4}",
            direct / original_direct
        );
        assert!(
            after < before * 0.8,
            "dereverberation must reduce echo error, not only change bytes"
        );
        assert!(
            direct / original_direct > 0.65,
            "direct signal must remain audible"
        );
    }
}
