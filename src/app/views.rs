use ratatui::{
    prelude::{
        Color, Constraint, Frame, Layout, Rect, Style, Stylize,
        style::Styled,
        symbols::{self, Marker},
    },
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Padding, Paragraph},
};

use crate::{
    Signal,
    app::PlaybackState,
    maths::fourier::{fft_signal, stft},
};

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
    const BORDER: Color = Color::Rgb(222, 126, 109);
    pub const ALL: [View; 3] = [View::Waveform, View::Spectrum, View::Spectrogram];

    pub fn title(self) -> &'static str {
        match self {
            View::Waveform => "1 Waveform",
            View::Spectrum => "2 Spectrum",
            View::Spectrogram => "3 Spectrogram",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            View::Waveform => "Waveform",
            View::Spectrum => "Spectrum",
            View::Spectrogram => "Spectrogram",
        }
    }

    /// The frame shared by every view
    pub fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::ROUNDED)
            .border_style(Style::new().fg(Self::BORDER))
            .padding(Padding::proportional(2))
            .title(self.name().fg(Self::BORDER))
    }
}
pub struct Views {
    pub waveform: WaveformView,
    pub spectrum: SpectrumView,
    pub spectrogram: SpectrogramView,
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
            return;
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
            .light_cyan();

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
                    .labels(["0".to_string(), signal.duration().to_string()])
                    .light_cyan(),
            )
            .y_axis(
                Axis::default()
                    .title("Amplitude")
                    .bounds([-1.0, 1.0])
                    .labels(["-1.0", "0", "1.0"])
                    .light_cyan(),
            );
        frame.render_widget(chart, area);
    }
}

pub struct SpectrumView {
    data: Vec<(f64, f64)>,
    max_freq: f64,
    max_amp: f64,
    computed: bool,
}

impl SpectrumView {
    pub fn new() -> Self {
        return Self {
            data: Vec::new(),
            max_freq: 1.0,
            max_amp: 1.0,
            computed: false,
        };
    }
    /// Call when the signal (or later, the mix) changes.
    #[allow(dead_code)]
    pub fn invalidate(&mut self) {
        self.computed = false;
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect, signal: &Signal) {
        if !self.computed {
            self.data = fft_signal(signal)
                .iter()
                // If your function returns the mirrored upper half too, drop it:
                .filter(|f| f.frequency <= signal.sample_rate as f64 / 2.0)
                .map(|f| (f.frequency, f.amplitude))
                .collect();
            self.max_freq = self.data.iter().map(|p| p.0).fold(0.0, f64::max).max(1.0);
            self.max_amp = self.data.iter().map(|p| p.1).fold(0.0, f64::max).max(1e-9);
            self.computed = true;
        }

        let data = Dataset::default()
            .data(&self.data)
            .graph_type(GraphType::Bar)
            .marker(Marker::Braille)
            .light_red();

        let chart = Chart::new(vec![data])
            .x_axis(
                Axis::default()
                    .title("Frequency (Hz)")
                    .bounds([0.0, self.max_freq])
                    .labels(["0".to_string(), format!("{:.0}", self.max_freq)])
                    .light_red(),
            )
            .y_axis(
                Axis::default()
                    .title("Amplitude")
                    .bounds([0.0, self.max_amp])
                    .labels(["0".to_string(), format!("{:.2}", self.max_amp)])
                    .light_red(),
            );
        frame.render_widget(chart, area);
    }
}

const WINDOW: usize = 2048;
const HOP: usize = 512;
const DB_RANGE: f64 = 80.0; // anything quieter than peak - 80 dB is black
const MAX_HZ: f64 = 8000.0;
pub struct SpectrogramView {
    frames: Vec<Vec<f64>>,
    computed: bool,
    grid: Vec<f64>,
    grid_size: (u16, u16),
}
impl SpectrogramView {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            computed: false,
            grid: Vec::new(),
            grid_size: (0, 0),
        }
    }
    pub fn invalidate(&mut self) {
        self.computed = false;
        self.grid_size = (0, 0);
    }

    fn compute(&mut self, signal: &Signal) {
        let magnitudes = stft(signal, WINDOW, HOP);
        let peak = magnitudes.iter().flatten().copied().fold(1e-12, f64::max);
        // magnitude -> dB relative to the loudest cell -> 0..=1
        self.frames = magnitudes
            .into_iter()
            .map(|frame| {
                frame
                    .into_iter()
                    .map(|m| {
                        let db = 20.0 * (m / peak).max(1e-12).log10();
                        (db.clamp(-DB_RANGE, 0.0) + DB_RANGE) / DB_RANGE
                    })
                    .collect()
            })
            .collect();
        self.computed = true;
        self.grid_size = (0, 0);
    }
    fn rebuild(&mut self, sample_rate: u32, width: u16, height: u16) {
        let cols = width as usize;
        let rows = height as usize * 2;
        let n_frames = self.frames.len();
        let n_bins = self.frames[0].len();
        let hz_per_bin = sample_rate as f64 / WINDOW as f64;
        let top_bin = ((MAX_HZ / hz_per_bin) as usize).clamp(1, n_bins - 1);

        self.grid = vec![0.0; cols * rows];
        for row in 0..rows {
            let from_bottom = rows - 1 - row;
            let b0 = from_bottom * top_bin / rows;
            let b1 = ((from_bottom + 1) * top_bin / rows).max(b0 + 1).min(n_bins);
            for col in 0..cols {
                let f0 = col * n_frames / cols;
                let f1 = ((col + 1) * n_frames / cols).max(f0 + 1).min(n_frames);
                // take the max over everything that lands in this pixel
                let mut v = 0.0f64;
                for f in f0..f1 {
                    for b in b0..b1 {
                        v = v.max(self.frames[f][b]);
                    }
                }
                self.grid[row * cols + col] = v;
            }
        }
        self.grid_size = (width, height);
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        signal: &Signal,
        playback: &PlaybackState,
    ) {
        // small gutter on the left for frequency labels
        let [gutter, plot] = area.layout(&Layout::horizontal([
            Constraint::Length(8),
            Constraint::Fill(1),
        ]));

        if !self.computed {
            self.compute(signal);
        }
        if self.frames.is_empty() || plot.width == 0 || plot.height == 0 {
            frame.render_widget(Paragraph::new("Signal too short"), area);
            return;
        }
        if self.grid_size != (plot.width, plot.height) {
            self.rebuild(signal.sample_rate, plot.width, plot.height);
        }

        let cols = plot.width as usize;
        let cursor_col = ((playback.position.as_secs_f64() / signal.duration()) * cols as f64)
            .clamp(0.0, cols as f64 - 1.0) as usize;

        let buffer = frame.buffer_mut();
        for cy in 0..plot.height as usize {
            for cx in 0..cols {
                let top = self.grid[(cy * 2) * cols + cx];
                let bottom = self.grid[(cy * 2 + 1) * cols + cx];
                if let Some(cell) = buffer.cell_mut((plot.x + cx as u16, plot.y + cy as u16)) {
                    if cx == cursor_col {
                        cell.set_char('│').set_fg(Color::White).set_bg(heat(bottom));
                    } else {
                        cell.set_char('▀').set_fg(heat(top)).set_bg(heat(bottom));
                    }
                }
            }
        }

        // frequency labels
        let top_label = Paragraph::new(format!("{:.0} Hz", MAX_HZ)).right_aligned();
        frame.render_widget(
            top_label,
            Rect {
                height: 1,
                ..gutter
            },
        );
        let bottom_label = Paragraph::new("0 Hz").right_aligned();
        frame.render_widget(
            bottom_label,
            Rect {
                y: gutter.y + gutter.height - 1,
                height: 1,
                ..gutter
            },
        );
    }
}

/// 0..=1 -> black -> purple -> orange -> pale yellow
fn heat(v: f64) -> Color {
    const STOPS: [(f64, [f64; 3]); 4] = [
        (0.0, [0.0, 0.0, 0.0]),
        (0.4, [110.0, 20.0, 130.0]),
        (0.75, [240.0, 110.0, 30.0]),
        (1.0, [255.0, 250.0, 200.0]),
    ];
    let v = v.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let (a, b) = (w[0], w[1]);
        if v <= b.0 {
            let t = (v - a.0) / (b.0 - a.0);
            let c = |i: usize| (a.1[i] + (b.1[i] - a.1[i]) * t) as u8;
            return Color::Rgb(c(0), c(1), c(2));
        }
    }
    Color::Rgb(255, 250, 200)
}
