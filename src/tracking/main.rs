use ais_common::common::{current_timestamp, AppName, AppStatus, Status};
use ais_common::dusa_wrapper::encrypt_text;
use ais_common::messages::report_status;
use ais_common::version::Version;
use chrono::{DateTime, Local};
use dusa_collection_utils::errors::{ErrorArray, WarningArray};
use dusa_collection_utils::functions::del_file;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::process::Command;
use tokio::time::{self, Duration};

const SUMMARY_FILE_PATH: &str = "/var/log/ais_security.log"; // Adjust this path as necessary
const TEMP_FILE_PATH: &str = "/var/log/ais_security_temp.log"; // Temporary file for capturing plain data

#[derive(Debug)]
#[allow(dead_code)]
struct UserSession {
    user: String,
    uid: String,
    login_time: Option<DateTime<Local>>,
    commands: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SessionSummary {
    user: String,
    uid: String,
    login_times: Vec<String>,
    command_count: usize,
    authentication_attempts: usize,
    anomaly_events: usize,
    keys: usize,
    journal_logs: HashMap<String, Vec<String>>, // Collecting logs per service
}

async fn get_lastlog() -> io::Result<Vec<UserSession>> {
    let output = Command::new("lastlog").output()?.stdout;
    let reader = BufReader::new(output.as_slice());

    let mut sessions = Vec::new();
    for line in reader.lines().skip(1) {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 4 {
            continue;
        }

        let user = parts[0].to_string();
        let login_time_str = parts[2..].join(" ");

        let login_time = if login_time_str == "**Never logged in**" {
            None
        } else {
            DateTime::parse_from_str(&login_time_str, "%a %b %d %H:%M:%S %z %Y")
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        };

        let uid = get_uid(&user);

        let user_session = UserSession {
            user: user.clone(),
            uid,
            login_time,
            commands: Vec::new(),
        };

        // Only include root and non-standard users
        if user == "root" || !is_standard_user(&user) {
            sessions.push(user_session);
        }
    }

    Ok(sessions)
}

fn is_standard_user(user: &str) -> bool {
    let standard_users = [
        "openvpn", "systemd-oom", "dbus", "ftp", "avahi", "geoclue", "rtkit", "rpcuser", 
        "polkitd", "rpc", "systemd-coredump", "git", "fwupd", "usbmux", "dusa", "flatpak", 
        "lightdm", "passim", "mysql", "www-data", "cups", "nobody", "tss", 
        "systemd-journal-remote", "nm-openvpn", "uuidd", "ollama", "http",
        "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail", "news", "uucp", "proxy",
        "backup", "list", "irc", "gnats", "_apt", "systemd-timesync", "systemd-network", 
        "systemd-resolve", "systemd-bus-proxy", "Debian-exim", "statd", "tcpdump", "sshd", "nslcd",
        "netdata", "_rpc", "messagebus", "_chrony", "redis", 
        "ais", // This is the internal ais user
        "cockpit-ws", "pcp", "cockpit-wsinstance", "dnsmasq",
    ];
    standard_users.contains(&user)
}

fn get_uid(user: &str) -> String {
    let output = Command::new("id")
        .arg("-u")
        .arg(user)
        .output()
        .expect("Failed to execute id command");

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::from("-1") // -1 indicates user not found or error
    }
}

async fn get_journalctl_logs() -> io::Result<HashMap<String, Vec<String>>> {
    let services = vec!["_COMM=sshd", "_COMM=apache2", "_COMM=ufw"];
    let mut logs: HashMap<String, Vec<String>> = HashMap::new();

    for service in services {
        let output = Command::new("journalctl")
            .arg(service)
            .arg("-o")
            .arg("cat")
            .arg("--lines=10") // Adjust the number of lines you want to keep
            .output()?
            .stdout;
        let reader = BufReader::new(output.as_slice());

        let mut service_logs = Vec::new();
        for line in reader.lines() {
            let line = line?;
            service_logs.push(line);
        }
        logs.insert(service.to_string(), service_logs);
    }

    Ok(logs)
}

async fn get_auditd_logs(uid: &str, event_type: &str) -> io::Result<usize> {
    let output = Command::new("ausearch")
        .arg("-ua")
        .arg(uid)
        .arg("-k")
        .arg(event_type)
        .arg("-ts")
        .arg("today")
        .output()?
        .stdout;

    let event_count = String::from_utf8_lossy(&output)
        .lines()
        .count();

    Ok(event_count)
}

async fn summarize_data(
    lastlog_sessions: Vec<UserSession>,
    journal_logs: HashMap<String, Vec<String>>,
) -> (HashMap<String, SessionSummary>, bool) {
    let mut summaries: HashMap<String, SessionSummary> = HashMap::new();
    let mut warning = false;

    for session in lastlog_sessions {
        let login_time_str = match session.login_time {
            Some(time) => time.to_string(),
            None => "never".to_string(),
        };

        summaries
            .entry(session.user.clone())
            .or_insert(SessionSummary {
                user: session.user.clone(),
                uid: session.uid.clone(),
                login_times: Vec::new(),
                command_count: 0,
                authentication_attempts: 0,
                anomaly_events: 0,
                keys: 0,
                journal_logs: HashMap::new(),
            })
            .login_times
            .push(login_time_str);
    }

    for (_, summary) in summaries.iter_mut() {
        if let Ok(command_count) = get_auditd_logs(&summary.uid, "command_exec").await {
            summary.command_count = command_count;
        }
        if let Ok(authentication_attempts) = get_auditd_logs(&summary.uid, "login").await {
            summary.authentication_attempts = authentication_attempts;
        }
        if let Ok(anomaly_events) = get_auditd_logs(&summary.uid, "anomaly").await {
            summary.anomaly_events = anomaly_events;
        }
        if let Ok(keys) = get_auditd_logs(&summary.uid, "key").await {
            summary.keys = keys;
        }

        if summary.authentication_attempts > 0 || summary.anomaly_events > 0 || summary.command_count > 1000000 {
            warning = true;
        }
    }

    // Add journal logs to each summary
    for (service, logs) in journal_logs {
        for summary in summaries.values_mut() {
            summary.journal_logs.insert(service.clone(), logs.clone());
        }
    }

    (summaries, warning)
}

async fn store_summary(summaries: HashMap<String, SessionSummary>) -> io::Result<()> {
    del_file(SUMMARY_FILE_PATH.into(), ErrorArray::new_container(), WarningArray::new_container()).unwrap();

    // Open the temp file to capture plain data
    let mut temp_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(TEMP_FILE_PATH)?;

    // Collect all summary data in one go
    let mut all_summary_data = String::new();
    for (user, summary) in &summaries {
        let summary_data = format!(
            "User: {}\nUID: {}\nLogin Times:\n  - {}\nCommand Count: {}\nAuthentication Attempts: {}\nAnomaly Events: {}\nKeys: {}\n",
            user, summary.uid, summary.login_times.join("\n  - "), summary.command_count, summary.authentication_attempts, summary.anomaly_events, summary.keys
        );

        // Write the plain summary data to the temp file
        temp_file.write_all(summary_data.as_bytes())?;

        // Collect all summary data
        all_summary_data.push_str(&summary_data);
    }

    // Append journal logs
    all_summary_data.push_str("Journal Logs:\n");
    for (_, summary) in &summaries {
        for (service, logs) in &summary.journal_logs {
            let service_logs = format!(
                "Service: {}\n  - {}\n",
                service,
                logs.join("\n  - ")
            );
            all_summary_data.push_str(&service_logs);
        }
    }
    all_summary_data.push_str("--------------------------------\n");

    // Encrypt and write the summary data to the final file
    let encrypted_data = encrypt_text(all_summary_data).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SUMMARY_FILE_PATH)?;
    file.write_all(&encrypted_data.as_bytes())?;

    Ok(())
}

async fn monitor_system_logs() -> io::Result<()> {
    loop {
        let lastlog_sessions = get_lastlog().await?;
        let journal_logs = get_journalctl_logs().await?;

        let (summaries, warning) = summarize_data(lastlog_sessions, journal_logs).await;

        store_summary(summaries).await?;

        // Update application status based on the presence of warnings
        let status = Status {
            app_name: AppName::Security,
            app_status: if warning {
                AppStatus::Warning
            } else {
                AppStatus::Running
            },
            timestamp: current_timestamp(),
            version: Version::get(),
        };
        _ = report_status(status).await;

        time::sleep(Duration::from_secs(60)).await; // Adjust the interval as needed
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    monitor_system_logs().await
}