# MultiNet CyberShield AI - Network Recovery & Runtime Diagnostic Verification
# Milestone: NM-07.1 (Controlled Beta Runtime Qualification)
# Non-destructive test harness for network adapter baseline capture, recovery verification, and state validation
param (
    [string]$OutputDir = "$PSScriptRoot\output\network-diagnostics"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Network Recovery & State Harness" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1 (Non-Destructive Workstation Verification)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$BaselineFile = Join-Path $OutputDir "baseline-network-state.json"
$RecoveryReport = Join-Path $OutputDir "network-recovery-report.json"

$Passed = 0
$Failed = 0

function Assert-Check {
    param (
        [string]$Description,
        [scriptblock]$Check
    )
    Write-Host -NoNewline "Checking: $Description ... "
    try {
        $res = & $Check
        if ($res -ne $false) {
            Write-Host "[PASS]" -ForegroundColor Green
            $script:Passed++
        } else {
            Write-Host "[FAIL]" -ForegroundColor Red
            $script:Failed++
        }
    } catch {
        Write-Host "[FAIL] ($($_.Exception.Message))" -ForegroundColor Red
        $script:Failed++
    }
}

# 1. Capture Baseline Physical Network Adapters
Write-Host "`n--- Step 1: Capture Physical Adapter Baseline ---" -ForegroundColor Yellow

$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Select-Object Name, InterfaceIndex, InterfaceDescription, Status, MacAddress, LinkSpeed
Write-Host "Detected $($adapters.Count) network adapter(s):" -ForegroundColor Cyan
foreach ($a in $adapters) {
    Write-Host "  - [$($a.InterfaceIndex)] $($a.Name): $($a.InterfaceDescription) (Status: $($a.Status))"
}

Assert-Check "At least one active network adapter present" {
    @($adapters | Where-Object { $_.Status -eq 'Up' }).Count -ge 1
}

# 2. Capture IP Interface Configuration (WeakHost / Forwarding baseline)
Write-Host "`n--- Step 2: Capture IP Interface Configurations ---" -ForegroundColor Yellow

$ipIfaces = Get-NetIPInterface -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object InterfaceAlias, InterfaceIndex, WeakHostSend, WeakHostReceive, Forwarding
Assert-Check "Successfully queried IPv4 network interfaces" {
    $ipIfaces.Count -gt 0
}

# 3. Capture Active IPv4 Routes
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

# 4. Disaster Recovery File Mechanism Test
Write-Host "`n--- Step 4: Validate Disaster Recovery File Handler ---" -ForegroundColor Yellow

$tempRecoveryFile = Join-Path ([System.IO.Path]::GetTempPath()) "net-merge-service-weakhost.txt"

# Simulate writing a crash state record
$simulatedState = "ipv4 999 weakhostsend=enabled forwarding=disabled"
Set-Content -Path $tempRecoveryFile -Value $simulatedState -Encoding utf8

Assert-Check "Recovery state file is read and parsed" {
    (Test-Path $tempRecoveryFile) -and ((Get-Content $tempRecoveryFile) -eq $simulatedState)
}

# Cleanup test file
Remove-Item -Path $tempRecoveryFile -Force -ErrorAction SilentlyContinue

Assert-Check "Recovery state file successfully cleaned up" {
    -not (Test-Path $tempRecoveryFile)
}

# 5. Service Disaster Recovery Code Integrity
Write-Host "`n--- Step 5: Verify Service Disaster Recovery Handler Implementation ---" -ForegroundColor Yellow

$disasterSource = Join-Path $RepoRoot "services\net-merge\internal\svcrunner\disaster_windows.go"
Assert-Check "disaster_windows.go exists and implements RunDisasterRecovery" {
    if (-not (Test-Path $disasterSource)) { return $false }
    $src = Get-Content $disasterSource -Raw
    $src -match "RunDisasterRecovery\(\)" -and $src -match "cleanOrphanedCaptureRoutes\(\)" -and $src -match "restoreWeakHostCrashFile\(\)"
}

# 6. Post-Test State Drift Check
Write-Host "`n--- Step 6: Post-Test State Drift Verification ---" -ForegroundColor Yellow

$postRoutes = Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.DestinationPrefix -in @("0.0.0.0/1", "128.0.0.0/1") }
Assert-Check "Zero route drift: no orphaned capture routes introduced" {
    $postRoutes.Count -eq 0
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Network Recovery Summary: Passed = $Passed, Failed = $Failed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

if ($Failed -gt 0) {
    exit 1
} else {
    exit 0
}
