use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use ais_common::messages::report_status;
use tokio::time::{self, Duration};
use ais_common::systemd::{self, ProcessInfo, Services};
use ais_common::common::{current_timestamp, AppName, AppStatus, Status};
use ais_common::version::Version;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use tokio::sync::Mutex;

async fn monitor_services() -> Result<(), Box<dyn Error>> {
    let status_state = Arc::new(Mutex::new(HashMap::new()));
    let status_state_clone = Arc::clone(&status_state);

    // Report status to the aggregator periodically
    tokio::spawn(async move {
        loop {
            report_status_to_aggregator(status_state_clone.clone()).await;
            time::sleep(Duration::from_secs(10)).await; // Report status every 10 seconds 
        }
    });

    // Monitor services
    let mut previous_status: HashMap<Services, ProcessInfo> = HashMap::new();

    loop {
        let mut services = vec![
            Services::PhpProcessor,
            Services::WebServer,
            Services::SshServer,
            // Services::Monitor, Removing netdata for now
            Services::Firewall,
            Services::Locker,
            Services::Database,
            // Services::Docker, Removing till we change snap to service
        ];

        // Shuffle services to avoid any bias in processing order
        let mut rng: StdRng = StdRng::from_entropy();
        services.shuffle(&mut rng);

        let mut current_status: HashMap<Services, ProcessInfo> = HashMap::new();

        for service in services {
            match ProcessInfo::get_info(service.clone()) {
                Ok(info) => {
                    current_status.insert(service.clone(), info.clone());

                    if let Some(prev_info) = previous_status.get(&service) {
                        if prev_info.status != info.status {
                            println!(
                                "Service {} changed status from {} to {}",
                                service, prev_info.status, info.status
                            );
                        }
                        #[allow(irrefutable_let_patterns)]
                        if let ais_common::systemd::Memory::MemoryConsumed(mem) = info.memory {
                            if mem.parse::<f64>().unwrap_or(0.0) > 1024.0 {
                                println!("Service {} is consuming more than 1GB of memory", service);
                            }
                        }
                    } else {
                        println!("Service {} is now being monitored", service);
                    }
                }
                Err(err) => {
                    eprintln!("Failed to get info for service {}: {}", service, err);
                }
            }
        }

        previous_status = current_status.clone();

        // Update status state for reporting
        let mut status_state_lock = status_state.lock().await;
        *status_state_lock = current_status;

        time::sleep(Duration::from_secs(5)).await; // Check every 10 seconds
    }
}

async fn report_status_to_aggregator(status_state: Arc<Mutex<HashMap<Services, ProcessInfo>>>) {
    let status_map = status_state.lock().await;

    let all_running = status_map.values().all(|info| info.status == systemd::Status::Running);

    let app_status = if all_running {
        AppStatus::Running
    } else {
        AppStatus::Warning
    };

    let status = Status {
        app_name: AppName::Systemd, // Adjust to your application name
        app_status,
        timestamp: current_timestamp(),
        version: Version::get(),
    };

    // Send the status message to the aggregator
    _ = report_status(status).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    monitor_services().await
}
