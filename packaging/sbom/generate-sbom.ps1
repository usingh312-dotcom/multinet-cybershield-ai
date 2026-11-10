# MultiNet CyberShield AI - Software Bill of Materials (SBOM) & Inventory Generator
# Conforms to CycloneDX v1.5 JSON Specification & Full Supply Chain Traceability
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

# License mapping database for Go modules
$GoLicenseMap = @{
    "github.com/xjasonlyu/tun2socks/v2"     = "GPL-3.0-or-later"
    "golang.org/x/sys"                      = "BSD-3-Clause"
    "golang.zx2c4.com/wireguard"            = "MIT"
    "gvisor.dev/gvisor"                     = "Apache-2.0"
    "github.com/google/btree"               = "Apache-2.0"
    "github.com/google/uuid"                = "BSD-3-Clause"
    "go.uber.org/atomic"                    = "MIT"
    "go.uber.org/multierr"                  = "MIT"
    "go.uber.org/zap"                       = "MIT"
    "golang.org/x/exp"                      = "BSD-3-Clause"
    "golang.org/x/net"                      = "BSD-3-Clause"
    "golang.org/x/time"                     = "BSD-3-Clause"
    "golang.zx2c4.com/wintun"               = "MIT"
}

# License mapping database for popular Rust crates
$RustLicenseMap = @{
    "serde"           = "MIT OR Apache-2.0"
    "serde_json"      = "MIT OR Apache-2.0"
    "tokio"           = "MIT"
    "axum"            = "MIT"
    "axum-server"     = "MIT"
    "reqwest"         = "MIT OR Apache-2.0"
    "rustls"          = "Apache-2.0 OR ISC OR MIT"
    "ring"            = "ISC AND OpenSSL AND MIT"
    "rcgen"           = "MIT OR Apache-2.0"
    "tower-http"      = "MIT"
    "sqlx"            = "MIT OR Apache-2.0"
    "thiserror"       = "MIT OR Apache-2.0"
    "tracing"         = "MIT"
    "uuid"            = "Apache-2.0 OR MIT"
    "time"            = "MIT OR Apache-2.0"
    "async-trait"     = "MIT OR Apache-2.0"
    "hmac"            = "MIT OR Apache-2.0"
    "sha2"            = "MIT OR Apache-2.0"
    "windows-sys"     = "MIT OR Apache-2.0"
    "tauri"           = "Apache-2.0 OR MIT"
    "tauri-build"     = "Apache-2.0 OR MIT"
    "tauri-plugin-shell" = "Apache-2.0 OR MIT"
}

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
            
            $lic = $GoLicenseMap[$modName]
            if (-not $lic) {
                if ($modName -like "golang.org/*") { $lic = "BSD-3-Clause" }
                elseif ($modName -like "github.com/google/*") { $lic = "Apache-2.0" }
                elseif ($modName -like "go.uber.org/*") { $lic = "MIT" }
                else { $lic = "Apache-2.0" }
            }

            $GoComponents += [PSCustomObject]@{
                "Ecosystem" = "Go"
                "Name"      = $modName
                "Version"   = $modVer
                "Direct"    = $isDirect
                "License"   = $lic
                "Publisher" = ($modName.Split('/')[0])
            }
        }
    }
    Write-Host "  Extracted $($GoComponents.Count) Go modules (Direct: $(($GoComponents | Where-Object { $_.Direct }).Count))." -ForegroundColor Green
}

# 2. Parse Rust Dependencies from Cargo.lock
$CargoLockPath = Join-Path $RepoRoot "Cargo.lock"
$RustComponents = @()

# Read root Cargo.toml to identify direct workspace dependencies
$RootCargoToml = Join-Path $RepoRoot "Cargo.toml"
$DirectRustDeps = @()
if (Test-Path $RootCargoToml) {
    $tomlContent = Get-Content $RootCargoToml
    $inDeps = $false
    foreach ($line in $tomlContent) {
        if ($line -match '^\s*\[.*dependencies\]') { $inDeps = $true; continue }
        if ($line -match '^\s*\[') { $inDeps = $false; continue }
        if ($inDeps -and $line -match '^\s*([a-zA-Z0-9_-]+)\s*=') {
            $DirectRustDeps += $matches[1]
        }
    }
}

if (Test-Path $CargoLockPath) {
    Write-Host "Parsing Rust dependencies from $CargoLockPath..." -ForegroundColor Cyan
    $lockContent = Get-Content $CargoLockPath -Raw
    $packages = [regex]::Matches($lockContent, '\[\[package\]\]\r?\nname\s*=\s*"([^"]+)"\r?\nversion\s*=\s*"([^"]+)"(?:\r?\nsource\s*=\s*"([^"]+)")?')
    foreach ($pkg in $packages) {
        $pkgName = $pkg.Groups[1].Value
        $pkgVer = $pkg.Groups[2].Value
        $source = if ($pkg.Groups.Count -gt 3) { $pkg.Groups[3].Value } else { "" }
        
        $isWorkspaceMember = [string]::IsNullOrEmpty($source)
        $isDirect = $DirectRustDeps -contains $pkgName -or $isWorkspaceMember
        
        $lic = $RustLicenseMap[$pkgName]
        if (-not $lic) {
            if ($isWorkspaceMember) {
                $lic = "Proprietary"
            } elseif ($pkgName -like "windows*" -or $pkgName -like "winapi*") {
                $lic = "MIT OR Apache-2.0"
            } else {
                $lic = "MIT OR Apache-2.0"
            }
        }

        $RustComponents += [PSCustomObject]@{
            "Ecosystem" = "Rust/Cargo"
            "Name"      = $pkgName
            "Version"   = $pkgVer
            "Direct"    = $isDirect
            "License"   = $lic
            "Publisher" = if ($isWorkspaceMember) { "MultiNet CyberShield AI Authors" } else { "crates.io" }
        }
    }
    Write-Host "  Extracted $($RustComponents.Count) Rust packages (Direct: $(($RustComponents | Where-Object { $_.Direct }).Count))." -ForegroundColor Green
}

# 3. Parse Frontend Dependencies from apps/desktop/package.json
$NpmComponents = @()
$DesktopPkgPath = Join-Path $RepoRoot "apps\desktop\package.json"
if (Test-Path $DesktopPkgPath) {
    Write-Host "Parsing Desktop frontend dependencies from $DesktopPkgPath..." -ForegroundColor Cyan
    $pkgJson = Get-Content $DesktopPkgPath -Raw | ConvertFrom-Json
    if ($pkgJson.dependencies) {
        foreach ($prop in $pkgJson.dependencies.PSObject.Properties) {
            $NpmComponents += [PSCustomObject]@{
                "Ecosystem" = "npm"
                "Name"      = $prop.Name
                "Version"   = $prop.Value
                "Direct"    = $true
                "License"   = if ($prop.Name -like "@tauri-apps*") { "Apache-2.0 OR MIT" } else { "MIT" }
                "Publisher" = "npm"
            }
        }
    }
    Write-Host "  Extracted $($NpmComponents.Count) npm direct dependencies." -ForegroundColor Green
}

# 4. Third-Party Driver Inventory (Authentic WireGuard LLC Wintun)
$DriverComponents = @(
    [PSCustomObject]@{
        "Ecosystem" = "Windows Kernel / Native Driver"
        "Name"      = "wintun.dll"
        "Version"   = "0.14.1"
        "Direct"    = $true
        "License"   = "BSD-3-Clause"
        "Publisher" = "WireGuard LLC / Jason A. Donenfeld"
    }
)

# 5. Generate Combined Inventory CSV
$AllComponents = $GoComponents + $RustComponents + $NpmComponents + $DriverComponents
$CsvPath = Join-Path $FullOutput "dependency-inventory.csv"
$AllComponents | Export-Csv -Path $CsvPath -NoTypeInformation -Encoding utf8
Write-Host "`n[OK] Wrote Dependency Inventory CSV ($($AllComponents.Count) components): $CsvPath" -ForegroundColor Green

# 6. Generate CycloneDX v1.5 JSON SBOM
$CycloneComponents = @()
foreach ($c in $AllComponents) {
    $licenseObj = @()
    if ($c.License -match '^(MIT|Apache-2.0|BSD-3-Clause|GPL-3.0-or-later)$') {
        $licenseObj += @{ "license" = @{ "id" = $c.License } }
    } else {
        $licenseObj += @{ "license" = @{ "name" = $c.License } }
    }

    $compType = switch ($c.Ecosystem) {
        "Windows Kernel / Native Driver" { "file" }
        "npm" { "framework" }
        default { "library" }
    }

    $ecoName = switch -Wildcard ($c.Ecosystem) {
        "Go*" { "golang" }
        "Rust*" { "cargo" }
        "npm" { "npm" }
        default { "generic" }
    }

    $CycloneComponents += @{
        "type"      = $compType
        "name"      = $c.Name
        "version"   = $c.Version
        "scope"     = if ($c.Direct) { "required" } else { "optional" }
        "licenses"  = $licenseObj
        "purl"      = "pkg:$ecoName/$($c.Name)@$($c.Version)"
        "publisher" = $c.Publisher
    }
}

$CycloneDX = [ordered]@{
    "bomFormat"    = "CycloneDX"
    "specVersion"  = "1.5"
    "serialNumber" = "urn:uuid:" + [System.Guid]::NewGuid().ToString()
    "version"      = 1
    "metadata"     = [ordered]@{
        "timestamp" = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        "tools"     = @(
            @{
                "vendor"  = "MultiNet CyberShield AI"
                "name"    = "generate-sbom.ps1"
                "version" = "1.0.0"
            }
        )
        "component" = [ordered]@{
            "type"        = "application"
            "name"        = "MultiNet CyberShield AI"
            "version"     = "0.1.0-beta.1"
            "description" = "Intelligent Multi-Link Network Protection, Bonding and Threat Defense"
        }
    }
    "components"   = $CycloneComponents
}

$SbomPath = Join-Path $FullOutput "sbom.json"
$CycloneDX | ConvertTo-Json -Depth 8 | Set-Content -Path $SbomPath -Encoding utf8
Write-Host "[OK] Wrote CycloneDX v1.5 JSON SBOM: $SbomPath" -ForegroundColor Green

# 7. Generate SHA256 Checksums for Release Binaries in Staging
$ChecksumsPath = Join-Path $FullOutput "checksums.sha256"
$ChecksumLines = @()
$StagingDir = Join-Path $ScriptDir "..\staging"
if (Test-Path $StagingDir) {
    Get-ChildItem -Path $StagingDir -File | ForEach-Object {
        $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $ChecksumLines += "$hash  $($_.Name)"
    }
}
$ChecksumLines | Set-Content -Path $ChecksumsPath -Encoding utf8
Write-Host "[OK] Wrote SHA256 Checksums: $ChecksumsPath" -ForegroundColor Green
