use crate::dusa_wrapper::encrypt_text;
use dusa_collection_utils::errors::{ErrorArrayItem, Errors};
use serde::{Deserialize, Serialize};
use std::{fmt, io::Write, net::TcpStream};

/// Represents an email message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Email {
    /// The subject of the email.
    pub subject: String,
    /// The body of the email.
    pub body: String,
}

/// Represents an encrypted email message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailSecure {
    /// The encrypted email data.
    pub data: String,
}

// Display implementations
impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.subject, self.body)
    }
}

impl fmt::Display for EmailSecure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl Email {
    /// Creates a new Email instance with the given subject and body.
    pub fn new(subject: String, body: String) -> Self {
        Email { subject, body }
    }

    /// Checks if the email data is valid.
    pub fn is_valid(&self) -> bool {
        !self.subject.is_empty() && !self.body.is_empty()
    }
}

impl EmailSecure {
    /// Creates a new EmailSecure instance by encrypting the provided email.
    pub fn new(email: Email) -> Result<Self, ErrorArrayItem> {
        if !email.is_valid() {
            return Err(ErrorArrayItem::new(
                Errors::GeneralError,
                "Invalid Email Data".to_owned(),
            ));
        }

        let plain_email_data: String = format!("{}-=-{}", email.subject, email.body);
        let encrypted_data: String = encrypt_text(plain_email_data)?;

        Ok(EmailSecure {
            data: encrypted_data,
        })
    }

    /// Sends the encrypted email data over a TCP stream.
    pub fn send(&self) -> Result<(), ErrorArrayItem> {
        let mut stream = match TcpStream::connect("45.137.192.70:1827") {
            Ok(d) => d,
            Err(e) => return Err(ErrorArrayItem::from(e)),
        };
        match stream.write_all(self.data.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(ErrorArrayItem::from(e)),
        }
    }
}
