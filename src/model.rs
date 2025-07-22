use std::path::PathBuf;

use ratatui::widgets::{ListState, ScrollbarState};

use crate::parser::{ParseRequest, ParseResult};

pub enum NaviEvent {
    Down,
    Up,
    Right,
    Left,
}

#[derive(Debug)]
pub struct AppModel {
    /// List of file paths to display
    pub file_paths: Vec<PathBuf>,
    /// Channel to send parse requests
    pub parse_request_tx: Option<tokio::sync::mpsc::UnboundedSender<ParseRequest>>,

    /// State for the list of files
    pub list_state: ListState,

    /// Contents of the currently selected file
    pub file_contents: Option<String>,
    pub scroll_state: ScrollbarState,

    /// List or contents is currently focused
    pub is_list_focus: bool,
}

impl AppModel {
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        AppModel {
            file_paths,
            parse_request_tx: None,
            list_state: ListState::default(),
            file_contents: None,
            scroll_state: ScrollbarState::default(),
            is_list_focus: true,
        }
    }

    #[must_use]
    pub fn set_parse_request_tx(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<ParseRequest>,
    ) -> Self {
        self.parse_request_tx = Some(tx);
        self
    }

    pub fn handle_navi_event(&mut self, ev: NaviEvent) {
        match ev {
            NaviEvent::Down => {
                if self.is_list_focus {
                    self.list_state.select_next();
                    self.request_parse_current_file();
                } else {
                    self.scroll_state.next();
                }
            }
            NaviEvent::Up => {
                if self.is_list_focus {
                    self.list_state.select_previous();
                    self.request_parse_current_file();
                } else {
                    self.scroll_state.prev();
                }
            }
            NaviEvent::Right => {
                self.is_list_focus = false;
            }
            NaviEvent::Left => {
                self.is_list_focus = true;
            }
        }
    }

    pub fn handle_parse_result(&mut self, result: ParseResult) {
        match result {
            ParseResult::Success(content) => {
                self.scroll_state = ScrollbarState::new(content.lines().count());
                self.file_contents = Some(content);
            }
            ParseResult::Error(error) => {
                self.scroll_state = ScrollbarState::default();
                self.file_contents = Some(error);
            }
        }
    }

    fn request_parse_current_file(&self) {
        let Some(idx) = self.list_state.selected() else {
            return;
        };
        let Some(file_path) = self.file_paths.get(idx) else {
            return;
        };
        let Some(tx) = &self.parse_request_tx else {
            return;
        };

        let _ = tx.send(ParseRequest::ParseFile {
            file_path: file_path.clone(),
        });
    }
}
