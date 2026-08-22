---
type: Pipeline
title: Self-Update Pipeline
description: Checks GitHub Releases for a newer version, downloads the matching platform asset, extracts the binary, and atomically replaces the running executable.
tags: [pipeline, update, github-releases, atomic-rename]
status: stable
---

# Self-Update Pipeline

Triggered by `at update`. Implemented in `updater::run`
([Updater Service](../services/updater.md)).

## Steps

```text
1. Resolve repo path from CARGO_PKG_REPOSITORY
   └─ "https://github.com/omauriciomaciel/activity-tracker" -> "omauriciomaciel/activity-tracker"

2. GET https://api.github.com/repos/{repo}/releases/latest
   └─ User-Agent: activity-tracker/{current_version}

3. Compare latest tag (strip 'v') vs CARGO_PKG_VERSION
   └─ if equal -> "already up to date", exit

4. Compute asset name from OS + ARCH
   ├─ linux  -> activity-tracker-{ver}-linux-{arch}.tar.gz
   ├─ macos  -> activity-tracker-{ver}-macos-{arch}.tar.gz
   └─ windows-> activity-tracker-{ver}-windows-{arch}.zip

5. Find matching asset, download browser_download_url bytes

6. Extract binary
   ├─ .tar.gz -> flate2 + tar (extract_from_targz)
   └─ .zip    -> zip crate    (extract_from_zip)

7. Atomic replace
   ├─ dest = canonicalize(current_exe)
   ├─ tmp  = dest.with_extension("_new")   # same filesystem
   ├─ write bytes to tmp
   ├─ chmod 0o755 (Unix)
   └─ rename(tmp, dest)                     # atomic on same fs
```

## Safety

- The temp file is written on the **same filesystem** as the target so `rename` is atomic;
  there is never a moment when no binary exists at the destination.
- Permissions are set before the rename so the new binary is immediately executable.

## Related Artifacts

The downloaded assets are produced by the [CI/CD Pipeline](../infrastructure/ci-cd.md) build
workflow and published to GitHub Releases by `semantic-release`.

## Related

- Implemented by [Updater Service](../services/updater.md)
- Consumes [CI/CD Pipeline](../infrastructure/ci-cd.md) release artifacts
- Same release source used by [Installer](../infrastructure/installer.md)
