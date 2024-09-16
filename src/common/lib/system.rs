use dusa_collection_utils::{errors::{ErrorArray, ErrorArrayItem}, stringy::Stringy};
use gethostname::gethostname;
use std::{collections::HashMap, fs, io::{self, Write}, path::Path, time::{Duration, SystemTime, UNIX_EPOCH}};
use sysinfo::System;
use uuid::Uuid;

pub const MACHINE_ID_FILE: &str = "/etc/artisan_id";

pub fn get_system_stats() -> HashMap<Stringy, Stringy> {
    let mut system = System::new_all();
    system.refresh_all();

    let mut stats: HashMap<Stringy, Stringy> = HashMap::new();
    stats.insert(
        Stringy::new("CPU Usage"),
        Stringy::from_string(format!("{:.2}%", system.global_cpu_info().cpu_usage())),
    );
    stats.insert(
        Stringy::new("Total RAM"),
        Stringy::from_string(format!("{} MB", system.total_memory() / 1024)),
    );
    stats.insert(
        Stringy::new("Used RAM"),
        Stringy::from(format!("{} MB", system.used_memory() / 1024)),
    );
    stats.insert(
        Stringy::new("Total Swap"),
        Stringy::from_string(format!("{} MB", system.total_swap() / 1024)),
    );
    stats.insert(
        Stringy::new("Used Swap"),
        Stringy::from_string(format!("{} MB", system.used_swap() / 1024)),
    );
    stats.insert(Stringy::new("Hostname"), Stringy::from_string(format!("{:?}", gethostname())));

    stats
}

pub fn get_machine_id() -> Stringy {
    if Path::new(MACHINE_ID_FILE).exists() {
        match fs::read_to_string(MACHINE_ID_FILE) {
            Ok(d) => Stringy::from_string(d),
            Err(e) => {
                ErrorArray::new(vec![ErrorArrayItem::from(e)]).display(false);
                generate_machine_id()
            }
        }
    } else {
        generate_machine_id()
    }
}

pub fn generate_machine_id() -> Stringy {
    let id = Uuid::new_v4().to_string();
    fs::write(MACHINE_ID_FILE, &id).expect("Unable to write machine ID file");
    Stringy::from_string(id)
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
pub fn format_unix_timestamp(timestamp: u64) -> Stringy {
    let duration: Duration = Duration::from_secs(timestamp);
    let datetime: SystemTime = UNIX_EPOCH + duration;
    let now: SystemTime = SystemTime::now();

    let data = if let Ok(elapsed) = now.duration_since(datetime) {
        let seconds = elapsed.as_secs();
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else if let Ok(elapsed) = datetime.duration_since(now) {
        let seconds = elapsed.as_secs();
        format!(
            "-{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else {
        "Error in computing time".to_string()
    };

    return Stringy::from_string(data)
}

pub fn prompt_input(prompt: &str) -> Stringy {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    Stringy::new(input.trim())
}