use ais_common::{constants::SYSTEM_DIRECTIVE_PATH, directive::{get_directive_id, get_parent_dir, parse_directive}};
use dusa_collection_utils::{errors::ErrorArrayItem, types::{ClonePath, PathType}};
use simple_pretty::notice;

#[tokio::main]
async fn main() -> Result<(), ErrorArrayItem> {
    let dummy_path = PathType::Str("/var/www/ais/63c35f4b".into());
    let directive_id = get_directive_id(dummy_path.clone_path());
    let directive_option = parse_directive(&directive_id).await?;
    let directive_parent = get_parent_dir(&dummy_path);
    notice(&format!("Executing directive: {}", directive_id));
    notice(&format!("Directive path: {}/{}.ais", SYSTEM_DIRECTIVE_PATH, directive_id));
    notice(&format!("{:?}", directive_option));
    Ok(())
}