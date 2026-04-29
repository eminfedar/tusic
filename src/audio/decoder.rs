use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecoderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Rodio error: {0}")]
    Rodio(String),
    #[error("No audio track found")]
    NoTrack,
}

pub struct AudioData {
    pub duration_ms: u64,
}

pub fn get_audio_info(path: &Path) -> Result<AudioData, DecoderError> {
    let file = File::open(path)?;

    let reader = BufReader::new(file);
    let decoder_result = Decoder::new(reader);

    let source = match decoder_result {
        Ok(s) => s,
        Err(e) => {
            return Err(DecoderError::Rodio(format!("{:?}", e)));
        }
    };

    let duration = source.total_duration().ok_or(DecoderError::NoTrack)?;

    let duration_ms = duration.as_millis() as u64;

    Ok(AudioData { duration_ms })
}
