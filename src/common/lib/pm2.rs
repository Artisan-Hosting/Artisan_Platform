use std::process::{Command, Output, Stdio};

use dusa_collection_utils::errors::ErrorArrayItem;

/// Enum representing different types of Node.js applications
pub enum AppType {
    NextJs,
    React,
    VueJs,
    Other, // For generic Node.js apps
}

/// Struct representing a Node.js project managed by PM2
pub struct NodeProject {
    pub name: String,
    pub path: String,
    pub app_type: AppType,
}

impl NodeProject {
    /// Creates a new instance of a `NodeProject`
    pub fn new(name: &str, path: &str, app_type: AppType) -> Self {
        NodeProject {
            name: name.to_string(),
            path: path.to_string(),
            app_type,
        }
    }
}

/// A struct representing a PM2 manager for deploying Node.js applications
pub struct Pm2Manager;

impl Pm2Manager {
    /// Starts the given Node.js application using PM2 with `npm run start`
    pub fn start(project: &NodeProject) -> Result<Output, ErrorArrayItem>{
        Command::new("pm2")
            .arg("start")
            .arg("npm")
            .arg("--name")
            .arg(&project.name)
            .arg("--")
            .arg("run")
            .arg("start")
            .current_dir(&project.path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|err| ErrorArrayItem::from(err))
            // .expect("Failed to start application with PM2")
    }

    /// Restarts the given Node.js application using PM2
    pub fn restart(project: &NodeProject) -> Result<Output, ErrorArrayItem> {
        Command::new("pm2")
            .arg("restart")
            .arg(&project.name)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|err| ErrorArrayItem::from(err))
            // .expect("Failed to restart application with PM2")
    }

    /// Stops the given Node.js application using PM2
    pub fn stop(project: &NodeProject) -> Result<Output, ErrorArrayItem> {
        Command::new("pm2")
            .arg("stop")
            .arg(&project.name)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|err| ErrorArrayItem::from(err))
            // .expect("Failed to stop application with PM2")
    }

    /// Rebuilds the Node.js application (install dependencies, build, and restart)
    pub fn rebuild_and_restart(project: &NodeProject) -> Result<Output, ErrorArrayItem> {
        // Step 1: Install dependencies using npm install
        let npm_install: Result<Output, ErrorArrayItem> = Command::new("npm")
            .arg("install")
            .current_dir(&project.path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|err| ErrorArrayItem::from(err));
            // .expect("Failed to install dependencies");

        // Step 2: Build the application according to the project type
        match project.app_type {
            AppType::NextJs => {
                Command::new("npm")
                    .arg("run")
                    .arg("build")
                    .current_dir(&project.path)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(|err| ErrorArrayItem::from(err))?;
                    // .expect("Failed to build Next.js application");

                Command::new("pm2")
                    .arg("start")
                    .arg("npm")
                    .arg("--name")
                    .arg(&project.name)
                    .arg("--")
                    .arg("run")
                    .arg("start")
                    .current_dir(&project.path)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(|err| ErrorArrayItem::from(err))?;
                    // .expect("Failed to start Next.js application with PM2");
            }
            AppType::React | AppType::VueJs => {
                // Build command for React or Vue.js apps
                Command::new("npm")
                    .arg("run")
                    .arg("build")
                    .current_dir(&project.path)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(|err| ErrorArrayItem::from(err))?;
                    // .expect("Failed to build application");

                // Serve the static files or restart using PM2
                Pm2Manager::restart(project)?;
            }
            AppType::Other => {
                // General build and restart for other Node.js apps
                Pm2Manager::restart(project)?;
            }
        }

        // Step 3: Return the output of the process
        npm_install
    }
}