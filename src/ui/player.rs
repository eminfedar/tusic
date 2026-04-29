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

    let block = Block::default()
        .title(" Now Playing ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 8 {
        return;
    }

    let track = model.current_track();
    let playback = &model.playback;
    let volume = model.volume;
    let repeat = &model.repeat;
    let shuffle = model.shuffle;

    let mut lines = Vec::new();

    if let Some(track) = track {
        lines.push(Line::from(vec![
            Span::raw("Title: "),
            Span::styled(&track.title, Style::default().bold()),
        ]));

        lines.push(Line::from(vec![
            Span::raw("Artist: "),
            Span::styled(&track.artist, Style::default()),
        ]));

        lines.push(Line::from(vec![
            Span::raw("Album: "),
            Span::styled(&track.album, Style::default()),
        ]));
    } else {
        lines.push(Line::from("No track selected"));
    }

    lines.push(Line::from(""));

    let duration_str = format_duration(playback.duration_ms);
    let position_str = format_duration(playback.position_ms);

    let progress_text = format!(" {} / {} ", position_str, duration_str);

    let progress_ratio = if playback.duration_ms > 0 {
        playback.position_ms as f64 / playback.duration_ms as f64
    } else {
        0.0
    };
    let progress_ratio = progress_ratio.clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .style(Modifier::BOLD)
        .ratio(progress_ratio)
        .label(progress_text)
        .gauge_style(Style::default().green().on_black())
        .use_unicode(true);

    let mut gauge_area = inner;
    gauge_area.y += lines.len() as u16;
    gauge_area.height = 1;
    f.render_widget(gauge, gauge_area);

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(render_controls(&playback.status, repeat, shuffle, volume));

    let status_text = match playback.status {
        PlaybackStatus::Playing => "▸ Playing",
        PlaybackStatus::Paused => "⏸ Paused",
        PlaybackStatus::Stopped => "⏹ Stopped",
    };

    lines.push(Line::from(vec![
        Span::raw("Status: "),
        Span::styled(status_text, Style::default().fg(Color::Green).bold()),
    ]));

    let p = Paragraph::new(lines)
        .style(Style::default())
        .block(Block::default());

    let mut render_area = inner;
    render_area.height = inner.height.saturating_sub(1);
    f.render_widget(p, render_area);
}

fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn render_controls(
    status: &PlaybackStatus,
    repeat: &RepeatMode,
    shuffle: bool,
    volume: i8,
) -> Line<'static> {
    let play_pause = match status {
        PlaybackStatus::Playing => "⏸ Pause(space)",
        _ => "▶ Play(space)",
    };

    let repeat_icon = match repeat {
        RepeatMode::None => "OFF",
        RepeatMode::All => "ALL",
        RepeatMode::One => "ON",
    };

    let repeat_text = format!(
        " | Vol: {:.0}%(-/+) | Loop(r):{} | Shuffle(s):{}",
        volume,
        repeat_icon,
        if shuffle { "ON" } else { "OFF" },
    );

    Line::from(vec![Span::raw(play_pause), Span::raw(repeat_text)])
}
