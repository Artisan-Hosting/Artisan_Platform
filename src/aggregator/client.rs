use ais_common::common::{AppName, AppStatus, GeneralMessage, MessageType, Status};
use ais_common::messages::{receive_message, send_message};
use ais_common::socket::get_socket_path;
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, WarningArray};
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    loop {
        match send_status_update().await {
            Ok(_) => println!("Status update sent successfully."),
            Err(e) => eprintln!("Failed to send status update: {:?}", e),
        }

        sleep(Duration::from_secs(3)).await; // Send an update every 10 seconds
    }
}

async fn send_status_update() -> Result<(), ErrorArrayItem> {
    let throw_away_array_warning = WarningArray::new_container();
    let throw_away_array_error = ErrorArray::new_container();
    let socket_path_result =
        get_socket_path(false, throw_away_array_error, throw_away_array_warning).uf_unwrap();
    let socket_path = match socket_path_result {
        Ok(d) => d.strip(),
        Err(mut e) => return Err(e.pop()),
    };

    let mut stream: UnixStream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    let status = Status {
        app_name: AppName::Github,
        app_status: AppStatus::Running,
        timestamp: current_timestamp(),
        version: Version::get(),
    };

    let message = GeneralMessage {
        version: Version::get(),
        msg_type: MessageType::StatusUpdate,
        payload: serde_json::to_value(&status).map_err(|e| ErrorArrayItem::from(e))?,
        error: None,
    };

    send_message(&mut stream, &message).await?;

    let response = receive_message(&mut stream).await?;
    println!("Received response: {:?}", response);

    Ok(())
}

/// Retrieves the current Unix timestamp in seconds.
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}
