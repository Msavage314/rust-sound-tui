use core::{error::Error, num::NonZero, time::Duration};
use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{
        self, Event,
        KeyCode::{self, Tab},
        KeyEventKind,
    },
    prelude::{
        Constraint, Frame, Layout, Rect, Style, Stylize,
        symbols::{self, Marker},
    },
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Padding, Row, Table, Tabs},
};
use rodio::{Player, buffer::SamplesBuffer};

use crate::Signal;

pub struct PlaybackState {
    pub position: Duration,
    pub playing: bool,
    pub finished: bool,
}

const FRAME: Duration = Duration::from_millis(33); // ~30 fps

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
    fn next(self) -> Self {
        match self {
            View::Waveform => View::Spectrum,
            View::Spectrum => View::Spectrogram,
            View::Spectrogram => View::Waveform,
        }
    }

    const ALL: [View; 3] = [View::Waveform, View::Spectrum, View::Spectrogram];

    fn title(self) -> &'static str {
        match self {
            View::Waveform => "1 Waveform",
            View::Spectrum => "2 Spectrum",
            View::Spectrogram => "3 Spectrogram",
        }
    }
}

pub struct AudioPlayer {
    _sink: rodio::MixerDeviceSink,
    player: rodio::Player,
}
impl AudioPlayer {
    /// Constructs a new AudioPlayer with the default sink
    /// and a new player object
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let _sink = rodio::DeviceSinkBuilder::open_default_sink()?;
        let player = Player::connect_new(_sink.mixer());
        return Ok(Self { _sink, player });
    }
    /// Loads a signal into the player and adds it to the source
    pub fn load(&mut self, signal: &Signal) {
        let samples: Vec<f32> = signal.samples.iter().map(|&sample| sample as f32).collect();
        let source = SamplesBuffer::new(
            NonZero::new(1).unwrap(),
            NonZero::new(signal.sample_rate).unwrap(),
            samples,
        );
        self.player.clear();
        self.player.append(source);
    }
    /// Starts plating the signal if paused, if not then pause
    pub fn toggle(&mut self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }
    pub fn stop(&mut self) {
        self.player.stop();
    }
    pub fn seek(&mut self, to: Duration) {
        self.player.try_seek(to).expect("Failed to seek");
    }
    pub fn state(&self) -> PlaybackState {
        let finished = self.player.empty();
        PlaybackState {
            position: self.player.get_pos(),
            playing: !finished && !self.player.is_paused(),
            finished,
        }
    }
    pub fn play(&self) {
        self.player.play();
    }
}
struct Views {
    waveform: WaveformView,
}

struct WaveformView {
    cached_width: u16,
    data: Vec<(f64, f64)>,
}

impl WaveformView {
    fn new() -> Self {
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
    fn render(&mut self, frame: &mut Frame, area: Rect, signal: &Signal, playback: &PlaybackState) {
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
pub struct App {
    signal: Signal,
    view: View,
    audio: AudioPlayer,
    running: bool,
    views: Views,
}
impl App {
    pub fn new(signal: Signal) -> Result<Self, Box<dyn Error>> {
        let mut audio = AudioPlayer::new()?;
        audio.load(&signal);
        Ok(Self {
            signal,
            view: View::Waveform,
            audio,
            running: true,
            views: Views {
                waveform: WaveformView::new(),
            },
        })
    }
    fn toggle_playback(&mut self) {
        if self.audio.state().finished {
            self.audio.load(&self.signal);
            self.audio.play();
        } else {
            self.audio.toggle();
        }
    }
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.running {
            let playback = self.audio.state();
            terminal.draw(|f| self.render(f, &playback))?;

            if event::poll(FRAME)? {
                'eventloop: loop {
                    self.handle_event(event::read()?);
                    if !event::poll(Duration::ZERO)? {
                        break 'eventloop;
                    }
                }
            }
            //self.handle_events()?;
        }
        return Ok(());
    }
    fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return }; // Resize etc. just trigger a redraw
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::Char('s') => self.audio.stop(),
            KeyCode::Tab => self.view = self.view.next(),
            _ => {} // view-specific keys
        }
    }

    fn render(&mut self, frame: &mut Frame, playback: &PlaybackState) {
        let [tabs_area, top, main] = frame.area().layout(&Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Fill(1),
        ]));
        let tabs = Tabs::new(View::ALL.iter().map(|v| v.title()))
            .select(self.view as usize)
            .highlight_style(Style::new().bold().yellow())
            .block(Block::bordered());
        frame.render_widget(tabs, tabs_area);
        self.render_table(frame, top);
        match self.view {
            View::Waveform => self
                .views
                .waveform
                .render(frame, main, &self.signal, playback),
            View::Spectrum | View::Spectrogram => {
                frame.render_widget(Block::bordered().title("Not implemented yet"), main);
            }
        }
    }
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let sample_count = self.signal.samples.len();
        let duration = sample_count as f64 / self.signal.sample_rate as f64;
        let table = Table::new(
            [
                Row::new([
                    "Sample rate".to_string(),
                    format!("{} Hz", self.signal.sample_rate),
                ]),
                Row::new(["Length".to_string(), format!("{:.2}s", duration)]),
                Row::new(["Samples".to_string(), format!("{}", sample_count)]),
            ],
            [
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ],
        )
        .block(
            Block::default()
                .title("Audio information")
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED)
                .dark_gray(),
        );
        frame.render_widget(table, area);
    }
}
