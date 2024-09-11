// Please ignore this trash

use {
    dusa_collection_utils::{
        errors::{
            ErrorArray, ErrorArrayItem, OkWarning, UnifiedResult as uf, WarningArray, WarningArrayItem, Warnings
        }, types::PathType
    }, dusa_common::{
        get_id, prefix::{receive_message, send_message}, set_file_ownership, DecryptResponseData, Message, MessageType, RequestPayload, RequestRecsPlainText, RequestRecsSimple, RequestRecsWrite, SOCKET_PATH, VERSION
    }, nix::unistd::geteuid, std::{
        fs, os::unix::net::UnixStream, path::PathBuf, time::Duration
    }
};

pub enum ProgramMode {
    StoreFile,
    RetrieveFile,
    EncryptText,
    DecryptText,
    RemoveFile,
}

#[allow(unused_variables)]
pub fn run(
    mode: ProgramMode,
    path: Option<String>,
    owner: Option<String>,
    name: Option<String>,
    data: Option<String>,
) -> uf<Option<String>> {
    let mut e1: ErrorArray = ErrorArray::new_container();
    let w1: WarningArray = WarningArray::new_container();

    let socket_path: PathType = match SOCKET_PATH(false, e1.clone(), w1.clone()).uf_unwrap() {
        Ok(d) => {
            d.warning.display();
            d.data
        }
        Err(e) => {
            e.display(true);
            unreachable!();
        }
    };

    let stream: UnixStream = match UnixStream::connect(socket_path.clone()) {
        Ok(d) => d,
        Err(e) => {
            e1.push(ErrorArrayItem::from(e));
            return uf::new(Err(e1))
        }
    };

    let result: uf<OkWarning<Option<String>>> = match mode {
        ProgramMode::StoreFile => encrypt_file(path, stream, w1.clone(), e1.clone()),
        ProgramMode::RetrieveFile => decrypt_file(path, stream, w1.clone(), e1.clone()),
        ProgramMode::EncryptText => encrypt_text(data, stream, w1.clone(), e1.clone()),
        ProgramMode::DecryptText => decrypt_text(data, stream, w1.clone(), e1.clone()),
        ProgramMode::RemoveFile => remove_file(path, stream, w1.clone(), e1.clone()),
    };

    match result.uf_unwrap() {
        Ok(d) => {
            d.warning.display();
            return uf::new(Ok(d.data))
        }
        Err(e) => return uf::new(Err(e)),
    }
}

fn encrypt_file(
    path: Option<String>,
    mut stream: UnixStream,
    mut warnings: WarningArray,
    errors: ErrorArray,
) -> uf<OkWarning<Option<String>>> {
    let file_path: PathType = match path {
        Some(p) => match get_file_path(errors.clone(), warnings.clone(), &PathBuf::from(p)).uf_unwrap() {
            Ok(d) => {
                d.warning.display();
                d.data
            }
            Err(e) => return uf::new(Err(e)),
        },
        None => return uf::new(Err(errors)),
    };

    // Changing ownership of the file
    let (uid, gid) = get_id();
    if let Err(err) = set_file_ownership(&file_path.to_path_buf(), uid, gid, errors.clone()).uf_unwrap() {
        return uf::new(Err(err))
    }

    // Creating the command to send
    let request_data = RequestRecsWrite {
        path: file_path,
        owner: String::from("system"),
        name: String::from("lost"),
        uid: u32::from(geteuid()),
    };

    let msg = Message {
        version: VERSION.to_owned(),
        msg_type: MessageType::Request,
        payload: serde_json::to_value(RequestPayload::Write(request_data)).unwrap(),
        error: None,
    };

    // Communicating with server
    if let Err(err) = send_message(&mut stream, &msg, errors.clone()).uf_unwrap() {
        return uf::new(Err(err))
    }
    std::thread::sleep(Duration::from_nanos(100));
    let response = receive_message(&mut stream, errors.clone()).unwrap();

    match response.msg_type {
        MessageType::Response => {
            let response_data = response.payload;
            let msg = response_data
                .get("Ok")
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            return uf::new(Ok(OkWarning {
                data: Some(msg.unwrap()),
                warning: warnings,
            }));
        }
        MessageType::ErrorResponse => {
            return uf::new(Err(errors))
        }
        _ => {
            let msg = String::from("Server responded in an unexpected way, ignoring ...");
            warnings.push(WarningArrayItem::new_details(
                Warnings::Warning,
                msg,
            ))
        }
    }

    uf::new(Ok(OkWarning {
        data: None,
        warning: warnings,
    }))
}

#[allow(unused_variables)]
fn decrypt_file(
    path: Option<String>,
    mut stream: UnixStream,
    mut warnings: WarningArray,
    mut errors: ErrorArray,
) -> uf<OkWarning<Option<String>>> {
    let request_data = RequestRecsSimple {
        command: dusa_common::Commands::DecryptFile,
        owner: String::from("system"),
        name: String::from("lost"),
        uid: u32::from(geteuid()),
    };

    let msg = Message {
        version: VERSION.to_owned(),
        msg_type: MessageType::Request,
        payload: serde_json::to_value(RequestPayload::Simple(request_data)).unwrap(),
        error: None,
    };

    // Communicating with server
    if let Err(err) = send_message(&mut stream, &msg, errors.clone()).uf_unwrap() {
        err.display(false);
        return uf::new(Err(errors));
    }
    let response = receive_message(&mut stream, errors.clone()).unwrap();

    match response.msg_type {
        MessageType::Response => {
            let response_data = response.payload;
            let data = DecryptResponseData {
                temp_p: response_data
                    .get("temp_p")
                    .and_then(|v| v.get("Content"))
                    .and_then(|v| v.as_str())
                    .map(|s| PathType::Content(s.to_string()))
                    .unwrap_or_else(|| PathType::Content("/tmp/null".to_string())),                
                orig_p: response_data
                    .get("orig_p")
                    .and_then(|v| v.get("PathBuf"))
                    .and_then(|v| v.as_str())
                    .map(|s| PathType::Content(s.to_string()))
                    .unwrap_or_else(|| PathType::Content("/tmp/null".to_string())),  
                ttl: response_data
                    .get("ttl")
                    .and_then(|v| v.get("secs"))
                    .and_then(|v| v.as_u64())
                    .map(|t| Duration::from_secs(t))
                    .unwrap_or(Duration::from_secs(5)), // keep the timing tight
            };

            // Send an ACK message
            let ack = Message {
                version: VERSION.to_owned(),
                msg_type: MessageType::Acknowledge,
                payload: serde_json::json!({}),
                error: None,
            };
            send_message(&mut stream, &ack, errors.clone());
            let _ = receive_message(&mut stream, errors.clone()).unwrap();

            // copy the file to the original path
            match fs::copy(data.temp_p, data.orig_p) {
                Ok(d) => if d != 0 {
                    // log(format!("{:#?}", data));
                    return uf::new(Ok(OkWarning {
                        data: Some("done".to_string()),
                        warning: warnings,
                    }));
                },
                Err(e) => {
                    errors.push(ErrorArrayItem::from(e));
                    errors.display(true);
                },
            }
        }
        MessageType::ErrorResponse => {
            return uf::new(Err(errors));
        }
        _ => {
            let msg = String::from("Server responded in an unexpected way, ignoring ...");
            warnings.push(WarningArrayItem::new_details(
                Warnings::Warning,
                msg,
            ))
        }
    }

    uf::new(Ok(OkWarning {
        data: None,
        warning: warnings,
    }))
}

fn encrypt_text(
    data: Option<String>,
    mut stream: UnixStream,
    mut warnings: WarningArray,
    errors: ErrorArray,
) -> uf<OkWarning<Option<String>>> {
    let data = data.unwrap_or("hello world".to_string());

    let request_data = RequestRecsPlainText {
        command: dusa_common::Commands::EncryptRawText,
        data,
        uid: u32::from(geteuid()),
    };

    let msg = Message {
        version: VERSION.to_owned(),
        msg_type: MessageType::Request,
        payload: serde_json::to_value(RequestPayload::PlainText(request_data)).unwrap(),
        error: None,
    };

    // Communicating with server
    let _ = send_message(&mut stream, &msg, errors.clone());
    std::thread::sleep(Duration::from_nanos(100));
    let response = receive_message(&mut stream, errors.clone()).unwrap();

    match response.msg_type {
        MessageType::Response => {
            let response_data = response.payload;

            // Send an ACK message
            let ack = Message {
                version: VERSION.to_owned(),
                msg_type: MessageType::Acknowledge,
                payload: serde_json::json!({}),
                error: None,
            };
            send_message(&mut stream, &ack, errors.clone());
            let _ = receive_message(&mut stream, errors.clone()).unwrap();

            return uf::new(Ok(OkWarning {
                data: response_data
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                warning: warnings,
            }));
        }
        MessageType::ErrorResponse => {
            return uf::new(Err(errors));
        }
        _ => {
            let msg = String::from("Server responded in an unexpected way, ignoring ...");
            warnings.push(WarningArrayItem::new_details(
                Warnings::Warning,
                msg,
            ))
        }
    };

    uf::new(Ok(OkWarning {
        warning: warnings,
        data: None,
    }))
}

fn decrypt_text(
    data: Option<String>,
    mut stream: UnixStream,
    mut warnings: WarningArray,
    errors: ErrorArray,
) -> uf<OkWarning<Option<String>>> {
    let data = data.unwrap_or("hello world".to_string());

    let request_data = RequestRecsPlainText {
        command: dusa_common::Commands::DecryptRawText,
        data,
        uid: u32::from(geteuid()),
    };

    let msg = Message {
        version: VERSION.to_owned(),
        msg_type: MessageType::Request,
        payload: serde_json::to_value(RequestPayload::PlainText(request_data)).unwrap(),
        error: None,
    };

    // Communicating with server
    let _ = send_message(&mut stream, &msg, errors.clone());
    std::thread::sleep(Duration::from_nanos(100));
    let response = receive_message(&mut stream, errors.clone()).unwrap();

    match response.msg_type {
        MessageType::Response => {
            let response_data = response.payload;

            // Send an ACK message
            let ack = Message {
                version: VERSION.to_owned(),
                msg_type: MessageType::Acknowledge,
                payload: serde_json::json!({}),
                error: None,
            };
            send_message(&mut stream, &ack, errors.clone());
            let _ = receive_message(&mut stream, errors.clone()).unwrap();

            return uf::new(Ok(OkWarning {
                data: response_data
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                warning: warnings,
            }));
        }
        MessageType::ErrorResponse => {
            return uf::new(Err(errors));
        }
        _ => {
            let msg = String::from("Server responded in an unexpected way, ignoring ...");
            warnings.push(WarningArrayItem::new_details(
                Warnings::Warning,
                msg,
            ))
        }
    };

    uf::new(Ok(OkWarning {
        warning: warnings,
        data: None,
    }))
}

#[allow(unused_variables)]
fn remove_file(
    path: Option<String>,
    mut stream: UnixStream,
    mut warnings: WarningArray,
    errors: ErrorArray,
) -> uf<OkWarning<Option<String>>> {
    let request_data = RequestRecsSimple {
        command: dusa_common::Commands::RemoveFile,
        owner: String::from("system"),
        name: String::from("lost"),
        uid: u32::from(geteuid()),
    };

    let msg = Message {
        version: VERSION.to_owned(),
        msg_type: MessageType::Request,
        payload: serde_json::to_value(RequestPayload::Simple(request_data)).unwrap(),
        error: None,
    };

    // Communicating with server
    let _ = send_message(&mut stream, &msg, errors.clone());
    std::thread::sleep(Duration::from_nanos(100));
    let response = receive_message(&mut stream, errors.clone()).unwrap();

    match response.msg_type {
        MessageType::Response => {
            let response_data = response.payload;

            // Send an ACK message
            let ack = Message {
                version: VERSION.to_owned(),
                msg_type: MessageType::Acknowledge,
                payload: serde_json::json!({}),
                error: None,
            };
            send_message(&mut stream, &ack, errors.clone());
            let _ = receive_message(&mut stream, errors.clone()).unwrap();

            return uf::new(Ok(OkWarning {
                data: response_data
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                warning: warnings,
            }));
        }
        MessageType::ErrorResponse => {
            return uf::new(Err(errors));
        }
        _ => {
            let msg = String::from("Server responded in an unexpected way, ignoring ...");
            warnings.push(WarningArrayItem::new_details(
                Warnings::Warning,
                msg,
            ))
        }
    };

    uf::new(Ok(OkWarning {
        warning: warnings,
        data: None,
    }))
}

fn get_file_path(
    mut errors: ErrorArray,
    _warnings: WarningArray,
    option_path_ref: &PathBuf,
) -> uf<OkWarning<PathType>> {
    let err = match option_path_ref.canonicalize() {
        Ok(d) => {
            let result = OkWarning {
                data: PathType::PathBuf(d),
                warning: _warnings,
            };
            return uf::new(Ok(result));
        }
        Err(err) => ErrorArrayItem::from(err),
    };
    errors.push(err);
    uf::new(Err(errors))
}
