use std::{env, future::Future, pin::Pin, process::Output};

use dusa_collection_utils::{
    errors::{ErrorArrayItem, Errors},
    types::PathType,
};
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
        return Err(ErrorArrayItem::new(
            Errors::GeneralError,
            String::from("Git not installed or not found"),
        ));
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
    SetTrack(PathType),
    Branch(PathType),
}

impl GitAction {
    /// Execute the Git action.
    pub fn execute<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Output>, ErrorArrayItem>> + 'a>> {
        Box::pin(async move {
            if let Err(errs) = check_git_installed().await {
                return Err(errs);
            };

            match self {
                GitAction::Clone {
                    destination,
                    repo_name,
                    repo_owner,
                } => {
                    let url = format!("https://github.com/{}/{}.git", repo_owner, repo_name);
                    execute_git_command(&["clone", &url, &destination.to_string()])
                        .await
                        .map(|o| Some(o))
                }
                GitAction::Pull {
                    target_branch,
                    destination,
                } => match destination.exists() {
                    true => {
                        execute_git_command(&["-C", &destination.to_string(), "pull"]).await?;
                        execute_git_command(&[
                            "-C",
                            &destination.to_string(),
                            "switch",
                            target_branch,
                        ])
                        .await
                        .map(|op| Some(op))
                    }
                    false => {
                        return Err(ErrorArrayItem::new(
                            Errors::InvalidFile,
                            String::from("Repo path not found"),
                        ))
                    }
                },
                GitAction::Push { directory } => match directory.exists() {
                    true => execute_git_command(&["-C", &directory.to_string(), "push"])
                        .await
                        .map(|op| Some(op)),
                    false => {
                        return Err(ErrorArrayItem::new(
                            Errors::InvalidFile,
                            String::from("Repo path not found"),
                        ))
                    }
                },
                GitAction::Stage { directory, files } => match directory.exists() {
                    true => {
                        let directory_string = directory.to_string();
                        let mut args = vec!["-C", &directory_string, "stage --all"];
                        args.extend(files.iter().map(|s| s.as_str()));
                        execute_git_command(&args).await.map(|op| Some(op))
                    }
                    false => {
                        return Err(ErrorArrayItem::new(
                            Errors::InvalidFile,
                            String::from("Repo path not found"),
                        ))
                    }
                },
                GitAction::Commit { directory, message } => match directory.exists() {
                    true => execute_git_command(&[
                        "-C",
                        &directory.to_string(),
                        "commit",
                        "-m",
                        message,
                    ])
                    .await
                    .map(|op| Some(op)),
                    false => {
                        return Err(ErrorArrayItem::new(
                            Errors::InvalidFile,
                            String::from("Repo path not found"),
                        ))
                    }
                },
                // THIS IS A CHEAP F*** HACK If remote is ahead the output will have data, else it's none
                GitAction::CheckRemoteAhead(directory) => {
                    let data = check_remote_ahead(directory).await?;
                    match data {
                        true => Ok(Some(
                            Command::new("echo")
                                .arg("hi")
                                .output()
                                .await
                                .map_err(|err| ErrorArrayItem::from(err))?,
                        )),
                        false => Ok(None),
                    }
                }
                GitAction::Switch {
                    branch,
                    destination,
                } => execute_git_command(&["-C", &destination.to_string(), "switch", branch])
                    .await
                    .map(|op| Some(op)),
                // ! This is patched out at the system level. git config --global --add safe.directory '*'
                // ! READ THIS: https://github.com/git/git/commit/8959555cee7ec045958f9b6dd62e541affb7e7d9
                GitAction::SetSafe(directory) => {
                    // Split the command correctly into separate arguments
                    execute_git_command(&[
                        "config",
                        "--global",
                        "--add",
                        "safe.directory",
                        &directory.to_string(),
                    ])
                    .await
                    .map(|op| Some(op))
                }
                GitAction::SetTrack(directory) => {
                    execute_git_command(&["fetch"]).await?;
                    let branch = Self::Branch(directory.clone()).execute().await?;
                    match branch {
                        Some(d) => {
                            let output = String::from_utf8_lossy(&d.stdout);
                            let filtered_lines: Vec<&str> =
                                output.lines().filter(|line| !line.contains("->")).collect();

                            for remote in filtered_lines {
                                // Remove ANSI escape codes with a simple replacement or regex if needed
                                let clean_remote = remote.replace("\x1B", "").replace("[0m", "");

                                // Extract the remote branch name without `origin/`
                                let branch_name = clean_remote
                                    .trim()
                                    .replace("origin/", "")
                                    .replace("main", "")
                                    .replace("master", "");

                                if !branch_name.is_empty() {
                                    execute_git_command(&[
                                        "branch",
                                        "--track",
                                        &branch_name,
                                        &clean_remote,
                                    ])
                                    .await?;
                                }
                            }
                            Ok(None)
                        }
                        None => Err(ErrorArrayItem::new(
                            Errors::Git,
                            format!("Invalid branch data given from the current repo"),
                        )),
                    }
                }
                GitAction::Branch(directory) => {
                    // let original_dir = env::current_dir()?;
                    env::set_current_dir(directory)?;
                    execute_git_command(&["branch", "-r"])
                        .await
                        .map(|op| Some(op))
                }
            }
        })
    }
}


/// Execute a Git command.
async fn execute_git_command(args: &[&str]) -> Result<Output, ErrorArrayItem> {
    let output: std::process::Output = match Command::new("git").args(args).output().await {
        Ok(output) => output,
        Err(io_err) => {
            return Err(ErrorArrayItem::from(io_err));
        }
    };

    if output.status.success() {
        Ok(output)
    } else {
        return Err(ErrorArrayItem::new(
            Errors::GeneralError,
            String::from_utf8(output.stderr).unwrap(),
        ));
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
    let output: std::process::Output = Command::new("git")
        .args(args)
        .output()
        .await
        .map_err(|err| ErrorArrayItem::from(err))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(ErrorArrayItem::new(
            Errors::Git,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
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
