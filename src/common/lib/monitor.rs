use std::fs::{File, create_dir_all};
use std::io::Write;

use dusa_collection_utils::errors::ErrorArrayItem;

pub const MONITOR_DIR: &str = "/opt/monitors/"; 

// Function to create a monitoring script file
pub fn create_monitoring_script(directory_to_watch: &str, service_id: &str) -> Result<(), ErrorArrayItem> {
    let script_content = format!(
        r#"#!/bin/bash

DIRECTORY_TO_WATCH="{}"
SERVICE_NAME="{}"

# Monitor the directory recursively for modifications, creations, or deletions
inotifywait -m -r -e modify,create,delete "$DIRECTORY_TO_WATCH" | while read -r directory events filename; do
  echo "Detected changes in $directory$filename"
  systemctl restart $SERVICE_NAME.service
done
"#,
        directory_to_watch, service_id
    );

    create_dir_all(MONITOR_DIR)?;
    let mut script_file = File::create(format!("{}{}.monitor", MONITOR_DIR, service_id))?;
    script_file.write_all(script_content.as_bytes())?;
    Ok(())
}

// Function to create a systemd service file for the monitoring script
pub fn create_monitoring_service(service_id: &str, script_path: &str) -> Result<(), ErrorArrayItem> {
    let service_file_content = format!(
        r#"[Unit]
Description=Recursive File Monitor for My Project

[Service]
ExecStart={}
Restart=always
User=root
Group=root

[Install]
WantedBy=multi-user.target
"#,
        script_path
    );

    let mut service_file = File::create(format!("/etc/systemd/system/{}_monitor.service", service_id))?;
    service_file.write_all(service_file_content.as_bytes())?;
    Ok(())
}

// Function to reload systemd and enable the new service
pub fn setup_systemd_service(service_id: &str) -> Result<(), ErrorArrayItem> {
    use std::process::Command;

    // Reload systemd daemon
    Command::new("systemctl").arg("daemon-reload").status()?;

    // Enable and start the new service
    Command::new("systemctl").arg("enable").arg(format!("{}_monitor", service_id)).status()?;
    Command::new("systemctl").arg("start").arg(format!("{}_monitor", service_id)).status()?;

    Ok(())
}

