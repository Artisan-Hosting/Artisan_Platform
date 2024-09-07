use std::{collections::HashMap, fs, path::Path};
use gethostname::gethostname;
use sysinfo::System;
use uuid::Uuid;

pub const MACHINE_ID_FILE: &str = "/etc/artisan_id";


pub fn get_system_stats() -> HashMap<String, String> {
    let mut system = System::new_all();
    system.refresh_all();

    let mut stats = HashMap::new();
    stats.insert("CPU Usage".to_string(), format!("{:.2}%", system.global_cpu_info().cpu_usage()));
    stats.insert("Total RAM".to_string(), format!("{} MB", system.total_memory() / 1024));
    stats.insert("Used RAM".to_string(), format!("{} MB", system.used_memory() / 1024));
    stats.insert("Total Swap".to_string(), format!("{} MB", system.total_swap() / 1024));
    stats.insert("Used Swap".to_string(), format!("{} MB", system.used_swap() / 1024));
    stats.insert("Hostname".to_string(), format!("{:?}", gethostname()));

    stats
}

pub fn get_machine_id() -> String {
    if Path::new(MACHINE_ID_FILE).exists() {
        fs::read_to_string(MACHINE_ID_FILE).unwrap_or_else(|_| generate_machine_id())
    } else {
        generate_machine_id()
    }
}

pub fn generate_machine_id() -> String {
    let id = Uuid::new_v4().to_string();
    fs::write(MACHINE_ID_FILE, &id).expect("Unable to write machine ID file");
    id
}