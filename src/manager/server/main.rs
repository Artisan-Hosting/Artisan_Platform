// main.rs
use ais_common::common::{
    AppName, GeneralMessage, MessageType, QueryMessage, QueryResponse, QueryType, Status,
};
use ais_common::constants::ARTISANCF;
use ais_common::git_data::{GitAuth, GitCredentials};
use ais_common::mailing::{Email, EmailSecure};
use ais_common::messages::{receive_message, send_message};
use ais_common::socket::get_socket_path;
use ais_common::system::{get_machine_id, get_system_stats};
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, WarningArray};
use dusa_collection_utils::stringy::Stringy;
use network::start_server;
use simple_pretty::warn;
use std::collections::HashMap;
use systemctl::Unit;
use tokio::net::UnixStream;

mod network;

#[tokio::main]
async fn main() -> Result<(), ErrorArrayItem> {
    let machine_id = get_machine_id();
    println!("Machine ID: {}", machine_id);

    let system_stats = get_system_stats();
    for (key, value) in &system_stats {
        println!("{}: {}", key, value);
    }

    match query_aggregator().await {
        Ok(_) => println!("Aggregator Communication: OK!"),
        Err(_) => {
            let unit: Unit = systemctl::Unit::from_systemctl("aggregator.service").unwrap();
            let result = unit.restart();
            match result {
                Ok(d) => println!("Fixed: Aggregator was in an unexpected state {}", d),
                Err(e) => {
                    warn("Aggregator is unresponsive, Calling for backup");
                    let email = Email {
                        subject: format!("{}", machine_id).into(),
                        body: format!(
                            "The aggregator on system: {}. Is not running 
                    or unreachable and systemd was unable the rectify the issue: {}",
                            machine_id, e
                        ).into(),
                    };
                    let _ = EmailSecure::new(email)
                        .inspect_err(|err| ErrorArray::new(vec![err.clone()]).display(false))
                        .inspect(|mail| mail.send().unwrap());
                }
            }
        }
    };

    start_server().await.unwrap();
    Ok(())
}

async fn query_aggregator() -> Result<HashMap<AppName, Status>, ErrorArrayItem> {
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

    let query_message = QueryMessage {
        query_type: QueryType::AllStatuses,
        app_name: None,
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
        if let Some(statuses) = response.all_statuses {
            Ok(statuses)
        } else {
            Err(ErrorArrayItem::new(
                dusa_collection_utils::errors::Errors::GeneralError,
                String::from("No statuses returned"),
            ))
        }
    } else {
        println!(
            "Non standard message received: {:?}",
            response_message.msg_type
        );
        Err(ErrorArrayItem::new(
            dusa_collection_utils::errors::Errors::GeneralError,
            String::from("Unexpected message"),
        ))
    }
}

// TODO Implement a fall back function that will use systemd and logs.
// TODO to determine the status of the system if the aggregator fails

async fn update_git_config(new_auth: Vec<GitAuth>) -> Result<(), ErrorArrayItem> {
    let mut new_git_data = GitCredentials { auth_items: vec![] };

    for git_item in new_auth {
        new_git_data.add_auth(git_item);
    }

    new_git_data.save(ARTISANCF)
}

async fn query_git_config() -> Result<HashMap<Stringy, GitAuth>, ErrorArrayItem> {
    let git_credentials: Vec<GitAuth> = GitCredentials::new_vec()?;
    let mut git_hashmap: HashMap<Stringy, GitAuth> = HashMap::new();

    for git_item in git_credentials {
        let name = git_item.clone().repo;
        git_hashmap.insert(name, git_item);
    }

    Ok(git_hashmap)
}
