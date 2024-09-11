use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};

use ais_common::constants::SERVERADDRESS;
use ais_common::manager::{NetworkRequest, NetworkRequestType, NetworkResponse};

fn main() -> io::Result<()> {
    // Function to send a request and print the response
    fn send_and_print_response(request: &NetworkRequest) -> io::Result<()> {
        let mut stream = TcpStream::connect(SERVERADDRESS)?;

        let request_json = serde_json::to_string(request).unwrap();
        stream.write_all(request_json.as_bytes())?;
        stream.flush()?;

        let mut buffer = vec![0; 1024];
        let n = stream.read(&mut buffer)?;

        let response: NetworkResponse = serde_json::from_slice(&buffer[0..n]).unwrap();
        println!("Response: {}", response);
        buffer.flush()?;

        stream.shutdown(Shutdown::Both)?;

        Ok(())
    }

    // Git status
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYGITREPO,
        data: None,
    };

    send_and_print_response(&request)?;

    // Aggregator status
    let request = NetworkRequest {
        request_type: NetworkRequestType::QUERYSTATUS,
        data: None,
    };

    send_and_print_response(&request)?;

    Ok(())
}

