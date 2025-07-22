// TODO: CORE: Usage of tokio is OK?
// TODO: CORE: Exit is not navi event...
// TODO: UI: Shift+scroll content by 10
// TODO: MISC: Signals based state management?

use std::{fs, path::PathBuf};

use clap::Parser as ClapParser;
use color_eyre::{Result, eyre::eyre};
use ratatui::{
    Terminal,
    crossterm::event::{Event, EventStream, KeyCode},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation},
};
use tokio_stream::StreamExt;

use oxc_parser_cli::{
    AppModel, NaviEvent,
    parser::{ParseRequest, parse_file},
};

#[derive(ClapParser)]
#[command(name = "oxc-parser-cli")]
#[command(about = "A TUI file viewer", long_about = None)]
struct Cli {
    dir_path: PathBuf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    // Parse CLI args
    let cli = Cli::parse();
    if !cli.dir_path.is_dir() {
        return Err(eyre!(
            "The provided path is not a directory: {}",
            cli.dir_path.display()
        ));
    }

    // Collect all file paths in the directory
    let mut file_paths = Vec::new();
    for entry in fs::read_dir(&cli.dir_path)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        file_paths.push(path);
    }
    file_paths.sort_unstable();

    // Run!
    ratatui::run(|terminal| {
        let model = AppModel::new(file_paths);
        // Use `block_on()` since `ratatui::run()` is sync
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(run(terminal, model));
    });

    Ok(())
}

async fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, model: AppModel) {
    let (navi_ev_tx, mut navi_ev_rx) = tokio::sync::mpsc::unbounded_channel();
    let (parse_result_tx, mut parse_result_rx) = tokio::sync::mpsc::unbounded_channel();
    let (parse_request_tx, mut parse_request_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut model = model.set_parse_request_tx(parse_request_tx);

    // Spawn keyboard event handler
    tokio::spawn(async move {
        let mut event_stream = EventStream::new();
        while let Some(Ok(ev)) = event_stream.next().await {
            if let Event::Key(key) = ev {
                if let Some(navi_ev) = match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => Some(NaviEvent::Quit),
                    KeyCode::Down => Some(NaviEvent::Down),
                    KeyCode::Up => Some(NaviEvent::Up),
                    KeyCode::Right => Some(NaviEvent::Right),
                    KeyCode::Left => Some(NaviEvent::Left),
                    _ => None,
                } {
                    if navi_ev_tx.send(navi_ev).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Spawn parser task handler
    tokio::spawn(async move {
        while let Some(request) = parse_request_rx.recv().await {
            match request {
                ParseRequest::ParseFile { file_path } => {
                    let result = parse_file(&file_path).await;
                    if parse_result_tx.send(result).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Main event loop, render + handle rx events
    loop {
        let _ = terminal.draw(|f| render(f, &mut model));

        tokio::select! {
            Some(event) = navi_ev_rx.recv() => {
                if matches!(event, NaviEvent::Quit) { return; }
                model.handle_navi_event(event);
            }
            Some(parse_result) = parse_result_rx.recv() => {
                model.handle_parse_result(parse_result);
            }
        }
    }
}

// `model` is mutable just for `render_stateful_widget` to work.
// Basically this is stateless functional render function.
fn render(f: &mut ratatui::Frame, model: &mut AppModel) {
    // Add debug area if needed
    let app_area = if let Some(h) = Some(40) {
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
                .title("Files")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(if model.is_list_focus {
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
    f.render_stateful_widget(files_list, side_area, &mut model.list_state);

    // Main app area
    let content = model
        .file_contents
        .as_deref()
        .unwrap_or("Select a file to view its contents");
    let file_content = Paragraph::new(Text::from(content))
        .block(
            Block::default()
                .title("Content")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(if model.is_list_focus {
                    Style::new()
                } else {
                    Style::new().magenta()
                }),
        )
        .scroll((model.scroll_state.get_position() as u16, 0));
    f.render_widget(file_content, main_area);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        main_area,
        &mut model.scroll_state,
    );
}
