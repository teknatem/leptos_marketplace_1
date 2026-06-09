<#
.SYNOPSIS
    Regenerates ARCHITECTURE.md from code (source of truth).

.DESCRIPTION
    Builds the project map straight from sources, without compiling and without a DB:
      - Aggregates a0XX    -> crates/contracts/src/domain/<a0XX>/metadata.json (+ dir names)
      - Projections p9XX   -> dir names crates/backend/src/projections/
      - Use-cases u5XX     -> dir names crates/backend/src/usecases/
      - Data schemes dsXX  -> dir names crates/backend/src/data_schemes/
      - Dashboards d4XX    -> dir names crates/backend/src/dashboards/
      - Tasks task0XX      -> file names crates/backend/src/system/tasks/managers/
      - Chart of accounts  -> ACCOUNT_REGISTRY (shared/analytics/account_registry.rs)
      - Turnover classes   -> TURNOVER_CLASSES (shared/analytics/turnover_registry.rs)
      - API routes         -> .route(...) in api/routes.rs

.NOTES
    Run from the repo root:
        powershell -File tools/gen_architecture.ps1
    ARCHITECTURE.md is GENERATED. Edit this script, not the output.
#>

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$out  = New-Object System.Text.StringBuilder
$BT   = [char]96   # backtick, to wrap code spans without in-string escaping

function W([string]$s = '') { [void]$out.AppendLine($s) }
function Q([string]$s)      { return "$BT$s$BT" }   # `value`

# code = "p904", rest = "sales_data" -> "sales data"
function Split-Code([string]$name) {
    if ($name -match '^([a-z]+\d+)[_-](.+)$') {
        [pscustomobject]@{ Code = $Matches[1]; Label = ($Matches[2] -replace '_', ' ') }
    } else {
        [pscustomobject]@{ Code = $name; Label = '' }
    }
}

function Truncate([string]$s, [int]$n) {
    if ([string]::IsNullOrWhiteSpace($s)) { return '' }
    $s = ($s -replace '\s+', ' ').Trim()
    if ($s.Length -le $n) { return $s }
    return $s.Substring(0, $n).TrimEnd() + [char]0x2026
}

# Enumerate a layer's items by prefix and emit a Code | Name table
function Add-Catalog([string]$relDir, [string]$prefix, [string]$title, [bool]$filesNotDirs = $false) {
    $path = Join-Path $root $relDir
    if (-not (Test-Path $path)) { return }
    $items = if ($filesNotDirs) {
        Get-ChildItem $path -File -Filter "$prefix*.rs" | Where-Object { $_.BaseName -ne 'mod' }
    } else {
        Get-ChildItem $path -Directory | Where-Object { $_.Name -match "^$prefix\d" }
    }
    if (-not $items) { return }
    W "## $title"
    W ''
    W '| Code | Name |'
    W '|------|------|'
    foreach ($it in ($items | Sort-Object Name)) {
        $n = if ($filesNotDirs) { $it.BaseName } else { $it.Name }
        $sc = Split-Code $n
        W "| $(Q $sc.Code) | $($sc.Label) |"
    }
    W ''
}

W '# ARCHITECTURE'
W ''
W "> **GENERATED file - do not edit by hand.** Source of truth is the code."
W "> Regenerate: $(Q 'powershell -File tools/gen_architecture.ps1')"
W '> Project object map (aggregates, projections, use-cases, chart of accounts, turnovers, API).'
W ''

# ----- Aggregates a0XX -----
$domainDir = Join-Path $root 'crates/contracts/src/domain'
$aggDirs = Get-ChildItem (Join-Path $root 'crates/backend/src/domain') -Directory |
    Where-Object { $_.Name -match '^a\d' } | Sort-Object Name
W '## Aggregates (a0XX)'
W ''
W '| Index | Entity | Table | Description | Related |'
W '|-------|--------|-------|-------------|---------|'
foreach ($d in $aggDirs) {
    $meta = Join-Path $domainDir (Join-Path $d.Name 'metadata.json')
    $sc = Split-Code $d.Name
    if (Test-Path $meta) {
        $j = Get-Content $meta -Raw -Encoding UTF8 | ConvertFrom-Json
        $entity = if ($j.ui.element_name) { $j.ui.element_name } elseif ($j.entity_name) { $j.entity_name } else { $sc.Label }
        $table  = if ($j.table_name) { $j.table_name } else { '' }
        $desc   = Truncate $j.ai.description 140
        $rel    = if ($j.ai.related) { ($j.ai.related -join ', ') } else { '' }
        W "| $(Q $j.entity_index) | $entity | $(Q $table) | $desc | $rel |"
    } else {
        W "| $(Q $sc.Code) | $($sc.Label) | | _(no metadata.json)_ | |"
    }
}
W ''

# ----- Name-based catalogs -----
Add-Catalog 'crates/backend/src/projections'           'p'    'Projections (p9XX)'
Add-Catalog 'crates/backend/src/usecases'              'u'    'Use-cases (u5XX)'
Add-Catalog 'crates/backend/src/data_schemes'          'ds'   'Data schemes (dsXX)'
Add-Catalog 'crates/backend/src/dashboards'            'd'    'Dashboards (d4XX)'
Add-Catalog 'crates/backend/src/system/tasks/managers' 'task' 'Scheduled tasks (task0XX)' $true

# ----- Chart of accounts -----
$accFile = Join-Path $root 'crates/backend/src/shared/analytics/account_registry.rs'
if (Test-Path $accFile) {
    $txt = Get-Content $accFile -Raw -Encoding UTF8
    $blocks = [regex]::Matches($txt, 'AccountDef\s*\{(.+?)\}', 'Singleline')
    if ($blocks.Count -gt 0) {
        W '## Chart of accounts (account_registry)'
        W ''
        W '| Account | Name | Parent | Section |'
        W '|---------|------|--------|---------|'
        foreach ($b in $blocks) {
            $body = $b.Groups[1].Value
            $code   = if ($body -match 'code:\s*"([^"]*)"')   { $Matches[1] } else { '' }
            $name   = if ($body -match 'name:\s*"([^"]*)"')   { $Matches[1] } else { '' }
            $parent = if ($body -match 'parent_code:\s*Some\("([^"]+)"\)') { $Matches[1] } else { '' }
            $sect   = if ($body -match 'section:\s*StatementSection::(\w+)') { $Matches[1] } else { '' }
            W "| $(Q $code) | $name | $parent | $sect |"
        }
        W ''
    }
}

# ----- Turnover classes -----
$turnFile = Join-Path $root 'crates/backend/src/shared/analytics/turnover_registry.rs'
if (Test-Path $turnFile) {
    $txt = Get-Content $turnFile -Raw -Encoding UTF8
    $blocks = [regex]::Matches($txt, 'TurnoverClassDef\s*\{(.+?)\}', 'Singleline')
    if ($blocks.Count -gt 0) {
        W '## Turnover classes (turnover_registry)'
        W ''
        W '| Code | Name | Debit | Credit | Entry |'
        W '|------|------|-------|--------|-------|'
        foreach ($b in $blocks) {
            $body = $b.Groups[1].Value
            $code = if ($body -match 'code:\s*"([^"]*)"') { $Matches[1] } else { '' }
            $name = if ($body -match 'name:\s*"([^"]*)"') { $Matches[1] } else { '' }
            $deb  = if ($body -match 'debit_account:\s*"([^"]*)"')  { $Matches[1] } else { '' }
            $cred = if ($body -match 'credit_account:\s*"([^"]*)"') { $Matches[1] } else { '' }
            $je   = if ($body -match 'generates_journal_entry:\s*true') { [char]0x2713 } else { '' }
            W "| $(Q $code) | $name | $deb | $cred | $je |"
        }
        W ''
    }
}

# ----- API routes -----
$routesFile = Join-Path $root 'crates/backend/src/api/routes.rs'
if (Test-Path $routesFile) {
    $txt = Get-Content $routesFile -Raw -Encoding UTF8
    $rx = [regex]'\.route\(\s*"(?<path>[^"]+)"\s*,(?<body>[\s\S]*?)\n\s*\)'
    $rows = @()
    foreach ($m in $rx.Matches($txt)) {
        $p = $m.Groups['path'].Value
        $verbs = [regex]::Matches($m.Groups['body'].Value, '\b(get|post|put|delete|patch)\(') |
            ForEach-Object { $_.Groups[1].Value.ToUpper() } | Select-Object -Unique
        $seg = if ($p -match '^/api/([^/]+)') { $Matches[1] } else { $p }
        $rows += [pscustomobject]@{ Group = $seg; Path = $p; Verbs = ($verbs -join ' ') }
    }
    if ($rows.Count -gt 0) {
        W "## API routes ($($rows.Count))"
        W ''
        foreach ($g in ($rows | Group-Object Group | Sort-Object Name)) {
            W "### $(Q ('/' + $g.Name))"
            foreach ($r in ($g.Group | Sort-Object Path)) {
                W "- $(Q $r.Verbs) $($r.Path)"
            }
            W ''
        }
    }
}

$dest = Join-Path $root 'ARCHITECTURE.md'
$enc  = New-Object System.Text.UTF8Encoding($true)   # UTF-8 with BOM
[System.IO.File]::WriteAllText($dest, $out.ToString(), $enc)
Write-Host "ARCHITECTURE.md regenerated: $dest"
