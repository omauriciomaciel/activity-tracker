---
type: Service
title: Updater Service
description: Self-update mechanism that queries GitHub Releases, downloads the matching platform asset, extracts the binary, and atomically replaces the running executable.
tags: [updater, self-update, github-releases, tar.gz, zip, atomic-rename]
status: stable
---

# Updater Service

**Module**: `src/updater.rs` (~161 lines)

Implements `at update`: checks GitHub for a newer release, downloads the appropriate
pre-built binary, extracts it from the archive, and replaces the current executable in place.

## Entry Point

```rust
pub fn run() -> Result<()>
```

## Flow

1. Determine the repo path from `CARGO_PKG_REPOSITORY` (strips `https://github.com/`).
2. GET `https://api.github.com/repos/{repo}/releases/latest` with a `activity-tracker/{ver}`
   user agent.
3. Compare the latest tag (stripped of leading `v`) against `CARGO_PKG_VERSION`. If equal,
   print "already up to date" and exit.
4. Compute the asset name from `std::env::consts::{OS, ARCH}`:
   - linux -> `activity-tracker-{ver}-linux-{arch}.tar.gz`
   - macos -> `activity-tracker-{ver}-macos-{arch}.tar.gz`
   - windows -> `activity-tracker-{ver}-windows-{arch}.zip`
5. Find the matching asset in the release and download its `browser_download_url`.
6. Extract the `activity-tracker` binary:
   - `.tar.gz` via `flate2` + `tar` (`extract_from_targz`)
   - `.zip` via `zip` crate (`extract_from_zip`)
7. Resolve `std::env::current_exe()` (canonicalized) and write the new bytes to a sibling
   `{exe}_new` temp file on the same filesystem.
8. Set mode `0o755` on Unix, then `std::fs::rename` the temp file over the real binary -
   **atomic** on the same filesystem, so there is never a window without a binary.

## Data Structures

```rust
struct Release { tag_name: String, assets: Vec<Asset> }
struct Asset   { name: String, browser_download_url: String }
```

## Related

- Release artifacts produced by [CI/CD Pipeline](../infrastructure/ci-cd.md)
- Installer (`install.sh`) also pulls from the same GitHub Releases endpoint: [Installer](../infrastructure/installer.md)
