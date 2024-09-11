use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

// Function to simulate a single HTTP request
async fn simulate_request(client: &Client, url: &str) -> Result<(), reqwest::Error> {
    let response = client.get(url).send().await?;
    println!("Status: {}", response.status());
    Ok(())
}

// Function to simulate multiple concurrent users
async fn simulate_traffic(url: &str, num_users: usize, requests_per_user: usize) {
    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(num_users));

    let mut tasks = vec![];

    for _ in 0..num_users {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client_clone = client.clone();
        let url = url.to_string();

        let task = tokio::spawn(async move {
            for _ in 0..requests_per_user {
                match simulate_request(&client_clone, &url).await {
                    Ok(_) => {}
                    Err(e) => panic!("{}", format!("Error: {}", e)),
                }
                sleep(Duration::from_millis(500)).await; // delay between requests
            }
            drop(permit); // release the semaphore
        });

        tasks.push(task);
        sleep(Duration::from_millis(100)).await;
    }

    for task in tasks {
        task.await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let url = "https://mito.artisanhosting.net"; // Replace with your website URL
                                                 // let url = "https://mitobyte.com"; // Replace with your website URL
    let num_users = 2500; // Number of concurrent users to simulate
    let requests_per_user = 20; // Number of requests each user sends

    println!("Starting load test...");
    simulate_traffic(url, num_users, requests_per_user).await;
    println!("Load test completed.");
}
