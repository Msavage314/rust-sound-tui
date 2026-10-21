use core::{f64::consts::PI, fmt::write, num::NonZero};
use std::{
    collections::HashMap,
    fs::{File, read},
    io::{
        self, BufReader, Error, SeekFrom,
        prelude::{Read, Seek},
    },
};

use ratatui::{
    crossterm::event::{
        self,
        Event::{self, Key},
        KeyEvent, KeyEventKind,
    },
    prelude::{
        Alignment, Color, Constraint, Frame, Layout, Rect, Span, Style, Stylize,
        symbols::{
            self,
            Marker::{self},
        },
    },
    widgets::{
        Axis, Block, Borders, Chart, Dataset,
        GraphType::{self, Line},
        Padding, Paragraph, Row, Table,
    },
};
use rodio::{Player, buffer::SamplesBuffer};

struct BinaryReader<R> {
    inner: R,
}
impl<R: Read + Seek> BinaryReader<R> {
    pub fn new(inner: R) -> Self {
        return Self { inner };
    }

    pub fn read_u8_le(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        return Ok(buf[0]);
    }
    pub fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        return Ok(u16::from_le_bytes(buf));
    }
    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        return Ok(u32::from_le_bytes(buf));
    }
    pub fn read_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.inner.read_exact(&mut buf)?;
        return Ok(buf);
    }
    pub fn read_chars(&mut self, n: usize) -> io::Result<String> {
        let bytes: Vec<u8> = self.read_bytes(n)?;
        let string = String::from_utf8(Vec::from(bytes)).expect("Unparseable string");
        return Ok(string);
    }
    pub fn position(&mut self) -> io::Result<u64> {
        return self.inner.stream_position();
    }
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }

    pub fn skip(&mut self, bytes: u64) -> io::Result<u64> {
        let offset =
            i64::try_from(bytes).map_err(|_| io::Error::other("skip distance too large"))?;

        self.inner.seek(SeekFrom::Current(offset))
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
    pub fn is_at_end(&mut self) -> io::Result<bool> {
        let current = self.inner.stream_position()?;
        let end = self.inner.seek(SeekFrom::End(0))?;

        self.inner.seek(SeekFrom::Start(current))?;

        Ok(current >= end)
    }
}
#[derive(Debug, Clone)]
struct Signal {
    samples: Vec<f64>,
    sample_rate: u32,
}
impl Signal {
    fn crop(&self, start: usize, end: usize) -> Self {
        assert!(start <= end, "start must be <= end");
        assert!(end <= self.samples.len(), "crop range out of bounds");

        Self {
            samples: self.samples[start..end].to_vec(),
            sample_rate: self.sample_rate,
        }
    }
    fn crop_seconds(&self, start: f64, end: f64) -> Self {
        let start_idx = (start * self.sample_rate as f64) as usize;
        let end_idx = (end * self.sample_rate as f64) as usize;

        let start_idx = start_idx.min(self.samples.len());
        let end_idx = end_idx.min(self.samples.len());

        let start_idx = start_idx.min(end_idx);

        Self {
            samples: self.samples[start_idx..end_idx].to_vec(),
            sample_rate: self.sample_rate,
        }
    }
}
fn parse_pcm(bytes: &[u8], bit_depth: u16) -> f64 {
    match bit_depth {
        // 8-bit PCM is unsigned
        8 => (bytes[0] as f64 - 128.0) / 128.0,
        // 16 bit PCM is signed
        16 => {
            let value = i16::from_le_bytes([bytes[0], bytes[1]]);
            value as f64 / 32768.0
        }
        // 24 bit is annoying
        24 => {
            let value = (bytes[0] as i32) | ((bytes[1] as i32) << 8) | ((bytes[2] as i32) << 16);

            let signed = if value & 0x800000 != 0 {
                value | !0xFFFFFF
            } else {
                value
            };

            signed as f64 / 8388608.0
        }
        // 32-bit PCM is signed
        32 => {
            let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            value as f64 / 2147483648.0
        }

        _ => panic!("Unsupported PCM bit depth: {bit_depth}"),
    }
}
struct FormatCode {}
impl FormatCode {
    const WAVE_FORMAT_EXETENSIBLE: u16 = 0xFFFE;
    const WAVE_FORMAT_PCM: u16 = 0x0001;
    const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
    const WAVE_FORMAT_ALAW: u16 = 0x0006;
    const WAVE_FORMAT_MULAW: u16 = 0x0007;
}

/// https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html
fn read_wav<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
) -> Result<Signal, Box<dyn std::error::Error>> {
    // Different wav file formats.
    let riff_id = reader.read_chars(4)?;
    let file_size = reader.read_chars(4)?;
    let wave_id = reader.read_chars(4)?;
    // to verify the integritry of wav files, all wav files begin with RIFF and WAVE
    assert!(riff_id == "RIFF" && wave_id == "WAVE", "Invalid .wav file");
    // keep a hashmap of different chunks.
    // The chunks we want are fmt (containing metadata) and data (containing the actual data)
    let mut chunk_lookup: HashMap<String, u64> = HashMap::new();
    while !(reader.is_at_end()?) {
        let chunk_offset = reader.position()?;
        let chunk_id = reader.read_chars(4)?;
        chunk_lookup.insert(chunk_id, chunk_offset);

        let chunk_size = reader.read_u32_le()? as u64;
        reader.skip(chunk_size)?;
    }
    println!("{:?}", chunk_lookup);

    reader.seek(SeekFrom::Start(chunk_lookup["fmt "] + 8))?;
    let format_code = reader.read_u16_le()?;
    let channels = reader.read_u16_le()?;

    let sample_rate = reader.read_u32_le()?;
    // skip unwanted fields
    reader.skip(6)?;
    let bit_depth = reader.read_u16_le()?;
    let bytes_per_sample = bit_depth / 8;
    println!("{}", format_code);
    if format_code == FormatCode::WAVE_FORMAT_EXETENSIBLE {
        reader.skip(8)?;
        let format_code = reader.read_u16_le()?;
    }

    // +4 to skip the first 4 bytes which are just chunk id
    reader.seek(SeekFrom::Start(chunk_lookup["data"] + 4))?;
    let num_bytes_data = reader.read_u32_le()?;
    let data = reader.read_bytes(num_bytes_data as usize)?;

    // We can have multiple channels e.g. mono vs stero vs surround sound.
    let bytes_per_frame = bytes_per_sample as usize * channels as usize;
    let frame_count = data.len() / bytes_per_frame;

    let mut samples = Vec::with_capacity(frame_count);
    for frame in data.chunks_exact(bytes_per_frame) {
        let mut sum = 0.0;

        for channel in 0..channels as usize {
            let offset = channel * bytes_per_sample as usize;
            let sample = &frame[offset..offset + bytes_per_sample as usize];
            sum += parse_pcm(sample, bit_depth);
        }
        samples.push(sum / channels as f64);
    }

    return Ok(Signal {
        samples,
        sample_rate,
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BinaryReader::new(File::open("./Beethoven - Für Elise.wav")?);
    let signal = read_wav(&mut reader)?;
    let new = signal.crop_seconds(0.0, 5.0);
    let new = generate_signal(440.0, 44100, 1.0);

    let sink_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
    let mut player: Option<rodio::Player> = None;

    ratatui::run(|terminal| {
        'mainloop: loop {
            let position = player
                .as_ref()
                .map(|p| p.get_pos())
                .unwrap_or(std::time::Duration::ZERO);
            let current_seconds = position.as_secs_f64();

            terminal.draw(|frame| render(frame, &new, current_seconds))?;
            if event::poll(std::time::Duration::from_millis(5))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        event::KeyCode::Char('q') => break 'mainloop Ok::<(), Error>(()),
                        event::KeyCode::Char(' ') => {
                            if let Some(sink) = &player {
                                if !sink.empty() {
                                    if sink.is_paused() {
                                        sink.play();
                                    } else {
                                        sink.pause();
                                    }
                                    continue;
                                }
                            }

                            let samples: Vec<f32> =
                                new.samples.iter().map(|&sample| sample as f32).collect();

                            let source = SamplesBuffer::new(
                                NonZero::new(1).unwrap(),
                                NonZero::new(new.sample_rate).unwrap(),
                                samples,
                            );

                            let sink = Player::connect_new(sink_handle.mixer());
                            sink.append(source);
                            player = Some(sink);
                        }
                        event::KeyCode::Char('s') => {
                            if let Some(sink) = player.take() {
                                sink.stop();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    })?;
    Ok(())
}
fn render(frame: &mut Frame, signal: &Signal, current_seconds: f64) {
    let layout = Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).spacing(1);
    let [top, main] = frame.area().layout(&layout);

    let sample_count = signal.samples.len();
    let duration = sample_count as f64 / signal.sample_rate as f64;

    let table = Table::new(
        [
            Row::new([
                "Sample rate".to_string(),
                format!("{} Hz", signal.sample_rate),
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
    frame.render_widget(table, top);
    render_chart(frame, main, signal, current_seconds);
}
fn render_chart(frame: &mut Frame, area: Rect, signal: &Signal, current_seconds: f64) {
    let sample_count = signal.samples.len();

    // Approximate the number of horizontal points the terminal can display.
    let buckets = (area.width as usize)
        .saturating_sub(10)
        .max(1)
        .min(sample_count);

    let bucket_size = sample_count.div_ceil(buckets);
    let mut data = Vec::with_capacity(buckets * 2);

    for start in (0..sample_count).step_by(bucket_size) {
        let end = (start + bucket_size).min(sample_count);
        let samples = &signal.samples[start..end];

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
            data.push(((start + min_offset) as f64, min_value));
            data.push(((start + max_offset) as f64, max_value));
        } else if max_offset < min_offset {
            data.push(((start + max_offset) as f64, max_value));
            data.push(((start + min_offset) as f64, min_value));
        } else {
            data.push(((start + min_offset) as f64, min_value));
        }
    }

    let waveform = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Area)
        .data(&data)
        .green();
    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(symbols::border::ROUNDED)
        .dark_gray()
        .padding(Padding::proportional(2))
        .title("Waveform");
    let x = (current_seconds * signal.sample_rate as f64)
        .clamp(0.0, sample_count.saturating_sub(1) as f64);

    let progress = vec![(x, -0.5), (x, 0.5)];
    let line = Dataset::default()
        .marker(Marker::Custom('|'))
        .graph_type(Line)
        .data(&progress)
        .white();
    let chart = Chart::new(vec![waveform, line])
        .x_axis(
            Axis::default()
                .title("Sample")
                .bounds([0.0, sample_count.saturating_sub(1) as f64])
                .green()
                .labels([
                    "0".to_string(),
                    (sample_count.saturating_sub(1) as f64).to_string(),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Amplitude")
                .bounds([-1.0, 1.0])
                .labels(["-0.5", "0", "0.5"])
                .green(),
        )
        .block(block);
    frame.render_widget(chart, area);
}

fn generate_signal(frequency: f64, sample_rate: u32, duration: f64) -> Signal {
    let num_samples = (sample_rate as f64 * duration) as usize;

    let samples = (0..num_samples)
        .map(|i| {
            let time = i as f64 / sample_rate as f64;
            (time * 2.0 * PI * frequency).cos() * 0.5
        })
        .collect();

    Signal {
        samples,
        sample_rate,
    }
}
