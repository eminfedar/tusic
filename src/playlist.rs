use std::path::{Path, PathBuf};

use rand::Rng;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Waiting for rodio support of: "ogg", "webm", "wma", "opus", "flac"
    "mp3", "wav", "m4a", "aac",
];

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default)]
pub enum RepeatMode {
    #[default]
    None,
    All,
    One,
}

impl RepeatMode {
    pub fn next(&self) -> Self {
        match self {
            RepeatMode::None => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}

impl Track {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        Self {
            path,
            title,
            artist: String::new(),
            album: String::new(),
            duration_ms: 0,
        }
    }

    pub fn display_name(&self) -> String {
        let ext = self.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_suffix = if !ext.is_empty() {
            format!(".{}", ext)
        } else {
            String::new()
        };

        if self.artist.is_empty() {
            format!("{}{}", self.title, ext_suffix)
        } else {
            format!("{} - {}{}", self.artist, self.title, ext_suffix)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Playlist {
    tracks: Vec<Track>,
    original_indices: Vec<usize>,
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            original_indices: Vec::new(),
        }
    }

    pub fn from_tracks(tracks: Vec<Track>) -> Self {
        let original_indices: Vec<usize> = (0..tracks.len()).collect();
        Self {
            tracks,
            original_indices,
        }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /*
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }*/

    pub fn get(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /*
    pub fn tracks_mut(&mut self) -> &mut Vec<Track> {
        &mut self.tracks
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.original_indices.clear();
    }

    */

    pub fn push(&mut self, track: Track) -> usize {
        if let Some(idx) = self.tracks.iter().position(|t| t.path == track.path) {
            // Already exists, don't add
            return idx;
        }

        self.tracks.push(track);
        self.original_indices.push(self.tracks.len() - 1);

        self.tracks.len() - 1
    }

    pub fn next_index(
        &self,
        current: Option<usize>,
        repeat: RepeatMode,
        shuffle: bool,
    ) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        let random_idx: usize = rand::thread_rng().gen_range(0..len);

        match current {
            None => Some(0),
            Some(i) if repeat == RepeatMode::One => Some(i),
            Some(_) if shuffle => Some(random_idx),
            Some(i) if i + 1 < len => Some(i + 1),
            Some(_) if repeat == RepeatMode::All => Some(0),
            Some(_) => Some(0),
        }
    }

    pub fn prev_index(&self, current: Option<usize>, repeat: RepeatMode) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        match current {
            None => Some(0),
            Some(i) if i > 0 => Some(i - 1),
            Some(_) if repeat == RepeatMode::All => Some(len - 1),
            Some(_) => Some(0),
        }
    }
}

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
