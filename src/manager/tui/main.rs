use std::{
    collections::{HashMap, VecDeque},
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

mod ui;

use ais_common::{
    common::{AppName, AppStatus, Status},
    constants::SERVERPORT,
    git_data::{GitAuth, GitCredentials},
    manager::{NetworkRequest, NetworkRequestType, NetworkResponse},
};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use tui::{
    backend::CrosstermBackend,
    layout::Alignment,
    style::{Color, Style},
    widgets::Paragraph,
    Terminal,
};
use ui::draw_ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up terminal
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let terminal = Arc::new(Mutex::new(Terminal::new(backend)?));

    // Prompt for IP address
    terminal.lock().unwrap().clear()?;
    terminal.lock().unwrap().set_cursor(0, 0)?;
    println!("Enter the IP address of the management server:");
    let mut ip_address = String::new();
    io::stdin().read_line(&mut ip_address)?;
    if ip_address.is_empty() {
        ip_address = "127.0.0.1".to_string()
    }
    let ip_address = ip_address.trim().to_string();

    // Set up terminal event handling
    enable_raw_mode()?;
    terminal.lock().unwrap().clear()?;

    let messages = Arc::new(Mutex::new(HashMap::new()));
    let flash_state = Arc::new(Mutex::new(false));
    let aggregator_data = Arc::new(Mutex::new(String::new()));
    let git_data = Arc::new(Mutex::new(String::new()));
    let aggregator_status = Arc::new(Mutex::new(String::from("UNAVAILABLE")));
    let cpu_usage = Arc::new(Mutex::new(0.0));
    let ram_usage = Arc::new(Mutex::new(0.0));
    let system_stats = Arc::new(Mutex::new(HashMap::new()));
    let cpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(100)));
    let ram_history = Arc::new(Mutex::new(VecDeque::with_capacity(100)));
    let redraw_ui = Arc::new(Mutex::new(true));

    // Spawn threads to update data
    spawn_data_update_threads(
        ip_address.clone(),
        aggregator_data.clone(),
        git_data.clone(),
        aggregator_status.clone(),
        cpu_usage.clone(),
        ram_usage.clone(),
        system_stats.clone(),
        terminal.clone(),
        messages.clone(),
        flash_state.clone(),
        cpu_history.clone(),
        ram_history.clone(),
        redraw_ui.clone(),
    );

    // Spawn a thread to toggle flash state
    spawn_flash_state_thread(
        flash_state.clone(),
        terminal.clone(),
        ip_address.clone(),
        aggregator_data.clone(),
        git_data.clone(),
        aggregator_status.clone(),
        messages.clone(),
        cpu_usage.clone(),
        ram_usage.clone(),
        system_stats.clone(),
        cpu_history.clone(),
        ram_history.clone(),
        redraw_ui.clone(),
    );

    // Main loop to handle user input
    loop {
        let redraw = *redraw_ui.lock().unwrap();
        if redraw {
            terminal.lock().unwrap().draw(|f| {
                draw_ui(
                    f,
                    &messages,
                    &flash_state,
                    &aggregator_data,
                    &git_data,
                    &aggregator_status,
                    &cpu_usage,
                    &ram_usage,
                    &system_stats,
                    &cpu_history,
                    &ram_history,
                )
            })?;
        }

        // Handle user input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('a') => handle_aggregator_query(&ip_address, &messages),
                KeyCode::Char('g') => handle_git_repo_query(&ip_address, &messages),
                KeyCode::Char('u') => {
                    *redraw_ui.lock().unwrap() = false;
                    handle_git_repo_update(&ip_address, &messages, &git_data, terminal.clone());
                    *redraw_ui.lock().unwrap() = true;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    terminal.lock().unwrap().show_cursor()?;
    terminal.lock().unwrap().clear()?;
    terminal.lock().unwrap().set_cursor(0, 0)?;
    std::process::exit(0);
    // Ok(())
}

fn spawn_data_update_threads(
    ip_address: String,
    aggregator_data: Arc<Mutex<String>>,
    git_data: Arc<Mutex<String>>,
    aggregator_status: Arc<Mutex<String>>,
    cpu_usage: Arc<Mutex<f64>>,
    ram_usage: Arc<Mutex<f64>>,
    system_stats: Arc<Mutex<HashMap<String, String>>>,
    terminal: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
    messages: Arc<Mutex<HashMap<String, (String, Color)>>>,
    flash_state: Arc<Mutex<bool>>,
    cpu_history: Arc<Mutex<VecDeque<(f64, f64)>>>,
    ram_history: Arc<Mutex<VecDeque<(f64, f64)>>>,
    redraw_ui: Arc<Mutex<bool>>,
) {
    // Thread for periodic data updates
    let ip_address_clone = ip_address.clone();
    let aggregator_data_clone = aggregator_data.clone();
    let git_data_clone = git_data.clone();
    let aggregator_status_clone = aggregator_status.clone();
    let cpu_usage_clone = cpu_usage.clone();
    let ram_usage_clone = ram_usage.clone();
    let system_stats_clone = system_stats.clone();
    let terminal_clone = terminal.clone();
    let messages_clone = messages.clone();
    let flash_state_clone = flash_state.clone();
    let cpu_history_clone = cpu_history.clone();
    let ram_history_clone = ram_history.clone();
    let redraw_ui_clone = redraw_ui.clone();

    thread::spawn(move || loop {
        update_data(
            &ip_address_clone,
            &aggregator_data_clone,
            &git_data_clone,
            &aggregator_status_clone,
            &cpu_usage_clone,
            &ram_usage_clone,
            &system_stats_clone,
            &messages_clone,
            &cpu_history_clone,
            &ram_history_clone,
        );

        let redraw = *redraw_ui_clone.lock().unwrap();
        if redraw {
            terminal_clone
                .lock()
                .unwrap()
                .draw(|f| {
                    draw_ui(
                        f,
                        &messages_clone,
                        &flash_state_clone,
                        &aggregator_data_clone,
                        &git_data_clone,
                        &aggregator_status_clone,
                        &cpu_usage_clone,
                        &ram_usage_clone,
                        &system_stats_clone,
                        &cpu_history_clone,
                        &ram_history_clone,
                    )
                })
                .unwrap();
        }
        thread::sleep(Duration::from_secs(5)); // Update every 5 seconds
    });
}

fn spawn_flash_state_thread(
    flash_state: Arc<Mutex<bool>>,
    terminal: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
    _ip_address: String,
    aggregator_data: Arc<Mutex<String>>,
    git_data: Arc<Mutex<String>>,
    aggregator_status: Arc<Mutex<String>>,
    messages: Arc<Mutex<HashMap<String, (String, Color)>>>,
    cpu_usage: Arc<Mutex<f64>>,
    ram_usage: Arc<Mutex<f64>>,
    system_stats: Arc<Mutex<HashMap<String, String>>>,
    cpu_history: Arc<Mutex<VecDeque<(f64, f64)>>>,
    ram_history: Arc<Mutex<VecDeque<(f64, f64)>>>,
    redraw_ui: Arc<Mutex<bool>>,
) {
    thread::spawn(move || loop {
        {
            let mut state = flash_state.lock().unwrap();
            *state = !*state;
        }
        // Redraw UI after updating flash state
        let redraw = *redraw_ui.lock().unwrap();
        if redraw {
            terminal
                .lock()
                .unwrap()
                .draw(|f| {
                    draw_ui(
                        f,
                        &messages,
                        &flash_state,
                        &aggregator_data,
                        &git_data,
                        &aggregator_status,
                        &cpu_usage,
                        &ram_usage,
                        &system_stats,
                        &cpu_history,
                        &ram_history,
                    )
                })
                .unwrap();
        }
        thread::sleep(Duration::from_millis(500));
    });
}

fn handle_aggregator_query(
    ip_address: &str,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
) {
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYSTATUS,
        data: None,
    };
    if let Ok(response) = send_request(ip_address, &request) {
        if let Some(response_data) = response.data {
            let app_statuses: HashMap<AppName, Status> =
                serde_json::from_str(&response_data).unwrap();
            let mut messages_lock = messages.lock().unwrap();

            for (app_name, status) in app_statuses {
                match status.app_status {
                    AppStatus::Warning => {
                        messages_lock.insert(
                            format!("{:?}", app_name),
                            (format!("Warning: {:?}", app_name), Color::Yellow),
                        );
                    }
                    AppStatus::Running => {
                        messages_lock.remove(&format!("{:?}", app_name));
                    }
                    AppStatus::TimedOut => {
                        messages_lock.insert(
                            format!("{:?}", app_name),
                            (format!("Timed Out: {:?}", app_name), Color::Gray),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}

fn handle_git_repo_query(
    ip_address: &str,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
) {
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYGITREPO,
        data: None,
    };
    if let Ok(response) = send_request(ip_address, &request) {
        if let Some(git_data_response) = response.data {
            let git_auths: HashMap<String, GitAuth> =
                serde_json::from_str(&git_data_response).unwrap();
            let mut messages_lock = messages.lock().unwrap();
            for (repo, auth) in git_auths {
                messages_lock.insert(
                    repo.clone(),
                    (
                        format!(
                            "User: {}\nRepo: {}\nBranch: {}\nToken: {}\n---",
                            auth.user, auth.repo, auth.branch, auth.token
                        ),
                        Color::White,
                    ),
                );
            }
        }
    }
}

fn handle_git_repo_update(
    ip_address: &str,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
    _git_data: &Arc<Mutex<String>>,
    terminal: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
) {
    // prepare terminal for input
    {
        let mut terminal_lock = terminal.lock().unwrap();
        terminal_lock.clear().unwrap();
        terminal_lock.set_cursor(0, 0).unwrap();
    }

    // Disable raw mode for input
    disable_raw_mode().unwrap();

    // Prompt for input
    let mut git_creds = GitCredentials::bootstrap_git_credentials().unwrap();

    let num_instances: usize = prompt_input("Enter the number of GitAuth instances to create: ")
        .parse()
        .expect("Invalid input");

    for i in 0..num_instances {
        println!("Enter details for GitAuth instance {}", i + 1);

        let user = prompt_input("User: ");
        let repo = prompt_input("Repo: ");
        let branch = prompt_input("Branch: ");

        let auth = GitAuth {
            user,
            repo,
            branch,
            token: "******".to_owned(),
        };

        git_creds.add_auth(auth);
    }

    // Re-enable raw mode after input
    enable_raw_mode().unwrap();

    // Getting the vec from the credentials item
    let git_vec: Vec<GitAuth> = git_creds.to_vec();

    // Send the updated git credentials to the server
    let request: NetworkRequest = NetworkRequest {
        request_type: NetworkRequestType::UPDATEGITREPO,
        data: Some(serde_json::to_string(&git_vec).unwrap()),
    };

    if let Ok(response) = send_request(ip_address, &request) {
        if response.status == "OK" {
            let mut messages_lock = messages.lock().unwrap();
            messages_lock.insert(
                "GitUpdate".to_string(),
                (
                    "Git repository updated successfully".to_string(),
                    Color::Green,
                ),
            );
        } else {
            let mut messages_lock = messages.lock().unwrap();
            messages_lock.insert(
                "GitUpdate".to_string(),
                ("Failed to update Git repository".to_string(), Color::Red),
            );
        }
    }

    // Terminal will be redrawn in the main loop after the flag is reset
}

fn prompt_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn update_data(
    ip_address: &str,
    aggregator_data: &Arc<Mutex<String>>,
    git_data: &Arc<Mutex<String>>,
    aggregator_status: &Arc<Mutex<String>>,
    cpu_usage: &Arc<Mutex<f64>>,
    ram_usage: &Arc<Mutex<f64>>,
    system_stats: &Arc<Mutex<HashMap<String, String>>>,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
    cpu_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
    ram_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
) {
    // Update aggregator status
    update_aggregator_status(ip_address, aggregator_data, aggregator_status, messages);
    // Update git data
    update_git_data(ip_address, git_data);
    // Update CPU and RAM usage
    update_system_data(
        ip_address,
        cpu_usage,
        ram_usage,
        system_stats,
        cpu_history,
        ram_history,
    );
}

fn update_aggregator_status(
    ip_address: &str,
    aggregator_data: &Arc<Mutex<String>>,
    aggregator_status: &Arc<Mutex<String>>,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
) {
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYSTATUS,
        data: None,
    };
    if let Ok(response) = send_request(ip_address, &request) {
        if let Some(data) = response.data {
            // Attempt to deserialize the data and process it
            match serde_json::from_str::<HashMap<AppName, Status>>(&data) {
                Ok(app_statuses) => {
                    let aggregator_str = app_statuses
                        .clone()
                        .into_iter()
                        .map(|(_, status)| {
                            format!(
                                "App: {:#?}\nStatus: {:#?}\nTimestamp: {}\nVersion: {}\n---",
                                status.app_name,
                                status.app_status,
                                status.timestamp,
                                status.version
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let mut aggregator_data_lock = aggregator_data.lock().unwrap();
                    *aggregator_data_lock = aggregator_str;

                    let mut status_lock = aggregator_status.lock().unwrap();
                    *status_lock = "OK".to_string();

                    let mut messages_lock = messages.lock().unwrap();
                    for (app_name, status) in app_statuses {
                        match status.app_status {
                            AppStatus::Warning => {
                                messages_lock.insert(
                                    format!("{:?}", app_name),
                                    (format!("Warning: {:?}", app_name), Color::Yellow),
                                );
                            }
                            AppStatus::Running => {
                                messages_lock.remove(&format!("{:?}", app_name));
                            }
                            AppStatus::TimedOut => {
                                messages_lock.insert(
                                    format!("{:?}", app_name),
                                    (format!("Timed Out: {:?}", app_name), Color::Gray),
                                );
                            }
                            AppStatus::Stopped => {
                                messages_lock.insert(
                                    format!("{:?}", app_name),
                                    (format!("Not Running: {:?}", app_name), Color::White),
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    let mut status_lock = aggregator_status.lock().unwrap();
                    *status_lock = "UNAVAILABLE".to_string();
                }
            }
        } else {
            let mut status_lock = aggregator_status.lock().unwrap();
            *status_lock = "UNAVAILABLE".to_string();
        }
    } else {
        let mut status_lock = aggregator_status.lock().unwrap();
        *status_lock = "UNAVAILABLE".to_string();
    }
}

fn update_git_data(ip_address: &str, git_data: &Arc<Mutex<String>>) {
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYGITREPO,
        data: None,
    };
    if let Ok(response) = send_request(ip_address, &request) {
        if let Some(git_data_response) = response.data {
            let git_auths: HashMap<String, GitAuth> =
                serde_json::from_str(&git_data_response).unwrap();
            let git_str = git_auths
                .into_iter()
                .map(|(_, auth)| {
                    format!(
                        "User: {}\nRepo: {}\nBranch: {}\nToken: {}\n---",
                        auth.user, auth.repo, auth.branch, auth.token
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let mut git_data_lock = git_data.lock().unwrap();
            *git_data_lock = git_str;
        }
    }
}

fn update_system_data(
    ip_address: &str,
    cpu_usage: &Arc<Mutex<f64>>,
    ram_usage: &Arc<Mutex<f64>>,
    system_stats: &Arc<Mutex<HashMap<String, String>>>,
    cpu_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
    ram_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
) {
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYSYSTEM,
        data: None,
    };
    if let Ok(response) = send_request(ip_address, &request) {
        if let Some(system_data_response) = response.data {
            let system_data: HashMap<String, String> =
                serde_json::from_str(&system_data_response).unwrap();
            let mut system_stats_lock = system_stats.lock().unwrap();
            for (key, value) in system_data {
                let formatted_value = if key.contains("CPU") {
                    value.clone()
                } else if key.contains("RAM") || key.contains("Swap") {
                    format!("{:.2} GB", format_number(&value.replace(" MB", "")))
                } else {
                    value.clone()
                };
                system_stats_lock.insert(key.clone(), formatted_value);
                if key.contains("CPU Usage") {
                    let cpu_val: f64 = value.trim_end_matches('%').parse().unwrap_or(0.0);
                    let mut cpu_usage_lock = cpu_usage.lock().unwrap();
                    *cpu_usage_lock = cpu_val;

                    let mut cpu_history_lock = cpu_history.lock().unwrap();
                    let cpu_history_lock_clone = cpu_history_lock.clone();
                    if cpu_history_lock.len() == 100 {
                        cpu_history_lock.pop_front();
                    }
                    cpu_history_lock.push_back((cpu_history_lock_clone.len() as f64, cpu_val));
                }
                if key.contains("Used RAM") {
                    let ram_val: f64 = format_number(&value);
                    let mut ram_usage_lock = ram_usage.lock().unwrap();
                    *ram_usage_lock = ram_val;

                    let mut ram_history_lock = ram_history.lock().unwrap();
                    let ram_history_lock_clone = ram_history_lock.clone();
                    if ram_history_lock.len() == 100 {
                        ram_history_lock.pop_front();
                    }
                    ram_history_lock.push_back((ram_history_lock_clone.len() as f64, ram_val));
                }
            }
        }
    }
}

fn send_request(ip_address: &str, request: &NetworkRequest) -> io::Result<NetworkResponse> {
    let server_address = format!("{}:{}", ip_address, SERVERPORT);
    let mut stream = TcpStream::connect(server_address)?;

    let request_json = serde_json::to_string(request)?;
    stream.write_all(request_json.as_bytes())?;
    stream.flush()?;

    let mut buffer = vec![0; 1024];
    let n = stream.read(&mut buffer)?;

    let response: NetworkResponse = serde_json::from_slice(&buffer[0..n])?;

    stream.shutdown(Shutdown::Both)?;

    Ok(response)
}

fn centered_paragraph(text: String) -> Paragraph<'static> {
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White))
}

fn format_number(value: &str) -> f64 {
    let num: f64 = value.parse().unwrap_or(0.0);
    num / 1024000.0
}
