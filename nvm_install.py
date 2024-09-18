import subprocess
import os

def install_nvm():
    # Step 1: Download and install NVM using the install script
    try:
        print("Downloading and installing NVM...")
        # Combine the wget and bash commands into a single string
        subprocess.run(
            'wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.4/install.sh | bash',
            check=True,
            shell=True
        )
        
        # Step 2: Update bash profile (or equivalent shell)
        print("Updating environment variables for NVM...")
        bash_profile = os.path.expanduser("~/.bashrc")  # Could also be ~/.zshrc or ~/.profile based on your shell
        
        # Add NVM environment variables to the bash profile
        with open(bash_profile, 'a') as bashrc:
            bashrc.write("\n# Load NVM\n")
            bashrc.write('export NVM_DIR="$HOME/.nvm"\n')

        print(f"NVM environment variables added to {bash_profile}")
        
        # Step 3: Instruct the user to source the profile
        print("Please run the following command to load NVM in your current terminal session:")
        print(f"    source {bash_profile}")
        
        # Alternatively, instruct the user to restart their terminal
        print("Or restart your terminal to apply the changes.")
        
        # Step 4: Verify installation (optional, requires manual sourcing of the bash profile)
        print("Once you've sourced the profile, you can verify the installation with:")
        print("    nvm --version")

        print("NVM installation process complete!")

    except subprocess.CalledProcessError as e:
        print(f"Error during installation: {e}")

if __name__ == "__main__":
    install_nvm()
