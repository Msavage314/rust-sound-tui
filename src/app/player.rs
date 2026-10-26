//! Audio player

use core::{error::Error, num::NonZero, time::Duration};

use rodio::{Player, buffer::SamplesBuffer};

use crate::{app::PlaybackState, signal::Signal};
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
