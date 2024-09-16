use ais_common::constants::SERVERADDRESS;
use ais_common::manager::{NetworkRequest, NetworkRequestType, NetworkResponse};
use ais_common::system::get_system_stats;
use dusa_collection_utils::errors::ErrorArrayItem;
use dusa_collection_utils::stringy::Stringy;
use systemctl::Unit;
// network.rs
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::{query_aggregator, query_git_config, update_git_config};

#[allow(unreachable_patterns)]
pub async fn start_server() -> Result<(), ErrorArrayItem> {
    let listener = TcpListener::bind(SERVERADDRESS).await?;
    println!("Server running on {}", SERVERADDRESS);

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            match socket.read(&mut buf).await {
                Ok(0) => return, // connection was closed
                Ok(n) => {
                    let request: NetworkRequest = match serde_json::from_slice(&buf[0..n]) {
                        Ok(req) => req,
                        Err(e) => {
                            eprintln!("Failed to parse request: {}", e);
                            let response = NetworkResponse {
                                status: String::from("Error"),
                                data: Some(Stringy::new("Invalid request format")),
                            };
                            let response = serde_json::to_string(&response).unwrap();
                            let _ = socket.write_all(response.as_bytes()).await;
                            return;
                        }
                    };

                    match request.request_type {
                        NetworkRequestType::QUERYSTATUS => match query_aggregator().await {
                            Ok(statuses) => {
                                let response = NetworkResponse {
                                    status: String::from("Success"),
                                    data: Some(Stringy::new(&serde_json::to_string(&statuses).unwrap())),
                                };
                                let response = serde_json::to_string(&response).unwrap();
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                            Err(e) => {
                                eprintln!("Failed to query aggregator: {}", e);
                                let response = NetworkResponse {
                                    status: String::from("Error"),
                                    data: Some(Stringy::new("Failed to query aggregator")),
                                };
                                let response = serde_json::to_string(&response).unwrap();
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        },
                        NetworkRequestType::UPDATEGITREPO => {
                            if let Some(data) = request.data {
                                match serde_json::from_str(&data) {
                                    Ok(new_auth) => {
                                        if let Err(e) = update_git_config(new_auth).await {
                                            eprintln!("Failed to update Git config: {}", e);
                                            let response = NetworkResponse {
                                                status: String::from("Error"),
                                                data: Some(Stringy::new(
                                                    "Failed to update Git config",
                                                )),
                                            };
                                            let response =
                                                serde_json::to_string(&response).unwrap();
                                            let _ = socket.write_all(response.as_bytes()).await;
                                        } else {
                                            let response = NetworkResponse {
                                                status: String::from("Success"),
                                                data: None,
                                            };
                                            let response =
                                                serde_json::to_string(&response).unwrap();
                                            let _ = socket.write_all(response.as_bytes()).await;
                                            let unit: Unit =
                                                systemctl::Unit::from_systemctl("git_monitor")
                                                    .unwrap();
                                            unit.restart().unwrap();
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse GitAuth data: {}", e);
                                        eprintln!("{:?}", data);
                                        let response = NetworkResponse {
                                            status: String::from("Error"),
                                            data: Some(Stringy::new("Invalid GitAuth data")),
                                        };
                                        let response = serde_json::to_string(&response).unwrap();
                                        let _ = socket.write_all(response.as_bytes()).await;
                                    }
                                }
                            } else {
                                let response = NetworkResponse {
                                    status: String::from("Error"),
                                    data: Some(Stringy::new("No data provided")),
                                };
                                let response = serde_json::to_string(&response).unwrap();
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        }
                        NetworkRequestType::QUERYGITREPO => match query_git_config().await {
                            Ok(git_statuses) => {
                                let response = NetworkResponse {
                                    status: String::from("Success"),
                                    data: Some(Stringy::new(&serde_json::to_string(&git_statuses).unwrap())),
                                };
                                let response = serde_json::to_string(&response).unwrap();
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                            Err(e) => {
                                eprintln!("Failed to query Git config: {}", e);
                                let response = NetworkResponse {
                                    status: String::from("Error"),
                                    data: Some(Stringy::new("Failed to query Git config")),
                                };
                                let response = serde_json::to_string(&response).unwrap();
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        },
                        NetworkRequestType::QUERYSYSTEM => {
                            let data = get_system_stats();
                            let response = NetworkResponse {
                                status: String::from("Success"),
                                data: Some(Stringy::new(&serde_json::to_string(&data).unwrap())),
                            };
                            let response = serde_json::to_string(&response).unwrap();
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                        _ => {
                            let response = NetworkResponse {
                                status: String::from("Error"),
                                data: Some(Stringy::from("Unknown request type")),
                            };
                            let response = serde_json::to_string(&response).unwrap();
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                }
            }
        });
    }
}
