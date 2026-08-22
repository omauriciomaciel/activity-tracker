---
type: Service
title: Daemon Service
description: Manages the background collection process lifecycle - foreground/background spawning, PID file, interval loop, and signal handling.
tags: [daemon, lifecycle, pid, tokio, interval, background]
status: stable
---

# Daemon Service

**Module**: `src/daemon.rs` (~137 lines)

The daemon provides the long-running process that triggers periodic collection. It supports
two execution modes and exposes simple stop/status controls.

## Entry Points

```rust
pub async fn run(interval_min: u64, foreground: bool) -> Result<()>
pub fn stop() -> Result<()>
pub fn status() -> Result<()>
pub fn pid_file() -> std::path::PathBuf
```

## Execution Modes

| Mode | Trigger | Behavior |
|---|---|---|
| Background | `at start` (default) | Re-spawns itself with `--foreground`, detaches stdio and a new process group, then the parent exits after printing the PID |
| Foreground | `at start --foreground` (used by systemd/launchd) | Runs the collection loop in-process; exits on Ctrl-C |

### Background spawn

`spawn_background` uses `std::os::unix::process::CommandExt` to set `process_group(0)` and
null stdio, then invokes the current executable (`std::env::current_exe()`) with
`start --foreground --interval N`.

## PID File

Written to `~/.local/share/activity-tracker/daemon.pid` (via
[Storage Layout](../infrastructure/storage-layout.md)). A `PidGuard` RAII guard ensures the
file is removed on drop (clean shutdown). `status()` checks for `/proc/{pid}` on Linux to
detect a stale PID file and removes it if the process is gone.

## Collection Loop

`run_foreground` runs an interval timer (`tokio::time::interval`, `interval_min * 60` seconds):

```text
do_collect()          // immediate first collect
loop {
    tokio::select! {
        _ = tick   => do_collect()
        _ = ctrl_c => break
    }
}
```

`do_collect` reloads [Config](../data/config.md) on every tick (so block-list / ignored-path
changes take effect without restart), then runs [Collector Service](collector.md)
`collect_all` inside `tokio::task::spawn_blocking`. Progress/errors are printed to stderr with
a `[HH:MM:SS]` prefix (routed to `daemon.log` by launchd, or journald by systemd).

## Related

- Calls [Collector Service](collector.md) on each tick
- Reads [Config](../data/config.md) for `blocked_patterns` and `ignored_git_paths`
- PID/log paths defined in [Storage Layout](../infrastructure/storage-layout.md)
- Autostart wiring set up by [Installer](../infrastructure/installer.md)
