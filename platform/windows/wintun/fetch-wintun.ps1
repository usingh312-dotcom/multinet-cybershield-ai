# MultiNet CyberShield AI - Upstream Wintun Acquisition Script
# Downloads and validates authentic Wintun driver library from official WireGuard source
param (
    [string]$TargetDir = $PSScriptRoot,
    [string]$Version = "0.14.1"
)

$ErrorActionPreference = "Stop"

$Url = "https://www.wintun.net/builds/wintun-$Version.zip"
$TempZip = Join-Path $env:TEMP "wintun-$Version.zip"
$TempExtract = Join-Path $env:TEMP "wintun-$Version-extract"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "Fetching Authentic Upstream Wintun Distribution v$Version" -ForegroundColor Cyan
Write-Host "Source: $Url" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Download official ZIP
Write-Host "Downloading $Url..." -ForegroundColor Cyan
curl.exe -s -L -o $TempZip $Url
if (-not (Test-Path $TempZip) -or (Get-Item $TempZip).Length -eq 0) {
    throw "Failed to download Wintun distribution from $Url"
}

# 2. Extract
if (Test-Path $TempExtract) {
    Remove-Item -Recurse -Force $TempExtract
}
Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

# 3. Locate AMD64 binary
$Amd64Dll = Join-Path $TempExtract "wintun\bin\amd64\wintun.dll"
if (-not (Test-Path $Amd64Dll)) {
    throw "wintun.dll not found in extracted archive at $Amd64Dll"
}

# 4. Validate Authenticode signature of upstream driver
Write-Host "`nValidating Wintun Authenticode Signature..." -ForegroundColor Cyan
$sig = Get-AuthenticodeSignature $Amd64Dll
if ($sig.Status -ne "Valid") {
    throw "Security Failure: Upstream Wintun DLL signature is invalid ($($sig.Status))"
}

$signer = $sig.SignerCertificate.Subject
if (-not ($signer -like "*WireGuard LLC*")) {
    throw "Security Failure: Unexpected signer for Wintun DLL: $signer"
}

Write-Host "[OK] Verified Authenticode Signer: $signer" -ForegroundColor Green
Write-Host "[OK] Signature Status: $($sig.Status)" -ForegroundColor Green

# 5. Validate PE Architecture (AMD64 / x64 = 0x8664)
$bytes = [System.IO.File]::ReadAllBytes($Amd64Dll)
if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
    throw "Invalid PE image: missing MZ signature"
}
$e_lfanew = [BitConverter]::ToInt32($bytes, 0x3C)
$peSig = [System.Text.Encoding]::ASCII.GetString($bytes[$e_lfanew..($e_lfanew+3)])
if ($peSig -ne "PE`0`0") {
    throw "Invalid PE signature at offset $e_lfanew"
}
$machine = [BitConverter]::ToUInt16($bytes, $e_lfanew + 4)
if ($machine -ne 0x8664) {
    throw "Architecture mismatch: expected AMD64 (0x8664), got 0x$($machine.ToString('X4'))"
}
Write-Host "[OK] Verified Architecture: AMD64 (x64)" -ForegroundColor Green

# 6. Copy to Target Directory
$Destination = Join-Path $TargetDir "wintun.dll"
Copy-Item $Amd64Dll $Destination -Force

$hash = (Get-FileHash $Destination -Algorithm SHA256).Hash
$size = (Get-Item $Destination).Length

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "Authentic Wintun Component Staged Successfully!" -ForegroundColor Green
Write-Host "Target:    $Destination" -ForegroundColor Green
Write-Host "Size:      $size bytes" -ForegroundColor Green
Write-Host "SHA256:    $hash" -ForegroundColor Green
Write-Host "Signer:    $signer" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

# 7. Cleanup temp files
Remove-Item -Force $TempZip -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $TempExtract -ErrorAction SilentlyContinue

