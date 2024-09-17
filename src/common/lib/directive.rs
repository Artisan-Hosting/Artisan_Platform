use dusa_collection_utils::errors::ErrorArrayItem;
use dusa_collection_utils::functions::{create_hash, open_file, truncate};
use dusa_collection_utils::stringy::Stringy;
use dusa_collection_utils::types::{ClonePath, PathType};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::constants::SYSTEM_DIRECTIVE_PATH;
use crate::structs::Directive;

// pub fn generate_directive_hash(directive_path: PathType) -> Result<String, ErrorArrayItem> {
//     let mut directive_file: std::fs::File = open_file(directive_path.clone(), false)?;

//     let directive_parent: PathType = get_parent_dir(&directive_path);

//     let service_id: String = directive_parent.to_string().replace("/var/www/ais/", "");

//     let mut directive_buffer: Vec<u8> = Vec::new();

//     directive_file
//         .read_to_end(&mut directive_buffer)
//         .map_err(|err| ErrorArrayItem::from(err))?;

//     let directive_hash: String =
//         String::from_utf8(directive_buffer).map_err(|err| ErrorArrayItem::from(err))?;

//     Ok(create_hash(format!("{}_{}", directive_hash, service_id)))
// }

// The directive functions will parse dependencies or programs that need to be ran when new data is pulled down.
pub async fn scan_directories(base_path: &str) -> Result<Vec<PathBuf>, ErrorArrayItem> {
    let mut directive_paths: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "directive.ais" {
            directive_paths.push(entry.path().to_path_buf());
        }
    }
    Ok(directive_paths)
}

pub async fn parse_directive(service_id: &Stringy) -> Result<Option<Directive>, ErrorArrayItem> {
    let directive_path: PathType = get_directive_path(service_id)?;

    if !directive_path.exists() {
        return Ok(None);
    };

    let content: Stringy = read_json_without_comments(directive_path)?;

    let directive: Directive = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(e) => return Err(ErrorArrayItem::from(e)),
    };

    return Ok(Some(directive));
}

fn get_directive_path(directive_id: &Stringy) -> Result<PathType, ErrorArrayItem> {
    let directive_path_stringy: Stringy =
        format!("{}/{}.ais", SYSTEM_DIRECTIVE_PATH, directive_id).into();

    let directive_path: PathType = PathType::Stringy(directive_path_stringy);

    if directive_path.exists() {
        return Ok(directive_path);
    } else {
        File::create(&directive_path)?;
        return Ok(directive_path);
    }
}

/// Reads a JSON file and removes lines starting with `#`
fn read_json_without_comments(file_path: PathType) -> Result<Stringy, ErrorArrayItem> {
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

    Ok(Stringy::new(&json_string))
}

pub fn get_parent_dir(directive_path: &PathType) -> PathType {
    PathType::Path(
        directive_path
            .clone()
            .parent()
            .or_else(|| Some(Path::new("/tmp"))) // this unwrap call should be safe because we can never end up with None for this item
            .unwrap()
            .to_owned()
            .into_boxed_path(),
    )
}

pub fn check_directive(directive: PathType) -> Result<bool, ErrorArrayItem> {
    let new_directive_path = PathType::Content(format!(
        "{}/{}.ais",
        SYSTEM_DIRECTIVE_PATH,
        get_directive_id(directive)
    ));

    Ok(new_directive_path.exists())
}

pub fn get_directive_id(path: PathType) -> Stringy {
    dusa_collection_utils::stringy::Stringy::Mutable(
        path.to_string_lossy()
            .to_string()
            .replace("/var/www/ais/", ""),
    )
}
