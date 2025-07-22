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
    // App states
    /// List of file paths to display
    pub file_paths: Vec<PathBuf>,
    /// Contents of the currently selected file
    pub file_contents: Option<String>,
    /// Channel to send parse requests
    pub parse_request_tx: Option<tokio::sync::mpsc::UnboundedSender<ParseRequest>>,
    // UI states
    pub ui_list_state: ListState,
    pub ui_vertical_scroll_state: ScrollbarState,
    pub ui_horizontal_scroll_state: ScrollbarState,
    /// List or contents is currently focused
    pub ui_is_list_focus: bool,
}

impl AppModel {
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        AppModel {
            file_paths,
            file_contents: None,
            parse_request_tx: None,
            ui_list_state: ListState::default(),
            ui_vertical_scroll_state: ScrollbarState::default(),
            ui_horizontal_scroll_state: ScrollbarState::default(),
            ui_is_list_focus: true,
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
            NaviEvent::Tab => self.ui_is_list_focus = !self.ui_is_list_focus,
            NaviEvent::ShiftDown | NaviEvent::Down => {
                if self.ui_is_list_focus {
                    self.ui_list_state.select_next();
                    self.request_parse_current_file();
                    return;
                }

                if matches!(ev, NaviEvent::ShiftDown) {
                    self.ui_vertical_scroll_state.last();
                } else {
                    self.ui_vertical_scroll_state.next();
                }
            }
            NaviEvent::ShiftUp | NaviEvent::Up => {
                if self.ui_is_list_focus {
                    self.ui_list_state.select_previous();
                    self.request_parse_current_file();
                    return;
                }
                if matches!(ev, NaviEvent::ShiftUp) {
                    self.ui_vertical_scroll_state.first();
                } else {
                    self.ui_vertical_scroll_state.prev();
                }
            }
            NaviEvent::ShiftRight | NaviEvent::Right if !self.ui_is_list_focus => {
                if matches!(ev, NaviEvent::ShiftRight) {
                    self.ui_horizontal_scroll_state.last();
                } else {
                    self.ui_horizontal_scroll_state.next();
                }
            }
            NaviEvent::ShiftLeft | NaviEvent::Left if !self.ui_is_list_focus => {
                if matches!(ev, NaviEvent::ShiftLeft) {
                    self.ui_horizontal_scroll_state.first();
                } else {
                    self.ui_horizontal_scroll_state.prev();
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
                self.ui_vertical_scroll_state = ScrollbarState::new(max_line_height);
                self.ui_horizontal_scroll_state = ScrollbarState::new(max_line_width);
                self.file_contents = Some(content);
            }
            ParseResult::Error(error) => {
                let max_line_height = error.lines().count();
                let max_line_width = error.lines().map(|line| line.len()).max().unwrap_or(0);
                self.ui_vertical_scroll_state = ScrollbarState::new(max_line_height);
                self.ui_horizontal_scroll_state = ScrollbarState::new(max_line_width);
                self.file_contents = Some(error);
            }
        }
    }

    fn request_parse_current_file(&mut self) {
        let Some(idx) = self.ui_list_state.selected() else {
            return;
        };
        let Some(file_path) = self.file_paths.get(idx) else {
            return;
        };
        let Some(tx) = &self.parse_request_tx else {
            return;
        };

        self.file_contents = Some("Loading...".to_string());
        let _ = tx.send(ParseRequest::ParseFile {
            file_path: file_path.clone(),
        });
    }
}
