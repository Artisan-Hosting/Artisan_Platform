use std::io::BufRead;
use std::io;
use std::path::{Path, PathBuf};
use dusa_collection_utils::errors::ErrorArrayItem;
use dusa_collection_utils::functions::open_file;
use dusa_collection_utils::types::PathType;
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
    let content = read_json_without_comments(PathType::Path(path.into())).map_err(|err| ErrorArrayItem::from(err))?;
    let directive: Directive = serde_json::from_str(&content).map_err(|err| ErrorArrayItem::from(err))?;
    Ok(directive)
}

/// Reads a JSON file and removes lines starting with `#`
fn read_json_without_comments(file_path: PathType) -> Result<String, ErrorArrayItem> {
    let file = open_file(file_path, false)?;
    let reader = io::BufReader::new(file);

    let mut json_string = String::new();

    for line in reader.lines() {
        let line = line?;
        // Skip lines that start with a `#`
        if !line.trim_start().starts_with('#') {
            json_string.push_str(&line);
            json_string.push('\n');
        }
    }

    Ok(json_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_scan_directories() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().to_str().unwrap();

        // Create a sample directive.ais file
        let directive_path = dir.path().join("directive.ais");
        File::create(&directive_path).unwrap();

        // Call scan_directories
        let paths = scan_directories(dir_path).await.unwrap();

        // Check if the directive.ais file is found
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], directive_path);
    }

    #[tokio::test]
    async fn test_parse_directive() {
        let dir = tempdir().unwrap();
        let directive_path = dir.path().join("directive.ais");

        // Write sample JSON content
        let mut file = File::create(&directive_path).unwrap();
        let json_content = r#"
        {
            "url": "http://example.com",
            "apache": true,
            "port": 8080,
            "php_fpm_version": null,
            "nodejs_bool": false,
            "nodejs_version": null,
            "nodejs_exec_command": null,
            "directive_executed": false
        }"#;
        file.write_all(json_content.as_bytes()).unwrap();

        // Call parse_directive
        let directive = parse_directive(&directive_path).await.unwrap();

        // Verify the parsed data
        assert_eq!(directive.url, "http://example.com");
        assert!(directive.apache);
        assert_eq!(directive.port, 8080);
        assert_eq!(directive.php_fpm_version, None);
        assert!(!directive.nodejs_bool);
        assert_eq!(directive.nodejs_version, None);
        assert!(!directive.directive_executed);
    }

    #[test]
    fn test_read_json_without_comments() {
        let dir = tempdir().unwrap();
        let directive_path = dir.path().join("directive_with_comments.ais");

        // Write JSON content with comments
        let mut file = File::create(&directive_path).unwrap();
        let json_with_comments = r#"
        # This is a comment
        {
            "url": "http://example.com",
            # Another comment
            "apache": true,
            "port": 8080
        }
        "#;
        file.write_all(json_with_comments.as_bytes()).unwrap();

        // Call read_json_without_comments
        let content = read_json_without_comments(PathType::Path(directive_path.into())).unwrap();

        // Verify the comments are removed
        let expected_content = r#"
        {
            "url": "http://example.com",
            "apache": true,
            "port": 8080
        }
        "#;
        assert_eq!(content.trim(), expected_content.trim());
    }
}
