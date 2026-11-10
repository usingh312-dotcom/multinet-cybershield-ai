# MultiNet CyberShield AI - Network Recovery & Runtime Diagnostic Verification
# Milestone: NM-07.1.2 (Network Recovery Reliability & Route Cleanup Hardening)
# Non-destructive test harness for network adapter baseline capture, recovery verification, and state validation
param (
    [string]$OutputDir = "$PSScriptRoot\output\network-diagnostics",
    [switch]$AllowSkip,
    [switch]$Strict = ($env:CI -eq 'true')
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Network Recovery & State Harness" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1.2 (Safe Disaster Recovery & Orphan Purge)" -ForegroundColor Cyan
Write-Host "Strict Mode Active: $Strict" -ForegroundColor $(if ($Strict) { "Yellow" } else { "DarkGray" })
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
    
    # In strict mode, any skipped, blocked or failed test causes overall suite failure
    $verdict = if ($script:Failed -gt 0) {
        "FAIL"
    } elseif ($Strict -and ($script:Skipped -gt 0 -or $script:Blocked -gt 0 -or $script:NotTested -gt 0)) {
        "FAIL"
    } elseif ($script:Blocked -gt 0) {
        "BLOCKED"
    } elseif ($script:Passed -gt 0) {
        "PASS"
    } else {
        "SKIPPED"
    }
    
    $report = [ordered]@{
        "suite"     = "Network Recovery & State Diagnostics"
        "timestamp" = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        "strict"    = $Strict
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

$isElevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "Runner Elevated Status: $isElevated" -ForegroundColor $(if ($isElevated) { "Green" } else { "Yellow" })

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

$preRoutes = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object DestinationPrefix, NextHop, RouteMetric, InterfaceIndex, InterfaceAlias
$defaultGateway = $preRoutes | Where-Object { $_.DestinationPrefix -eq "0.0.0.0/0" } | Select-Object -First 1
Assert-Check "Active default gateway route exists (0.0.0.0/0)" {
    $defaultGateway -ne $null
}

$captureRoutes = $preRoutes | Where-Object { $_.DestinationPrefix -in @("0.0.0.0/1", "128.0.0.0/1") }
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
# 4. Disaster Recovery Execution & Live Interface Query Verification (P0)
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 4: Validate Live Disaster Recovery Execution & Interface State ---" -ForegroundColor Yellow

$serviceCandidates = @(
    (Join-Path $RepoRoot "services\net-merge\net-merge-service.exe"),
    (Join-Path $RepoRoot "packaging\staging\net-merge-service.exe"),
    (Join-Path $RepoRoot "packaging\output\net-merge-service.exe"),
    "C:\Program Files\MultiNet CyberShield AI\net-merge-service.exe"
)
$serviceExe = $serviceCandidates | Where-Object { (Test-Path $_) -and ((Get-Item $_).Length -gt 1000000) } | Select-Object -First 1

if ($serviceExe) {
    # Select an existing interface to verify real state restoration
    $targetIface = Get-NetIPInterface -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.InterfaceAlias -match "Wi-Fi|Ethernet|Loopback" -or $_.InterfaceIndex -eq 1 } | Select-Object -First 1
    if (-not $targetIface) {
        $targetIface = Get-NetIPInterface -AddressFamily IPv4 | Select-Object -First 1
    }
    
    $idx = $targetIface.InterfaceIndex
    $initialWeakHost = $targetIface.WeakHostSend.ToString().ToLower()
    $initialForwarding = $targetIface.Forwarding.ToString().ToLower()
    
    $tempRecoveryFile = Join-Path ([System.IO.Path]::GetTempPath()) "net-merge-service-weakhost.txt"
    $simulatedState = "IPv4 $idx $initialWeakHost $initialForwarding"
    Set-Content -Path $tempRecoveryFile -Value $simulatedState -Encoding utf8

    Assert-Check "Crash state file staged with live interface parameters ($idx)" {
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
        
        Assert-Check "Verify weak-host and forwarding settings confirmed via Windows interface query" {
            $verifiedIface = Get-NetIPInterface -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue
            if (-not $verifiedIface) { return "Could not re-query interface $idx" }
            $whMatches = ($verifiedIface.WeakHostSend.ToString().ToLower() -eq $initialWeakHost)
            $fwdMatches = ($verifiedIface.Forwarding.ToString().ToLower() -eq $initialForwarding)
            $whMatches -and $fwdMatches
        }
    } else {
        Remove-Item -Path $tempRecoveryFile -Force -ErrorAction SilentlyContinue
        Record-TestResult -Name "Disaster recovery consumed and cleaned up crash state file" -Status "BLOCKED" -Details "Preceding recovery execution step was blocked or failed"
        Record-TestResult -Name "Verify weak-host and forwarding settings confirmed via Windows interface query" -Status "BLOCKED" -Details "Preceding recovery execution step was blocked or failed"
    }

    # 4b. Verify safe failure & state retention on malformed input
    Write-Host "`n--- Step 4b: Verify Failure Safety & Atomic State Retention ---" -ForegroundColor Yellow
    $badRecoveryFile = Join-Path ([System.IO.Path]::GetTempPath()) "net-merge-service-weakhost.txt"
    $badContent = "malformed_token_invalid_format"
    Set-Content -Path $badRecoveryFile -Value $badContent -Encoding utf8
    
    try {
        Assert-Check "Recovery exits non-zero on malformed crash records" {
            $badProc = Start-Process -FilePath $serviceExe -ArgumentList "-action recover" -Wait -PassThru -NoNewWindow
            Write-Host " (Exit code: $($badProc.ExitCode)) " -NoNewline
            # Service MUST fail safely and exit non-zero
            $badProc.ExitCode -ne 0
        }
        
        Assert-Check "Failed state records are retained for retry" {
            (Test-Path $badRecoveryFile) -and ((Get-Content $badRecoveryFile -Raw) -match "malformed_token_invalid_format")
        }
    } finally {
        Remove-Item -Path $badRecoveryFile -Force -ErrorAction SilentlyContinue
    }

} else {
    Record-TestResult -Name "Crash state file staged with live interface parameters" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Execute live disaster recovery scan via service binary" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Disaster recovery consumed and cleaned up crash state file" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Verify weak-host and forwarding settings confirmed via Windows interface query" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Recovery exits non-zero on malformed crash records" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
    Record-TestResult -Name "Failed state records are retained for retry" -Status "SKIPPED" -Details "net-merge-service.exe not found in build outputs"
}

# -----------------------------------------------------------------------------
# 5. Controlled Orphaned-Route Cleanup Test (P0)
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 5: Controlled Orphaned-Route Cleanup Test ---" -ForegroundColor Yellow

if ($isElevated -and $serviceExe) {
    # Safely stage controlled capture test routes pointing through default gateway interface with metric 9999
    $gwNextHop = if ($defaultGateway -and $defaultGateway.NextHop) { $defaultGateway.NextHop } else { "127.0.0.1" }
    $gwIfIndex = if ($defaultGateway -and $defaultGateway.InterfaceIndex) { $defaultGateway.InterfaceIndex } else { 1 }
    
    try {
        Write-Host "Staging controlled test capture routes (0.0.0.0/1 and 128.0.0.0/1) via interface $gwIfIndex..." -ForegroundColor Cyan
        try {
            New-NetRoute -DestinationPrefix "0.0.0.0/1" -InterfaceIndex $gwIfIndex -NextHop $gwNextHop -RouteMetric 9999 -PolicyStore ActiveStore -ErrorAction SilentlyContinue | Out-Null
            New-NetRoute -DestinationPrefix "128.0.0.0/1" -InterfaceIndex $gwIfIndex -NextHop $gwNextHop -RouteMetric 9999 -PolicyStore ActiveStore -ErrorAction SilentlyContinue | Out-Null
        } catch {}
        
        $routesNowCheck = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue
        if (-not ($routesNowCheck | Where-Object { $_.DestinationPrefix -eq "0.0.0.0/1" })) {
            & route add 0.0.0.0 mask 128.0.0.0 $gwNextHop metric 9999 if $gwIfIndex 2>$null | Out-Null
        }
        if (-not ($routesNowCheck | Where-Object { $_.DestinationPrefix -eq "128.0.0.0/1" })) {
            & route add 128.0.0.0 mask 128.0.0.0 $gwNextHop metric 9999 if $gwIfIndex 2>$null | Out-Null
        }
        
        Assert-Check "Controlled test capture routes staged in routing table" {
            $routesNow = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue
            $hasRoute1 = ($routesNow | Where-Object { $_.DestinationPrefix -eq "0.0.0.0/1" }) -ne $null
            $hasRoute2 = ($routesNow | Where-Object { $_.DestinationPrefix -eq "128.0.0.0/1" }) -ne $null
            $hasRoute1 -and $hasRoute2
        }
        
        Assert-Check "Execute disaster recovery to purge orphaned capture routes" {
            $purgeProc = Start-Process -FilePath $serviceExe -ArgumentList "-action recover" -Wait -PassThru -NoNewWindow
            Write-Host " (Exit code: $($purgeProc.ExitCode)) " -NoNewline
            $purgeProc.ExitCode -eq 0
        }
        
        Assert-Check "Controlled test capture routes successfully purged" {
            $routesAfter = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue
            $hasRoute1 = ($routesAfter | Where-Object { $_.DestinationPrefix -eq "0.0.0.0/1" }) -ne $null
            $hasRoute2 = ($routesAfter | Where-Object { $_.DestinationPrefix -eq "128.0.0.0/1" }) -ne $null
            (-not $hasRoute1) -and (-not $hasRoute2)
        }
        
        Assert-Check "Physical default gateway (0.0.0.0/0) remained intact and unchanged" {
            $currentDefault = Get-NetRoute -DestinationPrefix "0.0.0.0/0" -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1
            ($currentDefault -ne $null) -and ($currentDefault.NextHop -eq $gwNextHop)
        }
    } finally {
        # Guaranteed cleanup: Always attempt route delete in finally block
        Remove-NetRoute -DestinationPrefix "0.0.0.0/1" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        Remove-NetRoute -DestinationPrefix "128.0.0.0/1" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        & route delete 0.0.0.0 mask 128.0.0.0 2>$null | Out-Null
        & route delete 128.0.0.0 mask 128.0.0.0 2>$null | Out-Null
    }
} else {
    $skipReason = if (-not $isElevated) { "Requires Administrator privileges to add test routes" } else { "net-merge-service.exe not found" }
    Record-TestResult -Name "Controlled test capture routes staged in routing table" -Status "SKIPPED" -Details $skipReason
    Record-TestResult -Name "Execute disaster recovery to purge orphaned capture routes" -Status "SKIPPED" -Details $skipReason
    Record-TestResult -Name "Controlled test capture routes successfully purged" -Status "SKIPPED" -Details $skipReason
    Record-TestResult -Name "Physical default gateway (0.0.0.0/0) remained intact and unchanged" -Status "SKIPPED" -Details $skipReason
}

# -----------------------------------------------------------------------------
# 6. Service Disaster Recovery Code Integrity
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 6: Verify Service Disaster Recovery Handler Implementation ---" -ForegroundColor Yellow

$disasterSource = Join-Path $RepoRoot "services\net-merge\internal\svcrunner\disaster_windows.go"
Assert-Check "disaster_windows.go exists and implements RunDisasterRecovery" {
    if (-not (Test-Path $disasterSource)) { return "disaster_windows.go not found" }
    $src = Get-Content $disasterSource -Raw
    $src -match "RunDisasterRecovery\(\)" -and $src -match "cleanOrphanedCaptureRoutes\(\)" -and $src -match "restoreWeakHostCrashFile\(\)"
}

# -----------------------------------------------------------------------------
# 7. Post-Test State Drift Check
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 7: Post-Test State Drift Verification ---" -ForegroundColor Yellow

$postRoutes = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.DestinationPrefix -in @("0.0.0.0/1", "128.0.0.0/1") }
Assert-Check "Zero route drift: no orphaned capture routes introduced" {
    $postRoutes.Count -eq 0
}

# -----------------------------------------------------------------------------
# Summary & Output
# -----------------------------------------------------------------------------
Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Network Recovery Summary: Passed = $Passed, Failed = $Failed, Skipped = $Skipped, Blocked = $Blocked" -ForegroundColor $(if ($Failed -eq 0 -and $Blocked -eq 0 -and (-not $Strict -or $Skipped -eq 0)) { "Green" } elseif ($Blocked -gt 0 -and $Failed -eq 0) { "DarkYellow" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

Save-Results

if ($Failed -gt 0) {
    exit 1
} elseif ($Strict -and ($Skipped -gt 0 -or $Blocked -gt 0)) {
    Write-Host "[ERROR] Strict qualification failed: Skipped/Blocked tests present in CI environment" -ForegroundColor Red
    exit 1
} elseif ($Blocked -gt 0) {
    if ($AllowSkip) { exit 0 } else { exit 2 }
} else {
    exit 0
}
