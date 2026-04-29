# tusic

Lightweight TUI Music Player written in Rust + [ratatui](https://ratatui.rs/).

Download & Play songs from YouTube or ~/Music folder with one key press.

![Preview](export/preview.gif)

Uses [yt-dlp](https://docs.rs/yt-dlp/latest/yt_dlp/) to download audio from YouTube.

It automatically downloads the yt-dlp binary to ~/.config/tusic on first run, please wait for the download until it finishes.

### Why yt-dlp?

[rusty_ytdl](https://github.com/Mithronn/rusty_ytdl/issues) provides full Rust implementation but it doesn't work well on downloading.

This app also uses rusty_ytdl for searching videos *(as opposed to yt-dlp, it is good at searching :))*

### Roadmap

Feel free to open issue about the things you want