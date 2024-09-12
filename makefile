# Define variables
PROJECT_NAME := artisan_platform
BUILD_DIR := target/release
BIN_DIR := /opt/artisan_platform/bin
BINARIES := ais_aggregator ais_gitmon ais_services ais_directive ais_security ais_manager
#UNIFIED_NAMES := aggregator git_monitor services apache security manager

# Default task check
all: update build copy create_user install_deps configure_sshd setcap_permissions create_directory configure_auditd create_service_files configure_git

# Update dependencies
update:
	cargo update

# Check the project
check:
	cargo check

# Build the project
build:
	cargo build --release

# Copy binaries to the designated directory with unified names
copy:
	@-systemctl stop ais_*
	mkdir -p $(BIN_DIR)
	cp $(BUILD_DIR)/ais_aggregator $(BIN_DIR)/ais_aggregator
	cp $(BUILD_DIR)/ais_gitmon $(BIN_DIR)/ais_gitmon
	cp $(BUILD_DIR)/ais_services $(BIN_DIR)/ais_services
	cp $(BUILD_DIR)/ais_directive $(BIN_DIR)/ais_directive
	cp $(BUILD_DIR)/ais_security $(BIN_DIR)/ais_security
	cp $(BUILD_DIR)/ais_manager $(BIN_DIR)/ais_manager
	cp $(BUILD_DIR)/ais_credentials $(BIN_DIR)/credentials
	cp $(BUILD_DIR)/ais_welcome $(BIN_DIR)/welcome

# Create system user and group
create_user:
	@-useradd -r -s /bin/false ais
	@-groupadd ais
	@-usermod -a -G ais ais
	@-id -g dusa &>/dev/null || groupadd dusa
	@-usermod -a -G dusa ais

# Install dependencies if apt is available
install_deps:
	@command -v apt >/dev/null 2>&1 && apt update && apt install -y auditd audispd-plugins openssh-server git inotify-tools || echo "apt not found, skipping dependency installation"

# Configure sshd to set Debug level to LOGLEVEL2
configure_sshd:
	@sed -i 's/^#LogLevel INFO/LogLevel DEBUG2/' /etc/ssh/sshd_config
	@systemctl restart sshd

# Set capabilities for specific binaries
setcap_permissions:
	@setcap 'cap_chown,cap_fowner=eip' $(BIN_DIR)/ais_gitmon
	@setcap 'cap_chown,cap_fowner=eip' $(BIN_DIR)/ais_directive
	@setcap 'cap_chown,cap_fowner=eip' $(BIN_DIR)/ais_security

# Create directory and set ownership
create_directory:
	@mkdir -p /var/www/ais
	@chown ais:ais /var/www/ais

# Configure auditd with specific rules
configure_auditd:
	@echo "-a always,exit -F arch=b64 -S execve -k command_exec" >> /etc/audit/rules.d/audit.rules
	@echo "-a always,exit -F arch=b32 -S execve -k command_exec" >> /etc/audit/rules.d/audit.rules
	@systemctl kill auditd
	@systemctl start auditd

# Create systemd service files
create_service_files:
	@echo "Creating systemd service files"
	@for bin in $(BINARIES); do \
		echo "[Unit]" > /etc/systemd/system/$${bin}.service; \
		echo "Description=Service for $${bin}" >> /etc/systemd/system/$${bin}.service; \
		echo "After=network.target" >> /etc/systemd/system/$${bin}.service; \
		echo "Wants=aggregator.service" >> /etc/systemd/system/$${bin}.service; \
		echo "[Service]" >> /etc/systemd/system/$${bin}.service; \
		echo "Type=simple" >> /etc/systemd/system/$${bin}.service; \
		echo "ExecStart=/opt/artisan_platform/bin/$${bin}" >> /etc/systemd/system/$${bin}.service; \
		echo "Restart=on-failure" >> /etc/systemd/system/$${bin}.service; \
		echo "RestartSec=5" >> /etc/systemd/system/$${bin}.service; \
		echo "StartLimitInterval=10m" >> /etc/systemd/system/$${bin}.service; \
		echo "StartLimitBurst=5" >> /etc/systemd/system/$${bin}.service; \
		echo "User=root" >> /etc/systemd/system/$${bin}.service; \
		echo "Group=root" >> /etc/systemd/system/$${bin}.service; \
		echo "StandardOutput=file:/var/log/$${bin}.log" >> /etc/systemd/system/$${bin}.service; \
		echo "StandardError=file:/var/log/$${bin}.log" >> /etc/systemd/system/$${bin}.service; \
		echo "[Install]" >> /etc/systemd/system/$${bin}.service; \
		echo "WantedBy=multi-user.target" >> /etc/systemd/system/$${bin}.service; \
		echo "Created /etc/systemd/system/$${bin}.service"; \
	done
	@systemctl daemon-reload
	@for bin in $(BINARIES); do \
		systemctl enable $${bin}.service; \
		systemctl start $${bin}.service; \
	done

# Configure Git to disable safe directory checks
configure_git:
	@git config --system safe.directory '*'

# Clean the project
clean:
	cargo clean

# Run tests
test:
	cargo test

.PHONY: all update check build copy create_user install_deps configure_sshd setcap_permissions create_directory configure_auditd create_service_files configure_git clean test