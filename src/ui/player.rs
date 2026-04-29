use crate::model::{ActivePanel, Model, PlaybackStatus};
use crate::playlist::RepeatMode;
use ratatui::style::Modifier;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
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
        PlaybackStatus::Playing => "▶ Playing",
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

    let block = Block::default()
        .title(format!(" Player {status_text} "))
        .title_bottom(format!(" {shuffle_text} - {loop_text} "))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let track = model.current_track();

    let mut lines = Vec::new();

    // Title
    if let Some(track) = track {
        lines.push(Line::from(Span::styled(
            &track.title,
            Style::default().bold(),
        )));
    } else {
        lines.push(Line::from("No track selected"));
    }

    // Progress bar
    let gauge = render_progressbar(playback.position_ms, playback.duration_ms);
    let mut gauge_area = inner;
    gauge_area.y += lines.len() as u16;
    gauge_area.height = 1;
    f.render_widget(gauge, gauge_area);

    let p = Paragraph::new(lines)
        .style(Style::default())
        .block(Block::default());

    let mut render_area = inner;
    render_area.height = inner.height.saturating_sub(1);
    f.render_widget(p, render_area);
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
