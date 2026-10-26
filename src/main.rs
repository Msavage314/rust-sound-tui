pub mod app;
pub mod maths;
pub mod signal;
pub mod wav;

use rayon::vec;

use crate::{
    app::{App, Track},
    maths::fourier::{Bin, fft_signal},
    signal::Signal,
    wav::{BinaryReader, read_wav},
};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BinaryReader::new(File::open(
        std::env::args().nth(1).ok_or("usage: viz <file.wav>")?,
    )?);
    let signal = read_wav(&mut reader)
        .expect("Failed to read wav file")
        .crop_seconds(0.0, 4.0);

    let _fft = fft_signal(&signal);
    let a = Signal::sine(Bin::new(440.0, 0.0, 0.5), 44100, 1.0);
    let b = Signal::sine(Bin::new(880.0, 0.0, 0.5), 44100, 1.0);
    let a_track = Track::new("A".to_string(), a);
    let b_track = Track::new("B".to_string(), b);
    //let new = Signal::from_frequencies(&fft, signal.sample_rate, signal.duration());
    let mut app = App::new(vec![a_track, b_track])?;

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}

// TODO
// - Add new with top bar slider and adjustable frequency spacing allowing you to see the composition of frequencies through time
// - Vastly speed up frequency addition
