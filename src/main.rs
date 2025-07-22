// TODO: MISC: Signals based state management?

use std::{fs, path::PathBuf};

use clap::Parser as ClapParser;
use color_eyre::{Result, eyre::eyre};
use crossterm::event::KeyModifiers;
use ratatui::{
    Terminal,
    crossterm::event::{EventStream, KeyCode, KeyEvent},
};
use tokio_stream::StreamExt;

mod model;
mod parser;
mod view;

use crate::{
    model::{AppModel, NaviEvent},
    parser::{ParseRequest, parse_file},
    view::render,
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
    // Channel for one-time signal to exit: Controller -> Controller
    let (should_exit_tx, mut should_exit_rx) = tokio::sync::watch::channel(false);
    // Channel for navigation events: Controller -> Model
    let (navi_ev_tx, mut navi_ev_rx) = tokio::sync::mpsc::unbounded_channel();
    // Channel for parse request: Model -> Controller(Parser)
    let (parse_req_tx, mut parse_req_rx) = tokio::sync::mpsc::unbounded_channel();
    // Channel for parse result: Controller(Parser) -> Model
    let (parse_res_tx, mut parse_res_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut model = model.set_parse_request_tx(parse_req_tx);

    // Spawn keyboard event handler
    tokio::spawn(async move {
        let mut event_stream = EventStream::new();
        while let Some(Ok(ev)) = event_stream.next().await {
            if let Some(KeyEvent {
                modifiers, code, ..
            }) = ev.as_key_press_event()
            // Should check press for Windows
            {
                let with_shift = modifiers.contains(KeyModifiers::SHIFT);
                if let Some(navi_ev) = match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if should_exit_tx.send(true).is_err() {
                            break;
                        }
                        None
                    }
                    KeyCode::Down if with_shift => Some(NaviEvent::ShiftDown),
                    KeyCode::Up if with_shift => Some(NaviEvent::ShiftUp),
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
        while let Some(request) = parse_req_rx.recv().await {
            match request {
                ParseRequest::ParseFile { file_path } => {
                    let result = parse_file(&file_path).await;
                    if parse_res_tx.send(result).is_err() {
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
            Ok(_) = should_exit_rx.changed() => {
                if *should_exit_rx.borrow() {
                    break;
                }
            }
            Some(event) = navi_ev_rx.recv() => {
                model.handle_navi_event(event);
            }
            Some(parse_result) = parse_res_rx.recv() => {
                model.handle_parse_result(parse_result);
            }
        }
    }
}
