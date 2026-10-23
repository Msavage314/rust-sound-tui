//! Minimal WAV (RIFF/WAVE) decoding
//!
//! This module provides two things:
//!
//! - [`BinaryReader`] - a small wrapper over any `Read + Seek` source that reads little-endian
//!    integers, raw bytes and fixed-length ASCII strings
//! - [`read_wav`] - which uses the above to parse a `.wav` file into a mono [`Signal`] of `f64` samples
//!
//! Supports 8, 16, 24 and 32 bit sample widths and multi-channel audio is averages
//!
//!
//! <div class="warning">
//! https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html
//! </div>

use std::{
    collections::HashMap,
    io::{
        self, SeekFrom,
        prelude::{Read, Seek},
    },
};

use crate::signal::Signal;

/// Wrapper around a seekable source for reading binary data.
///
/// WAV files store multi-byte integers in little-endian order, so every integer
/// reader on this type is little-endian. The wrapper works with anything that
/// implements [`Read`] and [`Seek`], such as a [`std::fs::File`] or a
/// [`std::io::Cursor`] over an in-memory buffer.
pub struct BinaryReader<R> {
    inner: R,
}
impl<R: Read + Seek> BinaryReader<R> {
    /// Creates a new reader around `inner`, which provides access to the
    /// underlying stream (e.g. a [`std::fs::File`]).
    ///
    /// The reader starts at whatever position `inner` is currently at.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use std::fs::File;
    ///
    /// let mut reader = BinaryReader::new(File::open("song.wav")?);
    /// let magic = reader.read_chars(4)?; // "RIFF" for a valid WAV file
    /// ```
    pub fn new(inner: R) -> Self {
        return Self { inner };
    }

    /// Reads a single byte and advances the stream by 1.
    ///
    /// The `_le` suffix is for consistency with the other integer readers;
    /// endianness is irrelevant for one byte.
    ///
    /// # Errors
    /// Returns an error if the stream ends before a byte can be read.
    pub fn read_u8_le(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        return Ok(buf[0]);
    }

    /// Reads a little-endian `u16` and advances the stream by 2 bytes.
    ///
    /// # Errors
    /// Returns an error if fewer than 2 bytes remain.
    pub fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        return Ok(u16::from_le_bytes(buf));
    }

    /// Reads a little-endian `u32` and advances the stream by 4 bytes.
    ///
    /// # Errors
    /// Returns an error if fewer than 4 bytes remain.
    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        return Ok(u32::from_le_bytes(buf));
    }

    /// Reads exactly `n` bytes into a freshly allocated `Vec<u8>`.
    ///
    /// # Errors
    /// Returns an error if fewer than `n` bytes remain.
    ///
    /// # Panics
    /// May panic or abort on allocation failure if `n` is huge. When `n` comes
    /// from a file header, make sure it is plausible first.
    pub fn read_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.inner.read_exact(&mut buf)?;
        return Ok(buf);
    }

    /// Reads `n` bytes and interprets them as a UTF-8 string.
    ///
    /// Intended for short ASCII tags such as the four-character chunk IDs in
    /// RIFF files (`"RIFF"`, `"WAVE"`, `"fmt "`, `"data"`).
    ///
    /// # Errors
    /// Returns an error if fewer than `n` bytes remain.
    ///
    /// # Panics
    /// Panics if the bytes are not valid UTF-8.
    pub fn read_chars(&mut self, n: usize) -> io::Result<String> {
        let bytes: Vec<u8> = self.read_bytes(n)?;
        let string = String::from_utf8(Vec::from(bytes)).expect("Unparseable string");
        return Ok(string);
    }

    /// Returns the current offset from the start of the stream, in bytes.
    ///
    /// Takes `&mut self` because querying the position of a generic
    /// [`Seek`] type requires mutable access.
    pub fn position(&mut self) -> io::Result<u64> {
        return self.inner.stream_position();
    }

    /// Moves the stream to `pos` and returns the new offset from the start.
    ///
    /// This is a thin wrapper over [`Seek::seek`].
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }

    /// Skips `bytes` bytes forward from the current position without reading
    /// them, and returns the new offset from the start of the stream.
    ///
    /// # Errors
    /// Returns an error if `bytes` does not fit in an `i64`, or if the
    /// underlying seek fails. Seeking past the end of the stream is allowed by
    /// most [`Seek`] implementations and is not treated as an error here.
    pub fn skip(&mut self, bytes: u64) -> io::Result<u64> {
        let offset =
            i64::try_from(bytes).map_err(|_| io::Error::other("skip distance too large"))?;

        self.inner.seek(SeekFrom::Current(offset))
    }
    /// Consumes the reader and returns the wrapped stream.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns `true` if the current position is at (or beyond) the end of the
    /// stream.
    ///
    /// The read position is left unchanged: it seeks to the end to find the
    /// stream length, then seeks back.
    pub fn is_at_end(&mut self) -> io::Result<bool> {
        let current = self.inner.stream_position()?;
        let end = self.inner.seek(SeekFrom::End(0))?;

        self.inner.seek(SeekFrom::Start(current))?;

        Ok(current >= end)
    }
}

/// Decodes one integer PCM sample from its little-endian bytes into an `f64`.
///
/// The result is normalised so that full-scale negative maps to `-1.0` and
/// full-scale positive approaches (but never reaches) `1.0`.
///
/// | `bit_depth` | Encoding        | Divisor      |
/// |-------------|-----------------|--------------|
/// | 8           | unsigned        | 128 (after subtracting the 128 midpoint) |
/// | 16          | signed          | 2^15         |
/// | 24          | signed          | 2^23         |
/// | 32          | signed          | 2^31         |
///
/// `bytes` must contain at least `bit_depth / 8` bytes; any extra are ignored.
///
/// # Panics
/// Panics if `bit_depth` is not 8, 16, 24 or 32, or if `bytes` is too short.
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

/// Namespace for the WAV `wFormatTag` values found in the `fmt ` chunk.
///
/// Only [`WAVE_FORMAT_EXETENSIBLE`](Self::WAVE_FORMAT_EXETENSIBLE) is currently
/// used by the decoder; the rest are listed for reference.
struct FormatCode {}
impl FormatCode {
    const WAVE_FORMAT_EXETENSIBLE: u16 = 0xFFFE;
    const _WAVE_FORMAT_PCM: u16 = 0x0001;
    const _WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
    const _WAVE_FORMAT_ALAW: u16 = 0x0006;
    const _WAVE_FORMAT_MULAW: u16 = 0x0007;
}

// Parses a WAV file from `reader` into a mono [`Signal`].
///
/// Format reference:
/// <https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html>
///
/// # How it works
///
/// 1. Checks the `RIFF`/`WAVE` header.
/// 2. Walks the file once, recording the offset of every chunk by its
///    four-character ID. We only need `fmt ` (sample rate, channel count, bit
///    depth) and `data` (the raw samples); all other chunks are skipped.
/// 3. Reads the format fields from `fmt `.
/// 4. Reads the `data` chunk and decodes it frame by frame. A *frame* holds one
///    sample per channel; the channels are averaged into a single mono sample.
///
/// The reader may start anywhere, but is expected to be positioned at the very
/// start of the file when called, because the header is read from the current
/// position.
///
/// # Returns
///
/// A [`Signal`] containing one `f64` per frame, normalised to roughly
/// `[-1.0, 1.0)`, along with the file's sample rate in Hz.
///
/// # Errors
///
/// Returns an error if any underlying read or seek fails, which includes a
/// truncated file.
///
/// # Panics
///
/// This function panics rather than returning an error when:
/// - the file does not start with `RIFF` and contain `WAVE` in the header;
/// - a chunk ID is not valid UTF-8;
/// - the file has no `fmt ` or no `data` chunk;
/// - the bit depth is not 8, 16, 24 or 32.
///
/// # Limitations
///
/// - The format code is read but not validated, so non-PCM data (e.g.
///   IEEE float or A-law) will be decoded as if it were integer PCM.
/// - For `WAVE_FORMAT_EXTENSIBLE` files, the extension block is skipped and the
///   sub-format is ignored.
/// - If a chunk ID appears more than once, the last occurrence wins.
/// - Odd-sized chunks are not treated as padded to an even length, as the RIFF
///   specification requires, so files with such chunks before `data` may be
///   misread.
/// - Prints debugging output (the chunk table and format code) to stdout.
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
