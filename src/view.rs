use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation},
};

use crate::model::AppModel;

// `model` is mutable just for `render_stateful_widget` to work.
// Basically this is stateless functional render function.
pub fn render(f: &mut ratatui::Frame, model: &mut AppModel) {
    // DEBUG: Add debug area if needed
    let app_area = if let Some(h) = Some(0) {
        let [debug_area, app_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(h), Constraint::Fill(1)])
            .areas(f.area());
        f.render_widget(Text::from(format!("{model:#?}")), debug_area);
        app_area
    } else {
        f.area()
    };

    // Root layout
    let [side_area, main_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(app_area);

    // Left side area
    let items: Vec<ListItem> = model
        .file_paths
        .iter()
        .map(|file_path| {
            let file_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown");
            let content = Line::from(Span::raw(file_name));
            ListItem::new(content)
        })
        .collect();
    let files_list = List::new(items)
        .block(
            Block::default()
                .title(if model.ui_is_list_focus {
                    "Files [TAB: switch focus]"
                } else {
                    "Files"
                })
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(if model.ui_is_list_focus {
                    Style::new().magenta()
                } else {
                    Style::new()
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::LightMagenta)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(files_list, side_area, &mut model.ui_list_state);

    // Main app area
    let content = model
        .file_contents
        .as_deref()
        .unwrap_or("Select a file to view its contents");
    let file_content = Paragraph::new(Text::from(content))
        .block(
            Block::default()
                .title(if model.ui_is_list_focus {
                    "Content"
                } else {
                    "Content [TAB: switch focus | ←→: scroll | Shift+←→: jump]"
                })
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(if model.ui_is_list_focus {
                    Style::new()
                } else {
                    Style::new().magenta()
                }),
        )
        .scroll((
            model.ui_vertical_scroll_state.get_position() as u16,
            model.ui_horizontal_scroll_state.get_position() as u16,
        ));
    f.render_widget(file_content, main_area);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        main_area,
        &mut model.ui_vertical_scroll_state,
    );
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("←"))
            .end_symbol(Some("→")),
        main_area,
        &mut model.ui_horizontal_scroll_state,
    );
}
