use std::process::Output;
use ais_common::pm2::{Pm2Manager, NodeProject, AppType};
use dusa_collection_utils::errors::ErrorArrayItem;

fn main() -> Result<(), ErrorArrayItem> {
    // Example project details
    // let project_name = "my_node_project";
    // let project_path = "/path/to/my/node/project";
    let project_name = "artisan_test";
    let project_path = "/var/www/ais/12c15320";
    let app_type = AppType::NextJs;  // You can change this to AppType::React, AppType::VueJs, or AppType::Other

    // Initialize the NodeProject
    let project = NodeProject::new(project_name, project_path, app_type);

    // Start the project with PM2
    Pm2Manager::build(&project)?;
    match Pm2Manager::start(&project) {
        Ok(output) => handle_output(output),
        Err(e) => eprintln!("Error starting project: {:?}", e),
    }

    Ok(())
    // Rebuild and restart the project
    // match Pm2Manager::rebuild_and_restart(&project) {
    //     Ok(output) => handle_output(output),
    //     Err(e) => eprintln!("Error rebuilding project: {:?}", e),
    // }

    // Stop the project
    // match Pm2Manager::stop(&project) {
    //     Ok(output) => handle_output(output),
    //     Err(e) => eprintln!("Error stopping project: {:?}", e),
    // }
}

/// Helper function to print the command output
fn handle_output(output: Output) {
    if !output.stdout.is_empty() {
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}







// new directive logic

// use ais_common::{constants::SYSTEM_DIRECTIVE_PATH, directive::{get_directive_id, get_parent_dir, parse_directive}};
// use dusa_collection_utils::{errors::ErrorArrayItem, types::{ClonePath, PathType}};
// use simple_pretty::notice;

// #[tokio::main]
// async fn main() -> Result<(), ErrorArrayItem> {
//     let dummy_path = PathType::Str("/var/www/ais/63c35f4b".into());
//     let directive_id = get_directive_id(dummy_path.clone_path());
//     let directive_option = parse_directive(&directive_id).await?;
//     let directive_parent = get_parent_dir(&dummy_path);
//     notice(&format!("Executing directive: {}", directive_id));
//     notice(&format!("Directive path: {}/{}.ais", SYSTEM_DIRECTIVE_PATH, directive_id));
//     notice(&format!("{:?}", directive_option));
//     Ok(())
// }