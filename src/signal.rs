use core::f64::consts::PI;

use crate::maths::fourier::Bin;

/// A sound from a particular source. Contains a set of amplitude samples and the sampling rate
#[derive(Debug, Clone)]
pub struct Signal {
    pub samples: Vec<f64>,
    pub sample_rate: u32,
}
impl Signal {
    pub fn from_frequencies(frequencies: &[Bin]) -> Signal {
        let max = frequencies.iter().map(|f| f.amplitude).fold(0.0, f64::max);
        let threshold = max * 0.01; // keep bins within 40 dB of the peak
        let sample_rate = 44100;
        let num_samples = (sample_rate as f64 * 2.5) as usize;
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
    pub fn crop(&self, start: usize, end: usize) -> Self {
        assert!(start <= end, "start must be <= end");
        assert!(end <= self.samples.len(), "crop range out of bounds");

        Self {
            samples: self.samples[start..end].to_vec(),
            sample_rate: self.sample_rate,
        }
    }
    /// Crops the signal to a start and an end position in seconds
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
pub fn generate_signal(
    frequency: f64,
    sample_rate: u32,
    duration: f64,
    amplitude: f64,
    phase: f64,
) -> Signal {
    let num_samples = (sample_rate as f64 * duration) as usize;

    let samples = (0..num_samples)
        .map(|i| {
            let time = i as f64 / sample_rate as f64;
            (time * 2.0 * PI * frequency + phase).cos() * amplitude
        })
        .collect();

    Signal {
        samples,
        sample_rate,
    }
}
