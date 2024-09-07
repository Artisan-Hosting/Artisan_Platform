use std::{collections::HashMap, time::{Duration, SystemTime, UNIX_EPOCH}};

use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum QueryType {
    Status,
    AllStatuses,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryMessage {
    pub query_type: QueryType,
    pub app_name: Option<AppName>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub version: String,
    pub app_status: Option<Status>,
    pub all_statuses: Option<HashMap<AppName, Status>>, // New field for all statuses
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    StatusUpdate,
    Acknowledgment,
    Query,
}

/// General structure for messages
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralMessage {
    pub version: String,
    pub msg_type: MessageType,
    pub payload: serde_json::Value,
    pub error: Option<String>, // Simplified for this example
}

/// Enum representing the status of an application.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Running,
    Stopped,
    TimedOut,
    Warning,
}

/// Enum representing the name of an application.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum AppName { // These are artisan_platform components 
    Github,
    Apache,
    Systemd,
    // Firewall,
    Security,
}


/// Struct representing the status of an application at a specific time.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub app_name: AppName,
    pub app_status: AppStatus,
    pub timestamp: u64,
    pub version: String, // Add version field
}


/// Retrieves the current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}

/// Converts a Unix timestamp to a human-readable string.
pub fn format_unix_timestamp(timestamp: u64) -> String {
    let duration = Duration::from_secs(timestamp);
    let datetime = UNIX_EPOCH + duration;
    let now = SystemTime::now();
    
    if let Ok(elapsed) = now.duration_since(datetime) {
        let seconds = elapsed.as_secs();
        format!("{:02}:{:02}:{:02}", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    } else if let Ok(elapsed) = datetime.duration_since(now) {
        let seconds = elapsed.as_secs();
        format!("-{:02}:{:02}:{:02}", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    } else {
        "Error in computing time".to_string()
    }
}