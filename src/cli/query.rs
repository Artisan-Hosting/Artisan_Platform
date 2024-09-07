use ais_common::common::{AppName, GeneralMessage, MessageType, QueryMessage, QueryType, QueryResponse};
use ais_common::messages::{receive_message, send_message};
use ais_common::socket::get_socket_path;
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, WarningArray};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UnixStream;

#[tokio::main]
async fn main() {
    match query_status(AppName::Github).await {
        Ok(response) => println!("{}", format_query_response(response)),
        Err(e) => eprintln!("Failed to query status: {:?}", e),
    }
}

async fn query_status(app_name: AppName) -> Result<QueryResponse, ErrorArrayItem> {
    let throw_away_array_warning = WarningArray::new_container();
    let throw_away_array_error = ErrorArray::new_container();
    let socket_path_result = get_socket_path(false, throw_away_array_error, throw_away_array_warning).uf_unwrap();
    let socket_path = match socket_path_result {
        Ok(d) => d.strip(),
        Err(mut e) => return Err(e.pop()),
    };

    let mut stream: UnixStream = UnixStream::connect(socket_path).await.map_err(|e| ErrorArrayItem::from(e))?;

    let query_message = QueryMessage {
        query_type: QueryType::Status,
        app_name: Some(app_name),
    };

    let general_message = GeneralMessage {
        version: Version::get(),
        msg_type: MessageType::Query,
        payload: serde_json::to_value(&query_message)?,
        error: None,
    };

    send_message(&mut stream, &general_message).await?;
    let response_message = receive_message(&mut stream).await?;

    if response_message.msg_type == MessageType::Query {
        let response: QueryResponse = serde_json::from_value(response_message.payload)?;
        Ok(response)
    } else {
        Err(ErrorArrayItem::new(dusa_collection_utils::errors::Errors::GeneralError, String::from("Unexpected message")))
    }
}

/// Formats the `QueryResponse` for better readability.
fn format_query_response(response: QueryResponse) -> String {
    let mut formatted_response = format!("Query Response\n");
    
    if let Some(status) = response.app_status {
        let formatted_time = format_unix_timestamp(status.timestamp);
        formatted_response.push_str(&format!(
            "Application Name: {:?}\nStatus: {:?}\nTime since update: {}\nVersion: {}\n",
            status.app_name, status.app_status, formatted_time, status.version
        ));
    } else {
        formatted_response.push_str("No status information available.");
    }

    formatted_response
}

/// Converts a Unix timestamp to a human-readable string.
fn format_unix_timestamp(timestamp: u64) -> String {
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