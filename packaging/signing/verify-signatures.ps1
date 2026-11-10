# MultiNet CyberShield AI - Signature & Driver Verification Script
# Milestone: NM-07.0.1 (Release Integrity Remediation)
# Enforces explicit Beta and Production signing policies
param (
    [string]$TargetDir = "..\staging",
    [string]$OutputDir = "..\output",
    [ValidateSet("Beta", "Production")]
    [string]$Policy = "Beta",
    [string]$ExpectedPublisher = "MultiNet AI Technologies"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ResolvedTarget = Resolve-Path (Join-Path $ScriptDir $TargetDir) -ErrorAction SilentlyContinue
$ResolvedOutput = Resolve-Path (Join-Path $ScriptDir $OutputDir) -ErrorAction SilentlyContinue

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Authenticode Signature Verification" -ForegroundColor Cyan
Write-Host "Active Policy: $Policy" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Gather targets: executables, DLLs, drivers from staging, and MSI from output
$AuditFiles = @()
if ($ResolvedTarget -and (Test-Path $ResolvedTarget)) {
    $AuditFiles += Get-ChildItem -Path $ResolvedTarget -Include *.exe, *.dll, *.sys -Recurse
}
if ($ResolvedOutput -and (Test-Path $ResolvedOutput)) {
    $AuditFiles += Get-ChildItem -Path $ResolvedOutput -Include *.msi -Recurse
}

if ($AuditFiles.Count -eq 0) {
    throw "Signature Verification Error: No binary or installer targets found to audit."
}

$Results = @()
$Violations = @()
$ThirdPartyVerified = 0

foreach ($file in $AuditFiles) {
    $sig = Get-AuthenticodeSignature $file.FullName
    $status = $sig.Status.ToString()
    $isThirdPartyDriver = ($file.Name -eq "wintun.dll" -or $file.Extension -eq ".sys")
    $signerSubject = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { "None" }
    $issuer = if ($sig.SignerCertificate) { $sig.SignerCertificate.Issuer } else { "None" }
    $timeSigned = if ($sig.SignerCertificate) { $sig.SignerCertificate.NotBefore.ToString("yyyy-MM-dd") } else { "N/A" }
    $digestAlgo = if ($sig.SignerCertificate) { $sig.SignerCertificate.SignatureAlgorithm.FriendlyName } else { "N/A" }

    $policyResult = "UNKNOWN"

    if ($isThirdPartyDriver) {
        # Third-party kernel drivers MUST be validly signed under BOTH Beta and Production policies
        if ($status -eq "Valid" -and ($signerSubject -like "*WireGuard LLC*")) {
            $policyResult = "VALID_THIRD_PARTY_DRIVER"
            $ThirdPartyVerified++
        } else {
            $policyResult = "DRIVER_SIGNATURE_INVALID"
            $Violations += "Third-party driver $($file.Name) has invalid signature ($status) or untrusted publisher ($signerSubject)"
        }
    } else {
        # First-party binaries (net-merge-service.exe, multinet-cybershield-ai.exe, MSI)
        if ($status -eq "Valid") {
            # Check publisher
            if ($signerSubject -like "*$ExpectedPublisher*") {
                $policyResult = "SIGNED_VALID_FIRST_PARTY"
            } else {
                $policyResult = "SIGNED_UNEXPECTED_PUBLISHER"
                if ($Policy -eq "Production") {
                    $Violations += "First-party file $($file.Name) signed by unexpected publisher: $signerSubject"
                }
            }
        } else {
            if ($Policy -eq "Beta") {
                # In Beta policy, unsigned first-party binaries are permitted but MUST be explicitly reported
                $policyResult = "UNSIGNED_BETA"
            } else {
                # In Production policy, unsigned first-party binaries are STRICTLY FORBIDDEN
                $policyResult = "UNSIGNED_PRODUCTION_VIOLATION"
                $Violations += "Production Policy Violation: File $($file.Name) is unsigned ($status)"
            }
        }
    }

    $Results += [PSCustomObject]@{
        "File" = $file.Name
        "Type" = if ($isThirdPartyDriver) { "Driver" } else { "Application" }
        "SignatureStatus" = $status
        "PolicyClassification" = $policyResult
        "Signer" = $signerSubject
        "Algorithm" = $digestAlgo
    }
}

Write-Host "`nSignature Policy Audit Results ($Policy Policy):" -ForegroundColor Cyan
$Results | Format-Table File, Type, SignatureStatus, PolicyClassification, Algorithm -AutoSize

# Driver check summary
Write-Host "`nDriver Signature Audit:" -ForegroundColor Cyan
$driverEntries = $Results | Where-Object { $_.Type -eq "Driver" }
foreach ($d in $driverEntries) {
    if ($d.PolicyClassification -eq "VALID_THIRD_PARTY_DRIVER") {
        Write-Host "  [OK] $($d.File): Verified Authentic ($($d.Signer))" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] $($d.File): FAILED ($($d.SignatureStatus))" -ForegroundColor Red
    }
}

# First-party summary
Write-Host "`nFirst-Party Application Audit:" -ForegroundColor Cyan
$appEntries = $Results | Where-Object { $_.Type -eq "Application" }
foreach ($a in $appEntries) {
    if ($a.PolicyClassification -eq "SIGNED_VALID_FIRST_PARTY") {
        Write-Host "  [OK] $($a.File): Authenticode Signed & Trusted" -ForegroundColor Green
    } elseif ($a.PolicyClassification -eq "UNSIGNED_BETA") {
        Write-Host "  [NOTE] $($a.File): UNSIGNED BETA (Truthfully reported; SAC prompt expected on Win11)" -ForegroundColor Yellow
    } else {
        Write-Host "  [FAIL] $($a.File): Policy Violation ($($a.PolicyClassification))" -ForegroundColor Red
    }
}

if ($Violations.Count -gt 0) {
    Write-Host "`n==========================================================" -ForegroundColor Red
    Write-Host "Signature Policy Violations Detected:" -ForegroundColor Red
    foreach ($v in $Violations) {
        Write-Host "  - $v" -ForegroundColor Red
    }
    Write-Host "==========================================================" -ForegroundColor Red
    throw "Signature verification failed under $Policy policy with $($Violations.Count) violation(s)."
}

Write-Host "`n[SUCCESS] Signature Policy ($Policy) Verification Passed!" -ForegroundColor Green
