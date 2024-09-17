use std::pin::Pin;

use ais_common::common::{AppName, AppStatus, Status};
use ais_common::git::GitAction;
use ais_common::git_data::{GitAuth, GitCredentials};
use ais_common::messages::report_status;
use ais_common::setcap::{get_id, set_file_ownership, SystemUsers};
use ais_common::system::current_timestamp;
use ais_common::systemd::restart_if_exists;
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, Errors};
use dusa_collection_utils::functions::{create_hash, truncate};
use dusa_collection_utils::types::{ClonePath, PathType};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use simple_pretty::{notice, warn};
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    simple_pretty::output("GREEN", "Git monitor initialized");

    // Using `tokio::spawn` instead of blocking thread::spawn
    tokio::spawn(async {
        loop {
            time::sleep(Duration::from_secs(60)).await;
            notice("Git monitor running");
        }
    });

    loop {
        let credentials = match GitCredentials::new().into() {
            Some(Ok(cred_data)) => cred_data,
            Some(Err(e)) => {
                notice("No git credentials loaded");
                ErrorArray::new(vec![e]).display(false);
                time::sleep(Duration::from_secs(30)).await;
                return;
            }
            None => unreachable!(),
        };

        match git_loop(credentials.clone()).await {
            Ok(_) => {
                // Send okay
                let status: Status = Status {
                    app_name: AppName::Github,
                    app_status: AppStatus::Running,
                    timestamp: current_timestamp(),
                    version: Version::get(),
                };
                if let Err(err) = report_status(status).await {
                    ErrorArray::new(vec![err]).display(false)
                }
            }
            Err(e) => {
                ErrorArray::new(vec![e]).display(false);
                // Set the application status to warning in the aggregator
                let status: Status = Status {
                    app_name: AppName::Github,
                    app_status: AppStatus::Warning,
                    timestamp: current_timestamp(),
                    version: Version::get(),
                };
                if let Err(err) = report_status(status).await {
                    ErrorArray::new(vec![err]).display(false)
                }
            }
        }

        // Sleep between iterations
        time::sleep(Duration::from_secs(20)).await;
    }
}

async fn git_loop(credentials: GitCredentials) -> Result<(), ErrorArrayItem> {
    // Load Git credentials and shuffle them
    let mut credentials_shuffled = credentials.clone();
    let mut rng: StdRng = StdRng::from_entropy(); // Use a seedable RNG that is Send safe
    credentials_shuffled.auth_items.shuffle(&mut rng);

    for auth in &credentials_shuffled.auth_items {
        let ac = auth.clone();
        let git_project_path = generate_git_project_path(&ac);

        if git_project_path.exists() {
            handle_existing_repo(&ac, &git_project_path).await?;
        } else {
            handle_new_repo(&ac, &git_project_path).await?;
        }
    }

    Ok(())
}

// Generate the path for the git project based on branch, repo, and user
fn generate_git_project_path(auth: &GitAuth) -> PathType {
    PathType::Content(format!(
        "/var/www/ais/{}",
        truncate(
            &create_hash(format!("{}-{}-{}", auth.branch, auth.repo, auth.user)),
            8
        )
    ))
}

// Handle an existing repo: fetch, pull, set tracking, restart if needed
async fn handle_existing_repo(
    auth: &GitAuth,
    git_project_path: &PathType,
) -> Result<(), ErrorArrayItem> {
    set_safe_directory(git_project_path).await?;
    fetch_updates(git_project_path).await?;

    let new_data_downloaded = pull_updates(auth, git_project_path).await?;

    if new_data_downloaded {
        finalize_git_actions(auth, git_project_path).await?;
    } else {
        notice(&format!(
            "No new data pulled for {}.",
            truncate(
                &create_hash(format!("{}-{}-{}", auth.branch, auth.repo, auth.user)),
                8
            )
        ));
    }

    Ok(())
}

// Handle a new repo by cloning and setting up safe directories
async fn handle_new_repo(
    auth: &GitAuth,
    git_project_path: &PathType,
) -> Result<(), ErrorArrayItem> {
    // Clone the repository
    let git_clone = GitAction::Clone {
        repo_name: auth.clone().repo,
        repo_owner: auth.clone().user,
        destination: git_project_path.clone_path(),
        repo_branch: auth.clone().branch,
    };
    git_clone.execute().await?;

    // Set ownership to the web user
    let webuser = get_id(SystemUsers::Www)?;
    set_file_ownership(&git_project_path, webuser.0, webuser.1)?;

    // Set safe directory
    set_safe_directory(git_project_path).await?;

    // Force switch to the correct branch after cloning
    fetch_updates(git_project_path).await?;

    Ok(())
}

// Set the git project as a safe directory
async fn set_safe_directory(git_project_path: &PathType) -> Result<(), ErrorArrayItem> {
    let set_safe = GitAction::SetSafe(git_project_path.clone_path());
    set_safe.execute().await?;

    Ok(())
}

// Fetch updates from the remote repository
async fn fetch_updates(git_project_path: &PathType) -> Result<(), ErrorArrayItem> {
    let fetch_update = GitAction::Fetch {
        destination: git_project_path.clone_path(),
    };
    fetch_update.execute().await?;

    Ok(())
}

// Finalize git actions: set tracking, switch branch, restart service
async fn finalize_git_actions(
    auth: &GitAuth,
    git_project_path: &PathType,
) -> Result<(), ErrorArrayItem> {
    set_tracking(git_project_path).await?;
    switch_branch(auth, git_project_path).await?;
    restart_service(auth).await?;

    Ok(())
}

// Set the tracking branch
async fn set_tracking(git_project_path: &PathType) -> Result<(), ErrorArrayItem> {
    let git_set_tracking = GitAction::SetTrack(git_project_path.clone_path());
    git_set_tracking.execute().await?;

    Ok(())
}

// Switch to the appropriate branch
async fn switch_branch(auth: &GitAuth, git_project_path: &PathType) -> Result<(), ErrorArrayItem> {
    let git_switch = GitAction::Switch {
        branch: auth.branch.clone(),
        destination: git_project_path.clone(),
    };
    git_switch.execute().await?;

    Ok(())
}

// Restart the service using the hash
async fn restart_service(auth: &GitAuth) -> Result<(), ErrorArrayItem> {
    let data = create_hash(format!("{}-{}-{}", auth.branch, auth.repo, auth.user));
    let service_name = truncate(&data, 8);
    restart_if_exists(service_name.to_owned())?;
    notice(&format!("Service restarted: {}.", service_name));

    Ok(())
}

// Pull updates and return whether new data was pulled
async fn pull_updates(auth: &GitAuth, git_project_path: &PathType) -> Result<bool, ErrorArrayItem> {
    let pull_update = GitAction::Pull {
        target_branch: auth.branch.clone(),
        destination: git_project_path.clone_path(),
    };

    match pull_update.execute().await {
        Ok(output) => {
            if let Some(data) = output {
                let stdout_str = String::from_utf8_lossy(&data.stdout);

                if stdout_str.contains("Already up to date.") {
                    Ok(false) // No new data was pulled
                } else {
                    Ok(true) // New data was pulled
                }
            } else {
                Ok(false)
            }
        }
        Err(e) => {
            if e.err_type == Errors::GeneralError {
                warn("non-critical errors occurred");
                Ok(true) // Assume new data was pulled in case of non-critical error
            } else if e.to_string().contains("safe directory") {
                // Handle "safe directory" error by boxing recursive calls
                set_safe_directory(git_project_path).await?;
                fetch_updates(git_project_path).await?;

                // Recursively call pull_updates inside a Box to avoid infinite future size
                Pin::from(Box::new(pull_updates(auth, git_project_path))).await
            } else {
                Err(e)
            }
        }
    }
}

