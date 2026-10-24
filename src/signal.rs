//! Tools for building and slicing mono audio samples

use core::f64::consts::PI;

use crate::maths::fourier::Bin;

/// A sound from a particular source. Contains a set of amplitude samples and the sampling rate
///
/// Samples are mono, stored as `f64` amplitudes, and are conventionally
/// normalised to roughly `[-1.0, 1.0]` (as produced by the WAV decoder).
/// Sample `i` occurs at time `i / sample_rate` seconds.
#[derive(Debug, Clone)]
pub struct Signal {
    /// Amplitude of the signal at each sample point, in time order
    pub samples: Vec<f64>,
    /// The number of samples per second (Hz)
    pub sample_rate: u32,
}
impl Signal {
    pub fn from_frequencies(frequencies: &[Bin], sample_rate: u32, duration: f64) -> Signal {
        let max = frequencies.iter().map(|f| f.amplitude).fold(0.0, f64::max);
        let threshold = max * 0.01; // keep bins within 40 dB of the peak
        let num_samples = (sample_rate as f64 * duration) as usize;
        let mut samples = vec![0.0; num_samples];

        for f in frequencies.iter().filter(|f| f.amplitude > threshold) {
            for (i, s) in samples.iter_mut().enumerate() {
                let t = i as f64 / sample_rate as f64;
                *s += (t * 2.0 * PI * f.frequency + f.phase).cos() * f.amplitude;
            }
        }
        Signal {
            samples,
            sample_rate,
        }
    }
    /// Returns a copy of the samples in the half-open range `start..end`,
    /// given as sample indices, with the same sample rate.
    ///
    /// # Panics
    ///
    /// Panics if `start > end` or if `end` is greater than the number of
    /// samples. For a version that clamps instead, see
    /// [`crop_seconds`](Self::crop_seconds).
    pub fn crop(&self, start: usize, end: usize) -> Self {
        assert!(start <= end, "start must be <= end");
        assert!(end <= self.samples.len(), "crop range out of bounds");

        Self {
            samples: self.samples[start..end].to_vec(),
            sample_rate: self.sample_rate,
        }
    }
    /// Crops the signal to a start and an end position in seconds
    ///
    /// Returns a copy of the samples from `start` up to (but not including)
    /// `end`, with the same sample rate. Times are converted to sample indices
    /// by truncation.
    ///
    /// Unlike [`crop`](Self::crop), an `end` past the end of the signal is
    /// clamped to the signal's length rather than causing a panic, so the
    /// result may be shorter than requested, or empty.
    ///
    /// # Panics
    ///
    /// Panics if `start` is negative, `start > end`, or either value is NaN.q
    pub fn crop_seconds(&self, start: f64, end: f64) -> Self {
        assert!(start >= 0.0 && start <= end);

        let len = self.samples.len();
        let start_idx = ((start * self.sample_rate as f64) as usize).min(len);
        let end_idx = ((end * self.sample_rate as f64) as usize).min(len);

        Self {
            samples: self.samples[start_idx.min(end_idx)..end_idx].to_vec(),
            sample_rate: self.sample_rate,
        }
    }
    /// Returns the duration of the signal in seconds
    ///
    /// Calculated as `samples.len() / sample_rate`.

    pub fn duration(&self) -> f64 {
        return self.samples.len() as f64 / self.sample_rate as f64;
    }
}

impl core::ops::Add<Signal> for Signal {
    type Output = Signal;
    fn add(self, rhs: Signal) -> Self::Output {
        assert!(self.sample_rate == rhs.sample_rate);
        assert_eq!(self.samples.len(), rhs.samples.len());
        let new = self
            .samples
            .iter()
            .zip(rhs.samples)
            .map(|(a, b)| a + b)
            .collect();

        return Self {
            samples: new,
            sample_rate: self.sample_rate,
        };
    }
}
impl core::ops::AddAssign<Signal> for Signal {
    fn add_assign(&mut self, rhs: Signal) {
        assert!(self.sample_rate == rhs.sample_rate);
        assert_eq!(self.samples.len(), rhs.samples.len());
        self.samples = self
            .samples
            .iter()
            .zip(rhs.samples)
            .map(|(a, b)| a + b)
            .collect();
    }
}
