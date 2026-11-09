# MultiNet CyberShield AI - Authenticode Code Signing Pipeline
param (
    [string]$TargetDir = "..\staging",
    [string]$CertThumbprint = $env:SIGNING_CERT_THUMBPRINT,
    [string]$CertPath = $env:SIGNING_CERT_PATH,
    [string]$CertPassword = $env:SIGNING_CERT_PASS,
    [string]$TimestampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ResolvedTarget = Resolve-Path (Join-Path $ScriptDir $TargetDir)

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Authenticode Signing Automation" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Locate signtool.exe
$SignToolPath = ""
$WindowsKits = "C:\Program Files (x86)\Windows Kits\10\bin"
if (Test-Path $WindowsKits) {
    $latestKit = Get-ChildItem $WindowsKits | Where-Object { $_.Name -like "10.*" } | Sort-Object Name -Descending | Select-Object -First 1
    if ($latestKit) {
        $candidate = Join-Path $latestKit.FullName "x64\signtool.exe"
        if (Test-Path $candidate) {
            $SignToolPath = $candidate
        }
    }
}

if (-not $SignToolPath) {
    # Check PATH
    $cmd = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if ($cmd) {
        $SignToolPath = $cmd.Source
    }
}

if ($SignToolPath) {
    Write-Host "[OK] Located SignTool: $SignToolPath" -ForegroundColor Green
} else {
    Write-Warning "signtool.exe not found on system. Looking for Windows SDK."
}

# Identify targets for signing
$FilesToSign = @(
    "net-merge-service.exe",
    "multinet-cybershield-ai.exe",
    "MultiNetCyberShield-Setup.msi"
)

$FoundFiles = @()
foreach ($file in $FilesToSign) {
    $p = Join-Path $ResolvedTarget $file
    if (Test-Path $p) {
        $FoundFiles += $p
    }
}

if ($FoundFiles.Count -eq 0) {
    Write-Warning "No target binaries found in $ResolvedTarget to sign."
    exit 0
}

Write-Host "Found $($FoundFiles.Count) candidate binary/installer files to sign in $ResolvedTarget."

# Execute signing if certificate parameters are provided
if ($CertPath -and (Test-Path $CertPath)) {
    Write-Host "Signing using PFX certificate: $CertPath" -ForegroundColor Cyan
    foreach ($file in $FoundFiles) {
        Write-Host "  Signing $file..."
        if ($SignToolPath) {
            & $SignToolPath sign /f $CertPath /p $CertPassword /fd SHA256 /tr $TimestampServer /td SHA256 $file
            if ($LASTEXITCODE -ne 0) {
                Write-Error "Failed to sign $file"
            }
        }
    }
} elseif ($CertThumbprint) {
    Write-Host "Signing using certificate thumbprint: $CertThumbprint" -ForegroundColor Cyan
    foreach ($file in $FoundFiles) {
        Write-Host "  Signing $file..."
        if ($SignToolPath) {
            & $SignToolPath sign /sha1 $CertThumbprint /fd SHA256 /tr $TimestampServer /td SHA256 $file
            if ($LASTEXITCODE -ne 0) {
                Write-Error "Failed to sign $file"
            }
        }
    }
} else {
    Write-Host "`n[NOTE] No production signing certificate specified via environment variables." -ForegroundColor Yellow
    Write-Host "SIGNING_CERT_PATH or SIGNING_CERT_THUMBPRINT must be configured in production CI/CD." -ForegroundColor Yellow
    Write-Host "Smart App Control (SAC) compliance requires a trusted Extended Validation (EV) certificate enrolled with Microsoft." -ForegroundColor Yellow
}

