use std::path::PathBuf;

use oxc::{allocator::Allocator, parser::Parser as OxcParser, span::SourceType};
use ratatui::widgets::{ListState, ScrollbarState};

pub enum AppEvent {
    Quit,
    // Navigation
    Down,
    Up,
    Right,
    Left,
}

#[derive(Debug)]
pub struct AppModel {
    /// List of file paths to display
    pub file_paths: Vec<PathBuf>,
    /// State for the list of files
    pub list_state: ListState,

    /// Contents of the currently selected file
    pub file_contents: Option<String>,
    pub scroll_state: ScrollbarState,

    pub is_list_focus: bool,
}

impl AppModel {
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        AppModel {
            file_paths,
            list_state: ListState::default(),
            file_contents: None,
            scroll_state: ScrollbarState::default(),
            is_list_focus: true,
        }
    }

    pub async fn handle_event(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::Down => {
                if self.is_list_focus {
                    self.list_state.select_next();
                    self.load_file_contents().await;
                } else {
                    self.scroll_state.next();
                }
            }
            AppEvent::Up => {
                if self.is_list_focus {
                    self.list_state.select_previous();
                    self.load_file_contents().await;
                } else {
                    self.scroll_state.prev();
                }
            }
            AppEvent::Right => {
                self.is_list_focus = false;
            }
            AppEvent::Left => {
                self.is_list_focus = true;
            }
            AppEvent::Quit => {}
        }
    }

    async fn load_file_contents(&mut self) {
        let Some(idx) = self.list_state.selected() else {
            self.scroll_state = ScrollbarState::default();
            self.file_contents = Some("No file selected".to_string());
            return;
        };
        let Some(file_path) = self.file_paths.get(idx) else {
            self.scroll_state = ScrollbarState::default();
            self.file_contents = Some("Invalid file index".to_string());
            return;
        };
        let Ok(contents) = tokio::fs::read_to_string(file_path).await else {
            self.scroll_state = ScrollbarState::default();
            self.file_contents = Some(format!("Error reading file: {file_path:?}"));
            return;
        };
        let Ok(source_type) = SourceType::from_path(file_path) else {
            self.scroll_state = ScrollbarState::default();
            self.file_contents = Some(format!("Error determining source type for: {file_path:?}"));
            return;
        };

        // Parse the file contents using oxc
        let allocator = Allocator::default();
        let parser = OxcParser::new(&allocator, &contents, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            self.scroll_state = ScrollbarState::default();
            self.file_contents = Some(format!(
                "Errors in file {:?}:\n{}",
                file_path,
                parse_result
                    .errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            return;
        }

        let contents = format!("{:#?}", parse_result.program);
        self.scroll_state = ScrollbarState::new(contents.lines().count());
        self.file_contents = Some(contents);
    }
}
