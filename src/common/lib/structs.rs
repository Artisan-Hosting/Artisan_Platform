use core::fmt;

use dusa_collection_utils::stringy::Stringy;
use serde::{Deserialize, Serialize};

// Directive Structs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Directive {
    pub url: Stringy,
    pub port: u16,

    // Apache settings
    pub apache: bool, // Determines if a new Apache config is needed
    pub php_fpm_version: Option<Stringy>, // Specify PHP-FPM version

    // Node.js settings
    pub nodejs_bool: bool,
    pub nodejs_version: Option<Stringy>,
    
    // Systemd service file settings
    pub service_settings: ServiceSettings,

    // Directory tracking settings
    pub directory_tracking: bool, // Triggers a service restart if the project directory changes
    pub exec_pre_as_root: bool,   // Controls if PermissionsStartOnly is set

    // Internal state
    pub directive_executed: bool, // Should never be changed
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceSettings {
    pub exec_command: Option<Stringy>,    // Command to execute in the service file
    pub exec_pre_command: Option<Stringy>, // Pre-command for service file
    pub restart_policy: RestartPolicy,    // Restart behavior for the service
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RestartPolicy {
    Always,
    OnFailure { max_burst_limit: u8, retry_after_minutes: u8 },
    No,
}

impl fmt::Display for RestartPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestartPolicy::Always => write!(f, "Restart=always"),
            RestartPolicy::OnFailure { max_burst_limit, retry_after_minutes } => {
                write!(
                    f,
                    "Restart=on-failure\nStartLimitBurst={}\nRestartSec={}m",
                    max_burst_limit, retry_after_minutes
                )
            }
            RestartPolicy::No => write!(f, "Restart=no"),
        }
    }
}