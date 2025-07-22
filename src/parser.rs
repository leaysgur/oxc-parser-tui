use std::path::PathBuf;

use oxc::{allocator::Allocator, parser::Parser, span::SourceType};

pub fn can_parse(file_path: &PathBuf) -> bool {
    SourceType::from_path(file_path).is_ok()
}

#[derive(Debug, Clone)]
pub enum ParseRequest {
    ParseFile { file_path: PathBuf },
}

#[derive(Debug, Clone)]
pub enum ParseResult {
    Success(String),
    Error(String),
}

pub async fn parse_file(file_path: &PathBuf) -> ParseResult {
    let Ok(contents) = tokio::fs::read_to_string(file_path).await else {
        return ParseResult::Error("Error reading file".to_string());
    };

    let allocator = Allocator::default();
    let parser = Parser::new(
        &allocator,
        &contents,
        SourceType::from_path(file_path)
            .expect("SourceType should be valid for pre-checked listed files"),
    );
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

    ParseResult::Success(format!("{:#?}", parse_result.program.body))
}
