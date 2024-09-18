use crate::constants::{WEBSERVER_CONFIG_DIR, WEBSERVER_PORTS_CONFIG};
use crate::directive::{get_directive_id, parse_directive, scan_directories};
use crate::structs::Directive;
use crate::systemd::Services;
use dusa_collection_utils::errors::ErrorArrayItem;
use dusa_collection_utils::types::PathType;
use simple_pretty::notice;
use std::error::Error;
use std::fs;
use std::path::Path;

fn read_existing_apache_config(directive: &Directive) -> Option<String> {
    let config_path = Path::new(WEBSERVER_CONFIG_DIR).join(format!("{}.conf", directive.url));
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(config_path) {
            return Some(content);
        }
    }
    None
}

pub fn create_apache_config(
    directive: &Directive,
    base_path: &Path,
) -> Result<bool, ErrorArrayItem> {
    let php_fpm_config = match &directive.php_fpm_version {
        Some(version) if *version == "7.4".into() => {
            r#"SetHandler "proxy:unix:/var/run/php/php7.4-fpm.sock|fcgi://localhost/""#
        }
        Some(version) if *version == "8.1".into() => {
            r#"SetHandler "proxy:unix:/var/run/php/php8.1-fpm.sock|fcgi://localhost/""#
        }
        Some(version) if *version == "8.2".into() => {
            r#"SetHandler "proxy:unix:/var/run/php/php8.2-fpm.sock|fcgi://localhost/""#
        }
        _ => "", // No PHP-FPM handler if version is not specified or not recognized
    };

    let config_content = format!(
        r#"<VirtualHost *:{}>
    ServerName {}
    DocumentRoot {}
    
    <Directory "{}">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
        DirectoryIndex index.php
    </Directory>

    <FilesMatch \.php$>
        {}
    </FilesMatch>
    ErrorLog ${{APACHE_LOG_DIR}}/error.log
    CustomLog ${{APACHE_LOG_DIR}}/access.log combined
</VirtualHost>
        "#,
        directive.port,
        directive.url,
        base_path.display(),
        base_path.display(),
        php_fpm_config,
    );

    let config_path = Path::new(WEBSERVER_CONFIG_DIR).join(format!("{}.conf", directive.url));

    // Check if existing config matches the new config
    if let Some(existing_config) = read_existing_apache_config(directive) {
        if existing_config == config_content {
            println!("Config for {} is up to date", directive.url);
            return Ok(false); // No change made
        }
    }

    // Write the new config file if it doesn't match the existing one
    fs::write(config_path, config_content)?;
    Ok(true) // Configuration changed
}

async fn check_apache_ports(directive: &Directive) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(WEBSERVER_PORTS_CONFIG)?;
    if !content.contains(&directive.port.to_string()) {
        println!(
            "Port {} not found in Apache2 ports configuration, assuming port 80",
            directive.port
        );
    }
    Ok(())
}

pub async fn reload_apache() -> Result<bool, ErrorArrayItem> {
    let apache = Services::WebServer;
    apache.reload()
}

pub async fn process_directives(base_path: &str) -> Result<bool, ErrorArrayItem> {
    let directive_paths = scan_directories(base_path).await?;
    let mut config_changed = false;



    for directive_path in directive_paths {
        let directive_id = get_directive_id(PathType::PathBuf(directive_path.clone()));
        
        match parse_directive(&directive_id.into()).await {
            Ok(directive) => match directive {
                Some(directive) => {
                    if create_apache_config(&directive, &directive_path.parent().unwrap())? {
                        config_changed = true;
                    }
                    if let Err(e) = check_apache_ports(&directive).await {
                        eprintln!("Failed to check Apache ports configuration: {}", e);
                    }
                },
                None => {
                    notice("Directive provided was empty");
                },
            }
            Err(e) => return Err(e),
        }
    }

    Ok(config_changed)
}

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let base_path = "/var/www/ais"; // Adjust as necessary

//     loop {
//         match process_directives(base_path).await {
//             Ok(config_changed) => {
//                 if config_changed {
//                     match reload_apache().await {
//                         Ok(b) => {
//                             if !b {
//                                 eprintln!("My god we killed apache, quick email the admin");
//                                 eprintln!("The apache config we rolled out most likely killed apache");
//                             }
//                         }
//                         Err(e) => ErrorArray::new(vec![e]).display(false),
//                     }
//                 }
//                 // report to the aggregator
//                 let status = Status {
//                     app_name: AppName::Apache,
//                     app_status: AppStatus::Running,
//                     timestamp: current_timestamp(),
//                     version: Version::get(),
//                 };
//                 if let Err(err) = report_status(status).await {
//                     ErrorArray::new(vec![err]).display(false);
//                 }
//             }
//             Err(e) => {
//                 eprintln!("Error processing directives: {}", e);
//                 // report to the aggregator
//                 let status = Status {
//                     app_name: AppName::Apache,
//                     app_status: AppStatus::Warning,
//                     timestamp: current_timestamp(),
//                     version: Version::get(),
//                 };

//                 if let Err(err) = report_status(status).await {
//                     ErrorArray::new(vec![err]).display(false);
//                 }
//             }
//         }
//         time::sleep(Duration::from_secs(10)).await; // Adjust the interval as needed
//     }
// }
