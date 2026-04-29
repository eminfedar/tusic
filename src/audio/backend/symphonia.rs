use crate::audio::backend::AudioBackend;
use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, OutputCallbackInfo, StreamConfig};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

enum ThreadMessage {
    Play,
    Pause,
    Stop,
    Seek(u64),
}

pub struct SymphoniaBackend {
    device: Device,
    volume: i8,
    current_position_ms: u64,
    duration_ms: u64,
    is_playing: Arc<AtomicBool>,
    play_thread_sender: Option<Sender<ThreadMessage>>,
}

impl SymphoniaBackend {
    pub fn new() -> anyhow::Result<Self> {
        let device = match cpal::default_host().default_output_device() {
            Some(d) => d,
            None => return Err(anyhow!("cpal no default output device")),
        };

        Ok(Self {
            device,
            volume: 80,
            current_position_ms: 0,
            duration_ms: 0,
            is_playing: Arc::new(AtomicBool::new(false)),
            play_thread_sender: None,
        })
    }
}

impl AudioBackend for SymphoniaBackend {
    fn load_track(&mut self, path: &Path) -> anyhow::Result<()> {
        self.stop();

        let file = File::open(path).map_err(|_x| anyhow!("File couldn't opened"))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format_reader = probed.format;
        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(anyhow!("No supported audio tracks in file"))?;

        // Get codecs
        let codec_params = track.codec_params.clone();
        let mut decoder =
            symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

        let track_id = track.id;

        // Get music duration
        let n_frames = codec_params.n_frames;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let duration = n_frames.map(|n| n * 1000 / sample_rate as u64).unwrap_or(0);

        // Set start point to 0
        self.duration_ms = duration;
        self.current_position_ms = 0;

        let (tx, rx) = mpsc::channel::<ThreadMessage>();

        let stream_data = Arc::new(Mutex::new(Vec::new()));

        self.play_thread_sender = Some(tx);

        let config = self.device.default_output_config()?;

        let stream_config: StreamConfig = config.into();
        let is_playing_atomic = self.is_playing.clone();
        let _stream_data_arc = stream_data.clone();
        let stream = self.device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                let is_playing = is_playing_atomic.load(std::sync::atomic::Ordering::SeqCst);

                if !is_playing {
                    data.fill(0.0);
                }

                // TODO: FILL data WITH stream_data_arc Buffer to play sound!
            },
            |_err| {},
            None,
        )?;

        std::thread::spawn(move || {
            let mut play = false;
            let mut seek_ts = 0;

            loop {
                // Receive commands from the channel
                match rx.try_recv() {
                    Ok(m) => match m {
                        ThreadMessage::Play => play = true,
                        ThreadMessage::Pause => play = false,
                        ThreadMessage::Stop => play = false,
                        ThreadMessage::Seek(ts) => {
                            let seek_to = SeekTo::TimeStamp { ts, track_id };
                            seek_ts = match format_reader.seek(SeekMode::Accurate, seek_to) {
                                Ok(seeked_to) => seeked_to.required_ts,
                                Err(err) => {
                                    // Don't give-up on a seek error.
                                    eprintln!("seek error: {}", err);
                                    0
                                }
                            };
                        }
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => (),
                        TryRecvError::Disconnected => break,
                    },
                }

                // Wait for the play message if paused
                if !play {
                    match rx.recv() {
                        Ok(m) => match m {
                            ThreadMessage::Play => play = true,
                            _ => continue,
                        },
                        Err(_e) => break,
                    }
                }

                // Receive next packet
                let packet = match format_reader.next_packet() {
                    Ok(p) => p,
                    Err(_) => break,
                };

                if packet.track_id() != track_id {
                    continue;
                }

                // Read audio to buffer
                let audio_buffer = match decoder.decode(&packet) {
                    Ok(buf) => SampleBuffer::<f32>::new(buf.capacity() as u64, *buf.spec()),
                    Err(_) => continue,
                };

                let mut stream_data_guard = stream_data.lock().unwrap();
                stream_data_guard.copy_from_slice(audio_buffer.samples());

                // Should we seek for a pos?
                if seek_ts != 0 {}
            }
        });

        stream.play()?;

        Ok(())
    }

    fn play(&mut self) -> anyhow::Result<()> {
        match &self.play_thread_sender {
            Some(s) => s.send(ThreadMessage::Play).unwrap(),
            None => return Err(anyhow!("no audio to play")),
        }
        self.is_playing.store(true, Ordering::SeqCst);

        Ok(())
    }

    fn pause(&mut self) {
        if let Some(s) = &self.play_thread_sender { s.send(ThreadMessage::Pause).unwrap() }
    }

    fn stop(&mut self) {
        if let Some(s) = &self.play_thread_sender { s.send(ThreadMessage::Stop).unwrap() }
    }

    fn is_playing(&self) -> bool {
        self.is_playing.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn get_position(&self) -> u64 {
        self.current_position_ms
    }

    fn get_duration(&self) -> u64 {
        self.duration_ms
    }

    fn set_volume(&mut self, volume: i8) {
        self.volume = volume.clamp(0, 100);
    }

    fn seek_to(&mut self, position_ms: u64) -> anyhow::Result<()> {
        match &self.play_thread_sender {
            Some(s) => s.send(ThreadMessage::Seek(position_ms)).unwrap(),
            None => return Err(anyhow!("No audio to seek")),
        }

        self.current_position_ms = position_ms;

        Ok(())
    }
}
