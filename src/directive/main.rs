// This file monitors the directive.ais files and runs things like node, build scripts and so on
// We copy the directive.ais file to a system directory, then we execute what needs to be done, Ie configure apache or node install whatever.
// When change the directive_executed bool to true on the version we copied.
// We save two hashes to ensure we aren't changing thing when they arent needed. We save a hash before copy. and we save a hash that we modify.

use ais_common::{
    apache::{create_apache_config, reload_apache}, common::{AppName, AppStatus, Status}, constants::AIS_PROJECT_FOLDER, directive::{check_directive, get_directive_id, get_parent_dir, parse_directive, scan_directories}, messages::report_status, monitor::{create_monitoring_script, create_monitoring_service, MONITOR_DIR}, node::{create_node_systemd_service, run_npm_install}, system::current_timestamp, systemd::{create_service_file, enable_now, reload_systemd_daemon}, version::Version
};
use dusa_collection_utils::{
    errors::{ErrorArray, ErrorArrayItem}, stringy::Stringy, types::{ClonePath, PathType}
};
use simple_pretty::{notice, warn};
use std::{
    fs::{self, File},
    io::Write,
    thread,
    time::Duration,
};

async fn executing_directive(directive_path: PathType) -> Result<(), ErrorArrayItem> {
    let directive_id = get_directive_id(directive_path.clone_path());
    
    // Parse the directive and handle empty case
    let directive_opt = parse_directive(&directive_id).await?;
    let directive = match directive_opt {
        Some(d) => d,
        None => {
            // Skip if the directive is not complete
            println!("Directive {} skipped as file was incomplete", directive_id);
            return Ok(());
        }
    };

    let directive_parent = get_parent_dir(&directive_path);
    notice(&format!("Executing directive: {}", directive_parent));

    // Checking if we need to reconfigure Apache
    if directive.apache {
        let changed = create_apache_config(&directive, &directive_parent)?;
        if changed {
            match reload_apache().await {
                Ok(success) => {
                    if !success {
                        eprintln!("My god we killed Apache, quick email the admin");
                        eprintln!("The Apache config we rolled out most likely killed Apache");
                    } else {
                        println!("Apache config updated for {:#?}", directive_parent);
                    }
                }
                Err(e) => return Err(e),
            }
        } else {
            println!("The project {} needs no changes", directive_parent);
        }
    }

    // Checking if the project is a Node.js application
    if directive.nodejs_bool {
        let node_version = directive.nodejs_version.unwrap_or_else(|| Stringy::from("22"));
        
        // TODO: Add check with nvm to ensure the correct version is installed.

        // Build application by running npm install
        if let Ok(_) = run_npm_install(&directive_parent) {
            println!("Npm dependencies installed for {}", directive_path);
        } else {
            return Err(ErrorArrayItem::new(
                dusa_collection_utils::errors::Errors::GeneralError,
                String::from("An error occurred while installing npm dependencies"),
            ));
        }

        // Create systemd service file if needed
        if let Some(exec_start_command) = directive.service_settings.exec_command {
            let exec_pre_command = directive.service_settings.exec_pre_command.as_deref();
            let description = &format!("Ais project id {}", directive_parent);

            // Use the new service creation function
            let service_file_data = create_service_file(
                &exec_start_command,
                exec_pre_command,
                description,
                &directive_parent.to_string(),
                // &directive_parent,
                "www-data", // user running the service
                "www-data", // group
                &directive.service_settings.restart_policy, // defined restart policy
                directive.exec_pre_as_root,
            )?;

            // Write the systemd service file
            let service_id: String = directive_parent.to_string().replace("/var/www/ais/", "");
            let service_path = format!("/etc/systemd/system/{}.service", service_id);

            // Remove old service file if it exists
            if PathType::Content(service_path.clone()).exists() {
                fs::remove_file(service_path.clone())?;
            }

            let mut service_file = File::create(service_path.clone())?;
            service_file.write_all(service_file_data.as_bytes())?;

            // Set up monitoring
            create_monitoring_script(&directive_parent.to_string(), &service_id)?;
            create_monitoring_service(
                &service_id,
                &format!("{}{}.monitor", MONITOR_DIR, &service_id),
            )?;

            // Reload systemd daemon and enable services
            reload_systemd_daemon()?;
            enable_now(format!("{}", service_id))?;
            enable_now(format!("{}_monitor", service_id))?;
        } else {
            println!("No systemd service creation needed for {}", directive_parent);
        }
    }

    // Report to the aggregator
    let status = Status {
        app_name: AppName::Directive,
        app_status: AppStatus::Running,
        timestamp: current_timestamp(),
        version: Version::get(),
    };

    if let Err(err) = report_status(status).await {
        Err(err)
    } else {
        Ok(())
    }
}



#[tokio::main]
async fn main() {

    loop {
        let directive_paths = match scan_directories(AIS_PROJECT_FOLDER).await {
            Ok(d) => d,
            Err(e) => {
                // Set the application status to warning in the aggregator as it's running with faults
                let status: Status = Status {
                    app_name: AppName::Directive,
                    app_status: AppStatus::Warning,
                    timestamp: current_timestamp(),
                    version: Version::get(),
                };
                if let Err(err) = report_status(status).await {
                    ErrorArray::new(vec![e, err]).display(true)
                }
                unreachable!("Error scanning dirs")
            }
        };

        for directive_path_string in directive_paths {
            let directive_path: PathType = PathType::PathBuf(directive_path_string);


            // If we haven't already stored the directive data
            if !check_directive(directive_path.clone())
                .expect("Error while opening the directive path")
            {
                match executing_directive(directive_path.clone_path()).await {
                    Ok(_) => (),
                    Err(e1) => {
                        let status: Status = Status {
                            app_name: AppName::Directive,
                            app_status: AppStatus::Warning,
                            timestamp: current_timestamp(),
                            version: Version::get(),
                        };

                        let e2 = report_status(status).await;

                        match e2 {
                            Ok(_) => warn(&format!(
                                "Error executing directive, {}: {}",
                                directive_path, e1
                            )),
                            Err(e2) => ErrorArray::new(vec![e1, e2]).display(true),
                        }
                    }
                }

                // if store_directive(directive_path).is_ok() {
                //     return;
                // } else {
                //     print!("we have executed the directive but cannot store that we have. The directive may be in a loop");
                //     // Set the application status to warning in the aggregator as it's running with faults
                //     let status: Status = Status {
                //         app_name: AppName::Directive,
                //         app_status: AppStatus::Warning,
                //         timestamp: current_timestamp(),
                //         version: Version::get(),
                //     };
                //     if let Err(err) = report_status(status).await {
                //         ErrorArray::new(vec![err]).display(false)
                //     }
                //     return;
                // }
            }
        }

        // Send okay
        let status: Status = Status {
            app_name: AppName::Directive,
            app_status: AppStatus::Running,
            timestamp: current_timestamp(),
            version: Version::get(),
        };

        if let Err(err) = report_status(status).await {
            ErrorArray::new(vec![err]).display(false)
        }

        thread::sleep(Duration::from_secs(10));
    }
}
