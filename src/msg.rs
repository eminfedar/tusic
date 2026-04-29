use std::ffi::OsString;

use inotify::Event;

use crate::{playlist::Track, youtube::YoutubeTrack};

#[derive(Debug)]
pub enum Message {
    // Player controls
    Play,
    Pause,
    Next,
    Prev,

    SeekForward,
    SeekBackward,
    IncreaseVolume,
    DecreaseVolume,
    ToggleShuffle,
    CycleRepeat,

    // Playlist Controls
    ScrollUp,
    ScrollDown,
    ScrollUpHalf,
    ScrollDownHalf,
    ScrollTop,
    ScrollBottom,

    // Key press
    Enter,
    Escape,

    ToggleHelp,
    ToggleLogs,
    ToggleActivePanel,
    ToggleYoutube,

    // Youtube Search
    SearchInput(char),
    SearchBackspace,
    DoYoutubeSearch(String),
    YoutubeSearchResult(anyhow::Result<Vec<YoutubeTrack>>),
    YoutubeDownloadResult(anyhow::Result<Track>),
    DownloadYoutube(usize),

    LogScrollUp,
    LogScrollDown,
    LogScrollTop,
    LogScrollBottom,

    // Watch file changes:
    FileChanged(Event<OsString>),

    Tick,
    None,
    Quit,
}
