use chrono::{DateTime, Utc};
use dusa_collection_utils::errors::{ErrorArrayItem, Errors};
use std::{
    fmt, io,
    process::{Command, ExitStatus},
};
use systemctl::Unit;

/// Enum representing different services.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Services {
    PhpProcessor,
    WebServer,
    SshServer,
    Monitor,
    Firewall,
    Locker,
    Database,
    Docker,
}

/// Enum representing the status of a service.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Running,
    Stopped,
    Error,
}

/// Enum representing memory information.
#[derive(Debug, Clone, PartialEq)]
pub enum Memory {
    MemoryConsumed(String),
}

/// Enum representing subprocesses information.
#[derive(Debug, Clone, PartialEq)]
pub enum SubProcesses {
    Pid(u64),
    Tasks(u64),
}

/// Struct representing information about a process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub service: String,
    pub refered: Services,
    pub status: Status,
    pub memory: Memory,
    pub children: SubProcesses,
    pub timestamp: String,
    pub optional: bool,
}

/// Enum representing different types of processes.
#[derive(Debug, Clone)]
pub enum Processes {
    Services(Vec<ProcessInfo>),
}

impl Processes {
    /// Creates a new Processes instance containing information about various services.
    pub fn new() -> Result<Self, ErrorArrayItem> {
        let mut data: Vec<ProcessInfo> = Vec::new();
        data.push(ProcessInfo::get_info(Services::WebServer)?);
        data.push(ProcessInfo::get_info(Services::PhpProcessor)?);
        data.push(ProcessInfo::get_info(Services::Firewall)?);
        data.push(ProcessInfo::get_info(Services::Monitor)?);
        data.push(ProcessInfo::get_info(Services::SshServer)?);
        data.push(ProcessInfo::get_info(Services::Locker)?);

        Ok(Self::Services(data))
    }

    /// Updates the information of a specific service.
    pub fn update(service: Services) -> Result<ProcessInfo, ErrorArrayItem> {
        ProcessInfo::get_info(service)
    }

    /// Iterates over the Processes enum and returns a vector of ProcessInfo.
    pub fn itr(&self) -> Vec<ProcessInfo> {
        match self {
            Processes::Services(data) => data.clone(),
        }
    }
}

impl Services {
    /// Restarts the service and returns a bool based on the running status after the restart.
    pub fn restart(&self) -> Result<bool, ErrorArrayItem> {
        let unit_name: String = format!("{}", self);
        match systemctl::restart(&unit_name) {
            Ok(_) => match systemctl::is_active(&unit_name) {
                Ok(d) => Ok(d),
                Err(e) => Err(ErrorArrayItem::from(e)),
            },
            Err(e) => Err(ErrorArrayItem::from(e)),
        }
    }
    /// Re loads the service and returns a bool based on the running status after the restart.
    pub fn reload(&self) -> Result<bool, ErrorArrayItem> {
        let unit_name: String = format!("{}", self);
        match systemctl::reload(&unit_name) {
            Ok(_) => match systemctl::is_active(&unit_name) {
                Ok(d) => Ok(d),
                Err(e) => Err(ErrorArrayItem::from(e)),
            },
            Err(e) => Err(ErrorArrayItem::from(e)),
        }
    }
}

impl ProcessInfo {
    /// Retrieves information about a specific service.
    pub fn get_info(service: Services) -> Result<Self, ErrorArrayItem> {
        let unit_name: String = format!("{}", &service);
        let unit: Unit = match systemctl::Unit::from_systemctl(&unit_name) {
            Ok(d) => d,
            Err(e) => return Err(ErrorArrayItem::from(e)),
        };

        let status_data: Result<bool, std::io::Error> = unit.is_active();
        let status: Status = match status_data {
            Ok(true) => Status::Running,
            Ok(false) => Status::Stopped,
            Err(_) => Status::Error,
        };

        let memory_data: Option<String> = unit.memory;
        let memory: Memory = match memory_data {
            Some(d) => Memory::MemoryConsumed(d),
            None => Memory::MemoryConsumed(format!("{}B", 0.00.to_string())),
        };

        let (tasks, pid) = (unit.tasks, unit.pid);
        let children: SubProcesses = match (tasks, pid) {
            (Some(t), Some(_p)) => SubProcesses::Tasks(t),
            (_, _) => SubProcesses::Pid(0),
        };

        Ok(Self {
            service: unit_name,
            status,
            memory,
            children,
            timestamp: timestamp(),
            refered: service,
            optional: false,
        })
    }
}

pub fn reload_systemd_daemon() -> io::Result<ExitStatus> {
    let status = Command::new("systemctl").arg("daemon-reload").status()?;

    Ok(status)
}

pub fn enable_now(service_name: String) -> io::Result<ExitStatus> {
    let status = Command::new("systemctl")
        .arg("enable")
        .arg(&service_name)
        .arg("--now")
        .status()?;

    Ok(status)
}

pub fn is_service(service_name: String) -> Result<bool, ErrorArrayItem> {
    systemctl::exists(&service_name).map_err(|err| ErrorArrayItem::new(Errors::GeneralError, err.to_string()))
}

pub fn restart_service(service_name: String) -> io::Result<ExitStatus> {
    systemctl::restart(&service_name)
}

pub fn restart_if_exists(service_name: String) -> Result<bool, ErrorArrayItem> {
    match is_service(service_name.clone()) {
        Ok(d) => match d {
            true => restart_service(service_name)
                .map(|res| if res.success() { true } else {false})
                .map_err(|err| ErrorArrayItem::new(Errors::GeneralError, err.to_string())),
            false => return Ok(false),
        },
        Err(e) => return Err(e),
    }
}

// Displays

impl fmt::Display for Services {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name: &str = match self {
            Services::PhpProcessor => "php7.4-fpm.service",
            Services::WebServer => "apache2.service",
            Services::SshServer => "sshd.service",
            Services::Monitor => "netdata.service",
            Services::Firewall => "ufw.service",
            Services::Locker => "dusad.service",
            Services::Database => "mysql.service",
            Services::Docker => "docker.service",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status: &str = match self {
            Status::Running => "active",
            Status::Stopped => "stopped",
            Status::Error => "Error occurred while checking",
        };
        write!(f, "{}", status)
    }
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Memory::MemoryConsumed(d) => write!(f, "{}", d),
        }
    }
}

impl fmt::Display for SubProcesses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubProcesses::Pid(p) => write!(f, "{}", p),
            SubProcesses::Tasks(t) => write!(f, "{}", t),
        }
    }
}

/// Generates a timestamp string in the format: YYYY-MM-DD HH:MM:SS.
pub fn timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_services_display() {
        assert_eq!(format!("{}", Services::PhpProcessor), "php7.4-fpm.service");
        assert_eq!(format!("{}", Services::WebServer), "apache2.service");
        assert_eq!(format!("{}", Services::SshServer), "sshd.service");
        assert_eq!(format!("{}", Services::Monitor), "netdata.service");
        assert_eq!(format!("{}", Services::Firewall), "ufw.service");
        assert_eq!(format!("{}", Services::Locker), "dusad.service");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", Status::Running), "active");
        assert_eq!(format!("{}", Status::Stopped), "stopped");
        assert_eq!(
            format!("{}", Status::Error),
            "Error occurred while checking"
        );
    }

    #[test]
    fn test_memory_display() {
        assert_eq!(
            format!("{}", Memory::MemoryConsumed("2GB".to_string())),
            "2GB"
        );
    }

    #[test]
    fn test_subprocesses_display() {
        assert_eq!(format!("{}", SubProcesses::Pid(123)), "123");
        assert_eq!(format!("{}", SubProcesses::Tasks(456)), "456");
    }

    #[test]
    fn test_timestamp() {
        let timestamp = timestamp();
        assert!(timestamp.len() > 0);
    }

    // Additional tests can be added for other functions and scenarios.
}
