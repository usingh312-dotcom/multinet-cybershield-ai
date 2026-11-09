# MultiNet CyberShield AI - Signature & Driver Verification Script
param (
    [string]$TargetDir = "..\staging",
    [switch]$FailOnUnsigned = $false
)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ResolvedTarget = Resolve-Path (Join-Path $ScriptDir $TargetDir)

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Authenticode Signature Verification" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$Targets = Get-ChildItem -Path $ResolvedTarget -Include *.exe, *.dll, *.msi, *.sys -Recurse

$Results = @()
$UnsignedCount = 0

foreach ($file in $Targets) {
    $sig = Get-AuthenticodeSignature $file.FullName
    $status = $sig.Status.ToString()
    $signer = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { "None" }
    $issuer = if ($sig.SignerCertificate) { $sig.SignerCertificate.Issuer } else { "None" }
    $timeSigned = if ($sig.SignerCertificate) { $sig.SignerCertificate.NotBefore.ToString("yyyy-MM-dd") } else { "N/A" }

    if ($status -ne "Valid") {
        $UnsignedCount++
    }

    $entry = [PSCustomObject]@{
        "File" = $file.Name
        "Status" = $status
        "Signer" = $signer
        "Issuer" = $issuer
        "Date" = $timeSigned
    }
    $Results += $entry
}

Write-Host "`nSignature Verification Summary:" -ForegroundColor Cyan
$Results | Format-Table -AutoSize

# Check Wintun driver specifically if present
$wintun = $Results | Where-Object { $_.File -eq "wintun.dll" }
if ($wintun) {
    Write-Host "`nWintun Driver Integrity Check:" -ForegroundColor Cyan
    Write-Host "  Driver Status: $($wintun.Status)"
    Write-Host "  Signer: $($wintun.Signer)"
}

Write-Host "`nTotal files audited: $($Results.Count)"
Write-Host "Unsigned / Untrusted count: $UnsignedCount"

if ($FailOnUnsigned -and $UnsignedCount -gt 0) {
    Write-Error "Production release failure: Found $UnsignedCount unsigned or untrusted binaries."
    exit 1
}

