---
type: Data Source
title: Open Windows Capture
description: Multi-platform capture of open window titles using a fallback chain of WSL PowerShell, wmctrl, xdotool, osascript, and ps.
tags: [windows, wmctrl, xdotool, osascript, ps, wsl, data-source]
status: stable
---

# Open Windows Capture

**Source of**: `apps` [Log Entry](../data/log-entry.md) records
**Implemented in**: `capture_open_windows` ([Collector Service](../services/collector.md))

Captures the titles/names of currently open application windows using the first available
method in a platform-aware fallback chain. Produces a single `apps` entry per collection run.

## Fallback Chain (in order)

| # | Method | Platform | Source |
|---|---|---|---|
| 1 | `powershell.exe` interop | WSL | `Get-Process` where `MainWindowTitle` is non-empty, JSON output |
| 2 | `wmctrl -l` | Linux (X11) | Parses `0x.. desktop host title` lines, strips hostname prefix |
| 3 | `xdotool` | Linux (X11) | `search --onlyvisible --name ""` then `getwindowname` per id; capped at 30 |
| 4 | `osascript` | macOS | System Events: `name of every application process whose visible is true` |
| 5 | `ps axo comm` | Linux (last resort) | Filters process names against a GUI-app allowlist |

The chain short-circuits: each step only runs if `windows` is still empty, so a successful
earlier method wins.

## WSL Detection

`is_wsl()` reads `/proc/version` and checks for `microsoft` or `wsl` (case-insensitive). When
true, the PowerShell interop path runs first so that Windows-side windows are visible.

## Linux `ps` Allowlist

When no window manager tool is available, the final fallback matches running process names
against a hardcoded GUI-app hint set: `code`, `firefox`, `chrome`, `chromium`, `brave`,
`slack`, `discord`, `spotify`, `telegram`, `nautilus`, `thunar`, `alacritty`, `kitty`,
`wezterm`, `gnome-terminal`, `konsole`, `tilix`, `obs`, `gimp`, `inkscape`, `blender`,
`libreoffice`, `thunderbird`, `signal`, `vlc`, `mpv`.

## Filtering

Every captured title is checked against [Config](../data/config.md) `blocked_patterns`
(case-insensitive substring); blocked titles are dropped. Hostname prefixes are stripped by
`strip_hostname_prefix` at aggregation time.

## Output

```json
{ "type": "apps", "ts": "2026-08-22T14:30:00", "windows": ["src/main.rs - activity-tracker - Code", "Firefox"] }
```

## Platform Permissions

- **macOS**: requires **Accessibility** permission for `osascript` to read window titles
  (configured by the [Installer](../infrastructure/installer.md)).
- **Linux X11**: best results with `wmctrl` installed (`sudo apt install wmctrl`).

## Related

- Part of the [Collection Pipeline](../pipeline/collection.md)
- Aggregated into app counts by [Summarizer Service](../services/summarizer.md)
