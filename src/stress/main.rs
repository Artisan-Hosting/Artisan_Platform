use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::{Duration, Instant};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use tokio::sync::{Semaphore, Mutex};
use tokio::time::sleep;
use tokio::task;

#[derive(Default)]
struct Metrics {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    total_duration: Duration,
}

// Function to extract links from a page
async fn extract_links(client: &Client, url: &str) -> Vec<String> {
    let mut links = vec![];
    match client.get(url).send().await {
        Ok(response) => {
            if let Ok(body) = response.text().await {
                let document = Html::parse_document(&body);
                let selector = Selector::parse("a[href]").unwrap();
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if href.starts_with('/') || href.starts_with("http") {
                            let full_url = if href.starts_with('/') {
                                format!("{}{}", url.trim_end_matches('/'), href)
                            } else {
                                href.to_string()
                            };
                            links.push(full_url);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error extracting links from {}: {}", url, e);
        }
    }
    links
}

async fn simulate_request(client: &Client, url: &str, metrics: Arc<Mutex<Metrics>>) {
    let start = Instant::now();
    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            println!("Status: {} for URL: {}", status, url);
            let mut metrics = metrics.lock().await;
            metrics.total_requests += 1;
            metrics.total_duration += start.elapsed();
            if status.is_success() {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            let mut metrics = metrics.lock().await;
            metrics.total_requests += 1;
            metrics.failed_requests += 1;
        }
    }
}

// Function to simulate multiple concurrent users
async fn simulate_traffic(url: &str, num_users: usize, requests_per_user: usize) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let semaphore = Arc::new(Semaphore::new(num_users));
    let metrics = Arc::new(Mutex::new(Metrics::default()));

    // Extract initial links from the home page
    let initial_links = extract_links(&client, url).await;
    let shared_links = Arc::new(Mutex::new(initial_links));

    let mut tasks = vec![];

    for _ in 0..num_users {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client_clone = client.clone();
        let metrics_clone = metrics.clone();
        let shared_links_clone = shared_links.clone();

        let url = url.to_string();
        let task = task::spawn(async move {
            let mut rng = StdRng::from_entropy();
            for _ in 0..requests_per_user {
                let chosen_url = {
                    let links = shared_links_clone.lock().await;
                    if links.is_empty() {
                        url.clone()
                    } else {
                        links[rng.gen_range(0..links.len())].clone()
                    }
                };
                simulate_request(&client_clone, &chosen_url, metrics_clone.clone()).await;
                let delay = rng.gen_range(300..700); // Random delay between 300ms to 700ms
                sleep(Duration::from_millis(delay)).await;
            }
            drop(permit); // release the semaphore
        });

        tasks.push(task);
        sleep(Duration::from_millis(50)).await; // Reduce initial delay to ramp up faster
    }

    for task in tasks {
        task.await.unwrap();
    }

    let metrics = metrics.lock().await;
    println!("Load test completed.");
    println!("Total Requests: {}", metrics.total_requests);
    println!("Successful Requests: {}", metrics.successful_requests);
    println!("Failed Requests: {}", metrics.failed_requests);
    if metrics.total_requests > 0 {
        println!(
            "Average Response Time: {:.2?}",
            metrics.total_duration / metrics.total_requests as u32
        );
    }
}

#[tokio::main]
async fn main() {
    let url = "https://www.mitobyte.com"; // Replace with your website URL
    let num_users = 3000; // Number of concurrent users to simulate
    let requests_per_user = 20; // Number of requests each user sends

    println!("Starting load test...");
    simulate_traffic(url, num_users, requests_per_user).await;
}
