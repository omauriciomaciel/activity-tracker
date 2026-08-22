---
type: Infrastructure
title: Installer
description: Cross-platform POSIX shell installer that downloads the latest pre-built binary from GitHub Releases, installs it, configures autostart (macOS LaunchAgent or Linux systemd), adds shell aliases, and prompts for macOS permissions.
tags: [installer, install-sh, launchagent, systemd, autostart, posix]
status: stable
---

# Installer

**File**: `install.sh` (~256 lines, POSIX `sh`)

One-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/omauriciomaciel/activity-tracker/main/install.sh | sh
```

## Flow

1. **Detect platform**: `uname -s` (Linux/Darwin) and `uname -m` (x86_64/aarch64). Rejects
   unsupported OS/arch.
2. **Check dependencies**: requires `curl`; warns if `ollama` is missing (default provider
   won't work without it).
3. **Fetch latest release tag** from `https://api.github.com/repos/{REPO}/releases/latest`.
4. **Download** `activity-tracker-{ver}-{os}-{arch}.tar.gz` from the release assets and
   extract the `activity-tracker` binary.
5. **Install** to `${INSTALL_DIR:-$HOME/.local/bin}` with mode 755.
6. **PATH check**: warns if `INSTALL_DIR` is not in `PATH` and prints the export line.
7. **Configure autostart** (platform-specific, below).
8. **Add shell aliases** to `.zshrc` / `.bashrc` / `.bash_profile` (skips if `alias at=`
   already present):
   ```sh
   alias at='activity-tracker'
   alias ats='activity-tracker summary'
   ```

## Custom Install Directory

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL .../install.sh | sh
```

## Autostart: macOS (launchd)

Creates `~/Library/LaunchAgents/com.activity-tracker.plist`:

- `ProgramArguments`: `{INSTALL_DIR}/activity-tracker start --foreground`
- `RunAtLoad: true`, `KeepAlive` on non-zero exit
- stdout/stderr -> `~/.local/share/activity-tracker/daemon.log`
- Loads it via `launchctl load -w`, after unloading any existing one.

Then (when interactive) opens System Settings to **Full Disk Access** and **Accessibility**
and waits for the user to grant each.

## Autostart: Linux (systemd)

Creates `~/.config/systemd/user/activity-tracker.service`:

```ini
[Unit]
Description=Activity Tracker daemon
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart={INSTALL_DIR}/activity-tracker start --foreground
Restart=on-failure
RestartSec=30
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

Reloads the daemon, enables + starts the service. Recommends
`loginctl enable-linger $USER` for autostart without an active graphical session.

## macOS Permissions

The installer opens the relevant System Settings panes and prompts the user to add the binary:

- **Full Disk Access** - to read Chrome/Brave SQLite history ([Browser Tabs Capture](../data-sources/browser-tabs.md))
- **Accessibility** - for `osascript` window titles ([Open Windows Capture](../data-sources/open-windows.md))

When piped (non-interactive), it prints manual instructions instead.

## Related

- Downloads from [CI/CD Pipeline](ci-cd.md) release artifacts
- Wires up [Daemon Service](../services/daemon.md) autostart
- Daemon writes to [Storage Layout](storage-layout.md) paths
