use std::path::PathBuf;

use ratatui::widgets::{ListState, ScrollbarState};

use crate::parser::{ParseRequest, ParseResult};

pub enum NaviEvent {
    Tab,
    ShiftDown,
    ShiftUp,
    Down,
    Up,
    ShiftRight,
    ShiftLeft,
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
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,

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
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
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
            NaviEvent::Tab => self.is_list_focus = !self.is_list_focus,
            NaviEvent::ShiftDown | NaviEvent::Down => {
                if self.is_list_focus {
                    self.list_state.select_next();
                    self.request_parse_current_file();
                    return;
                }

                if matches!(ev, NaviEvent::ShiftDown) {
                    self.vertical_scroll_state.last();
                } else {
                    self.vertical_scroll_state.next();
                }
            }
            NaviEvent::ShiftUp | NaviEvent::Up => {
                if self.is_list_focus {
                    self.list_state.select_previous();
                    self.request_parse_current_file();
                    return;
                }
                if matches!(ev, NaviEvent::ShiftUp) {
                    self.vertical_scroll_state.first();
                } else {
                    self.vertical_scroll_state.prev();
                }
            }
            NaviEvent::ShiftRight | NaviEvent::Right if !self.is_list_focus => {
                if matches!(ev, NaviEvent::ShiftRight) {
                    self.horizontal_scroll_state.last();
                } else {
                    self.horizontal_scroll_state.next();
                }
            }
            NaviEvent::ShiftLeft | NaviEvent::Left if !self.is_list_focus => {
                if matches!(ev, NaviEvent::ShiftLeft) {
                    self.horizontal_scroll_state.first();
                } else {
                    self.horizontal_scroll_state.prev();
                }
            }
            _ => {}
        }
    }

    pub fn handle_parse_result(&mut self, result: ParseResult) {
        match result {
            ParseResult::Success(content) => {
                let max_line_height = content.lines().count();
                let max_line_width = content.lines().map(|line| line.len()).max().unwrap_or(0);
                self.vertical_scroll_state = ScrollbarState::new(max_line_height);
                self.horizontal_scroll_state = ScrollbarState::new(max_line_width);
                self.file_contents = Some(content);
            }
            ParseResult::Error(error) => {
                let max_line_height = error.lines().count();
                let max_line_width = error.lines().map(|line| line.len()).max().unwrap_or(0);
                self.vertical_scroll_state = ScrollbarState::new(max_line_height);
                self.horizontal_scroll_state = ScrollbarState::new(max_line_width);
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
