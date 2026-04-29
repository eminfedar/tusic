pub trait AudioBackend {
    fn play(&mut self) -> anyhow::Result<()>;
    fn pause(&mut self);
    fn stop(&mut self);
    fn is_playing(&self) -> bool;
    fn get_position(&self) -> u64;
    fn get_duration(&self) -> u64;
    fn set_volume(&mut self, volume: i8);
    fn seek_to(&mut self, position_ms: u64) -> anyhow::Result<()>;
    fn load_track(&mut self, path: &std::path::Path) -> anyhow::Result<()>;
}

pub mod rodio;
// pub mod symphonia;
