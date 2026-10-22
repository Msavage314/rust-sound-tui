use ratatui::{
    prelude::{
        Frame, Rect, Stylize,
        symbols::{self, Marker},
    },
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Padding},
};

use crate::{Signal, app::PlaybackState};

/// represents the different views that the TUI app can provide
/// More information is underneath each one
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Shows a wave graph for the sound sample
    Waveform,
    /// Shows the intensity of different frequencies
    Spectrum,
    /// Shows a spectrogram showing the intensity of different frequencies over time
    Spectrogram,
}
impl View {
    pub fn next(self) -> Self {
        match self {
            View::Waveform => View::Spectrum,
            View::Spectrum => View::Spectrogram,
            View::Spectrogram => View::Waveform,
        }
    }

    pub const ALL: [View; 3] = [View::Waveform, View::Spectrum, View::Spectrogram];

    pub fn title(self) -> &'static str {
        match self {
            View::Waveform => "1 Waveform",
            View::Spectrum => "2 Spectrum",
            View::Spectrogram => "3 Spectrogram",
        }
    }
}
pub struct Views {
    pub waveform: WaveformView,
}

pub struct WaveformView {
    cached_width: u16,
    data: Vec<(f64, f64)>,
}

impl WaveformView {
    pub fn new() -> Self {
        Self {
            cached_width: 0,
            data: Vec::new(),
        }
    }
    #[allow(dead_code)]
    fn invalidate(&mut self) {
        self.cached_width = 0;
    }
    fn rebuild(&mut self, signal: &Signal, width: u16) {
        self.cached_width = width;
        self.data.clear();
        let sample_count = signal.samples.len();

        if sample_count == 0 {
            return;
        }

        let buckets = (width as usize).saturating_sub(10).max(1).min(sample_count);
        let bucket_size = sample_count.div_ceil(buckets);

        for start in (0..sample_count).step_by(bucket_size) {
            let end = (start + bucket_size).min(sample_count);
            let samples = &signal.samples[start..end];

            // find the minimum amplitude for this section
            let (min_offset, &min_value) = samples
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .unwrap();
            let (max_offset, &max_value) = samples
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap();
            // Keep the extrema in chronological order.
            if min_offset < max_offset {
                self.data.push(((start + min_offset) as f64, min_value));
                self.data.push(((start + max_offset) as f64, max_value));
            } else if max_offset < min_offset {
                self.data.push(((start + max_offset) as f64, max_value));
                self.data.push(((start + min_offset) as f64, min_value));
            } else {
                self.data.push(((start + min_offset) as f64, min_value));
            }
        }
    }
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        signal: &Signal,
        playback: &PlaybackState,
    ) {
        if signal.samples.is_empty() {
            frame.render_widget(Block::bordered().title("Waveform (empty"), area);
        }
        if self.cached_width != area.width {
            self.rebuild(signal, area.width);
        }
        let last = signal.samples.len().saturating_sub(1) as f64;
        let x = (playback.position.as_secs_f64() * signal.sample_rate as f64).clamp(0.0, last);

        let waveform = Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Area)
            .data(&self.data)
            .green();

        let progress = vec![(x, -1.0), (x, 1.0)];

        let cursor = Dataset::default()
            .marker(Marker::Custom('|'))
            .graph_type(GraphType::Line)
            .data(&progress)
            .white();

        let chart = Chart::new(vec![waveform, cursor])
            .x_axis(
                Axis::default()
                    .title("Sample")
                    .bounds([0.0, last])
                    .green()
                    .labels(["0".to_string(), last.to_string()]),
            )
            .y_axis(
                Axis::default()
                    .title("Amplitude")
                    .bounds([-1.0, 1.0])
                    .labels(["-0.5", "0", "0.5"])
                    .green(),
            )
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED)
                    .dark_gray()
                    .padding(Padding::proportional(2))
                    .title("Waveform"),
            );
        frame.render_widget(chart, area);
    }
}

pub struct SpectrogramView {}
