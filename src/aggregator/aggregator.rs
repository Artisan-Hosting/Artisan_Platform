use ais_common::common::{
    current_timestamp, AppName, AppStatus, GeneralMessage, MessageType, QueryMessage,
    QueryResponse, QueryType, Status,
};
use ais_common::log::{log, Names};
use ais_common::mailing::{Email, EmailSecure};
use ais_common::messages::{receive_message, send_acknowledge, send_message};
use ais_common::socket::get_socket_path;
use ais_common::system::get_machine_id;
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, UnifiedResult, WarningArray};
use dusa_collection_utils::rwarc::LockWithTimeout;
use dusa_collection_utils::types::PathType;
use simple_pretty::warn;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream as TokioUnixStream};
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

/// Type alias for shared state using a `RwLock`-protected `HashMap` with `AppName` as key.
pub type SharedState = Arc<RwLock<HashMap<AppName, Status>>>;

#[tokio::main]
async fn main() {
    let state: LockWithTimeout<HashMap<AppName, Status>> = LockWithTimeout::new(HashMap::new());
    let state_clone: LockWithTimeout<HashMap<AppName, Status>> = state.clone();
    let errors: ErrorArray = ErrorArray::new_container();
    let warnings: WarningArray = WarningArray::new_container();
    let socket_path_result: UnifiedResult<dusa_collection_utils::errors::OkWarning<PathType>> =
        get_socket_path(true, errors, warnings);

    let socket_path: dusa_collection_utils::errors::OkWarning<PathType> =
        match socket_path_result.uf_unwrap() {
            Ok(path) => path,
            Err(err) => {
                err.display(true);
                return;
            }
        };

    tokio::spawn(async move {
        loop {
            check_for_timeouts(state_clone.clone()).await;
            time::sleep(Duration::from_secs(15)).await; // Check every 15 seconds
        }
    });

    let listener: UnixListener = match UnixListener::bind(socket_path.strip()) {
        Ok(d) => d,
        Err(e) => {
            ErrorArray::new(vec![ErrorArrayItem::from(e)]).display(true);
            unreachable!()
        }
    };

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(sock) => sock,
            Err(err) => {
                warn("Failed to accept connection check logs!");
                log(
                    format!("Failed to accept connection: {:?}", err),
                    Names::AisAggregator,
                );
                continue;
            }
        };

        let new_state: LockWithTimeout<HashMap<AppName, Status>> = state.clone();

        tokio::spawn(async move {
            let message: GeneralMessage = receive_message(&mut stream).await.unwrap();
            handle_message(&new_state, message, &mut stream).await;
        });
    }
}

/// Handles incoming general messages and updates the shared state if it's a status update.
pub async fn handle_message(
    state: &LockWithTimeout<HashMap<AppName, Status>>,
    message: GeneralMessage,
    stream: &mut TokioUnixStream,
) {
    let version_ok: bool = Version::comp(message.version);

    // If the version check passes let's process the data
    match version_ok {
        true => match message.msg_type {
            MessageType::StatusUpdate => {
                if let Ok(status) = serde_json::from_value::<Status>(message.payload) {
                    if let Err(err) = handle_status_update(state.clone(), status).await{
                        ErrorArray::new(vec![err]).display(false);
                    };
                    send_acknowledge(stream).await;
                }
            }
            MessageType::Acknowledgment => {
                let email: Email = Email { subject: format!("Connection dropped Erroneous communication"), 
                body: format!("Machine: {} has dropped a connection due to non standard communication", get_machine_id()) };
                if let Err(err) = EmailSecure::new(email) {
                    ErrorArray::new(vec![err]).display(false);
                };
                panic!("Connection dropped non standard communication");
            }
            MessageType::Query => {
                if let Ok(query) = serde_json::from_value::<QueryMessage>(message.payload) {
                    handle_query(state.clone(), query, stream).await;
                }
            }
        },
        false => warn("Connection dropped client out of date"),
    }
}

/// Handles incoming status updates and updates the shared state.
pub async fn handle_status_update(
    our_state: LockWithTimeout<HashMap<AppName, Status>>,
    new_state: Status,
) -> Result<(), ErrorArrayItem> {
    if let Ok(mut our_state_locked) =
        LockWithTimeout::try_write_with_timeout(&our_state, Some(Duration::from_secs(2))).await
    {
        let new_state_clone: Status = new_state.clone();
        let app_name: AppName = new_state_clone.app_name;
        let app_status: AppStatus = new_state_clone.app_status;

        // Checking if the application is registered
        if our_state_locked.contains_key(&app_name) {
            // checking if the status has changed
            // let mut registered_app = our_state_locked.get(&app_name).unwrap();
            let registered_app = our_state_locked.get_mut(&app_name).unwrap();
            if registered_app.app_status == app_status {
                registered_app.timestamp = new_state_clone.timestamp;
                log(
                    format!("App {:?}, status unchanged: {:?}", app_name, app_status),
                    Names::AisAggregator,
                );
                // return;
            } else {
                // Update the time received
                registered_app.timestamp = new_state_clone.timestamp;
                // print out stuff
                match app_status {
                    AppStatus::Running => {
                        notify_status(&app_name, &app_status)?;
                        log(
                            format!(
                                "{:?} status changed from {:?}, to {:?}",
                                app_name, registered_app.app_status, app_status
                            ),
                            Names::AisAggregator,
                        )
                    }
                    AppStatus::Stopped => {
                        notify_status(&app_name, &app_status)?;
                        log(
                            format!(
                                "{:?} status changed from {:?}, to {:?}",
                                app_name, registered_app.app_status, app_status
                            ),
                            Names::AisAggregator,
                        );
                    }
                    AppStatus::TimedOut => {
                        notify_status(&app_name, &app_status)?;
                        log(
                            format!(
                                "{:?} status changed from {:?}, to {:?}",
                                app_name, registered_app.app_status, app_status
                            ),
                            Names::AisAggregator,
                        )
                    }
                    AppStatus::Warning => {
                        notify_status(&app_name, &app_status)?;
                        log(
                            format!(
                                "{:?} status changed from {:?}, to {:?}",
                                app_name, registered_app.app_status, app_status
                            ),
                            Names::AisAggregator,
                        )
                    }
                }
            }
        } else {
            log(
                format!("New application registered: {:?}", app_name),
                Names::AisAggregator,
            );
        }

        our_state_locked.insert(new_state.app_name.clone(), new_state);
    }
    Ok(())
}

pub async fn handle_query(
    state: LockWithTimeout<HashMap<AppName, Status>>,
    query: QueryMessage,
    stream: &mut TokioUnixStream,
) {
    let response: QueryResponse = {
        let state_lock: tokio::sync::RwLockWriteGuard<HashMap<AppName, Status>> =
            LockWithTimeout::try_write(&state).await.unwrap();
        match query.query_type {
            QueryType::Status => {
                let app_status = query
                    .app_name
                    .and_then(|name| state_lock.get(&name).cloned());
                QueryResponse {
                    version: Version::get(),
                    app_status,
                    all_statuses: None,
                }
            }
            QueryType::AllStatuses => QueryResponse {
                version: Version::get(),
                app_status: None,
                all_statuses: Some(state_lock.clone()),
            },
        }
    };

    let general_response = GeneralMessage {
        version: Version::get(),
        msg_type: MessageType::Query,
        payload: serde_json::to_value(&response).unwrap(),
        error: None,
    };

    send_message(stream, &general_response).await.unwrap();
}

/// Checks for timeouts in the shared state and takes appropriate actions.
pub async fn check_for_timeouts(state: LockWithTimeout<HashMap<AppName, Status>>) {
    if let Ok(mut state_lock) =
        LockWithTimeout::try_write_with_timeout(&state, Some(Duration::from_secs(2))).await
    {
        let current_time = current_timestamp();
        for (app_name, status) in state_lock.iter_mut() {
            if current_time - status.timestamp > 60 {
                warn(&format!(
                    "The module: {:?} has entered a timed out state at {}",
                    status.app_name,
                    current_timestamp()
                ));
                *status = Status {
                    app_name: app_name.clone(),
                    app_status: AppStatus::TimedOut,
                    timestamp: current_timestamp(),
                    version: status.version.clone(),
                };
                let email = Email {
                    subject: format!("Application timed out"),
                    body: format!(
                        "The application {:?} on host {} has timed out",
                        app_name,
                        get_machine_id()
                    ),
                };
                let result = match EmailSecure::new(email) {
                    Ok(d) => d.send(),
                    Err(e) => Err(e),
                };
                if let Err(err) = result {
                    ErrorArray::new(vec![err]).display(false)
                }
            }
        }
    }
}

fn notify_status(name: &AppName, status: &AppStatus) -> Result<(), ErrorArrayItem> {
    let subject: String = format!("Machine update: {}", get_machine_id());
    let body: String = format!("The application: {:?} has change to {:?}", name, status);
    let email: Email = Email { subject, body };
    let secure_email: EmailSecure = EmailSecure::new(email)?;
    secure_email.send()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use ais_common::common::{current_timestamp, AppName, AppStatus, Status};

    use super::*;

    #[tokio::test]
    async fn test_handle_status_update() {
        let state = LockWithTimeout::new(HashMap::new());
        let status = Status {
            app_name: AppName::Github,
            app_status: AppStatus::Running,
            timestamp: current_timestamp(),
            version: Version::get(),
        };

        let _ = handle_status_update(state.clone(), status.clone()).await;

        let state_guard: tokio::sync::RwLockReadGuard<HashMap<_, Status>> =
            LockWithTimeout::try_read(&state).await.unwrap();
        assert_eq!(state_guard.get(&AppName::Github), Some(&status));
    }

    #[tokio::test]
    async fn test_check_for_timeouts() {
        let state: LockWithTimeout<HashMap<AppName, Status>> = LockWithTimeout::new(HashMap::new());
        let status = Status {
            app_name: AppName::Apache,
            app_status: AppStatus::Running,
            timestamp: current_timestamp() - 120, // Simulating a timeout
            version: Version::get(),
        };

        {
            let mut state_guard = LockWithTimeout::try_write(&state).await.unwrap();
            state_guard.insert(AppName::Apache, status);
        }

        check_for_timeouts(state.clone()).await;

        let state_guard = LockWithTimeout::try_read(&state).await.unwrap();
        assert_eq!(
            state_guard.get(&AppName::Apache),
            Some(&Status {
                app_name: AppName::Apache,
                app_status: AppStatus::TimedOut,
                timestamp: state_guard.get(&AppName::Apache).unwrap().timestamp,
                version: Version::get(),
            })
        );
    }
}
