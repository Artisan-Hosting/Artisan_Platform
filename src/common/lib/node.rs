use std::{
    io,
    process::{Command, ExitStatus},
};

use dusa_collection_utils::{errors::ErrorArrayItem, types::PathType};

/// Function to create a systemd service file dynamically
pub fn create_node_systemd_service(
    exec_start: &str,
    working_dir: &PathType,
    description: &str,
) -> Result<String, ErrorArrayItem> {
    // Setting environmental variables depending on the directive file
    let service_file_content = format!(
        r#"[Unit]
Description={}
After=network.target

[Service]
PermissionsStartOnly=false
ExecStart={}
ExecStartPre=/usr/bin/npm run build
Restart=always
# running as a user that has the permissions to bind to the ports needed
User=www-data
Group=www-data
Environment=PATH=/usr/bin:/usr/local/bin
#Environment=NODE_ENV=production
WorkingDirectory={}

[Install]
WantedBy=multi-user.target
"#,
        description, exec_start, working_dir
    );

    Ok(service_file_content)
}

pub fn run_npm_install(working_dir: &PathType) -> io::Result<ExitStatus> {
    // Use `Command` to run `npm install` in the specified directory
    let status = Command::new("npm")
        .arg("install")
        .current_dir(working_dir) // Set the working directory
        .status()?; // Run the command and capture its exit status

    Ok(status)
}
