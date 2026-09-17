//! Ogg Vorbis q4 encode/decode for SFX goldens.
//!
//! Pico-8 still captures WAV. Expected and test-actual files on disk are
//! `.ogg`. Both sides of a compare go through this encoder so codec error
//! is symmetric. Used by the sfx harness and `examples/wav-to-ogg.rs`.
//!
//! Encode uses `vorbis_rs` (libvorbisenc). `rusty_vorbis` is stereo-only.

use lewton::inside_ogg::OggStreamReader;
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::num::{NonZeroU32, NonZeroU8};
use std::path::Path;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

pub const SAMPLE_RATE: u32 = 22_050;
/// Vorbis `-q` (pinned so ingest and tests match). libvorbis quality is q/10.
pub const VORBIS_Q: f32 = 4.0;
/// Samples with |s| <= this count as Pico-8 EXPORT / capture lead-in.
pub const SILENCE_THRESHOLD: i16 = 64;
const STREAM_SERIAL: i32 = 1;
const ENCODE_BLOCK: usize = 1024;

pub fn load_wav(path: &Path) -> Vec<i16> {
    let mut file = File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    decode_wav(&bytes, path)
}

fn decode_wav(bytes: &[u8], path: &Path) -> Vec<i16> {
    assert!(
        bytes.len() >= 44 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "{}: not a WAVE file",
        path.display()
    );
    let channels = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
    let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let bits = u16::from_le_bytes(bytes[34..36].try_into().unwrap());
    assert_eq!(
        channels,
        1,
        "{}: expected mono, got {channels}",
        path.display()
    );
    assert_eq!(
        rate,
        SAMPLE_RATE,
        "{}: expected {SAMPLE_RATE} Hz, got {rate}",
        path.display()
    );
    assert_eq!(bits, 16, "{}: expected 16-bit, got {bits}", path.display());
    let Some(data_at) = bytes.windows(4).position(|w| w == b"data") else {
        panic!("{}: missing data chunk", path.display());
    };
    let size = u32::from_le_bytes(bytes[data_at + 4..data_at + 8].try_into().unwrap()) as usize;
    let start = data_at + 8;
    let pcm = &bytes[start..start + size.min(bytes.len().saturating_sub(start))];
    pcm.chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect()
}

/// Drop Pico-8's silent EXPORT / audio_rec lead-in so it is not smeared by Vorbis.
pub fn strip_leading_silence(samples: &[i16]) -> &[i16] {
    let start = samples
        .iter()
        .position(|s| s.abs() > SILENCE_THRESHOLD)
        .unwrap_or(0);
    &samples[start..]
}

pub fn encode_ogg(pcm: &[i16]) -> Result<Vec<u8>, String> {
    let samples: Vec<f32> = pcm.iter().map(|s| *s as f32 / 32768.0).collect();
    let sink = Cursor::new(Vec::new());
    let mut encoder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(SAMPLE_RATE).unwrap(),
        NonZeroU8::new(1).unwrap(),
        sink,
        STREAM_SERIAL,
    )
    .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: VORBIS_Q / 10.0,
    })
    .build()
    .map_err(|e| format!("vorbis encode: {e}"))?;

    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + ENCODE_BLOCK).min(samples.len());
        encoder
            .encode_audio_block([&samples[pos..end]])
            .map_err(|e| format!("vorbis block: {e}"))?;
        pos = end;
    }
    let cursor = encoder
        .finish()
        .map_err(|e| format!("vorbis finish: {e}"))?;
    Ok(cursor.into_inner())
}

pub fn write_ogg(path: &Path, pcm: &[i16]) -> Result<(), String> {
    let bytes = encode_ogg(pcm)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

#[allow(dead_code)] // used by the sfx harness; the wav-to-ogg example only encodes
pub fn load_ogg(path: &Path) -> Vec<i16> {
    let file = File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut reader = OggStreamReader::new(BufReader::new(file))
        .unwrap_or_else(|e| panic!("ogg {}: {e}", path.display()));
    let ch = reader.ident_hdr.audio_channels;
    let rate = reader.ident_hdr.audio_sample_rate;
    assert_eq!(ch, 1, "{}: expected mono, got {ch}", path.display());
    assert_eq!(
        rate,
        SAMPLE_RATE,
        "{}: expected {SAMPLE_RATE} Hz, got {rate}",
        path.display()
    );
    let mut pcm = Vec::new();
    loop {
        match reader.read_dec_packet_itl() {
            Ok(Some(samples)) => pcm.extend(samples),
            Ok(None) => break,
            Err(e) => panic!("decode {}: {e}", path.display()),
        }
    }
    pcm
}
