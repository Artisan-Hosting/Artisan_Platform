use dusa_collection_utils::{errors::{ErrorArray, ErrorArrayItem, Errors}, functions::path_present, types::PathType};
use tokio::process::Command;

/// Function to check if Git is installed.
async fn check_git_installed() -> Result<(), ErrorArrayItem> {
    let output: std::process::Output = match Command::new("git").arg("--version").output().await {
        Ok(output) => output,
        Err(io_err) => {
            return Err(ErrorArrayItem::from(io_err));

        }
    };

    if output.status.success() {
        Ok(())
    } else {
        return Err(ErrorArrayItem::new(Errors::GeneralError, String::from("Git not installed or not found"))) 
    }
}

/// Enum representing Git actions.
#[derive(Debug)]
pub enum GitAction {
    Clone {
        repo_name: String,
        repo_owner: String,
        destination: PathType,
    },
    Pull {
        target_branch: String,
        destination: PathType,
    },
    Push {
        directory: PathType,
    },
    Stage {
        directory: PathType,
        files: Vec<String>,
    },
    Commit {
        directory: PathType,
        message: String,
    },
    CheckRemoteAhead(PathType),
    Switch {
        branch: String,
        destination: PathType,
    },
    // git config --global --add safe.directory /var/www/current/path
    SetSafe(PathType),
}

impl GitAction {
    /// Execute the Git action.
    pub async fn execute(&self) -> Result<bool, ErrorArrayItem> {
        let err_to_drop = ErrorArray::new_container();
        if let Err(errs) = check_git_installed().await {
            return Err(errs)
        };

        match self {
            GitAction::Clone {
                destination,
                repo_name,
                repo_owner,
            } => {

                let val = path_present(destination, err_to_drop.clone());
                let url = format!("https://github.com/{}/{}.git", repo_owner, repo_name);
                let val2 = execute_git_command(&["clone --mirror", &url, destination.to_str().unwrap()]).await;

                if val.is_ok() {
                    match val2 {
                        Ok(_) => return Ok(true),
                        Err(e) => return Err(e),
                    }
                } else {
                    return Err(ErrorArrayItem::new(Errors::InvalidFile, String::from("Repo path not found")))
                }
                
            }
            GitAction::Pull {
                target_branch,
                destination,
            } => {
                let path = path_present(destination, err_to_drop.clone());
                if path.is_ok() {
                    let _data = path.get_ok().unwrap();
                    let _ = execute_git_command(&["-C", destination.to_str().unwrap(), "pull"]).await?;
                    let _ = execute_git_command(&["-C", destination.to_str().unwrap(), "switch", target_branch]).await?;
                    return Ok(true)
                } else {
                    return Err(path
                            .get_err()
                            .unwrap()
                            .pop());
                }
            }
            GitAction::Push { directory } => {
                let path = path_present(directory, err_to_drop.clone());
                if path.is_ok() {
                    execute_git_command(&["-C", directory.to_str().unwrap(), "push"]).await?;
                    return Ok(true)
                } else {
                    return Err(path
                        .get_err()
                        .unwrap()
                        .pop());
                }
            }
            GitAction::Stage { directory, files } => {
                let path = path_present(directory, err_to_drop.clone());
                if path.is_ok() {
                    let mut args = vec!["-C", directory.to_str().unwrap(), "stage --all"];
                    args.extend(files.iter().map(|s| s.as_str()));
                    execute_git_command(&args).await?;
                    return Ok(true)
                } else {
                    return Err(path
                        .get_err()
                        .unwrap()
                        .pop());
                }
            }
            GitAction::Commit { directory, message } => {
                let path = path_present(directory, err_to_drop.clone());
                if path.is_ok() {
                    execute_git_command(&["-C", directory.to_str().unwrap(), "commit", "-m", message]).await?;
                    return Ok(true)
                } else {
                    return Err(path
                        .get_err()
                        .unwrap()
                        .pop());
                }
            }
            GitAction::CheckRemoteAhead(directory) => {
                match directory.exists() {
                    true => check_remote_ahead(directory).await,
                    false => return Ok(false),
                }
            }
            GitAction::Switch {
                branch,
                destination,
            } => execute_git_command(&["-C", destination.to_str().unwrap(), "switch", branch]).await.map(|_ok| {
                true
            }),
            // ! This is patched out at the system level. git config --global --add safe.directory '*' 
            // ! READ THIS: https://github.com/git/git/commit/8959555cee7ec045958f9b6dd62e541affb7e7d9
            GitAction::SetSafe(directory) => {
                // Split the command correctly into separate arguments
                execute_git_command(&["config", "--global", "--add", "safe.directory", directory.to_str().unwrap()]).await.map(|_ok| true)
            },
        }
    }
}

/// Execute a Git command.
async fn execute_git_command(args: &[&str]) -> Result<(), ErrorArrayItem> {
    let output: std::process::Output = match Command::new("git").args(args).output().await {
        Ok(output) => output,
        Err(io_err) => {
            return Err(ErrorArrayItem::from(io_err));
        }
    };

    if output.status.success() {
        Ok(())
    } else {
        return Err(ErrorArrayItem::new(Errors::GeneralError, String::from_utf8(output.stderr).unwrap()));
    }
}

/// Check if the remote repository is ahead of the local repository.
async fn check_remote_ahead(directory: &PathType) -> Result<bool, ErrorArrayItem> {
    execute_git_command(&["-C", directory.to_str().unwrap(), "fetch"]).await?;

    let local_hash: String =
        execute_git_hash_command(&["-C", directory.to_str().unwrap(), "rev-parse", "@"]).await?;
    let remote_hash: String =
        execute_git_hash_command(&["-C", directory.to_str().unwrap(), "rev-parse", "@{u}"]).await?;

    Ok(remote_hash != local_hash)
}

/// Execute a Git hash command.
async fn execute_git_hash_command(args: &[&str]) -> Result<String, ErrorArrayItem> {
    let output: std::process::Output = match Command::new("git").args(args).output().await {
        Ok(output) => output,
        Err(io_err) => {
            return Err(ErrorArrayItem::from(io_err))
        }
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(ErrorArrayItem::new(Errors::Git, String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

#[cfg(feature = "git")]
#[cfg(test)]
mod tests {
    use system::del_dir;

    use super::*;
    use std::fs;

    const TEST_REPO_URL: &str = "https://github.com/Artisan-Hosting/dummy.git";
    const TEST_DESTINATION: &str = "/tmp/test_repo";

    #[test]
    fn test_check_git_installed() {
        // Assuming Git is installed on the system
        assert!(check_git_installed().is_ok());

        // Assuming Git is not installed on the system
        // Uninstall Git before running this test
        // assert!(check_git_installed().is_err());
    }

    #[test]
    fn test_git_clone() {
        let _ = del_dir(&PathType::Content(TEST_REPO_URL.to_string()));
        let _result = GitAction::Clone {
            repo_url: TEST_REPO_URL.to_string(),
            destination: PathType::Content(TEST_DESTINATION.to_string()),
        }
        .execute();
        // assert!(result.is_ok());
        assert!(fs::metadata(TEST_DESTINATION).is_ok());
    }

    // #[test]
    // #[ignore = "Out of date"]
    // fn test_git_pull() {
    //     let result = GitAction::Pull(PathType::Content(TEST_DESTINATION.to_string()))
    //         .execute()
    //         .unwrap();
    //     assert_eq!(result, true);
    // }

    #[test]
    fn test_check_remote_ahead() {
        // Assuming Git is configured with a remote repository
        let result =
            GitAction::CheckRemoteAhead(PathType::Content(TEST_DESTINATION.to_string())).execute();
        assert!(result.is_ok());
    }
}
