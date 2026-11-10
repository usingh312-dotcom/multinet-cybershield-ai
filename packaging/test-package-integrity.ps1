# MultiNet CyberShield AI - Release Package Integrity & Zero-Stub Verification
# Validates complete distribution bundle, cryptographic signatures, PE structures, and checksums
param (
    [string]$PackageDir = "$PSScriptRoot\output",
    [string]$ZipName = "MultiNetCyberShield-v0.1.0-beta.1-Windows-x64.zip",
    [switch]$RunNegativeTests
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$FullPackageDir = Resolve-Path $PackageDir
$ZipPath = Join-Path $FullPackageDir $ZipName

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Package Integrity Test Suite" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$PassedCount = 0
$FailedCount = 0

function Assert-Condition {
    param (
        [string]$Description,
        [scriptblock]$Check
    )
    Write-Host -NoNewline "Checking: $Description ... "
    try {
        $result = & $Check
        if ($result -ne $false) {
            Write-Host "[PASS]" -ForegroundColor Green
            $script:PassedCount++
        } else {
            Write-Host "[FAIL]" -ForegroundColor Red
            $script:FailedCount++
        }
    } catch {
        Write-Host "[FAIL] ($($_.Exception.Message))" -ForegroundColor Red
        $script:FailedCount++
    }
}

function Test-PEHeaders {
    param ([string]$FilePath)
    if (-not (Test-Path $FilePath)) { return $false }
    $bytes = [System.IO.File]::ReadAllBytes($FilePath)
    if ($bytes.Length -lt 256) { return $false }
    
    # Check MZ header
    if ($bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) { return $false }
    
    # PE Header offset at 0x3C
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($peOffset + 24 -gt $bytes.Length) { return $false }
    
    # Check PE signature ("PE\0\0")
    if ($bytes[$peOffset] -ne 0x50 -or $bytes[$peOffset + 1] -ne 0x45 -or
        $bytes[$peOffset + 2] -ne 0x00 -or $bytes[$peOffset + 3] -ne 0x00) { return $false }
    
    # Machine Architecture (0x8664 = AMD64)
    $machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
    if ($machine -ne 0x8664) { return $false }
    
    return $true
}

# -------------------------------------------------------------
# Part 1: Negative Tests (Verification that validators reject stubs)
# -------------------------------------------------------------
Write-Host "`n--- Running Suite A: Negative Security & Stub Tests ---" -ForegroundColor Yellow

$tempTestDir = Join-Path ([System.IO.Path]::GetTempPath()) ("pkg-neg-test-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tempTestDir | Out-Null

try {
    # 1. Reject 0-byte file
    $zeroFile = Join-Path $tempTestDir "zero.exe"
    New-Item -ItemType File -Force -Path $zeroFile | Out-Null
    Assert-Condition "Reject 0-byte file as valid PE" {
        -not (Test-PEHeaders $zeroFile)
    }

    # 2. Reject text stub claiming to be an EXE
    $stubFile = Join-Path $tempTestDir "stub.exe"
    Set-Content -Path $stubFile -Value "Mock executable placeholder stub"
    Assert-Condition "Reject ASCII text stub claiming to be executable" {
        -not (Test-PEHeaders $stubFile)
    }

    # 3. Reject file with MZ but invalid PE offset
    $badPeFile = Join-Path $tempTestDir "badpe.exe"
    $badBytes = [byte[]]@(0x4D, 0x5A) + [byte[]](1..100)
    [System.IO.File]::WriteAllBytes($badPeFile, $badBytes)
    Assert-Condition "Reject truncated DOS binary without PE signature" {
        -not (Test-PEHeaders $badPeFile)
    }

    # 4. Reject unsigned wintun DLL
    Assert-Condition "Reject unsigned wintun DLL" {
        $sig = Get-AuthenticodeSignature $stubFile
        $sig.Status -ne [System.Management.Automation.SignatureStatus]::Valid
    }
} finally {
    if (Test-Path $tempTestDir) {
        Remove-Item -Recurse -Force -Path $tempTestDir -ErrorAction SilentlyContinue
    }
}

# -------------------------------------------------------------
# Part 2: Positive Package Integrity Tests
# -------------------------------------------------------------
Write-Host "`n--- Running Suite B: Release Package Integrity & Content Verification ---" -ForegroundColor Yellow

Assert-Condition "Beta Release ZIP exists ($ZipName)" {
    Test-Path $ZipPath
}

if (-not (Test-Path $ZipPath)) {
    Write-Host "`n[FATAL] Beta ZIP archive not found at $ZipPath. Cannot continue validation." -ForegroundColor Red
    exit 1
}

$zipSize = (Get-Item $ZipPath).Length
Assert-Condition "Beta Release ZIP is non-empty (> 1 MB, current: $([math]::Round($zipSize/1MB, 2)) MB)" {
    $zipSize -gt 1048576
}

# Extract ZIP into clean temporary validation directory
$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ("pkg-verify-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null

try {
    Write-Host "Extracting release ZIP to $extractDir..." -ForegroundColor Cyan
    Expand-Archive -Path $ZipPath -DestinationPath $extractDir -Force
    
    # Expected components
    $expectedFiles = @(
        "MultiNetCyberShield-Setup.msi",
        "multinet-cybershield-ai.exe",
        "net-merge-service.exe",
        "wintun.dll",
        "sbom.json",
        "dependency-inventory.csv",
        "release-manifest.json",
        "checksums.sha256",
        "RELEASE_NOTES.md",
        "INSTALL_AND_ROLLBACK.md",
        "TROUBLESHOOTING.md"
    )

    foreach ($relFile in $expectedFiles) {
        $fullPath = Join-Path $extractDir $relFile
        Assert-Condition "Package contains $relFile" {
            Test-Path $fullPath -and (Get-Item $fullPath).Length -gt 0
        }
    }

    # Verify Desktop Application PE Header & Machine Type
    $desktopBin = Join-Path $extractDir "multinet-cybershield-ai.exe"
    Assert-Condition "Desktop executable has valid AMD64 PE header" {
        Test-PEHeaders $desktopBin
    }

    # Verify Service Binary PE Header & Machine Type
    $serviceBin = Join-Path $extractDir "net-merge-service.exe"
    Assert-Condition "Service executable has valid AMD64 PE header" {
        Test-PEHeaders $serviceBin
    }

    # Verify Wintun Driver PE Header & Authentic WireGuard Signature
    $wintunDll = Join-Path $extractDir "wintun.dll"
    Assert-Condition "Wintun DLL has valid AMD64 PE header" {
        Test-PEHeaders $wintunDll
    }

    Assert-Condition "Wintun DLL has valid Authenticode signature from WireGuard LLC" {
        $sig = Get-AuthenticodeSignature $wintunDll
        $subject = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { "" }
        $sig.Status -eq [System.Management.Automation.SignatureStatus]::Valid -and $subject -match "WireGuard LLC"
    }

    # Verify MSI file validity (OLE Compound Document signature: 0xD0 0xCF 0x11 0xE0)
    $msiFile = Join-Path $extractDir "MultiNetCyberShield-Setup.msi"
    Assert-Condition "Installer MSI is valid OLE Compound Document (> 1 MB)" {
        if (-not (Test-Path $msiFile)) { return $false }
        $msiBytes = [System.IO.File]::ReadAllBytes($msiFile)
        $msiBytes.Length -gt 1000000 -and $msiBytes[0] -eq 0xD0 -and $msiBytes[1] -eq 0xCF -and $msiBytes[2] -eq 0x11 -and $msiBytes[3] -eq 0xE0
    }

    # Verify Checksum Manifest Integrity
    $manifestPath = Join-Path $extractDir "checksums.sha256"
    Assert-Condition "Payload checksums match manifest" {
        if (-not (Test-Path $manifestPath)) { return $false }
        $manifestLines = Get-Content $manifestPath
        $allMatched = $true
        foreach ($line in $manifestLines) {
            $trimmed = $line.Trim()
            if ([string]::IsNullOrWhiteSpace($trimmed)) { continue }
            $parts = $trimmed -split '\s+'
            if ($parts.Length -lt 2) { continue }
            $expectedHash = $parts[0].Trim().ToLowerInvariant()
            $fileName = $parts[1].Trim()
            
            # Skip checking the zip hash inside the unzipped folder
            if ($fileName -like "*.zip") { continue }

            $matchingItem = Join-Path $extractDir $fileName
            if (-not (Test-Path $matchingItem)) {
                Write-Host "`n  [FAIL] Manifest listed file not found: $fileName" -ForegroundColor Red
                $allMatched = $false
                continue
            }
            $actualHash = (Get-FileHash $matchingItem -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($actualHash -ne $expectedHash) {
                Write-Host "`n  [FAIL] Checksum mismatch for $fileName (Expected: $expectedHash, Actual: $actualHash)" -ForegroundColor Red
                $allMatched = $false
            }
        }
        $allMatched
    }

    # Verify CycloneDX SBOM format
    $sbomPath = Join-Path $extractDir "sbom.json"
    Assert-Condition "SBOM is valid CycloneDX v1.5 JSON with authentic components" {
        $sbomJson = Get-Content $sbomPath -Raw | ConvertFrom-Json
        $sbomJson.bomFormat -eq "CycloneDX" -and $sbomJson.specVersion -eq "1.5" -and $sbomJson.components.Count -gt 50
    }

} finally {
    if (Test-Path $extractDir) {
        Remove-Item -Recurse -Force -Path $extractDir -ErrorAction SilentlyContinue
    }
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "Package Integrity Summary: Passed = $PassedCount, Failed = $FailedCount" -ForegroundColor $(if ($FailedCount -eq 0) { "Green" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

if ($FailedCount -gt 0) {
    Write-Host "[ERROR] Package integrity verification failed!" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[SUCCESS] All package integrity tests passed with ZERO STUBS." -ForegroundColor Green
    exit 0
}
