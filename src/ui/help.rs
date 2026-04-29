use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame, area: Rect) {
    if area.height < 1 {
        return;
    }

    let help_text = Line::from(vec![
        Span::styled("[space] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Play/Pause  "),
        Span::styled("[←/→] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("-/+5s  "),
        Span::styled("[ctrl+←/→] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Prev/Next  "),
        Span::styled("[r] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Loop  "),
        Span::styled("[s] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Shuffle  "),
        Span::styled("[tab] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Switch panel  "),
        Span::styled("[y] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Toggle YouTube Panel  "),
        Span::styled("[q] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Quit"),
    ]);

    let p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(p, area);
}
