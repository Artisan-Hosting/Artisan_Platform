use crate::{
    constants::ARTISANCF,
    dusa_wrapper::{decrypt_text, encrypt_text},
};
use dusa_collection_utils::errors::ErrorArrayItem;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitCredentials {
    pub auth_items: Vec<GitAuth>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitAuth {
    pub user: String,
    pub repo: String,
    pub branch: String,
    pub token: String, // TODO REMOVE LATER
}

// TODO ensure we are creating an Array of GitAuth items to parse in loops
impl GitCredentials {
    pub fn new() -> Result<Self, ErrorArrayItem> {
        let encrypted_credentials = Self::read_file(ARTISANCF)?;

        let decrypted_string = decrypt_text(encrypted_credentials)?.replace("\n", "");

        let data: GitCredentials = serde_json::from_str(&decrypted_string)?;

        Ok(data)
    }

    pub fn new_vec() -> Result<Vec<GitAuth>, ErrorArrayItem> {
        let git_credential = Self::new()?;
        let git_vec = git_credential.auth_items.clone();
        Ok(git_vec)
    }

    pub fn to_vec(self) -> Vec<GitAuth> {
        self.auth_items.clone()
    }

    pub fn save(&self, file_path: &str) -> Result<(), ErrorArrayItem> {
        // Serialize GitCredentials to JSON
        let json_data = serde_json::to_string(self)?;

        // Encrypt the JSON data
        let encrypted_data = encrypt_text(json_data)?;

        // Write the encrypted data to the file
        let mut file = File::create(file_path)?;
        file.write_all(encrypted_data.as_bytes())?;

        Ok(())
    }

    fn read_file(file_path: &str) -> Result<String, ErrorArrayItem> {
        let mut file = File::open(file_path)?;
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)?;
        Ok(file_contents.replace("\n", ""))
    }

    pub fn add_auth(&mut self, auth: GitAuth) {
        self.auth_items.push(auth);
    }

    pub fn bootstrap_git_credentials() -> Result<GitCredentials, ErrorArrayItem> {
        match GitCredentials::new() {
            Ok(creds) => Ok(creds),
            Err(_) => {
                let default_creds = GitCredentials {
                    auth_items: Vec::new(),
                };
                default_creds.save("/etc/artisan.cf")?;
                Ok(default_creds)
            }
        }
    }
}
