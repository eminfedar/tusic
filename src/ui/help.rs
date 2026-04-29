use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame, area: Rect) {
    if area.height < 2 {
        return;
    }

    let line1 = Line::from(vec![
        Span::styled("[space] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Play/Pause  "),
        Span::styled(" [-][+] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Volume  "),
        Span::styled("[←/→] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("-/+5s  "),
        Span::styled("[ctrl+←/→] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Prev/Next  "),
        Span::styled("[r] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Loop  "),
        Span::styled("[s] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Shuffle  "),
    ]);

    let line2 = Line::from(vec![
        Span::styled("[enter] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Select & Play  "),
        Span::styled("[tab] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Switch panel  "),
        Span::styled("[y] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Toggle YouTube Panel  "),
        Span::styled("[q] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Quit"),
    ]);

    let lines = vec![line1, line2];

    let p = Paragraph::new(lines)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(p, area);
}
