pub mod app;
pub mod fourier;
pub mod signal;
pub mod utils;
pub mod views;
pub mod wav;
use std::fs::File;

use crate::{
    app::App,
    signal::Signal,
    wav::{BinaryReader, read_wav},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BinaryReader::new(File::open(
        std::env::args().nth(1).ok_or("usage: viz <file.wav>")?,
    )?);
    let signal = read_wav(&mut reader)
        .expect("Failed to read wav file")
        .crop_seconds(0.0, 4.0);

    let mut app = App::new(signal)?;
    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
