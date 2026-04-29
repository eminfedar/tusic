use crate::audio::backend::AudioBackend;
use crate::audio::decoder::get_audio_info;
use anyhow::Result;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::File;
use std::path::Path;

pub struct RodioBackend {
    _sink: MixerDeviceSink,
    player: Option<Player>,
    volume: i8,
    position_ms: u64,
    duration_ms: u64,
}

impl RodioBackend {
    pub fn new() -> Result<Self> {
        let mut sink = DeviceSinkBuilder::open_default_sink()?;
        sink.log_on_drop(false);

        Ok(Self {
            _sink: sink,
            player: None,
            volume: 80,
            position_ms: 0,
            duration_ms: 0,
        })
    }
}

impl AudioBackend for RodioBackend {
    fn load_track(&mut self, path: &Path) -> anyhow::Result<()> {
        self.stop();

        let audio_info = get_audio_info(path)?;

        self.duration_ms = audio_info.duration_ms;
        self.position_ms = 0;

        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let decoder = Decoder::builder()
            .with_coarse_seek(true)
            .with_seekable(true)
            .with_byte_len(len)
            .with_data(file)
            .build()?;

        let player = Player::connect_new(self._sink.mixer());

        let vol = self.volume as f32 / 100.0;
        player.set_volume(vol);
        player.append(decoder);

        self.player = Some(player);

        Ok(())
    }

    fn play(&mut self) -> anyhow::Result<()> {
        if let Some(ref player) = self.player {
            player.play();
        }
        Ok(())
    }

    fn pause(&mut self) {
        if let Some(ref player) = self.player {
            player.pause();
            self.position_ms = player.get_pos().as_millis() as u64;
        }
    }

    fn stop(&mut self) {
        if let Some(ref player) = self.player {
            player.stop();
        }
        self.position_ms = 0;
    }

    fn is_playing(&self) -> bool {
        if let Some(ref player) = self.player {
            return !player.is_paused();
        }

        false
    }

    fn get_position(&self) -> u64 {
        if let Some(ref player) = self.player {
            let pos = player.get_pos().as_millis() as u64;
            if pos > 0 {
                return pos;
            }
            if player.empty() {
                return self.duration_ms;
            }
        }

        self.position_ms
    }

    fn get_duration(&self) -> u64 {
        self.duration_ms
    }

    fn set_volume(&mut self, volume: i8) {
        self.volume = volume.clamp(0, 100);

        let float_volume: f32 = self.volume as f32 / 100.0;

        if let Some(ref player) = self.player {
            player.set_volume(float_volume);
        }
    }

    fn seek_to(&mut self, position_ms: u64) -> anyhow::Result<()> {
        if let Some(ref player) = self.player {
            self.position_ms = position_ms;

            let duration = std::time::Duration::from_millis(position_ms);
            let _ = player.try_seek(duration);
        }
        Ok(())
    }
}
