use ais_common::common::{current_timestamp, AppName, AppStatus, Status};
use ais_common::git::GitAction;
use ais_common::git_data::GitCredentials;
use ais_common::messages::report_status;
use ais_common::setcap::{get_id, set_file_ownership, SystemUsers};
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, Errors};
use dusa_collection_utils::functions::{create_hash, truncate};
use dusa_collection_utils::types::{ClonePath, PathType};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use simple_pretty::notice;
use tokio::task;
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
    // Load Git credentials
    let mut credentials_shuffled: GitCredentials = credentials.clone();
    let mut rng: StdRng = StdRng::from_entropy(); // Use a seedable RNG that is Send safe
    credentials_shuffled.auth_items.shuffle(&mut rng);

    for auth in &credentials_shuffled.auth_items {
        let git_project_path = PathType::Content(format!(
            "/var/www/ais/{}",
            truncate(&create_hash(auth.clone().repo), 8)
        ));

        // Log available branches (for debugging)
        // let list_branches = GitAction::ListBranches {
        //     destination: git_project_path.clone_path(),
        // };
        // list_branches.execute().await?;

        // Set up switching to the target branch explicitly using refs/heads/
        let git_switch = GitAction::Switch {
            branch: format!("refs/heads/{}", auth.branch.clone()), // Force the branch reference
            destination: git_project_path.clone(),
        };

        let git_set_tracking = GitAction::SetTrack(git_project_path.clone_path());

        if git_project_path.exists() {
            // Set safe directory
            let set_safe = GitAction::SetSafe(git_project_path.clone_path());
            set_safe.execute().await?;

            // Fetch branches to ensure the latest remote branches are available
            let fetch_branches = GitAction::Fetch {
                destination: git_project_path.clone_path(),
            };
            fetch_branches.execute().await?;

            // Pull update
            let pull_update = GitAction::Pull {
                target_branch: auth.clone().branch,
                destination: git_project_path.clone_path(),
            };

            match pull_update.execute().await {
                Ok(_) => {
                    // Set tracking branch and switch branch if pull succeeds
                    git_set_tracking.execute().await?;
                    git_switch.execute().await?;
                }
                Err(e) => {
                    // If pull fails due to safe directory error, retry setting it as safe
                    if e.to_string().contains("safe directory") {
                        let set_safe = GitAction::SetSafe(git_project_path.clone_path());
                        set_safe.execute().await?;
                        git_set_tracking.execute().await?;
                        pull_update.execute().await?; // Retry pull
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            // If the directory doesn't exist, clone the repo
            let git_clone = GitAction::Clone {
                repo_name: auth.clone().repo,
                repo_owner: auth.clone().user,
                destination: git_project_path.clone_path(),
            };
            git_clone.execute().await?;
            git_set_tracking.execute().await?;

            // Set ownership to the web user
            let webuser = get_id(SystemUsers::Www)?;
            set_file_ownership(&git_project_path, webuser.0, webuser.1)?;

            // Set the directory as safe
            let set_safe = GitAction::SetSafe(git_project_path.clone_path());
            set_safe.execute().await?;

            // Switch to the correct branch after cloning
            git_switch.execute().await?;
        }
    }

    Ok(())
}
