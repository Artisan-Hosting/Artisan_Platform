use std::{
    collections::HashMap,
};

use dusa_collection_utils::stringy::Stringy;
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
    pub version: Stringy,
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
    pub version: Stringy,
    pub msg_type: MessageType,
    pub payload: serde_json::Value,
    pub error: Option<Stringy>, // Simplified for this example
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
pub enum AppName {
    // These are artisan_platform components
    Github,
    Directive,
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
    pub version: Stringy, // Add version field
}
