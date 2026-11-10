# MultiNet CyberShield AI - Network Recovery & Runtime Diagnostic Verification
# Milestone: NM-07.1.1 (Controlled Beta Runtime Qualification)
# Non-destructive test harness for network adapter baseline capture, recovery verification, and state validation
param (
    [string]$OutputDir = "$PSScriptRoot\output\network-diagnostics",
    [switch]$AllowSkip
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Network Recovery & State Harness" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1.1 (Real Disaster Recovery & State Drift)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$BaselineFile = Join-Path $OutputDir "baseline-network-state.json"
$ResultsFile = Join-Path $OutputDir "test-results.json"
$SuiteStopwatch = [System.Diagnostics.Stopwatch]::StartNew()

$TestResults = [System.Collections.Generic.List[PSCustomObject]]::new()
$script:Passed = 0
$script:Failed = 0
$script:Skipped = 0
$script:Blocked = 0
$script:NotTested = 0

function Record-TestResult {
    param (
        [string]$Name,
        [string]$Status, # PASS, FAIL, SKIPPED, BLOCKED, NOT TESTED
        [string]$Details = "",
        [double]$DurationSeconds = 0.0
    )
    $obj = [PSCustomObject]@{
        "Name"            = $Name
        "Status"          = $Status
        "Details"         = $Details
        "DurationSeconds" = [Math]::Round($DurationSeconds, 4)
    }
    $script:TestResults.Add($obj)
    switch ($Status) {
        "PASS"       { Write-Host "[$Status]" -ForegroundColor Green; $script:Passed++ }
        "FAIL"       { Write-Host "[$Status] $Details" -ForegroundColor Red; $script:Failed++ }
        "SKIPPED"    { Write-Host "[$Status] $Details" -ForegroundColor Yellow; $script:Skipped++ }
        "BLOCKED"    { Write-Host "[$Status] $Details" -ForegroundColor DarkYellow; $script:Blocked++ }
        "NOT TESTED" { Write-Host "[$Status] $Details" -ForegroundColor Gray; $script:NotTested++ }
    }
}

function Save-Results {
    $SuiteStopwatch.Stop()
    $total = $script:Passed + $script:Failed + $script:Skipped + $script:Blocked + $script:NotTested
    $verdict = if ($script:Failed -gt 0) { "FAIL" } elseif ($script:Blocked -gt 0) { "BLOCKED" } elseif ($script:Passed -gt 0) { "PASS" } else { "SKIPPED" }
    
    $report = [ordered]@{
        "suite"     = "Network Recovery & State Diagnostics"
        "timestamp" = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        "summary"   = [ordered]@{
            "total"            = $total
            "passed"           = $script:Passed
            "failed"           = $script:Failed
            "skipped"          = $script:Skipped
            "blocked"          = $script:Blocked
            "not_tested"       = $script:NotTested
            "duration_seconds" = [Math]::Round($SuiteStopwatch.Elapsed.TotalSeconds, 2)
            "verdict"          = $verdict
        }
        "tests"     = $script:TestResults
    }
    $report | ConvertTo-Json -Depth 6 | Set-Content -Path $ResultsFile -Encoding utf8
    Write-Host "`n[OK] Machine-readable test results saved to: $ResultsFile" -ForegroundColor Cyan
}

function Assert-Check {
    param (
        [string]$Description,
        [scriptblock]$Check
    )
    Write-Host -NoNewline "Checking: $Description ... "
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $res = & $Check
        $sw.Stop()
        if ($res -eq $true) {
            Record-TestResult -Name $Description -Status "PASS" -DurationSeconds $sw.Elapsed.TotalSeconds
        } else {
            $msg = if ($res -is [string]) { $res } else { "Assertion returned false" }
            if ($msg -match "Application Control|blocked by rule|Smart App Control") {
                Record-TestResult -Name $Description -Status "BLOCKED" -Details "Blocked by Application Control: $msg" -DurationSeconds $sw.Elapsed.TotalSeconds
            } else {
                Record-TestResult -Name $Description -Status "FAIL" -Details $msg -DurationSeconds $sw.Elapsed.TotalSeconds
            }
        }
    } catch {
        $sw.Stop()
        $msg = $_.Exception.Message
        if ($msg -match "Application Control|blocked by rule|Smart App Control") {
            Record-TestResult -Name $Description -Status "BLOCKED" -Details "Blocked by Application Control: $msg" -DurationSeconds $sw.Elapsed.TotalSeconds
        } else {
            Record-TestResult -Name $Description -Status "FAIL" -Details $msg -DurationSeconds $sw.Elapsed.TotalSeconds
        }
    }
}

# -----------------------------------------------------------------------------
# 1. Capture Baseline Physical Network Adapters
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 1: Capture Physical Adapter Baseline ---" -ForegroundColor Yellow

$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Select-Object Name, InterfaceIndex, InterfaceDescription, Status, MacAddress, LinkSpeed
Write-Host "Detected $($adapters.Count) network adapter(s):" -ForegroundColor Cyan
foreach ($a in $adapters) {
    Write-Host "  - [$($a.InterfaceIndex)] $($a.Name): $($a.InterfaceDescription) (Status: $($a.Status))"
}

Assert-Check "At least one active network adapter present" {
    @($adapters | Where-Object { $_.Status -eq 'Up' }).Count -ge 1
}

# -----------------------------------------------------------------------------
# 2. Capture IP Interface Configuration (WeakHost / Forwarding baseline)
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 2: Capture IP Interface Configurations ---" -ForegroundColor Yellow

$ipIfaces = Get-NetIPInterface -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object InterfaceAlias, InterfaceIndex, WeakHostSend, WeakHostReceive, Forwarding
Assert-Check "Successfully queried IPv4 network interfaces" {
    $ipIfaces.Count -gt 0
}

# -----------------------------------------------------------------------------
# 3. Capture Active IPv4 Routes
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 3: Capture Routing Table State ---" -ForegroundColor Yellow

$activeRoutes = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object DestinationPrefix, NextHop, RouteMetric, InterfaceIndex, InterfaceAlias
$defaultGateway = $activeRoutes | Where-Object { $_.DestinationPrefix -eq "0.0.0.0/0" }
Assert-Check "Active default gateway route exists (0.0.0.0/0)" {
    $defaultGateway -ne $null
}

# Check for lingering orphaned capture routes from previous crashes
$captureRoutes = $activeRoutes | Where-Object { $_.DestinationPrefix -in @("0.0.0.0/1", "128.0.0.0/1") }
Assert-Check "No lingering orphaned Wintun capture routes active before test" {
    $captureRoutes.Count -eq 0
}

# Save complete baseline snapshot
$BaselineSnapshot = [ordered]@{
    "Timestamp"      = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    "HostName"       = $env:COMPUTERNAME
    "Adapters"       = $adapters
    "IPInterfaces"   = $ipIfaces
    "DefaultGateway" = $defaultGateway
}
$BaselineSnapshot | ConvertTo-Json -Depth 4 | Set-Content -Path $BaselineFile -Encoding utf8
Write-Host "[OK] Recorded baseline snapshot to $BaselineFile" -ForegroundColor Green

# -----------------------------------------------------------------------------
# 4. Disaster Recovery Execution Test (P2)
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 4: Validate Live Disaster Recovery Execution ---" -ForegroundColor Yellow

$serviceCandidates = @(
    (Join-Path $RepoRoot "services\net-merge\net-merge-service.exe"),
    (Join-Path $RepoRoot "packaging\staging\net-merge-service.exe"),
    (Join-Path $RepoRoot "packaging\output\net-merge-service.exe"),
    "C:\Program Files\MultiNet CyberShield AI\net-merge-service.exe"
)
$serviceExe = $serviceCandidates | Where-Object { (Test-Path $_) -and ((Get-Item $_).Length -gt 1000000) } | Select-Object -First 1

if ($serviceExe) {
    $tempRecoveryFile = Join-Path ([System.IO.Path]::GetTempPath()) "net-merge-service-weakhost.txt"
    $simulatedState = "IPv4 1 disabled disabled"
    Set-Content -Path $tempRecoveryFile -Value $simulatedState -Encoding utf8

    Assert-Check "Crash state file staged: net-merge-service-weakhost.txt" {
        Test-Path $tempRecoveryFile
    }

    $recoveryExecuted = $false
    try {
        Assert-Check "Execute live disaster recovery scan via service binary" {
            $proc = Start-Process -FilePath $serviceExe -ArgumentList "-action recover" -Wait -PassThru -NoNewWindow
            Write-Host " (Exit code: $($proc.ExitCode)) " -NoNewline
            if ($proc.ExitCode -eq 0) {
                $script:recoveryExecuted = $true
                $true
            } else {
                "Service recovery returned non-zero exit code: $($proc.ExitCode)"
            }
        }
    } finally {}

    if ($recoveryExecuted) {
        Assert-Check "Disaster recovery consumed and cleaned up crash state file" {
            -not (Test-Path $tempRecoveryFile)
        }
    } else {
        # Clean up staged file if recovery execution did not run
        Remove-Item -Path $tempRecoveryFile -Force -ErrorAction SilentlyContinue
        Record-TestResult -Name "Disaster recovery consumed and cleaned up crash state file" -Status "BLOCKED" -Details "Preceding recovery execution step was blocked or failed"
    }
} else {
    Record-TestResult -Name "Crash state file staged: net-merge-service-weakhost.txt" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Execute live disaster recovery scan via service binary" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Disaster recovery consumed and cleaned up crash state file" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
}

# -----------------------------------------------------------------------------
# 5. Service Disaster Recovery Code Integrity
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 5: Verify Service Disaster Recovery Handler Implementation ---" -ForegroundColor Yellow

$disasterSource = Join-Path $RepoRoot "services\net-merge\internal\svcrunner\disaster_windows.go"
Assert-Check "disaster_windows.go exists and implements RunDisasterRecovery" {
    if (-not (Test-Path $disasterSource)) { return "disaster_windows.go not found" }
    $src = Get-Content $disasterSource -Raw
    $src -match "RunDisasterRecovery\(\)" -and $src -match "cleanOrphanedCaptureRoutes\(\)" -and $src -match "restoreWeakHostCrashFile\(\)"
}

# -----------------------------------------------------------------------------
# 6. Post-Test State Drift Check
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 6: Post-Test State Drift Verification ---" -ForegroundColor Yellow

$postRoutes = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.DestinationPrefix -in @("0.0.0.0/1", "128.0.0.0/1") }
Assert-Check "Zero route drift: no orphaned capture routes introduced" {
    $postRoutes.Count -eq 0
}

# -----------------------------------------------------------------------------
# Summary & Output
# -----------------------------------------------------------------------------
Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Network Recovery Summary: Passed = $Passed, Failed = $Failed, Skipped = $Skipped, Blocked = $Blocked" -ForegroundColor $(if ($Failed -eq 0 -and $Blocked -eq 0) { "Green" } elseif ($Blocked -gt 0 -and $Failed -eq 0) { "DarkYellow" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

Save-Results

if ($Failed -gt 0) {
    exit 1
} elseif ($Blocked -gt 0) {
    if ($AllowSkip) { exit 0 } else { exit 2 }
} else {
    exit 0
}
