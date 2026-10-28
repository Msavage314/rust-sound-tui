pub mod app;
pub mod maths;
pub mod signal;
pub mod wav;

use crate::{
    app::{App, Track},
    maths::fourier::fft_signal,
    wav::{BinaryReader, read_wav},
};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BinaryReader::new(File::open(
        std::env::args().nth(1).ok_or("usage: viz <file.wav>")?,
    )?);
    let signal = read_wav(&mut reader)
        .expect("Failed to read wav file")
        .crop_seconds(0.0, 1.0);

    let _fft = fft_signal(&signal);

    //let new = Signal::from_frequencies(&fft, signal.sample_rate, signal.duration());
    let mut app = App::new(vec![Track::new("Piano C4".to_string(), signal)])?;

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}

// TODO
// - Add new with top bar slider and adjustable frequency spacing allowing you to see the composition of frequencies through time
// - Vastly speed up frequency addition
