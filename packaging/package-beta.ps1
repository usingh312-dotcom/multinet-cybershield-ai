# MultiNet CyberShield AI - Beta Distribution Packaging Script
# Milestone: NM-07.0.1 (Release Integrity Remediation)
# Enforces strict zero-placeholder policy and bundles genuine release assets
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
Write-Host "Version: $Version ($ReleaseTag) - Milestone NM-07.0.1" -ForegroundColor Cyan
Write-Host "Policy: StrictZeroPlaceholder (No Stubs Permitted)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$FullOutput = Join-Path $ScriptDir $OutputDir
$FullStaging = Join-Path $ScriptDir $StagingDir
$BetaStaging = Join-Path $ScriptDir "beta-staging"

# Clean and recreate beta staging directory
if (Test-Path $BetaStaging) {
    Remove-Item -Recurse -Force $BetaStaging
}
New-Item -ItemType Directory -Force -Path $BetaStaging | Out-Null
New-Item -ItemType Directory -Force -Path $FullOutput | Out-Null

# -----------------------------------------------------------------------------
# 1. Stage and Validate Installer MSI
# -----------------------------------------------------------------------------
$MsiSource = Join-Path $FullOutput "MultiNetCyberShield-Setup.msi"
if (-not (Test-Path $MsiSource)) {
    $MsiSource = Join-Path $FullStaging "MultiNetCyberShield-Setup.msi"
}
if (-not (Test-Path $MsiSource)) {
    throw "Packaging Failure: Real MultiNetCyberShield-Setup.msi is missing from $FullOutput"
}

$msiItem = Get-Item $MsiSource
if ($msiItem.Length -lt 1000000) {
    throw "Packaging Failure: MultiNetCyberShield-Setup.msi is suspiciously small ($($msiItem.Length) bytes). Real components missing."
}
$msiBytes = [System.IO.File]::ReadAllBytes($MsiSource)
if ($msiBytes[0] -ne 0xD0 -or $msiBytes[1] -ne 0xCF -or $msiBytes[2] -ne 0x11 -or $msiBytes[3] -ne 0xE0) {
    throw "Packaging Failure: MultiNetCyberShield-Setup.msi is not a valid Windows Installer package."
}
Copy-Item $MsiSource (Join-Path $BetaStaging "MultiNetCyberShield-Setup.msi") -Force
Write-Host "[OK] Staged genuine MultiNetCyberShield-Setup.msi ($($msiItem.Length) bytes)" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 2. Stage and Validate Desktop Application Binary
# -----------------------------------------------------------------------------
$DesktopApp = Join-Path $FullStaging "multinet-cybershield-ai.exe"
if (-not (Test-Path $DesktopApp)) {
    $DesktopApp = Join-Path $RepoRoot "target\release\multinet-cybershield-ai.exe"
}
if (-not (Test-Path $DesktopApp)) {
    $DesktopApp = Join-Path $RepoRoot "target\release\multinet-cybershield-ai-desktop.exe"
}
if (-not (Test-Path $DesktopApp)) {
    throw "Packaging Failure: Genuine multinet-cybershield-ai.exe is missing."
}
$desktopItem = Get-Item $DesktopApp
if ($desktopItem.Length -lt 1024) {
    throw "Packaging Failure: multinet-cybershield-ai.exe is suspiciously small ($($desktopItem.Length) bytes). Stub detected."
}
$desktopBytes = [System.IO.File]::ReadAllBytes($DesktopApp)
if ($desktopBytes[0] -ne 0x4D -or $desktopBytes[1] -ne 0x5A) {
    throw "Packaging Failure: multinet-cybershield-ai.exe missing MZ signature."
}
$e_lfanew = [BitConverter]::ToInt32($desktopBytes, 0x3C)
$peSig = [System.Text.Encoding]::ASCII.GetString($desktopBytes[$e_lfanew..($e_lfanew+3)])
if ($peSig -ne "PE`0`0") {
    throw "Packaging Failure: multinet-cybershield-ai.exe missing PE signature."
}
$machine = [BitConverter]::ToUInt16($desktopBytes, $e_lfanew + 4)
if ($machine -ne 0x8664) {
    throw "Packaging Failure: multinet-cybershield-ai.exe architecture is not AMD64 (0x$($machine.ToString('X4')))."
}
Copy-Item $DesktopApp (Join-Path $BetaStaging "multinet-cybershield-ai.exe") -Force
Write-Host "[OK] Staged genuine multinet-cybershield-ai.exe ($($desktopItem.Length) bytes, AMD64)" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 3. Stage and Validate Service Executable
# -----------------------------------------------------------------------------
$SvcApp = Join-Path $FullStaging "net-merge-service.exe"
if (-not (Test-Path $SvcApp)) {
    $SvcApp = Join-Path $RepoRoot "services\net-merge\net-merge-service.exe"
}
if (-not (Test-Path $SvcApp)) {
    throw "Packaging Failure: Genuine net-merge-service.exe is missing."
}
$svcItem = Get-Item $SvcApp
if ($svcItem.Length -lt 1024) {
    throw "Packaging Failure: net-merge-service.exe is suspiciously small ($($svcItem.Length) bytes). Stub detected."
}
$svcBytes = [System.IO.File]::ReadAllBytes($SvcApp)
if ($svcBytes[0] -ne 0x4D -or $svcBytes[1] -ne 0x5A) {
    throw "Packaging Failure: net-merge-service.exe missing MZ signature."
}
Copy-Item $SvcApp (Join-Path $BetaStaging "net-merge-service.exe") -Force
Write-Host "[OK] Staged genuine net-merge-service.exe ($($svcItem.Length) bytes, AMD64)" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 4. Stage and Validate Authentic Wintun Driver DLL
# -----------------------------------------------------------------------------
$WintunDll = Join-Path $FullStaging "wintun.dll"
if (-not (Test-Path $WintunDll)) {
    $WintunDll = Join-Path $RepoRoot "platform\windows\wintun\wintun.dll"
}
if (-not (Test-Path $WintunDll) -or (Get-Item $WintunDll).Length -lt 10000) {
    throw "Packaging Failure: Authentic wintun.dll is missing."
}
$wintunSig = Get-AuthenticodeSignature $WintunDll
if ($wintunSig.Status -ne "Valid" -or -not ($wintunSig.SignerCertificate.Subject -like "*WireGuard LLC*")) {
    throw "Packaging Failure: wintun.dll signature is invalid or not signed by WireGuard LLC."
}
Copy-Item $WintunDll (Join-Path $BetaStaging "wintun.dll") -Force
Write-Host "[OK] Staged authentic wintun.dll ($((Get-Item $WintunDll).Length) bytes, WireGuard LLC signed)" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 5. Stage Documentation & Beta User Guides
# -----------------------------------------------------------------------------
$DocsBeta = Join-Path $RepoRoot "docs\beta"
$DocMap = @{
    "RELEASE_NOTES_v0.1.0-beta.1.md" = "RELEASE_NOTES.md"
    "INSTALL_AND_ROLLBACK.md"        = "INSTALL_AND_ROLLBACK.md"
    "TROUBLESHOOTING.md"             = "TROUBLESHOOTING.md"
}

foreach ($doc in $DocMap.Keys) {
    $src = Join-Path $DocsBeta $doc
    $dest = Join-Path $BetaStaging $DocMap[$doc]
    if (-not (Test-Path $src) -or (Get-Item $src).Length -eq 0) {
        throw "Packaging Failure: Required documentation file missing or empty: $src"
    }
    Copy-Item $src $dest -Force
    Write-Host "[OK] Staged $($DocMap[$doc]) ($((Get-Item $dest).Length) bytes)" -ForegroundColor Green
}

# -----------------------------------------------------------------------------
# 6. Stage CycloneDX SBOM, Inventory & Manifest
# -----------------------------------------------------------------------------
$SbomSource = Join-Path $FullOutput "sbom.json"
if (-not (Test-Path $SbomSource) -or (Get-Item $SbomSource).Length -eq 0) {
    throw "Packaging Failure: CycloneDX sbom.json is missing or empty at $SbomSource"
}
Copy-Item $SbomSource (Join-Path $BetaStaging "sbom.json") -Force
Write-Host "[OK] Staged sbom.json" -ForegroundColor Green

$DepCsvSource = Join-Path $FullOutput "dependency-inventory.csv"
if (-not (Test-Path $DepCsvSource) -or (Get-Item $DepCsvSource).Length -eq 0) {
    throw "Packaging Failure: dependency-inventory.csv is missing or empty at $DepCsvSource"
}
Copy-Item $DepCsvSource (Join-Path $BetaStaging "dependency-inventory.csv") -Force
Write-Host "[OK] Staged dependency-inventory.csv" -ForegroundColor Green

$ManifestSource = Join-Path $FullOutput "release-manifest.json"
if (-not (Test-Path $ManifestSource) -or (Get-Item $ManifestSource).Length -eq 0) {
    throw "Packaging Failure: release-manifest.json is missing or empty at $ManifestSource"
}
Copy-Item $ManifestSource (Join-Path $BetaStaging "release-manifest.json") -Force
Write-Host "[OK] Staged release-manifest.json" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 7. Generate Comprehensive Checksums for Staged Distribution Payload
# -----------------------------------------------------------------------------
Write-Host "`nCalculating Beta Distribution Checksums:" -ForegroundColor Cyan
$ChecksumLines = @()
Get-ChildItem -Path $BetaStaging | Where-Object { $_.Name -ne "checksums.sha256" } | Sort-Object Name | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
    Write-Host "  $($_.Name): $hash ($($_.Length) bytes)"
    $ChecksumLines += "$hash  $($_.Name)"
}
$ChecksumPath = Join-Path $BetaStaging "checksums.sha256"
$ChecksumLines | Set-Content -Path $ChecksumPath
Copy-Item $ChecksumPath (Join-Path $FullOutput "checksums.sha256") -Force
Write-Host "[OK] Wrote payload checksums.sha256" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 8. Create Distribution ZIP Archive
# -----------------------------------------------------------------------------
$ZipOutput = Join-Path $FullOutput "MultiNetCyberShield-$ReleaseTag-Windows-x64.zip"
if (Test-Path $ZipOutput) {
    Remove-Item $ZipOutput -Force
}

Write-Host "`nCompressing Beta Release Archive: $ZipOutput..." -ForegroundColor Cyan
Compress-Archive -Path "$BetaStaging\*" -DestinationPath $ZipOutput -Force

if (-not (Test-Path $ZipOutput) -or (Get-Item $ZipOutput).Length -eq 0) {
    throw "Packaging Failure: Failed to generate zip distribution archive."
}

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

# Clean up temporary staging
Remove-Item -Recurse -Force -Path $BetaStaging -ErrorAction SilentlyContinue
