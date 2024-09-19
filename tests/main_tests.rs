use std::fs;
use std::path::Path;
use std::process::Command;
use ais_common::pm2::{AppType, NodeProject, Pm2Manager};

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    fn setup_project(app_type: AppType) -> NodeProject {
        let test_project_name = "artisan_test";
        let test_project_path = "/var/www/ais/12c15320";

        // Ensure the dummy project exists for testing
        if !Path::new(test_project_path).exists() {
            fs::create_dir_all(test_project_path).expect("Failed to create test project directory");
        }

        NodeProject::new(test_project_name, test_project_path, app_type)
    }

    #[test]
    fn test_start() {
        let project = setup_project(AppType::NextJs); // You can change the app type here

        match Pm2Manager::start(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 start command failed");
            }
            Err(err) => {
                panic!("Error starting Node.js app: {:?}", err);
            }
        }

        // Additional check: Ensure the app was registered with PM2
        let pm2_status = Command::new("pm2")
            .arg("list")
            .output()
            .expect("Failed to get PM2 list");
        let pm2_stdout = String::from_utf8(pm2_status.stdout).unwrap();

        assert!(pm2_stdout.contains("test_app"), "PM2 did not register the app");
    }

    #[test]
    fn test_restart() {
        let project = setup_project(AppType::React);

        // Start the app first to ensure it exists
        Pm2Manager::start(&project).expect("Failed to start the Node.js app before restart");

        match Pm2Manager::restart(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 restart command failed");
            }
            Err(err) => {
                panic!("Error restarting Node.js app: {:?}", err);
            }
        }
    }

    #[test]
    fn test_stop() {
        let project = setup_project(AppType::VueJs);

        // Start the app first to ensure it exists
        Pm2Manager::start(&project).expect("Failed to start the Node.js app before stopping");

        match Pm2Manager::stop(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 stop command failed");
            }
            Err(err) => {
                panic!("Error stopping Node.js app: {:?}", err);
            }
        }

        // Ensure the app is no longer in the PM2 list
        let pm2_status = Command::new("pm2")
            .arg("list")
            .output()
            .expect("Failed to get PM2 list");
        let pm2_stdout = String::from_utf8(pm2_status.stdout).unwrap();

        assert!(!pm2_stdout.contains("test_app"), "PM2 did not stop the app");
    }

    #[test]
    fn test_rebuild_and_restart_nextjs() {
        let project = setup_project(AppType::NextJs);

        match Pm2Manager::rebuild_and_restart(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 rebuild and restart command failed for Next.js");
            }
            Err(err) => {
                panic!("Error rebuilding and restarting Next.js app: {:?}", err);
            }
        }
    }

    #[test]
    fn test_rebuild_and_restart_react() {
        let project = setup_project(AppType::React);

        match Pm2Manager::rebuild_and_restart(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 rebuild and restart command failed for React");
            }
            Err(err) => {
                panic!("Error rebuilding and restarting React app: {:?}", err);
            }
        }
    }

    #[test]
    fn test_rebuild_and_restart_vue() {
        let project = setup_project(AppType::VueJs);

        match Pm2Manager::rebuild_and_restart(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 rebuild and restart command failed for Vue.js");
            }
            Err(err) => {
                panic!("Error rebuilding and restarting Vue.js app: {:?}", err);
            }
        }
    }

    #[test]
    fn test_rebuild_and_restart_other() {
        let project = setup_project(AppType::Other);

        match Pm2Manager::rebuild_and_restart(&project) {
            Ok(output) => {
                assert!(output.status.success(), "PM2 rebuild and restart command failed for generic Node.js app");
            }
            Err(err) => {
                panic!("Error rebuilding and restarting generic Node.js app: {:?}", err);
            }
        }
    }
}