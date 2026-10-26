pub mod views;
use core::{error::Error, num::NonZero, time::Duration};
use std::{fmt::format, io};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{
        self, Event,
        KeyCode::{self},
        KeyEventKind,
    },
    prelude::{
        Color, Constraint, Frame, Layout, Rect, Style, Stylize,
        symbols::{self},
    },
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Row, Table, Tabs},
};
use rodio::{Player, buffer::SamplesBuffer};

use crate::{
    Signal,
    app::views::{SpectrogramView, SpectrumView, View, Views, WaveformView},
    maths::fourier::Bin,
};

pub struct PlaybackState {
    pub position: Duration,
    pub playing: bool,
    pub finished: bool,
}

const FRAME: Duration = Duration::from_millis(33); // ~30 fps

pub struct Track {
    pub name: String,
    pub signal: Signal,
    pub enabled: bool,
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
        self.player.clear();
        if signal.samples.is_empty() {
            return; // nothing to play (all tracks switched off)
        }
        let samples: Vec<f32> = signal.samples.iter().map(|&s| s as f32).collect();
        let source = SamplesBuffer::new(
            NonZero::new(1).unwrap(),
            NonZero::new(signal.sample_rate).unwrap(),
            samples,
        );
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
    /// Put position/play state back after a reload
    pub fn restore(&mut self, pos: Duration, playing: bool) {
        let _ = self.player.try_seek(pos);
        if playing {
            self.player.play();
        } else {
            self.player.pause();
        }
    }
}

pub struct App {
    tracks: Vec<Track>,
    mix: Signal,
    sample_rate: u32,
    list_state: ListState,
    input: Option<String>,
    view: View,
    audio: AudioPlayer,
    running: bool,
    pub views: Views,
}
impl App {
    pub fn new(tracks: Vec<Track>) -> Result<Self, Box<dyn Error>> {
        let sample_rate = tracks.first().map_or(44100, |t| t.signal.sample_rate);
        let mix = Signal::mix(
            tracks.iter().filter(|t| t.enabled).map(|t| &t.signal),
            sample_rate,
        );

        let mut audio = AudioPlayer::new()?;
        let mut list_state = ListState::default();
        if !tracks.is_empty() {
            list_state.select(Some(0));
        }
        Ok(Self {
            tracks,
            mix,
            sample_rate,
            list_state,
            input: None,
            view: View::Waveform,
            audio,
            running: true,
            views: Views {
                waveform: WaveformView::new(),
                spectrum: SpectrumView::new(),
                spectrogram: SpectrogramView::new(),
            },
        })
    }

    /// Rebuild the mixed signal from all enabled tracks
    fn remix(&mut self) {
        let before = self.audio.state();

        let mix = Signal::mix(self.tracks.iter().map(|t| &t.signal), self.sample_rate);

        self.audio.load(&mix);
        self.audio.restore(before.position, before.playing);
        self.views.waveform.invalidate();
        self.views.spectrum.invalidate();
        self.views.spectrogram.invalidate();
    }
    fn add_sine(&mut self, freq: f64) {
        if !(freq > 0.0 && freq < self.sample_rate as f64 / 2.0) {
            return; // Must be below the Nyquist rate
        }
        let longest = self
            .tracks
            .iter()
            .map(|t| t.signal.duration())
            .fold(0.0, f64::max);

        let duration = if longest > 0.0 { longest } else { 4.0 };
        self.tracks.push(Track {
            name: format!("{freq} Hz sine"),
            signal: Signal::sine(
                Bin {
                    frequency: freq,
                    phase: 0.0,
                    amplitude: 0.5,
                },
                self.sample_rate,
                duration,
            ),
            enabled: true,
        });
        self.list_state.select(Some(self.tracks.len() - 1));
        self.remix();
    }
    fn toggle_selected(&mut self) {
        if let Some(t) = self
            .list_state
            .selected()
            .and_then(|i| self.tracks.get_mut(i))
        {
            t.enabled = !t.enabled;
            self.remix();
        }
    }
    fn remove_selected(&mut self) {
        let Some(i) = self.list_state.selected() else {
            return;
        };
        if i >= self.tracks.len() {
            return;
        }
        self.tracks.remove(i);
        self.list_state.select(if self.tracks.is_empty() {
            None
        } else {
            Some(i.min(self.tracks.len() - 1))
        });
        self.remix();
    }
    fn toggle_playback(&mut self) {
        if self.mix.samples.is_empty() {
            return;
        }
        if self.audio.state().finished {
            self.audio.load(&self.mix);
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
        let [tabs_area, top, body] = frame.area().layout(&Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Fill(1),
        ]));
        let [side, main] = body.layout(&Layout::horizontal([
            Constraint::Length(28),
            Constraint::Fill(1),
        ]));
        let tabs = Tabs::new(View::ALL.iter().map(|v| v.title()))
            .select(self.view as usize)
            .highlight_style(Style::new().bold().yellow())
            .block(Block::bordered().border_set(symbols::border::ROUNDED));
        frame.render_widget(tabs, tabs_area);

        self.render_table(frame, side);
        self.render_tracks(frame, side);

        let block = self.view.block();
        let inner = block.inner(main); // accounts for borders and padding
        frame.render_widget(block, main);
        match self.view {
            View::Waveform => self
                .views
                .waveform
                .render(frame, inner, &self.mix, playback),

            View::Spectrum => {
                self.views.spectrum.render(frame, inner, &self.mix);
            }
            View::Spectrogram => {
                self.views
                    .spectrogram
                    .render(frame, inner, &self.mix, playback);
            }
        }
    }
    fn render_tracks(&mut self, frame: &mut Frame, area: Rect) {
        let items = self.tracks.iter().map(|t| {
            let mark = if t.enabled { "[x]" } else { "[ ]" };
            let item = ListItem::new(format!("{mark} {}", t.name));
            if t.enabled { item } else { item.dark_gray() }
        });
        let hint = match &self.input {
            Some(buf) => format!("Hz: {buf}_ (Enter/Esc)"),
            None => "n new  e on/off  d del".to_string(),
        };
        let list = List::new(items).block(
            Block::bordered()
                .title("Signals")
                .title_bottom(hint)
                .border_set(symbols::border::ROUNDED),
        );
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
    fn handle_input_key(&mut self, code: KeyCode) {
        let Some(buf) = self.input.as_mut() else {
            return;
        };
        match code {
            KeyCode::Esc => self.input = None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => buf.push(c),
            KeyCode::Enter => {
                let text = self.input.take().unwrap_or_default();
                if let Ok(freq) = text.parse::<f64>() {
                    self.add_sine(freq);
                }
            }
            _ => {}
        }
    }
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let sample_count = self.mix.samples.len();
        let table = Table::new(
            [
                Row::new(["Rate".to_string(), format!("{} Hz", self.mix.sample_rate)]),
                Row::new(["Length".to_string(), format!("{:.2}s", self.mix.duration())]),
                Row::new(["Samples".to_string(), format!("{}", sample_count)]),
            ],
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .block(
            Block::default()
                .title("Audio information")
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED)
                .style(Style::new().fg(Color::Rgb(61, 172, 187)))
                .padding(Padding::vertical(1)),
        );
        frame.render_widget(table, area);
    }
}
