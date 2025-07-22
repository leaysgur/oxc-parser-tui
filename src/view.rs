use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation},
};

use crate::model::AppModel;

const FOCUS_COLOR: Color = Color::Rgb(100, 200, 255); // Soft blue
const ACCENT_COLOR: Color = Color::Rgb(255, 180, 100); // Warm orange
const TEXT_COLOR: Color = Color::Rgb(220, 220, 220); // Light gray
const BORDER_COLOR: Color = Color::Rgb(70, 80, 90); // Medium gray

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
        .constraints([Constraint::Length(30), Constraint::Fill(1)])
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
                    Style::new().fg(FOCUS_COLOR).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(BORDER_COLOR)
                })
                .title_style(if model.ui_is_list_focus {
                    Style::new().fg(FOCUS_COLOR).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(TEXT_COLOR)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(FOCUS_COLOR)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(files_list, side_area, &mut model.ui_list_state);

    // Main app area
    let content = model.file_contents.as_deref().unwrap_or(
        "Select a file to view its contents\n\n💡 Use ↑↓ to navigate files\n   TAB to switch focus",
    );

    let styled_content = if model.file_contents.is_some() {
        Text::from(content).style(Style::default().fg(TEXT_COLOR))
    } else {
        // Style placeholder text differently
        Text::from(content).style(
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::ITALIC),
        )
    };

    let file_content = Paragraph::new(styled_content)
        .block(
            Block::default()
                .title(if model.ui_is_list_focus {
                    "Content"
                } else {
                    "Content [TAB: switch focus | ←→↑↓: scroll(+SHIFT: jump)]"
                })
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(if model.ui_is_list_focus {
                    Style::new().fg(BORDER_COLOR)
                } else {
                    Style::new().fg(FOCUS_COLOR).add_modifier(Modifier::BOLD)
                })
                .title_style(if model.ui_is_list_focus {
                    Style::new().fg(TEXT_COLOR)
                } else {
                    Style::new().fg(FOCUS_COLOR).add_modifier(Modifier::BOLD)
                }),
        )
        .scroll((
            model.ui_vertical_scroll_state.get_position() as u16,
            model.ui_horizontal_scroll_state.get_position() as u16,
        ));
    f.render_widget(file_content, main_area);

    // Scroll bars
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(if model.ui_is_list_focus {
                BORDER_COLOR
            } else {
                FOCUS_COLOR
            })),
        main_area,
        &mut model.ui_vertical_scroll_state,
    );
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("◀"))
            .end_symbol(Some("▶"))
            .track_symbol(Some("─"))
            .thumb_symbol("█")
            .style(Style::default().fg(if model.ui_is_list_focus {
                BORDER_COLOR
            } else {
                FOCUS_COLOR
            })),
        main_area,
        &mut model.ui_horizontal_scroll_state,
    );
}
