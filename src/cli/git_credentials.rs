use std::io::{self, Write};

use ais_common::git_data::{GitAuth, GitCredentials};
use simple_pretty::{halt, pass};

fn prompt_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() {
    let mut git_creds = GitCredentials::bootstrap_git_credentials().unwrap();

    let num_instances: usize = prompt_input("Enter the number of GitAuth instances to create: ")
        .parse()
        .expect("Invalid input");

    for i in 0..num_instances {
        println!("Enter details for GitAuth instance {}", i + 1);

        let user = prompt_input("User: ");
        let repo = prompt_input("Repo: ");
        let branch = prompt_input("Branch: ");

        let auth = GitAuth {
            user,
            repo,
            branch,
            token: "******".to_owned(),
        };

        git_creds.add_auth(auth);
    }

    match git_creds.save("/etc/artisan.cf") {
        Ok(_) => pass("New multiplexed file created"),
        Err(e) => halt(&format!("Error while creating manifest: {}", &e.to_string())),
    }
}