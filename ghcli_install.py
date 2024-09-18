import subprocess
import os

def install_gh_cli():
    # Step 1: Download and install the latest GitHub CLI
    try:
        print("Downloading and installing GitHub CLI...")
        
        # Determine the platform and install the appropriate package
        platform = subprocess.run(['uname', '-s'], capture_output=True, text=True).stdout.strip()

        if platform == 'Linux':
            # Install GitHub CLI on Linux using the official apt or yum repos
            print("Detected Linux platform. Installing GitHub CLI...")
            
            # For Debian/Ubuntu based systems
            subprocess.run(
                'type -p curl >/dev/null || apt install curl software-properties-common -y', shell=True, check=True
            )
            subprocess.run(
                'curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | '
                'dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg', shell=True, check=True
            )
            subprocess.run(
                'apt-add-repository https://cli.github.com/packages', shell=True, check=True
            )
            subprocess.run(
                'apt update && apt install gh -y', shell=True, check=True
            )

        elif platform == 'Darwin':
            # Install GitHub CLI on macOS using Homebrew
            print("Detected macOS platform. Installing GitHub CLI with Homebrew...")
            subprocess.run(
                'brew install gh', shell=True, check=True
            )
        else:
            print("Unsupported platform. Exiting.")
            return

        # Step 2: Add `gh` to PATH (if not already present)
        gh_path_check = subprocess.run('which gh', shell=True, capture_output=True, text=True)
        if gh_path_check.returncode != 0:
            print("GitHub CLI (gh) is not in your PATH. Updating environment variables...")
            bash_profile = os.path.expanduser("~/.bashrc")  # Adjust this for other shells (e.g., ~/.zshrc)

            with open(bash_profile, 'a') as bashrc:
                bashrc.write("\n# Add GitHub CLI to PATH\n")
                bashrc.write('export PATH="/usr/local/bin:$PATH"\n')

            print(f"Environment variables updated. Please run 'source {bash_profile}' to refresh the environment.")

        # Step 3: Verify installation
        print("Verifying GitHub CLI installation...")
        subprocess.run(['gh', '--version'], check=True)

        print("GitHub CLI installation successful!")

    except subprocess.CalledProcessError as e:
        print(f"Error during installation: {e}")

if __name__ == "__main__":
    install_gh_cli()
