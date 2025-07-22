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
