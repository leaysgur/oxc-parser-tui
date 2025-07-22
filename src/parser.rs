use std::path::PathBuf;

use oxc::{allocator::Allocator, parser::Parser, span::SourceType};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ParseRequest {
    ParseFile { file_path: PathBuf },
}

#[derive(Debug, Clone)]
pub enum ParseResult {
    Success(String),
    Error(String),
}

pub struct ParserTask {
    request_rx: mpsc::UnboundedReceiver<ParseRequest>,
    result_tx: mpsc::UnboundedSender<ParseResult>,
}

impl ParserTask {
    pub fn new(
        request_rx: mpsc::UnboundedReceiver<ParseRequest>,
        result_tx: mpsc::UnboundedSender<ParseResult>,
    ) -> Self {
        Self {
            request_rx,
            result_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(request) = self.request_rx.recv().await {
            match request {
                ParseRequest::ParseFile { file_path } => {
                    let result = self.parse_file(&file_path).await;
                    if self.result_tx.send(result).is_err() {
                        break;
                    }
                }
            }
        }
    }

    async fn parse_file(&self, file_path: &PathBuf) -> ParseResult {
        // Read file contents
        let Ok(contents) = tokio::fs::read_to_string(file_path).await else {
            return ParseResult::Error("Error reading file".to_string());
        };

        // Determine source type
        let Ok(source_type) = SourceType::from_path(file_path) else {
            return ParseResult::Error("Error determining source type".to_string());
        };

        // Parse the file contents using oxc
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, &contents, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            let error_msg = parse_result
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return ParseResult::Error(error_msg);
        }

        let content = format!("{:#?}", parse_result.program);

        ParseResult::Success(content)
    }
}
