# Artisan_Platform

**Artisan_Platform** is a comprehensive Rust-based tool designed for managing automatic code pulling and deployment on Debian/Ubuntu systems. It integrates various functionalities, from handling GitHub repository updates to monitoring security logs and managing server configurations, all with a built-in remote server for real-time monitoring.

## Features

### 1. Automatic Code Pulling & Deployment
- Automatically monitors and pulls updates from GitHub repositories.
- Seamlessly deploys the latest code updates on Debian/Ubuntu servers.

### 2. Apache Configuration Management
- Creates and modifies Apache configurations on the fly.
- Ensures smooth updates and modifications to virtual host settings without manual intervention.

### 3. Security Monitoring
- Monitors `auditd` logs for auditing and event monitoring.
- Tracks `ssh` logs to monitor access attempts and security breaches.

### 4. Remote Status Monitoring
- Built-in server allows clients to connect remotely.
- Displays status information for different components running on the server.
- Enables real-time monitoring of key operations and logs.

### 5. Viewer Client
- Artisan_Platform includes a viewer client that provides a terminal-based user interface for monitoring.
- If you only need the viewer client, you can build and use it separately by running:
   ```bash
   cargo build --release --bin artisan_manager_tui
   ```

## Usage

### Platform Usage
Artisan_Platform runs on infrastructure provided by Artisan Hosting, so installation on your system is not necessary.

Once Artisan_Platform is running, it will automatically:
- Monitor the specified GitHub repositories for new commits and pull the latest code.
- Update Apache configurations dynamically as needed.
- Monitor security logs in real-time and flag any suspicious activity.
- Allow remote clients to view the status of the system via the built-in server interface.

### Monitoring Components
To monitor components using the terminal-based viewer, call the program:
```bash
artisan_tui
```
This provides access to:
- GitHub pull and deployment status
- Apache config status
- Security events from `auditd` and `ssh` logs

## Requirements

- Debian/Ubuntu-based system
- Rust (for building the application if needed)
- Apache server installed
- `auditd` and SSH enabled for monitoring (on the Artisan Hosting infrastructure)

## License

This project is licensed under the ___ License - see the [LICENSE](LICENSE) file for details.

-thank chatgpt for this readme
