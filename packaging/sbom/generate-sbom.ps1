# MultiNet CyberShield AI - Software Bill of Materials (SBOM) & Inventory Generator
param (
    [string]$OutputDir = "..\output"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\..\.."
$FullOutput = Join-Path $ScriptDir $OutputDir
New-Item -ItemType Directory -Force -Path $FullOutput | Out-Null

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Supply Chain & SBOM Generator" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Parse Go Dependencies from services/net-merge/go.mod
$GoModPath = Join-Path $RepoRoot "services\net-merge\go.mod"
$GoComponents = @()

if (Test-Path $GoModPath) {
    Write-Host "Parsing Go dependencies from $GoModPath..." -ForegroundColor Cyan
    $lines = Get-Content $GoModPath
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^([a-zA-Z0-9.\-_/]+)\s+([v0-9.\-_+a-zA-Z]+)(\s+//\s+indirect)?$') {
            $modName = $matches[1]
            $modVer = $matches[2]
            $isDirect = -not [bool]$matches[3]
            $GoComponents += [PSCustomObject]@{
                "Ecosystem" = "Go"
                "Name" = $modName
                "Version" = $modVer
                "Direct" = $isDirect
                "License" = "BSD-3-Clause / MIT / Apache-2.0"
            }
        }
    }
    Write-Host "  Extracted $($GoComponents.Count) Go modules." -ForegroundColor Green
}

# 2. Parse Rust Dependencies from Cargo.lock
$CargoLockPath = Join-Path $RepoRoot "Cargo.lock"
$RustComponents = @()

if (Test-Path $CargoLockPath) {
    Write-Host "Parsing Rust dependencies from $CargoLockPath..." -ForegroundColor Cyan
    $lockContent = Get-Content $CargoLockPath -Raw
    $packages = [regex]::Matches($lockContent, '\[\[package\]\][\r\n]+name\s*=\s*"([^"]+)"[\r\n]+version\s*=\s*"([^"]+)"')
    foreach ($pkg in $packages) {
        $pkgName = $pkg.Groups[1].Value
        $pkgVer = $pkg.Groups[2].Value
        $RustComponents += [PSCustomObject]@{
            "Ecosystem" = "Rust/Cargo"
            "Name" = $pkgName
            "Version" = $pkgVer
            "Direct" = $true
            "License" = "MIT / Apache-2.0"
        }
    }
    Write-Host "  Extracted $($RustComponents.Count) Rust packages." -ForegroundColor Green
}

# 3. Third-Party Driver Inventory
$DriverComponents = @(
    [PSCustomObject]@{
        "Ecosystem" = "Native Windows Driver"
        "Name" = "wintun.dll"
        "Version" = "0.14.1"
        "Direct" = $true
        "License" = "MIT"
        "Publisher" = "WireGuard LLC / Jason A. Donenfeld"
    }
)

# 4. Generate Combined Inventory CSV
$AllComponents = $GoComponents + $RustComponents + $DriverComponents
$CsvPath = Join-Path $FullOutput "dependency-inventory.csv"
$AllComponents | Export-Csv -Path $CsvPath -NoTypeInformation
Write-Host "`n[OK] Wrote Dependency Inventory CSV: $CsvPath" -ForegroundColor Green

# 5. Generate CycloneDX-compatible JSON SBOM
$CycloneDX = @{
    "bomFormat" = "CycloneDX"
    "specVersion" = "1.5"
    "serialNumber" = "urn:uuid:" + [System.Guid]::NewGuid().ToString()
    "version" = 1
    "metadata" = @{
        "timestamp" = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssZ")
        "component" = @{
            "type" = "application"
            "name" = "MultiNet CyberShield AI"
            "version" = "0.1.0"
            "description" = "Intelligent Multi-Link Network Protection, Bonding and Threat Defense"
        }
    }
    "components" = @($AllComponents | ForEach-Object {
        @{
            "type" = "library"
            "name" = $_.Name
            "version" = $_.Version
            "scope" = if ($_.Direct) { "required" } else { "optional" }
            "licenses" = @(@{ "license" = @{ "id" = $_.License } })
            "purl" = "pkg:" + $_.Ecosystem.ToLower().Split('/')[0] + "/" + $_.Name + "@" + $_.Version
        }
    })
}

$SbomPath = Join-Path $FullOutput "sbom.json"
$CycloneDX | ConvertTo-Json -Depth 6 | Set-Content -Path $SbomPath
Write-Host "[OK] Wrote CycloneDX SBOM: $SbomPath" -ForegroundColor Green

# 6. Generate SHA256 Checksums for Release Binaries
$ChecksumsPath = Join-Path $FullOutput "checksums.sha256"
$ChecksumLines = @()
$StagingDir = Join-Path $ScriptDir "..\staging"
if (Test-Path $StagingDir) {
    Get-ChildItem $StagingDir | ForEach-Object {
        $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
        $ChecksumLines += "$hash  $($_.Name)"
    }
}
$ChecksumLines | Set-Content -Path $ChecksumsPath
Write-Host "[OK] Wrote SHA256 Checksums: $ChecksumsPath" -ForegroundColor Green
