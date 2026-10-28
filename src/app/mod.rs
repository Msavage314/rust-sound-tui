pub mod player;
pub mod views;
use core::{error::Error, time::Duration};
use std::io;

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

use crate::{
    app::{
        player::AudioPlayer,
        views::{SpectrogramView, SpectrumView, View, Views, WaveformView},
    },
    maths::fourier::Bin,
    signal::Signal,
};

pub struct PlaybackState {
    pub position: Duration,
    pub playing: bool,
    pub finished: bool,
}

const FRAME: Duration = Duration::from_millis(17); // ~30 fps

pub struct Track {
    pub name: String,
    pub signal: Signal,
    pub enabled: bool,
}

impl Track {
    pub fn new(name: String, signal: Signal) -> Self {
        Self {
            name,
            signal,
            enabled: true,
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

        let audio = AudioPlayer::new()?;
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

        self.mix = Signal::mix(
            self.tracks.iter().filter(|t| t.enabled).map(|t| &t.signal),
            self.sample_rate,
        );

        self.audio.load(&self.mix);
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
        }
        return Ok(());
    }
    fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return }; // Resize etc. just trigger a redraw
        if key.kind != KeyEventKind::Press {
            return;
        }
        if self.input.is_some() {
            self.handle_input_key(key.code);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::Char('s') => self.audio.stop(),
            KeyCode::Tab => self.view = self.view.next(),
            KeyCode::Up | KeyCode::Char('k') => self.list_state.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.list_state.select_next(),
            KeyCode::Enter | KeyCode::Char('e') => self.toggle_selected(),
            KeyCode::Char('n') => self.input = Some(String::new()),
            KeyCode::Char('d') | KeyCode::Delete => self.remove_selected(),
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

        self.render_table(frame, top);
        self.render_tracks(frame, side);

        let block = self.view.block();
        let inner = block.inner(main);
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
            let mark = if t.enabled { " 󰥛 " } else { " 󰥛 " };
            let item = ListItem::new(format!("{mark} {}", t.name));
            if t.enabled {
                item.bg(Color::DarkGray)
            } else {
                item.dark_gray()
            }
        });
        let hint = match &self.input {
            Some(buf) => format!("Hz: {buf}_ (Enter/Esc)"),
            None => "n new  e on/off  d del".to_string(),
        };
        let list = List::new(items)
            .block(
                Block::bordered()
                    .title("Signals")
                    .title_bottom(hint)
                    .border_set(symbols::border::ROUNDED)
                    .padding(Padding::horizontal(1)),
            )
            .highlight_style(Style::new().bold())
            .highlight_symbol("> ");
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
                .padding(Padding::horizontal(1)),
        );
        frame.render_widget(table, area);
    }
}
