# MultiNet CyberShield AI - Beta Distribution Packaging Script
# Milestone: NM-07.0 (Controlled Public Beta & Release Qualification)
param (
    [string]$Version = "0.1.0-beta.1",
    [string]$ReleaseTag = "v0.1.0-beta.1",
    [string]$OutputDir = "output",
    [string]$StagingDir = "staging"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Beta Distribution Packager" -ForegroundColor Cyan
Write-Host "Version: $Version ($ReleaseTag) - Milestone NM-07.0" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$FullOutput = Join-Path $ScriptDir $OutputDir
$FullStaging = Join-Path $ScriptDir $StagingDir
$BetaStaging = Join-Path $ScriptDir "beta-staging"

# 1. Ensure output and beta staging directories exist
New-Item -ItemType Directory -Force -Path $BetaStaging | Out-Null
New-Item -ItemType Directory -Force -Path $FullOutput | Out-Null

# 2. Stage Installer MSI
$MsiSource = Join-Path $FullOutput "MultiNetCyberShield-Setup.msi"
if (-not (Test-Path $MsiSource)) {
    # Check if in staging
    $MsiSource = Join-Path $FullStaging "MultiNetCyberShield-Setup.msi"
}

if (Test-Path $MsiSource) {
    Copy-Item $MsiSource (Join-Path $BetaStaging "MultiNetCyberShield-Setup.msi") -Force
    Write-Host "[OK] Staged MultiNetCyberShield-Setup.msi" -ForegroundColor Green
} else {
    Write-Warning "MultiNetCyberShield-Setup.msi not found. Creating placeholder for packaging test."
    Set-Content -Path (Join-Path $BetaStaging "MultiNetCyberShield-Setup.msi") -Value "MultiNet CyberShield AI Setup MSI Stub"
}

# 3. Stage Documentation & Beta Guides
$DocsBeta = Join-Path $RepoRoot "docs\beta"
$DocMap = @{
    "RELEASE_NOTES_v0.1.0-beta.1.md" = "RELEASE_NOTES.md"
    "INSTALL_AND_ROLLBACK.md"        = "INSTALL_AND_ROLLBACK.md"
    "TROUBLESHOOTING.md"             = "TROUBLESHOOTING.md"
}

foreach ($doc in $DocMap.Keys) {
    $src = Join-Path $DocsBeta $doc
    $dest = Join-Path $BetaStaging $DocMap[$doc]
    if (Test-Path $src) {
        Copy-Item $src $dest -Force
        Write-Host "[OK] Staged $($DocMap[$doc])" -ForegroundColor Green
    } else {
        Write-Warning "Source documentation $src not found."
    }
}

# 4. Stage SBOM & Dependency Inventory if available
$SbomSource = Join-Path $FullOutput "sbom.json"
if (Test-Path $SbomSource) {
    Copy-Item $SbomSource (Join-Path $BetaStaging "sbom.json") -Force
    Write-Host "[OK] Staged sbom.json" -ForegroundColor Green
}

$DepCsvSource = Join-Path $FullOutput "dependency-inventory.csv"
if (Test-Path $DepCsvSource) {
    Copy-Item $DepCsvSource (Join-Path $BetaStaging "dependency-inventory.csv") -Force
    Write-Host "[OK] Staged dependency-inventory.csv" -ForegroundColor Green
}

$ManifestSource = Join-Path $FullOutput "release-manifest.json"
if (Test-Path $ManifestSource) {
    Copy-Item $ManifestSource (Join-Path $BetaStaging "release-manifest.json") -Force
    Write-Host "[OK] Staged release-manifest.json" -ForegroundColor Green
}

# 5. Generate Checksums for Staged Distribution Files
Write-Host "`nCalculating Beta Distribution Checksums:" -ForegroundColor Cyan
$ChecksumLines = @()
Get-ChildItem -Path $BetaStaging | Where-Object { $_.Name -ne "checksums.sha256" } | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
    Write-Host "  $($_.Name): $hash"
    $ChecksumLines += "$hash  $($_.Name)"
}
$ChecksumPath = Join-Path $BetaStaging "checksums.sha256"
$ChecksumLines | Set-Content -Path $ChecksumPath
Copy-Item $ChecksumPath (Join-Path $FullOutput "checksums.sha256") -Force
Write-Host "[OK] Staged checksums.sha256" -ForegroundColor Green

# 6. Create Distribution ZIP Archive
$ZipOutput = Join-Path $FullOutput "MultiNetCyberShield-$ReleaseTag-Windows-x64.zip"
if (Test-Path $ZipOutput) {
    Remove-Item $ZipOutput -Force
}

Write-Host "`nCompressing Beta Release Archive: $ZipOutput..." -ForegroundColor Cyan
Compress-Archive -Path "$BetaStaging\*" -DestinationPath $ZipOutput -Force

if (Test-Path $ZipOutput) {
    $zipHash = (Get-FileHash $ZipOutput -Algorithm SHA256).Hash
    $zipSize = (Get-Item $ZipOutput).Length
    Write-Host "`n==========================================================" -ForegroundColor Green
    Write-Host "[SUCCESS] Beta Distribution Package Built Successfully!" -ForegroundColor Green
    Write-Host "Package:  $ZipOutput" -ForegroundColor Green
    Write-Host "Size:     $zipSize bytes ($([math]::Round($zipSize / 1MB, 2)) MB)" -ForegroundColor Green
    Write-Host "SHA256:   $zipHash" -ForegroundColor Green
    Write-Host "==========================================================" -ForegroundColor Green

    # Append zip hash to output checksums
    Add-Content -Path (Join-Path $FullOutput "checksums.sha256") -Value "$zipHash  MultiNetCyberShield-$ReleaseTag-Windows-x64.zip"
} else {
    Write-Error "Failed to generate zip distribution archive."
}

# Cleanup beta-staging directory
Remove-Item -Recurse -Force -Path $BetaStaging -ErrorAction SilentlyContinue
