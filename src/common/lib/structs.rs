use serde::{Deserialize, Serialize};

// Directive Structs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Directive {
    pub url: String,
    pub apache: bool, // This will determine if a new apache config is needed
    pub port: u16,
    pub php_fpm_version: Option<String>, // Add this field to specify PHP-FPM version
    pub nodejs_bool: bool,
    pub nodejs_version: Option<String>,
    // pub nodejs_exec_command: Option<String>, // This field will change what is written to the service file
    pub directive_executed: bool, // This should never be changed
}