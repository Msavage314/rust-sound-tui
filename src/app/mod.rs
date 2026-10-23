pub mod views;
use core::{error::Error, num::NonZero, time::Duration};
use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{
        self, Event,
        KeyCode::{self},
        KeyEventKind,
    },
    prelude::{
        Constraint, Frame, Layout, Rect, Style, Stylize,
        symbols::{self},
    },
    widgets::{Block, Borders, Row, Table, Tabs},
};
use rodio::{Player, buffer::SamplesBuffer};

use crate::{
    Signal,
    app::views::{SpectrumView, View, Views, WaveformView},
};

pub struct PlaybackState {
    pub position: Duration,
    pub playing: bool,
    pub finished: bool,
}

const FRAME: Duration = Duration::from_millis(33); // ~30 fps

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

pub struct App {
    signal: Signal,
    view: View,
    audio: AudioPlayer,
    running: bool,
    pub views: Views,
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
                spectrum: SpectrumView::new(),
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

            View::Spectrum => {
                self.views.spectrum.render(frame, main, &self.signal);
            }
            View::Spectrogram => {
                frame.render_widget(Block::bordered().title("Not implemented yet"), main);
            }
        }
    }
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let sample_count = self.signal.samples.len();
        let table = Table::new(
            [
                Row::new([
                    "Sample rate".to_string(),
                    format!("{} Hz", self.signal.sample_rate),
                ]),
                Row::new([
                    "Length".to_string(),
                    format!("{:.2}s", self.signal.duration()),
                ]),
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
