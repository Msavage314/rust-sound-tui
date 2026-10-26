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
    let a = Signal::sine(
        Bin {
            frequency: 440.0,
            phase: 0.0,
            amplitude: 0.1,
        },
        44100,
        1.0,
    );
    let b = Signal::sine(
        Bin {
            frequency: 880.0,
            phase: 0.0,
            amplitude: 0.1,
        },
        44100,
        1.0,
    );
    let a_track = Track {
        name: "A".to_string(),
        signal: a,
        enabled: true,
    };
    let b_track = Track {
        name: "B".to_string(),
        signal: b,
        enabled: true,
    };
    //let new = Signal::from_frequencies(&fft, signal.sample_rate, signal.duration());
    let mut app = App::new(vec![a_track, b_track])?;

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}

// TODO
// - Add new with top bar slider and adjustable frequency spacing allowing you to see the composition of frequencies through time
// - Vastly speed up frequency addition
