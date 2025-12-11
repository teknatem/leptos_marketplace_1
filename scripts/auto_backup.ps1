# Auto Backup Script - Run before starting the application
# This creates a backup only if database has changed since last backup

$ProjectRoot = "E:\dev\rust\leptos_marketplace_1"
$DbPath = "$ProjectRoot\data\app.db"
$BackupDir = "$ProjectRoot\data\backups"
$LatestBackup = "$BackupDir\app_latest.db"

# Check if database exists
if (-not (Test-Path $DbPath)) {
    Write-Host "Database not found, no backup needed." -ForegroundColor Yellow
    exit 0
}

# Create backup directory if needed
if (-not (Test-Path $BackupDir)) {
    New-Item -ItemType Directory -Path $BackupDir -Force | Out-Null
}

# Check if we need a backup (compare with latest)
$NeedsBackup = $true
if (Test-Path $LatestBackup) {
    $DbHash = (Get-FileHash $DbPath -Algorithm MD5).Hash
    $BackupHash = (Get-FileHash $LatestBackup -Algorithm MD5).Hash
    
    if ($DbHash -eq $BackupHash) {
        $NeedsBackup = $false
        Write-Host "✓ Database unchanged, no backup needed." -ForegroundColor Gray
    }
}

if ($NeedsBackup) {
    Write-Host "Creating automatic backup..." -ForegroundColor Cyan
    & "$ProjectRoot\scripts\backup_db.ps1"
}

