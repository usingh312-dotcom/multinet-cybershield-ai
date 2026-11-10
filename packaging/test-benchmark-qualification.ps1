<#
.SYNOPSIS
    Automated qualification test suite for MultiNet Benchmark Lab (Milestone NM-08.0).
.DESCRIPTION
    Validates deterministic synthetic scheduler scenarios, loopback wire sockets,
    competitor import, comparability gate evaluation, and multi-format report generation.
    Strictly asserts zero failures, schema compliance, and correct BLOCKED registration
    for bonded end-to-end tests.
#>

[CmdletBinding()]
param(
    [string]$OutputDir = "artifacts/benchmark"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " NM-08.0: Benchmark Lab Qualification Test Suite" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$testResults = @()

function Record-Test($id, $name, $status, $detail) {
    $script:testResults += [PSCustomObject]@{
        Id     = $id
        Name   = $name
        Status = $status
        Detail = $detail
    }
    $color = if ($status -eq "PASS") { "Green" } elseif ($status -eq "BLOCKED") { "Yellow" } else { "Red" }
    Write-Host "[$status] $name : $detail" -ForegroundColor $color
}

# 1. Clean output directory
$runArtifactsDir = Join-Path $OutputDir "runs"
if (Test-Path $runArtifactsDir) {
    Remove-Item -Recurse -Force $runArtifactsDir
}
New-Item -ItemType Directory -Force -Path $runArtifactsDir | Out-Null

# 2. Run deterministic synthetic benchmark via netbench-cli
Write-Host "`n[Step 1] Running deterministic synthetic benchmark via netbench-cli..." -ForegroundColor Yellow
$proc = Start-Process -FilePath "cargo" -ArgumentList "run -p netbench-cli -- run --profile synthetic-3wan --seed 42 --output $runArtifactsDir" -NoNewWindow -PassThru -Wait

if ($proc.ExitCode -ne 0) {
    Record-Test "CLI_RUN" "Execution of netbench-cli run" "FAIL" "Process exited with code $($proc.ExitCode)"
    exit 1
} else {
    Record-Test "CLI_RUN" "Execution of netbench-cli run" "PASS" "Command completed with exit code 0"
}

# 3. Locate generated run directory
$runDirs = Get-ChildItem -Directory $runArtifactsDir
if ($runDirs.Count -eq 0) {
    Record-Test "RUN_DIR" "Locate output run directory" "FAIL" "No run directory produced in $runArtifactsDir"
    exit 1
}
$targetRunDir = $runDirs[0].FullName
$runId = $runDirs[0].Name
Write-Host "Found benchmark run ID: $runId at $targetRunDir" -ForegroundColor Green
Record-Test "RUN_DIR" "Locate output run directory" "PASS" "Run directory verified: $runId"

# 4. Verify all 5 artifact formats
$requiredArtifacts = @("manifest.json", "results.json", "summary.md", "report.html", "results.csv")
foreach ($art in $requiredArtifacts) {
    $artPath = Join-Path $targetRunDir $art
    if (Test-Path $artPath) {
        $size = (Get-Item $artPath).Length
        if ($size -gt 0) {
            Record-Test "ARTIFACT_$art" "Verify existence and non-zero size of $art" "PASS" "Size: $size bytes"
        } else {
            Record-Test "ARTIFACT_$art" "Verify existence and non-zero size of $art" "FAIL" "File is 0 bytes"
        }
    } else {
        Record-Test "ARTIFACT_$art" "Verify existence and non-zero size of $art" "FAIL" "File not found"
    }
}

# 5. Validate schema and results structure
$resultsJson = Get-Content (Join-Path $targetRunDir "results.json") -Raw | ConvertFrom-Json

if ($resultsJson.schema_version -eq "1.0.0") {
    Record-Test "SCHEMA_VERSION" "Validate schema_version == 1.0.0" "PASS" "Schema version is 1.0.0"
} else {
    Record-Test "SCHEMA_VERSION" "Validate schema_version == 1.0.0" "FAIL" "Got $($resultsJson.schema_version)"
}

if ($resultsJson.tier -eq "SYNTHETIC") {
    Record-Test "RUN_TIER" "Validate run tier is SYNTHETIC" "PASS" "Tier is SYNTHETIC"
} else {
    Record-Test "RUN_TIER" "Validate run tier is SYNTHETIC" "FAIL" "Got $($resultsJson.tier)"
}

if ($resultsJson.seed -eq 42) {
    Record-Test "RUN_SEED" "Validate deterministic seed" "PASS" "Seed is 42"
} else {
    Record-Test "RUN_SEED" "Validate deterministic seed" "FAIL" "Got $($resultsJson.seed)"
}

# Verify total tests and counts
$totalTests = $resultsJson.tests.Count
$passed = ($resultsJson.tests | Where-Object { $_.status -eq "PASS" }).Count
$blocked = ($resultsJson.tests | Where-Object { $_.status -eq "BLOCKED" }).Count
$failed = ($resultsJson.tests | Where-Object { $_.status -eq "FAIL" }).Count

if ($failed -eq 0) {
    Record-Test "ZERO_FAILURES" "Zero test failures in deterministic suite" "PASS" "Failed: 0"
} else {
    Record-Test "ZERO_FAILURES" "Zero test failures in deterministic suite" "FAIL" "Failed: $failed"
}

if ($blocked -ge 5) {
    Record-Test "BONDED_BLOCKED" "Bonded E2E tests registered as BLOCKED with rationale" "PASS" "Blocked count: $blocked"
} else {
    Record-Test "BONDED_BLOCKED" "Bonded E2E tests registered as BLOCKED with rationale" "FAIL" "Expected >= 5 blocked, got $blocked"
}

# 6. Test Competitor Import
Write-Host "`n[Step 2] Testing competitor import..." -ForegroundColor Yellow
$compOutDir = Join-Path $OutputDir "competitor"
$fixturePath = "tests/fixtures/speedify-sample.csv"

$compProc = Start-Process -FilePath "cargo" -ArgumentList "run -p netbench-cli -- competitor-import --input $fixturePath --output $compOutDir" -NoNewWindow -PassThru -Wait

if ($compProc.ExitCode -eq 0) {
    $imported = Get-ChildItem -Filter "*.json" $compOutDir
    if ($imported.Count -gt 0) {
        Record-Test "COMPETITOR_IMPORT" "Import competitor CSV fixture" "PASS" "Imported $($imported.Count) record(s)"
    } else {
        Record-Test "COMPETITOR_IMPORT" "Import competitor CSV fixture" "FAIL" "No JSON records created in $compOutDir"
    }
} else {
    Record-Test "COMPETITOR_IMPORT" "Import competitor CSV fixture" "FAIL" "Process exited with $($compProc.ExitCode)"
}

# 7. Test Comparability Evaluation Gate
Write-Host "`n[Step 3] Testing comparability evaluation gate..." -ForegroundColor Yellow
$compJsonFile = Join-Path $compOutDir "speedify-001.json"
$comparisonsDir = Join-Path $OutputDir "comparisons"

$compareProc = Start-Process -FilePath "cargo" -ArgumentList "run -p netbench-cli -- compare --multi-net $targetRunDir --competitor $compJsonFile --output $comparisonsDir" -NoNewWindow -PassThru -Wait

if ($compareProc.ExitCode -eq 0) {
    $compReports = Get-ChildItem -Filter "*.json" $comparisonsDir
    if ($compReports.Count -gt 0) {
        $compReportJson = Get-Content $compReports[0].FullName -Raw | ConvertFrom-Json
        # Must evaluate to NON_COMPARABLE because MultiNet run is SYNTHETIC
        if ($compReportJson.status.status -eq "NON_COMPARABLE") {
            Record-Test "COMPARABILITY_GATE" "Comparability rejects synthetic vs physical competitor" "PASS" "Correctly evaluated as NON_COMPARABLE"
        } else {
            Record-Test "COMPARABILITY_GATE" "Comparability rejects synthetic vs physical competitor" "FAIL" "Allowed synthetic comparison: $($compReportJson.status.status)"
        }
    } else {
        Record-Test "COMPARABILITY_GATE" "Generate comparison report" "FAIL" "No comparison report found"
    }
} else {
    Record-Test "COMPARABILITY_GATE" "Generate comparison report" "FAIL" "Process exited with $($compareProc.ExitCode)"
}

# 8. Summary & Exit Gate
Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host " Qualification Summary" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$passCount = ($testResults | Where-Object { $_.Status -eq "PASS" }).Count
$failCount = ($testResults | Where-Object { $_.Status -eq "FAIL" }).Count
$totalCount = $testResults.Count

Write-Host "Total Qualification Checks: $totalCount"
Write-Host "Passed:  $passCount" -ForegroundColor Green
Write-Host "Failed:  $failCount" -ForegroundColor Red

# Save machine-readable qualification results
$summaryReport = @{
    milestone = "NM-08.0"
    timestamp_utc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    checks_total = $totalCount
    checks_passed = $passCount
    checks_failed = $failCount
    results = $testResults
}
$summaryJson = $summaryReport | ConvertTo-Json -Depth 5
$summaryFile = Join-Path $OutputDir "benchmark-qualification-summary.json"
Set-Content -Path $summaryFile -Value $summaryJson -Encoding UTF8
Write-Host "Qualification summary written to: $summaryFile" -ForegroundColor Cyan

if ($failCount -gt 0) {
    Write-Error "Benchmark qualification gate failed with $failCount error(s)."
    exit 1
}

Write-Host "`nAll benchmark qualification criteria passed successfully." -ForegroundColor Green
exit 0
