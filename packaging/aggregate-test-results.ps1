# MultiNet CyberShield AI - Unified Test Results Aggregator
# Milestone: NM-07.1.2 (Runtime Assertion Hardening & Strict Gate Qualification)
param (
    [string]$LifecycleResults = "$PSScriptRoot\output\lifecycle-logs\test-results.json",
    [string]$DesktopResults   = "$PSScriptRoot\output\desktop-runtime-logs\test-results.json",
    [string]$NetworkResults   = "$PSScriptRoot\output\network-diagnostics\test-results.json",
    [string]$OutputFile       = "$PSScriptRoot\output\final-qualification-report.json",
    [switch]$Strict = ($env:CI -eq 'true')
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Test Results Aggregation" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1.2 (Unified Qualification Report)" -ForegroundColor Cyan
Write-Host "Strict Mode Active: $Strict" -ForegroundColor $(if ($Strict) { "Yellow" } else { "DarkGray" })
Write-Host "==========================================================" -ForegroundColor Cyan

$sources = @(
    @{ Name = "MSI Lifecycle Qualification"; Path = $LifecycleResults },
    @{ Name = "Desktop Runtime Qualification"; Path = $DesktopResults },
    @{ Name = "Network Recovery Qualification"; Path = $NetworkResults }
)

$allSuites = [System.Collections.Generic.List[PSCustomObject]]::new()
$allTests = [System.Collections.Generic.List[PSCustomObject]]::new()

$totalPassed = 0
$totalFailed = 0
$totalSkipped = 0
$totalBlocked = 0
$totalNotTested = 0
$missingSuites = 0

foreach ($src in $sources) {
    Write-Host -NoNewline "Loading results from $($src.Name)... "
    if (-not (Test-Path $src.Path)) {
        Write-Host "[MISSING] at $($src.Path)" -ForegroundColor Red
        $missingSuites++
        $missingSuiteObj = [ordered]@{
            "suite"     = $src.Name
            "path"      = $src.Path
            "status"    = "MISSING"
            "summary"   = [ordered]@{
                "total"      = 0
                "passed"     = 0
                "failed"     = 1
                "skipped"    = 0
                "blocked"    = 0
                "not_tested" = 0
                "verdict"    = "FAIL"
            }
            "tests"     = @(
                [PSCustomObject]@{
                    "Name"    = "$($src.Name) execution"
                    "Status"  = "FAIL"
                    "Details" = "Results file not generated: $($src.Path)"
                }
            )
        }
        $allSuites.Add($missingSuiteObj)
        $totalFailed++
        continue
    }

    try {
        $raw = Get-Content -Path $src.Path -Raw -Encoding utf8
        $data = $raw | ConvertFrom-Json
        Write-Host "[LOADED] ($($data.summary.total) tests)" -ForegroundColor Green

        $allSuites.Add($data)
        $totalPassed += [int]($data.summary.passed)
        $totalFailed += [int]($data.summary.failed)
        $totalSkipped += [int]($data.summary.skipped)
        $totalBlocked += [int]($data.summary.blocked)
        $totalNotTested += [int]($data.summary.not_tested)

        if ($data.tests) {
            foreach ($t in $data.tests) {
                $attributed = [PSCustomObject]@{
                    "Suite"           = $data.suite
                    "Name"            = $t.Name
                    "Status"          = $t.Status
                    "Details"         = $t.Details
                    "DurationSeconds" = $t.DurationSeconds
                }
                $allTests.Add($attributed)
            }
        }
    } catch {
        Write-Host "[PARSE ERROR] $($_.Exception.Message)" -ForegroundColor Red
        $totalFailed++
        $missingSuites++
    }
}

$grandTotal = $totalPassed + $totalFailed + $totalSkipped + $totalBlocked + $totalNotTested

# Determine overall verdict
$finalVerdict = "PASS"
if ($totalFailed -gt 0 -or $missingSuites -gt 0) {
    $finalVerdict = "FAIL"
} elseif ($Strict -and ($totalSkipped -gt 0 -or $totalBlocked -gt 0)) {
    $finalVerdict = "FAIL"
} elseif ($grandTotal -eq 0) {
    $finalVerdict = "FAIL"
}

$unifiedReport = [ordered]@{
    "metadata" = [ordered]@{
        "product"          = "MultiNet CyberShield AI"
        "version"          = "0.1.0-beta.1"
        "milestone"        = "NM-07.1.2"
        "generated_at"     = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        "strict_mode"      = [bool]$Strict
        "ci_execution"     = ($env:CI -eq 'true')
        "platform"         = "Windows"
    }
    "summary"  = [ordered]@{
        "verdict"          = $finalVerdict
        "total_tests"      = $grandTotal
        "passed"           = $totalPassed
        "failed"           = $totalFailed
        "skipped"          = $totalSkipped
        "blocked"          = $totalBlocked
        "not_tested"       = $totalNotTested
        "missing_suites"   = $missingSuites
    }
    "suites"   = $allSuites
    "all_tests"= $allTests
}

# Ensure output directory exists and write JSON report without BOM
$outDir = Split-Path -Parent $OutputFile
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$jsonUtf8 = $unifiedReport | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($OutputFile, $jsonUtf8, $utf8NoBom)

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Unified Qualification Summary:" -ForegroundColor Cyan
Write-Host "  Total Tests:    $grandTotal" -ForegroundColor White
Write-Host "  Passed:         $totalPassed" -ForegroundColor Green
Write-Host "  Failed:         $totalFailed" -ForegroundColor $(if ($totalFailed -eq 0) { "Green" } else { "Red" })
Write-Host "  Skipped:        $totalSkipped" -ForegroundColor $(if ($totalSkipped -eq 0) { "Green" } else { if ($Strict) { "Red" } else { "Yellow" } })
Write-Host "  Blocked:        $totalBlocked" -ForegroundColor $(if ($totalBlocked -eq 0) { "Green" } else { if ($Strict) { "Red" } else { "Yellow" } })
Write-Host "  Verdict:        $finalVerdict" -ForegroundColor $(if ($finalVerdict -eq "PASS") { "Green" } else { "Red" })
Write-Host "Saved report to:  $OutputFile" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Enforce strict policy assertions
if ($Strict) {
    if ($missingSuites -gt 0) {
        Write-Host "[FAIL] Strict mode: $missingSuites test suite result files were missing." -ForegroundColor Red
        exit 1
    }
    if ($totalFailed -gt 0) {
        Write-Host "[FAIL] Strict mode: $totalFailed test(s) failed." -ForegroundColor Red
        exit 1
    }
    if ($totalSkipped -gt 0) {
        Write-Host "[FAIL] Strict mode: $totalSkipped test(s) were skipped (zero skipped allowed in strict CI)." -ForegroundColor Red
        exit 1
    }
    if ($totalBlocked -gt 0) {
        Write-Host "[FAIL] Strict mode: $totalBlocked test(s) were blocked (zero blocked allowed in strict CI)." -ForegroundColor Red
        exit 1
    }
    if ($totalPassed -eq 0) {
        Write-Host "[FAIL] Strict mode: No tests passed." -ForegroundColor Red
        exit 1
    }
    Write-Host "[SUCCESS] Strict qualification criteria met: Passed=$totalPassed, Failed=0, Skipped=0, Blocked=0." -ForegroundColor Green
    exit 0
} else {
    if ($finalVerdict -ne "PASS") {
        Write-Host "[FAIL] Non-strict qualification failed (failures detected)." -ForegroundColor Red
        exit 1
    }
    Write-Host "[SUCCESS] Qualification report generated successfully." -ForegroundColor Green
    exit 0
}
