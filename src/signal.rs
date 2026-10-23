use core::f64::consts::PI;

/// A sound from a particular source. Contains a set of amplitude samples and the sampling rate
#[derive(Debug, Clone)]
pub struct Signal {
    pub samples: Vec<f64>,
    pub sample_rate: u32,
}
impl Signal {
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
        let start_idx = (start * self.sample_rate as f64) as usize;
        let end_idx = (end * self.sample_rate as f64) as usize;

        let start_idx = start_idx.min(self.samples.len());
        let end_idx = end_idx.min(self.samples.len());

        let start_idx = start_idx.min(end_idx);

        Self {
            samples: self.samples[start_idx..end_idx].to_vec(),
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
