# Database Backup Scripts

## Overview

This directory contains scripts for backing up and restoring the marketplace database.

## Problem
The default database location (`target/db/app.db`) can be **deleted** when running:
- `cargo clean`
- Clearing build artifacts
- Switching git branches with different build states

## Solution
1. **config.toml** moves database to `data/app.db` (outside target/)
2. **Automatic backups** before each run
3. **Manual backup/restore scripts**

## Quick Start

### 1. Initial Setup (First Time Only)
```powershell
# Copy your current database to the new location
Copy-Item "target\db\app.db" "data\app.db"

# Or if you have old data in crates\backend\target\db\:
Copy-Item "crates\backend\target\db\app.db" "data\app.db"
```

### 2. Daily Usage

**Before starting work:**
```powershell
.\scripts\auto_backup.ps1
```

**Manual backup anytime:**
```powershell
.\scripts\backup_db.ps1
```

**Restore from backup:**
```powershell
.\scripts\restore_db.ps1
# Then select from list of available backups
```

## Files

| File | Purpose |
|------|---------|
| `config.toml` | Configures database path (outside target/) |
| `backup_db.ps1` | Creates timestamped backup |
| `restore_db.ps1` | Restores database from backup |
| `auto_backup.ps1` | Auto-backup if database changed |

## Backup Location

```
leptos_marketplace_1/
├── data/
│   ├── app.db                          # Main database
│   └── backups/
│       ├── app_backup_2025-12-10_14-30-00.db
│       ├── app_backup_2025-12-10_15-45-00.db
│       └── app_latest.db               # Most recent backup
```

## Backup Retention

- Keeps **last 30 backups** automatically
- Older backups are automatically deleted
- `app_latest.db` is always the most recent

## Integration with Development

Add to your workflow:

```powershell
# Start backend (with auto-backup)
.\scripts\auto_backup.ps1; cargo run --bin backend

# Or create a convenience script
.\run_backend.ps1
```

## Scheduled Backups (Optional)

Create a Windows Task Scheduler task to run `backup_db.ps1` daily:

```powershell
# Run as Administrator
$Action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-File E:\dev\rust\leptos_marketplace_1\scripts\backup_db.ps1"
$Trigger = New-ScheduledTaskTrigger -Daily -At 9am
Register-ScheduledTask -TaskName "Marketplace DB Backup" -Action $Action -Trigger $Trigger
```

## Emergency Recovery

If you lost data:

1. Check `data\backups\` for recent backups
2. Run `.\scripts\restore_db.ps1`
3. Select the backup to restore
4. Your current DB is automatically backed up before restore

## .gitignore

Database files are excluded from git:
```
/data/app.db
/data/backups/*.db
/target/db/*.db
```

But scripts and config are tracked for team sharing.

