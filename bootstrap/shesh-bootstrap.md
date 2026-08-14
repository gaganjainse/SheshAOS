# Shesh Bootstrap Script

This script bootstraps and updates a Ubuntu workstation for AI and software development.

## Modes

- `--install` install and configure common tools
- `--update` refresh apt, snap, flatpak, and run Topgrade
- `--full` do both

## Usage

```bash
chmod +x shesh-bootstrap.sh
./shesh-bootstrap.sh
```

## What it does

- Installs common development tools
- Creates the `~/Workspace` layout
- Configures shell helpers
- Configures Git defaults
- Updates apt, snap, flatpak, and firmware metadata
- Runs `topgrade` if available
- Leaves user files alone
