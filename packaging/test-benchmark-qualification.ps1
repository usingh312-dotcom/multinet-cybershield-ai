<#
.SYNOPSIS
    Automated qualification test suite for MultiNet Benchmark Lab (Milestone NM-08.0.2).
.DESCRIPTION
    Validates deterministic synthetic scheduler scenarios, loopback wire sockets,
    interface IP fail-closed validation, canonical & legacy competitor import,
    16-gate comparability evaluation, and multi-format report generation.
    Strictly asserts zero failures, schema compliance, and correct BLOCKED registration
    for bonded end-to-end tests.
#>

[CmdletBinding()]
param(
    [string]$OutputDir = "artifacts/benchmark"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " NM-08.0.2: Benchmark Lab Qualification Test Suite" -ForegroundColor Cyan
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
cargo run -p netbench-cli -- run --profile synthetic-3wan --seed 42 --output $runArtifactsDir
$runExitCode = $LASTEXITCODE

if ($runExitCode -ne 0) {
    Record-Test "CLI_RUN" "Execution of netbench-cli run" "FAIL" "Process exited with code $runExitCode"
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

# Individual test assertions: verify every expected test ID exists and matches expected status
$expectedPassTests = @(
    "synth_sched_single_path",
    "synth_sched_dual_equal",
    "synth_sched_tri_asymmetric",
    "synth_sched_degraded_shedding",
    "synth_sched_loss_and_recovery",
    "synth_sched_all_unavailable",
    "synth_sched_mtu_exclusion",
    "synth_sched_anomaly_handling",
    "synth_sched_zero_capacity",
    "loopback_udp_framing"
)

foreach ($tid in $expectedPassTests) {
    $tObj = $resultsJson.tests | Where-Object { $_.test_id -eq $tid }
    if ($null -ne $tObj) {
        if ($tObj.status -eq "PASS") {
            Record-Test "TEST_ID_$tid" "Test ID $tid executed and passed" "PASS" "Status: PASS"
        } else {
            Record-Test "TEST_ID_$tid" "Test ID $tid executed and passed" "FAIL" "Expected PASS, got $($tObj.status): $($tObj.reason)"
        }
    } else {
        Record-Test "TEST_ID_$tid" "Test ID $tid executed and passed" "FAIL" "Test ID not found in results"
    }
}

$expectedBlockedTests = @(
    "bonded_e2e_2wan_single_flow",
    "bonded_e2e_3wan_asymmetric",
    "bonded_e2e_relay_reassembly",
    "bonded_e2e_single_flow_failover",
    "bonded_e2e_return_traffic"
)

foreach ($tid in $expectedBlockedTests) {
    $tObj = $resultsJson.tests | Where-Object { $_.test_id -eq $tid }
    if ($null -ne $tObj) {
        if ($tObj.status -eq "BLOCKED") {
            if (-not [string]::IsNullOrWhiteSpace($tObj.reason) -and $tObj.reason.StartsWith("BLOCKED:")) {
                Record-Test "TEST_ID_$tid" "Test ID $tid registered as BLOCKED with architectural rationale" "PASS" "Rationale verified"
            } else {
                Record-Test "TEST_ID_$tid" "Test ID $tid registered as BLOCKED with architectural rationale" "FAIL" "Empty or invalid rationale"
            }
        } else {
            Record-Test "TEST_ID_$tid" "Test ID $tid registered as BLOCKED with architectural rationale" "FAIL" "Expected BLOCKED, got $($tObj.status)"
        }
    } else {
        Record-Test "TEST_ID_$tid" "Test ID $tid registered as BLOCKED with architectural rationale" "FAIL" "Test ID not found in results"
    }
}

# Verify total tests and counts
$totalTests = $resultsJson.tests.Count
$passed = ($resultsJson.tests | Where-Object { $_.status -eq "PASS" }).Count
$blocked = ($resultsJson.tests | Where-Object { $_.status -eq "BLOCKED" }).Count
$failed = ($resultsJson.tests | Where-Object { $_.status -eq "FAIL" }).Count

if ($failed -eq 0 -and $passed -eq 10 -and $blocked -eq 5) {
    Record-Test "SUITE_INTEGRITY" "Suite totals match specification (10 pass, 5 blocked, 0 fail)" "PASS" "Total: $totalTests (10 PASS, 5 BLOCKED, 0 FAIL)"
} else {
    Record-Test "SUITE_INTEGRITY" "Suite totals match specification (10 pass, 5 blocked, 0 fail)" "FAIL" "Got $passed PASS, $blocked BLOCKED, $failed FAIL"
}

# 6. Test Competitor Import (Canonical and Legacy Schemas)
Write-Host "`n[Step 2] Testing competitor import..." -ForegroundColor Yellow
$compOutDir = Join-Path $OutputDir "competitor"
$legacyFixturePath = "tests/fixtures/speedify-sample.csv"
$canonicalFixturePath = "tests/fixtures/speedify-canonical.csv"

cargo run -p netbench-cli -- competitor-import --input $legacyFixturePath --output $compOutDir
$compLegacyExitCode = $LASTEXITCODE

if ($compLegacyExitCode -eq 0) {
    Record-Test "COMPETITOR_IMPORT_LEGACY" "Import legacy 13-field competitor CSV fixture" "PASS" "Imported legacy record"
} else {
    Record-Test "COMPETITOR_IMPORT_LEGACY" "Import legacy 13-field competitor CSV fixture" "FAIL" "Process exited with $compLegacyExitCode"
}

cargo run -p netbench-cli -- competitor-import --input $canonicalFixturePath --output $compOutDir
$compCanonicalExitCode = $LASTEXITCODE

if ($compCanonicalExitCode -eq 0) {
    Record-Test "COMPETITOR_IMPORT_CANONICAL" "Import canonical 16-field competitor CSV fixture" "PASS" "Imported canonical record"
} else {
    Record-Test "COMPETITOR_IMPORT_CANONICAL" "Import canonical 16-field competitor CSV fixture" "FAIL" "Process exited with $compCanonicalExitCode"
}

# 7. Test Comparability Evaluation Gate (16 Gates Enforced)
Write-Host "`n[Step 3] Testing comparability evaluation gate..." -ForegroundColor Yellow
$compJsonFile = Join-Path $compOutDir "speedify-001.json"
$comparisonsDir = Join-Path $OutputDir "comparisons"

cargo run -p netbench-cli -- compare --multi-net $targetRunDir --competitor $compJsonFile --output $comparisonsDir
$compareExitCode = $LASTEXITCODE

if ($compareExitCode -eq 0) {
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
    Record-Test "COMPARABILITY_GATE" "Generate comparison report" "FAIL" "Process exited with $compareExitCode"
}

# 8. Test Interface IP Parsing Fail-Closed Behavior
Write-Host "`n[Step 4] Testing CLI interface IP validation fail-closed behavior..." -ForegroundColor Yellow

# Negative Test 1: Explicit malformed IP address must fail closed with non-zero exit code
cargo run -p netbench-cli -- baseline --enable-live-network-tests --server-url "https://127.0.0.1:9000" --interface-local-ip "192.168.1.999" --allow-self-signed-loopback
$malformedIpExitCode = $LASTEXITCODE
if ($malformedIpExitCode -ne 0) {
    Record-Test "CLI_FAIL_CLOSED_MALFORMED_IP" "Explicit malformed interface IP terminates CLI with non-zero exit code" "PASS" "Exited with code $malformedIpExitCode (rejected as expected)"
} else {
    Record-Test "CLI_FAIL_CLOSED_MALFORMED_IP" "Explicit malformed interface IP terminates CLI with non-zero exit code" "FAIL" "Incorrectly succeeded with exit code 0"
}

# Negative Test 2: Explicit unassigned local IP address must fail closed with non-zero exit code
cargo run -p netbench-cli -- baseline --enable-live-network-tests --server-url "https://127.0.0.1:9000" --interface-local-ip "10.254.254.254" --allow-self-signed-loopback
$unassignedIpExitCode = $LASTEXITCODE
if ($unassignedIpExitCode -ne 0) {
    Record-Test "CLI_FAIL_CLOSED_UNASSIGNED_IP" "Unassigned interface IP terminates CLI with non-zero exit code" "PASS" "Exited with code $unassignedIpExitCode (rejected as expected)"
} else {
    Record-Test "CLI_FAIL_CLOSED_UNASSIGNED_IP" "Unassigned interface IP terminates CLI with non-zero exit code" "FAIL" "Incorrectly succeeded with exit code 0"
}

# 9. Summary & Exit Gate
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
    milestone = "NM-08.0.2"
    timestamp_utc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    checks_total = $totalCount
    checks_passed = $passCount
    checks_failed = $failCount
    results = $testResults
}
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

