use dusa_collection_utils::{
    errors::{ErrorArray, ErrorArrayItem, WarningArray},
    types::PathType,
};
use serde::Serialize;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use crate::{
    common::{GeneralMessage, MessageType, Status},
    socket::get_socket_path,
    version::Version,
};

/// Encodes a message with a length prefix and sends it over the stream.
pub async fn send_message<T: Serialize>(
    stream: &mut UnixStream,
    message: &T,
) -> Result<(), ErrorArrayItem> {
    let message_bytes: Vec<u8> =
        serde_json::to_vec(message).map_err(|e| ErrorArrayItem::from(e))?;

    let length: u32 = message_bytes.len() as u32;
    let length_bytes: [u8; 4] = length.to_be_bytes();

    stream
        .write_all(&length_bytes)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    stream
        .write_all(&message_bytes)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    Ok(())
}

/// Reads a length-prefixed message from the stream and decodes it.
pub async fn receive_message(stream: &mut UnixStream) -> Result<GeneralMessage, ErrorArrayItem> {
    let mut length_bytes: [u8; 4] = [0u8; 4];

    stream
        .read_exact(&mut length_bytes)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    let length: usize = u32::from_be_bytes(length_bytes) as usize;

    let mut message_bytes = vec![0u8; length];

    stream
        .read_exact(&mut message_bytes)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    let message: GeneralMessage =
        serde_json::from_slice(&message_bytes).map_err(|e| ErrorArrayItem::from(e))?;

    Ok(message)
}

pub async fn send_acknowledge(stream: &mut UnixStream) {
    let ack_message = GeneralMessage {
        version: Version::get(),
        msg_type: MessageType::Acknowledgment,
        payload: json!({"message_received": true}),
        error: None,
    };
    // Since we don't expect a response after this we don't care about its result
    _ = send_message(stream, &ack_message).await;
}

/// Report status to the aggregator
pub async fn report_status(status: Status) -> Result<(), ErrorArrayItem> {
    let throw_away_array_warning: WarningArray = WarningArray::new_container();
    let throw_away_array_error: ErrorArray = ErrorArray::new_container();
    let socket_path_result: Result<dusa_collection_utils::errors::OkWarning<PathType>, ErrorArray> =
        get_socket_path(false, throw_away_array_error, throw_away_array_warning).uf_unwrap();
    let socket_path = match socket_path_result {
        Ok(d) => d.strip(),
        Err(mut e) => return Err(e.pop()),
    };

    let mut stream: UnixStream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| ErrorArrayItem::from(e))?;

    let general_message = GeneralMessage {
        version: Version::get(),
        msg_type: MessageType::StatusUpdate,
        payload: serde_json::to_value(&status).map_err(|e| ErrorArrayItem::from(e))?,
        error: None,
    };

    send_message(&mut stream, &general_message).await
}
