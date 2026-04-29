use crate::model::{ActivePanel, Model, PlaybackStatus};
use crate::playlist::RepeatMode;
use ratatui::style::Modifier;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge},
    Frame,
};

pub fn render_player(f: &mut Frame, area: Rect, model: &Model) {
    let is_active = matches!(model.ui.active_panel, ActivePanel::Playlist);
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let playback = &model.playback;

    let status_text = match playback.status {
        PlaybackStatus::Playing => "▸ Playing",
        PlaybackStatus::Paused => "⏸ Paused",
        PlaybackStatus::Stopped => "⏹ Stopped",
    };

    let shuffle_text = match model.shuffle {
        true => "Shuffle:On",
        false => "Shuffle:Off",
    };

    let loop_text = match model.repeat {
        RepeatMode::All => "Loop:All",
        RepeatMode::One => "Loop:One",
        RepeatMode::None => "Loop:Off",
    };

    // Title
    let track = model.current_track();
    let title = if let Some(track) = track {
        Span::styled(
            format!(" {status_text} - {} ", &track.title),
            Style::reset().bold(),
        )
    } else {
        Span::raw(" Player ")
    };

    let block = Block::default()
        .title(title)
        .title_bottom(format!(" {shuffle_text} - {loop_text} "))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Progress bar
    let gauge = render_progressbar(playback.position_ms, playback.duration_ms);
    f.render_widget(gauge, inner);
}

fn render_progressbar<'a>(position_ms: u64, duration_ms: u64) -> Gauge<'a> {
    let duration_str = format_duration(duration_ms);
    let position_str = format_duration(position_ms);

    let progress_text = format!(" {} / {} ", position_str, duration_str);

    let progress_ratio = if duration_ms > 0 {
        position_ms as f64 / duration_ms as f64
    } else {
        0.0
    };
    let progress_ratio = progress_ratio.clamp(0.0, 1.0);

    Gauge::default()
        .style(Modifier::BOLD)
        .ratio(progress_ratio)
        .label(progress_text)
        .gauge_style(Style::default().green())
        .use_unicode(true)
}

fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
