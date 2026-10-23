//! Contains stuff for processing wav files, decoding formats etc

use std::{
    collections::HashMap,
    io::{
        self, SeekFrom,
        prelude::{Read, Seek},
    },
};

use crate::signal::Signal;
pub struct BinaryReader<R> {
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
    const _WAVE_FORMAT_PCM: u16 = 0x0001;
    const _WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
    const _WAVE_FORMAT_ALAW: u16 = 0x0006;
    const _WAVE_FORMAT_MULAW: u16 = 0x0007;
}

/// https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html
pub fn read_wav<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
) -> Result<Signal, Box<dyn std::error::Error>> {
    // Different wav file formats.
    let riff_id = reader.read_chars(4)?;
    let _file_size = reader.read_chars(4)?;
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
        let _format_code = reader.read_u16_le()?;
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
