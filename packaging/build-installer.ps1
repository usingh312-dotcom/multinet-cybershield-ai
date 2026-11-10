# MultiNet CyberShield AI - Windows Installer Packaging Script
# Milestone: NM-07.0.1 (Release Integrity Remediation)
# Requires: .NET SDK and WiX Toolset v5 (dotnet tool install -g wix --version 5.0.2)
param (
    [string]$Configuration = "Release",
    [string]$OutputDir = "output",
    [string]$StagingDir = "staging"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Windows Installer Build (WiX v5)" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.0.1 (Strict Release Integrity Enforced)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# -----------------------------------------------------------------------------
# Binary & PE Validation Helper
# -----------------------------------------------------------------------------
function Assert-ValidPEImage {
    param (
        [string]$FilePath,
        [string]$ExpectedName = "",
        [uint16]$ExpectedMachine = 0x8664 # AMD64 / x64
    )
    if (-not (Test-Path $FilePath)) {
        throw "Packaging Failure: Required binary does not exist: $FilePath"
    }
    $item = Get-Item $FilePath
    if ($item.Length -eq 0) {
        throw "Packaging Failure: Binary file is 0 bytes (empty): $FilePath"
    }
    if ($item.Length -lt 1024) {
        $content = Get-Content $FilePath -Raw -ErrorAction SilentlyContinue
        if ($content -match "Stub|Mock|Placeholder") {
            throw "Packaging Failure: Placeholder stub detected in $FilePath"
        }
    }
    $bytes = [System.IO.File]::ReadAllBytes($FilePath)
    if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
        throw "Packaging Failure: Missing MZ header in $FilePath (not a valid Windows executable)"
    }
    $e_lfanew = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($e_lfanew -lt 0 -or $e_lfanew + 4 -ge $bytes.Length) {
        throw "Packaging Failure: Invalid e_lfanew offset in $FilePath"
    }
    $peSig = [System.Text.Encoding]::ASCII.GetString($bytes[$e_lfanew..($e_lfanew+3)])
    if ($peSig -ne "PE`0`0") {
        throw "Packaging Failure: Missing PE signature in $FilePath"
    }
    if ($ExpectedMachine -ne 0) {
        $machine = [BitConverter]::ToUInt16($bytes, $e_lfanew + 4)
        if ($machine -ne $ExpectedMachine) {
            throw "Packaging Failure: Architecture mismatch in $FilePath. Expected 0x$($ExpectedMachine.ToString('X4')), got 0x$($machine.ToString('X4'))"
        }
    }
    Write-Host "[OK] Verified PE Image: $FilePath (AMD64, $($item.Length) bytes)" -ForegroundColor Green
    return $true
}

# 1. Resolve & create output directories
$FullStaging = Join-Path $ScriptDir $StagingDir
$FullOutput = Join-Path $ScriptDir $OutputDir
New-Item -ItemType Directory -Force -Path $FullStaging | Out-Null
New-Item -ItemType Directory -Force -Path $FullOutput | Out-Null

# 2. Stage Service Binary (net-merge-service.exe)
$ServiceBinary = Join-Path $RepoRoot "services\net-merge\net-merge-service.exe"
if (-not (Test-Path $ServiceBinary)) {
    # Check staging directory if already staged by CI download
    $ServiceBinary = Join-Path $FullStaging "net-merge-service.exe"
}
Assert-ValidPEImage -FilePath $ServiceBinary -ExpectedName "net-merge-service.exe" -ExpectedMachine 0x8664
Copy-Item $ServiceBinary (Join-Path $FullStaging "net-merge-service.exe") -Force
Write-Host "[OK] Staged genuine net-merge-service.exe" -ForegroundColor Green

# 3. Stage Authentic Wintun Driver DLL (wintun.dll)
$WintunDll = Join-Path $RepoRoot "platform\windows\wintun\wintun.dll"
if (-not (Test-Path $WintunDll) -or (Get-Item $WintunDll).Length -lt 10000) {
    # Fetch authentic upstream Wintun if missing
    $fetchScript = Join-Path $RepoRoot "platform\windows\wintun\fetch-wintun.ps1"
    if (Test-Path $fetchScript) {
        Write-Host "Fetching authentic upstream Wintun distribution..." -ForegroundColor Cyan
        & $fetchScript -TargetDir (Join-Path $RepoRoot "platform\windows\wintun")
    }
}
Assert-ValidPEImage -FilePath $WintunDll -ExpectedName "wintun.dll" -ExpectedMachine 0x8664

# Validate Wintun Authenticode Signature from WireGuard LLC
$wintunSig = Get-AuthenticodeSignature $WintunDll
if ($wintunSig.Status -ne "Valid") {
    throw "Packaging Failure: wintun.dll signature is invalid ($($wintunSig.Status))"
}
if (-not ($wintunSig.SignerCertificate.Subject -like "*WireGuard LLC*")) {
    throw "Packaging Failure: wintun.dll is not signed by WireGuard LLC ($($wintunSig.SignerCertificate.Subject))"
}
Write-Host "[OK] Verified Wintun Authenticode Signature: $($wintunSig.SignerCertificate.Subject)" -ForegroundColor Green

Copy-Item $WintunDll (Join-Path $FullStaging "wintun.dll") -Force
Write-Host "[OK] Staged authentic wintun.dll" -ForegroundColor Green

# 4. Stage Desktop Application Binary (multinet-cybershield-ai.exe)
$DesktopBinary = Join-Path $RepoRoot "target\release\multinet-cybershield-ai.exe"
if (-not (Test-Path $DesktopBinary)) {
    $DesktopBinary = Join-Path $RepoRoot "target\release\multinet-cybershield-ai-desktop.exe"
}
if (-not (Test-Path $DesktopBinary)) {
    $DesktopBinary = Join-Path $RepoRoot "apps\desktop\src-tauri\target\release\multinet-cybershield-ai.exe"
}
if (-not (Test-Path $DesktopBinary)) {
    $DesktopBinary = Join-Path $FullStaging "multinet-cybershield-ai.exe"
}
Assert-ValidPEImage -FilePath $DesktopBinary -ExpectedName "multinet-cybershield-ai.exe" -ExpectedMachine 0x8664
Copy-Item $DesktopBinary (Join-Path $FullStaging "multinet-cybershield-ai.exe") -Force
Write-Host "[OK] Staged genuine multinet-cybershield-ai.exe" -ForegroundColor Green

# 5. Stage Application Icon
$IconSrc = Join-Path $ScriptDir "wix\resources\icon.ico"
if (-not (Test-Path $IconSrc) -or (Get-Item $IconSrc).Length -eq 0) {
    throw "Packaging Failure: Application icon missing or empty: $IconSrc"
}
Copy-Item $IconSrc (Join-Path $FullStaging "icon.ico") -Force
Write-Host "[OK] Staged icon.ico" -ForegroundColor Green

# 6. Calculate Staged Component Hashes
Write-Host "`nStaged Component SHA256 Hashes:" -ForegroundColor Cyan
$ManifestItems = @{}
Get-ChildItem $FullStaging | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
    Write-Host "  $($_.Name): $hash ($($_.Length) bytes)"
    $ManifestItems[$_.Name] = @{
        "SizeBytes" = $_.Length
        "SHA256" = $hash
    }
}

# 7. Build MSI with WiX Toolset
$WixInstalled = $false
try {
    $wixVersion = & wix --version 2>$null
    if ($LASTEXITCODE -eq 0 -and $wixVersion) {
        $WixInstalled = $true
        Write-Host "`nFound WiX Toolset version: $wixVersion" -ForegroundColor Green
    }
} catch {
    $WixInstalled = $false
}

$MsiOutput = Join-Path $FullOutput "MultiNetCyberShield-Setup.msi"

if ($WixInstalled) {
    Write-Host "Building MSI installer using WiX Toolset v5..." -ForegroundColor Cyan
    $env:WIX_ACCEPT_EULA = "yes"
    $WxsPath = Join-Path $ScriptDir "wix\MultiNetCyberShield.wxs"
    & wix build $WxsPath -d "SourceDir=$FullStaging" -o $MsiOutput
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $MsiOutput)) {
        throw "WiX build failed with exit code $LASTEXITCODE"
    }

    # Validate MSI file (OLE Compound Document signature: 0xD0 0xCF 0x11 0xE0)
    $msiBytes = [System.IO.File]::ReadAllBytes($MsiOutput)
    if ($msiBytes.Length -lt 8 -or $msiBytes[0] -ne 0xD0 -or $msiBytes[1] -ne 0xCF -or $msiBytes[2] -ne 0x11 -or $msiBytes[3] -ne 0xE0) {
        throw "Packaging Failure: Output MSI is not a valid OLE Compound Document: $MsiOutput"
    }
    if ((Get-Item $MsiOutput).Length -lt 1000000) {
        throw "Packaging Failure: Output MSI is suspiciously small ($((Get-Item $MsiOutput).Length) bytes). Embedded cabinet may be missing."
    }

    $msiHash = (Get-FileHash $MsiOutput -Algorithm SHA256).Hash
    Write-Host "`n[SUCCESS] Installer built successfully: $MsiOutput" -ForegroundColor Green
    Write-Host "MSI Size: $((Get-Item $MsiOutput).Length) bytes" -ForegroundColor Green
    Write-Host "MSI SHA256: $msiHash" -ForegroundColor Green
    $ManifestItems["MultiNetCyberShield-Setup.msi"] = @{
        "SizeBytes" = (Get-Item $MsiOutput).Length
        "SHA256" = $msiHash
    }
} else {
    Write-Host "`n[NOTE] WiX Toolset CLI ('wix') is not installed in this environment." -ForegroundColor Yellow
    Write-Host "To install WiX v5 on Windows: dotnet tool install --global wix --version 5.0.2" -ForegroundColor Yellow
    Write-Host "Packaging metadata and staged directory verified." -ForegroundColor Yellow
}

# 8. Generate Release Manifest JSON
$Manifest = @{
    "Product" = "MultiNet CyberShield AI"
    "Version" = "0.1.0-beta.1"
    "ReleaseTag" = "v0.1.0-beta.1"
    "ReleaseMilestone" = "NM-07.0.1"
    "BuildDate" = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssZ")
    "TargetOS" = "Windows 10 / 11 (x64)"
    "IntegrityPolicy" = "StrictZeroPlaceholder"
    "Components" = $ManifestItems
}
$ManifestPath = Join-Path $FullOutput "release-manifest.json"
$Manifest | ConvertTo-Json -Depth 5 | Set-Content -Path $ManifestPath
Write-Host "`nWrote Release Manifest: $ManifestPath" -ForegroundColor Green
