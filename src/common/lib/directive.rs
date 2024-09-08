use std::fs;
use std::path::{Path, PathBuf};
use dusa_collection_utils::errors::ErrorArrayItem;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

// The directive functions will parse dependencies or programs that need to be ran when new data is pulled down.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Directive {
    pub url: String,
    pub apache: bool, // This will determine if a new apache config is needed
    pub port: u16,
    pub php_fpm_version: Option<String>, // Add this field to specify PHP-FPM version
    pub nodejs_bool: bool,
    pub nodejs_version: Option<String>,
    pub nodejs_exec_command: Option<String>, // This field will change what is written to the service file 
    pub directive_executed: bool, // This should never be changed
}

pub async fn scan_directories(base_path: &str) -> Result<Vec<PathBuf>, ErrorArrayItem> {
    let mut directive_paths = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "directive.ais" {
            directive_paths.push(entry.path().to_path_buf());
        }
    }

    Ok(directive_paths)
}

pub async fn parse_directive(path: &Path) -> Result<Directive, ErrorArrayItem> {
    let content = fs::read_to_string(path).map_err(|err| ErrorArrayItem::from(err))?;
    let directive: Directive = serde_json::from_str(&content).map_err(|err| ErrorArrayItem::from(err))?;
    Ok(directive)
}
