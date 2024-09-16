use ais_common::{
    mailing::{Email, EmailSecure},
    version::Version,
};
use dusa_collection_utils::{errors::ErrorArray, stringy::Stringy};

// Simple mailer test
fn main() {
    let email_data = Email {
        subject: Stringy::from("Emailing system test"),
        body: format!(
            "This is a test of the updated email system on ais platform: {}",
            Version::get()
        ).into(),
    };
    let email_secure = EmailSecure::new(email_data);
    match email_secure {
        Ok(d) => d.send().unwrap(),
        Err(e) => ErrorArray::new(vec![e]).display(true),
    }
}
