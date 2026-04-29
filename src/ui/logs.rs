use crate::model::Model;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Scrollbar, ScrollbarOrientation},
    Frame,
};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(Default)]
pub struct LogsState {
    pub list_state: ListState,
}

pub fn render_logs(f: &mut Frame, area: Rect, model: &Model, state: &mut LogsState) {
    if area.height < 3 {
        return;
    }

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Logs (press L to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .on_black();

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let messages = &model.ui.log_messages;

    if messages.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No logs")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default());
        f.render_widget(empty, inner);
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    let list = ListView::new(
        ListBuilder::new(move |context| {
            let idx = context.index;
            let msg = &messages[messages.len() - 1 - idx];
            let line = Line::from(msg.as_str());
            (line, 1)
        }),
        messages.len(),
    )
    .block(Block::default())
    .scrollbar(scrollbar)
    .infinite_scrolling(true);

    state.list_state.select(Some(model.ui.log_selected));

    f.render_stateful_widget(list, inner, &mut state.list_state);
}
