use ais_common::common::{current_timestamp, AppName, AppStatus, Status};
use ais_common::git::GitAction;
use ais_common::git_data::GitCredentials;
use ais_common::messages::report_status;
use ais_common::setcap::{get_id, set_file_ownership, SystemUsers};
use ais_common::version::Version;
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem};
use dusa_collection_utils::functions::{create_hash, truncate};
use dusa_collection_utils::types::{ClonePath, PathType};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use simple_pretty::notice;
use std::thread;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    simple_pretty::output("GREEN", &format!("Git monitor initialized"));

    thread::spawn(|| loop {
        thread::sleep(Duration::from_secs(60));
        notice("Git monitor running");
    });

    loop {
        let credentials = if let Some(data) = GitCredentials::new().into() {
            let cred_data = match data {
                Ok(d) => d,
                Err(e) => {
                    notice("No git credentials loaded");
                    ErrorArray::new(vec![e]).display(false);
                    std::thread::sleep(Duration::from_secs(30));
                    return;
                }
            };
            cred_data
        } else {
            unreachable!();
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
                // Set the application status to warning in the aggregator as it's running with faults
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

        time::sleep(Duration::from_secs(20)).await; // Report status every 10 seconds
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

        // Switching to target branch
        let git_switch = GitAction::Switch {
            branch: auth.branch.clone(),
            destination: git_project_path.clone(),
        };

        if git_project_path.exists() {
            // Set safe directory
            let set_safe = GitAction::SetSafe(git_project_path.clone_path());
            set_safe.execute().await?;

            // Pull update
            let pull_update = GitAction::Pull {
                target_branch: auth.clone().branch,
                destination: git_project_path.clone_path(),
            };

            match pull_update.execute().await {
                Ok(_) => _ = git_switch.execute().await?,
                Err(e) => {
                    // If pull fails due to safe directory error, set the directory as safe and retry
                    if e.to_string().contains("safe directory") {
                        let set_safe = GitAction::SetSafe(git_project_path.clone_path());
                        set_safe.execute().await?;
                        pull_update.execute().await?;
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            let git_clone = GitAction::Clone {
                repo_name: auth.clone().repo,
                repo_owner: auth.clone().user,
                destination: git_project_path.clone_path(),
            };
            git_clone.execute().await?;

            // Setting ownership to the web user
            let webuser = get_id(SystemUsers::Www)?;
            set_file_ownership(&git_project_path, webuser.0, webuser.1)?;

            // Setting safe directory
            let set_safe = GitAction::SetSafe(git_project_path.clone_path());
            set_safe.execute().await?;

            git_switch.execute().await?;
        }
    }

    Ok(())
}
