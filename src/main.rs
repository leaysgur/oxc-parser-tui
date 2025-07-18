// TODO: CORE: Usage of tokio
// TODO: CORE: load_file_contents.await is OK?
// TODO: UI: Invalid file index when calling next() at the end of the list => manual idx is needed
// TODO: UI: Scroll content by 10
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

use oxc_parser_cli::{AppEvent, AppModel};

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

async fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut model: AppModel) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut event_stream = EventStream::new();
        while let Some(Ok(ev)) = event_stream.next().await {
            if let Event::Key(key) = ev {
                if let Some(app_event) = match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => Some(AppEvent::Quit),
                    KeyCode::Down => Some(AppEvent::Down),
                    KeyCode::Up => Some(AppEvent::Up),
                    KeyCode::Right => Some(AppEvent::Right),
                    KeyCode::Left => Some(AppEvent::Left),
                    _ => None,
                } {
                    if tx.send(app_event).is_err() {
                        break;
                    }
                }
            }
        }
    });

    loop {
        let _ = terminal.draw(|f| render(f, &mut model));

        tokio::select! {
            Some(event) = rx.recv() => {
                if matches!(event, AppEvent::Quit) { return; }
                model.handle_event(event).await;
            }
        }
    }
}

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
