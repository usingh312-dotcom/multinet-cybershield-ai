# MultiNet CyberShield AI - MSI Installation & Lifecycle Qualification Test Harness
# Milestone: NM-07.1.2 (Runtime Test Integrity & Final Beta Qualification)
# Validates clean installation, service registration, Wintun placement, repair, and clean uninstallation
param (
    [string]$MsiPath = "$PSScriptRoot\output\MultiNetCyberShield-Setup.msi",
    [string]$LogDir = "$PSScriptRoot\output\lifecycle-logs",
    [switch]$SkipRepair,
    [switch]$SkipUninstall,
    [switch]$AllowSkip,
    [switch]$Strict = ($env:CI -eq 'true')
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - MSI Lifecycle Qualification Test" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1.2 (Runtime Environment Verification)" -ForegroundColor Cyan
Write-Host "Strict Mode Active: $Strict" -ForegroundColor $(if ($Strict) { "Yellow" } else { "DarkGray" })
Write-Host "==========================================================" -ForegroundColor Cyan

# Ensure log directory exists
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$ResultsFile = Join-Path $LogDir "test-results.json"
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
    $verdict = if ($script:Failed -gt 0) {
        "FAIL"
    } elseif ($Strict -and ($script:Skipped -gt 0 -or $script:Blocked -gt 0)) {
        "FAIL"
    } elseif ($script:Blocked -gt 0) {
        "BLOCKED"
    } elseif ($script:Passed -gt 0) {
        "PASS"
    } else {
        "SKIPPED"
    }
    
    $report = [ordered]@{
        "suite"     = "MSI Installation & Lifecycle Qualification"
        "timestamp" = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        "strict"    = [bool]$Strict
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
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    $json = $report | ConvertTo-Json -Depth 6
    [System.IO.File]::WriteAllText($ResultsFile, $json, $utf8NoBom)
    Write-Host "`n[OK] Machine-readable test results saved to: $ResultsFile" -ForegroundColor Cyan
}

function Assert-Step {
    param (
        [string]$Description,
        [scriptblock]$Check
    )
    Write-Host -NoNewline "Testing: $Description ... "
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $res = & $Check
        $sw.Stop()
        if ($res -eq $true) {
            Record-TestResult -Name $Description -Status "PASS" -DurationSeconds $sw.Elapsed.TotalSeconds
        } else {
            $msg = if ($res -is [string]) { $res } else { "Assertion returned false" }
            Record-TestResult -Name $Description -Status "FAIL" -Details $msg -DurationSeconds $sw.Elapsed.TotalSeconds
        }
    } catch {
        $sw.Stop()
        Record-TestResult -Name $Description -Status "FAIL" -Details $_.Exception.Message -DurationSeconds $sw.Elapsed.TotalSeconds
    }
}

# -----------------------------------------------------------------------------
# Check Administrator privileges
# -----------------------------------------------------------------------------
$isElevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "Runner Elevated Status: $isElevated" -ForegroundColor $(if ($isElevated) { "Green" } else { "Yellow" })

if (-not $isElevated) {
    Write-Host "[WARNING] This test harness requires Administrator privileges for real MSI installation & service control." -ForegroundColor Yellow
    Write-Host "When run under non-elevated user, live Windows Installer actions are BLOCKED." -ForegroundColor Yellow
    
    if ($AllowSkip -and -not $Strict) {
        Record-TestResult -Name "Administrator Privilege Requirement" -Status "SKIPPED" -Details "Runner is not elevated; skipped per -AllowSkip switch"
        Save-Results
        exit 0
    } else {
        Record-TestResult -Name "Administrator Privilege Requirement" -Status "BLOCKED" -Details "Runner is not elevated; Administrator privileges required"
        Save-Results
        exit 1
    }
}

# Resolve MSI file
if (-not (Test-Path $MsiPath)) {
    $altMsi = Join-Path $ScriptDir "staging\MultiNetCyberShield-Setup.msi"
    if (Test-Path $altMsi) {
        $MsiPath = $altMsi
    } else {
        $found = Get-ChildItem -Path $RepoRoot -Filter "MultiNetCyberShield-Setup.msi" -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) {
            $MsiPath = $found.FullName
        } else {
            Record-TestResult -Name "Resolve MSI Package" -Status "FAIL" -Details "MSI installer not found at $MsiPath or in repository tree"
            Save-Results
            exit 1
        }
    }
}
$FullMsiPath = (Resolve-Path $MsiPath).Path
Write-Host "Target MSI: $FullMsiPath ($((Get-Item $FullMsiPath).Length) bytes)" -ForegroundColor Cyan

$InstallLog = Join-Path $LogDir "msi-install.log"
$RepairLog = Join-Path $LogDir "msi-repair.log"
$UninstallLog = Join-Path $LogDir "msi-uninstall.log"

$ServiceName = "MultiNetNetMerge"
$DataDir = "C:\ProgramData\MultiNetCyberShield"

# -----------------------------------------------------------------------------
# 1. Clean Pre-Execution State
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 1: Pre-Execution Sanitation ---" -ForegroundColor Yellow

$existingSvc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existingSvc) {
    Write-Host "Stopping and removing existing pre-test service..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

# -----------------------------------------------------------------------------
# 2. Clean Installation Test
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 2: MSI Clean Installation ---" -ForegroundColor Yellow

Assert-Step "Execute MSI Clean Silent Installation" {
    Write-Host "`nRunning msiexec /i `"$FullMsiPath`" /qn /l*v `"$InstallLog`"..." -ForegroundColor Cyan
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList "/i `"$FullMsiPath`" /qn /l*v `"$InstallLog`"" -Wait -PassThru
    Write-Host "msiexec exited with code: $($process.ExitCode)" -ForegroundColor $(if ($process.ExitCode -eq 0) { "Green" } else { "Red" })
    if ($process.ExitCode -ne 0 -and (Test-Path $InstallLog)) {
        Write-Host "Install Log Tail:" -ForegroundColor Red
        Get-Content $InstallLog -Tail 40
    }
    $process.ExitCode -eq 0
}

# -----------------------------------------------------------------------------
# 3. Verify Installed Binaries & Signatures
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 3: Installed Assets & File Verification ---" -ForegroundColor Yellow

$resolvedDir = "C:\Program Files\MultiNet CyberShield AI"
if (-not (Test-Path $resolvedDir)) {
    $alt = "C:\Program Files (x86)\MultiNet CyberShield AI"
    if (Test-Path $alt) {
        $resolvedDir = $alt
    } else {
        $svcEntry = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
        if ($svcEntry -and $svcEntry.PathName) {
            $exe = $svcEntry.PathName.Trim('"').Trim()
            $resolvedDir = Split-Path -Parent $exe
        }
    }
}
$InstallDir = $resolvedDir
Write-Host "Resolved Installation Directory: $InstallDir" -ForegroundColor Cyan

$desktopExe = Join-Path $InstallDir "multinet-cybershield-ai.exe"
Assert-Step "Desktop GUI installed: multinet-cybershield-ai.exe" {
    (Test-Path $desktopExe) -and ((Get-Item $desktopExe).Length -gt 1000000)
}

$serviceExe = Join-Path $InstallDir "net-merge-service.exe"
Assert-Step "Service executable installed: net-merge-service.exe" {
    (Test-Path $serviceExe) -and ((Get-Item $serviceExe).Length -gt 1000000)
}

$wintunRoot = Join-Path $InstallDir "wintun.dll"
Assert-Step "Wintun DLL placed in application root" {
    (Test-Path $wintunRoot) -and ((Get-Item $wintunRoot).Length -gt 100000)
}

$wintunDrivers = Join-Path $InstallDir "drivers\wintun.dll"
Assert-Step "Wintun DLL placed in drivers\ subfolder" {
    (Test-Path $wintunDrivers) -and ((Get-Item $wintunDrivers).Length -gt 100000)
}

Assert-Step "Installed wintun.dll has valid WireGuard LLC signature" {
    $sig = Get-AuthenticodeSignature $wintunRoot
    $sig.Status -eq [System.Management.Automation.SignatureStatus]::Valid -and $sig.SignerCertificate.Subject -match "WireGuard LLC"
}

Assert-Step "Application icon resource present in installed desktop executable" {
    Add-Type -AssemblyName System.Drawing
    $extractedIcon = [System.Drawing.Icon]::ExtractAssociatedIcon($desktopExe)
    ($extractedIcon -ne $null) -and ($extractedIcon.Width -gt 0) -and ($extractedIcon.Height -gt 0)
}

# -----------------------------------------------------------------------------
# 4. Verify Shortcuts
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 4: Shortcuts & System Integration ---" -ForegroundColor Yellow

$startMenuFolder = "C:\ProgramData\Microsoft\Windows\Start Menu\Programs\MultiNet CyberShield AI"
Assert-Step "Start Menu folder created" {
    Test-Path $startMenuFolder
}

Assert-Step "Start Menu shortcut points to valid desktop executable target" {
    $smLnk = Join-Path $startMenuFolder "MultiNet CyberShield AI.lnk"
    if (-not (Test-Path $smLnk)) {
        $candidate = Get-ChildItem -Path $startMenuFolder -Filter "*.lnk" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($candidate) { $smLnk = $candidate.FullName }
    }
    if (-not (Test-Path $smLnk)) { return "Start menu shortcut file not found" }
    
    $sh = New-Object -ComObject WScript.Shell
    $sc = $sh.CreateShortcut($smLnk)
    $cleanTarget = $sc.TargetPath.Trim('"').Trim()
    $cleanWorkDir = $sc.WorkingDirectory.Trim('"').Trim()
    $expectedTarget = $desktopExe.Trim('"').Trim()
    $expectedWorkDir = $InstallDir.Trim('"').Trim()
    
    if ($cleanTarget -ne $expectedTarget) {
        return "Start menu shortcut TargetPath '$cleanTarget' does not match expected '$expectedTarget'"
    }
    if ($cleanWorkDir -ne $expectedWorkDir) {
        return "Start menu shortcut WorkingDirectory '$cleanWorkDir' does not match expected '$expectedWorkDir'"
    }
    $true
}

Assert-Step "Desktop shortcut created with valid target" {
    $desktopLnkCandidates = @(
        (Join-Path ([Environment]::GetFolderPath('CommonDesktopDirectory')) "MultiNet CyberShield AI.lnk"),
        "C:\Users\Public\Desktop\MultiNet CyberShield AI.lnk",
        (Join-Path ([Environment]::GetFolderPath('Desktop')) "MultiNet CyberShield AI.lnk")
    )
    $desktopLnk = $desktopLnkCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $desktopLnk) {
        $foundLnk = Get-ChildItem -Path "C:\Users\*\Desktop" -Filter "MultiNet CyberShield AI.lnk" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($foundLnk) { $desktopLnk = $foundLnk.FullName }
    }
    
    if (-not $desktopLnk) { return "Desktop shortcut (.lnk) file not found on public or user desktop" }
    
    $sh = New-Object -ComObject WScript.Shell
    $sc = $sh.CreateShortcut($desktopLnk)
    $cleanTarget = $sc.TargetPath.Trim('"').Trim()
    $cleanWorkDir = $sc.WorkingDirectory.Trim('"').Trim()
    $expectedTarget = $desktopExe.Trim('"').Trim()
    $expectedWorkDir = $InstallDir.Trim('"').Trim()
    
    if ($cleanTarget -ne $expectedTarget) {
        return "Desktop shortcut TargetPath '$cleanTarget' does not match expected '$expectedTarget'"
    }
    if ($cleanWorkDir -ne $expectedWorkDir) {
        return "Desktop shortcut WorkingDirectory '$cleanWorkDir' does not match expected '$expectedWorkDir'"
    }
    $true
}

# -----------------------------------------------------------------------------
# 5. Verify Windows Service Registration & Runtime State
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 5: SCM Service Registration & Runtime ---" -ForegroundColor Yellow

Assert-Step "Windows Service registered in SCM with LocalSystem account ($ServiceName)" {
    $cim = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
    if (-not $cim) { return "Service not found in SCM" }
    $cleanPath = $cim.PathName.Trim('"').Trim()
    $expectedPath = $serviceExe.Trim('"').Trim()
    $pathMatches = ($cleanPath -eq $expectedPath -or $cleanPath -like "*net-merge-service.exe*")
    $accountMatches = ($cim.StartName -match "LocalSystem|SYSTEM")
    if (-not $pathMatches) { return "Service PathName '$cleanPath' does not match '$expectedPath'" }
    if (-not $accountMatches) { return "Service StartName '$($cim.StartName)' is not LocalSystem" }
    $true
}

Assert-Step "Windows Service auto-started by installer upon completion" {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) { return "Service $ServiceName not found in SCM" }
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Running) {
        return "Service was not running immediately after install (Status: $($svc.Status))"
    }
    $true
}

Assert-Step "Manual service stop and start lifecycle verification" {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) { return "Service $ServiceName not found in SCM" }
    Stop-Service -Name $ServiceName -Force
    Start-Sleep -Seconds 2
    $svc.Refresh()
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
        return "Failed to stop service manually (Status: $($svc.Status))"
    }
    Start-Service -Name $ServiceName
    Start-Sleep -Seconds 2
    $svc.Refresh()
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Running) {
        return "Failed to restart service manually (Status: $($svc.Status))"
    }
    $true
}

Assert-Step "Service named-pipe IPC responds to health query (\\.\pipe\multinet-netmerge-v1)" {
    $connected = $false
    $response = $null
    for ($i = 0; $i -lt 5; $i++) {
        try {
            $pipeClient = New-Object System.IO.Pipes.NamedPipeClientStream(".", "multinet-netmerge-v1", [System.IO.Pipes.PipeDirection]::InOut)
            $pipeClient.Connect(2000)
            if ($pipeClient.IsConnected) {
                $writer = New-Object System.IO.StreamWriter($pipeClient)
                $writer.AutoFlush = $true
                $reader = New-Object System.IO.StreamReader($pipeClient)
                
                $req = '{"id":"lifecycle-health-chk","pipe_version":1,"method":"health"}'
                $writer.WriteLine($req)
                $response = $reader.ReadLine()
                $connected = $true
                $pipeClient.Dispose()
                break
            }
        } catch {
            Start-Sleep -Milliseconds 1000
        }
    }
    if (-not $connected) { return "Could not connect to named pipe \\.\pipe\multinet-netmerge-v1 after retries" }
    if (-not $response) { return "Received empty response from named pipe" }
    Write-Host " (IPC Response: $response) " -NoNewline
    
    try {
        $json = $response | ConvertFrom-Json
    } catch {
        return "Failed to parse IPC response as JSON: $response"
    }
    if ($json.ok -ne $true) {
        return "IPC health response returned ok != true: $response"
    }
    if ($json.pipe_version -ne 1) {
        return "IPC health response returned pipe_version != 1 (was $($json.pipe_version)): $response"
    }
    $svcName = if ($json.result -and $json.result.service) { $json.result.service } elseif ($json.service) { $json.service } else { $null }
    if ($svcName -ne "net-merge") {
        return "IPC health response service != 'net-merge' (was '$svcName'): $response"
    }
    $true
}

# -----------------------------------------------------------------------------
# 6. MSI Repair Verification
# -----------------------------------------------------------------------------
if (-not $SkipRepair) {
    Write-Host "`n--- Stage 6: MSI Repair Validation ---" -ForegroundColor Yellow
    
    Write-Host "Simulating file corruption: deleting $wintunDrivers..." -ForegroundColor Cyan
    Remove-Item $wintunDrivers -Force -ErrorAction SilentlyContinue

    Assert-Step "Verification that corrupted file is missing" {
        -not (Test-Path $wintunDrivers)
    }

    Assert-Step "Execute MSI Repair (/fa)" {
        Write-Host "Running msiexec /fa `"$FullMsiPath`" /qn /l*v `"$RepairLog`"..." -ForegroundColor Cyan
        $repairProc = Start-Process -FilePath "msiexec.exe" -ArgumentList "/fa `"$FullMsiPath`" /qn /l*v `"$RepairLog`"" -Wait -PassThru
        Write-Host "msiexec repair exited with code: $($repairProc.ExitCode)" -ForegroundColor $(if ($repairProc.ExitCode -eq 0) { "Green" } else { "Red" })
        if ($repairProc.ExitCode -ne 0 -and (Test-Path $RepairLog)) {
            Write-Host "Repair Log Tail:" -ForegroundColor Red
            Get-Content $RepairLog -Tail 40
        }
        $repairProc.ExitCode -eq 0
    }

    Assert-Step "MSI Repair restored deleted file with valid Authenticode signature (drivers\wintun.dll)" {
        if (-not (Test-Path $wintunDrivers)) { return "Restored file drivers\wintun.dll not found" }
        $item = Get-Item $wintunDrivers
        if ($item.Length -lt 100000) { return "Restored file drivers\wintun.dll length ($($item.Length)) is unexpectedly small" }
        $sig = Get-AuthenticodeSignature $wintunDrivers
        if ($sig.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or $sig.SignerCertificate.Subject -notmatch "WireGuard LLC") {
            return "Restored drivers\wintun.dll signature verification failed: $($sig.StatusMessage)"
        }
        $true
    }
} else {
    Record-TestResult -Name "MSI Repair Validation" -Status "SKIPPED" -Details "Skipped via -SkipRepair switch"
}

# -----------------------------------------------------------------------------
# 7. Uninstallation & Cleanup Verification
# -----------------------------------------------------------------------------
if (-not $SkipUninstall) {
    Write-Host "`n--- Stage 7: Clean Uninstallation & Cleanup ---" -ForegroundColor Yellow

    Assert-Step "Execute MSI Uninstallation (/x)" {
        Write-Host "Running msiexec /x `"$FullMsiPath`" /qn /l*v `"$UninstallLog`"..." -ForegroundColor Cyan
        $uninstProc = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x `"$FullMsiPath`" /qn /l*v `"$UninstallLog`"" -Wait -PassThru
        Write-Host "msiexec uninstall exited with code: $($uninstProc.ExitCode)" -ForegroundColor $(if ($uninstProc.ExitCode -eq 0) { "Green" } else { "Red" })
        if ($uninstProc.ExitCode -ne 0 -and (Test-Path $UninstallLog)) {
            Write-Host "Uninstall Log Tail:" -ForegroundColor Red
            Get-Content $UninstallLog -Tail 40
        }
        $uninstProc.ExitCode -eq 0
    }

    Start-Sleep -Seconds 3

    Assert-Step "Service removed from Windows Service Control Manager" {
        $svcAfter = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        $svcAfter -eq $null
    }

    Assert-Step "Application installation directory removed" {
        -not (Test-Path (Join-Path $InstallDir "net-merge-service.exe"))
    }

    Assert-Step "Start Menu shortcut directory removed" {
        -not (Test-Path $startMenuFolder)
    }

    Assert-Step "Zero orphaned routes (0.0.0.0/1, 128.0.0.0/1)" {
        $routes = Get-NetRoute -DestinationPrefix "0.0.0.0/1", "128.0.0.0/1" -ErrorAction SilentlyContinue
        $routes -eq $null -or $routes.Count -eq 0
    }
} else {
    Record-TestResult -Name "MSI Uninstallation & Cleanup" -Status "SKIPPED" -Details "Skipped via -SkipUninstall switch"
}

# -----------------------------------------------------------------------------
# Summary & Output
# -----------------------------------------------------------------------------
Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Lifecycle Test Summary: Passed = $Passed, Failed = $Failed, Skipped = $Skipped, Blocked = $Blocked" -ForegroundColor $(if ($Failed -eq 0 -and $Blocked -eq 0) { "Green" } else { "Red" })
Write-Host "Logs Staged to: $LogDir" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Save-Results

if ($Failed -gt 0) {
    Write-Host "[ERROR] Installer lifecycle qualification failed ($Failed failures)!" -ForegroundColor Red
    exit 1
} elseif ($Blocked -gt 0 -or $Skipped -gt 0) {
    if ($Strict) {
        Write-Host "[ERROR] Strict mode active: Skipped ($Skipped) or Blocked ($Blocked) tests cause failure." -ForegroundColor Red
        exit 1
    }
    if ($Blocked -gt 0) {
        Write-Host "[WARNING] Installer lifecycle qualification blocked ($Blocked blocked)." -ForegroundColor DarkYellow
        if ($AllowSkip) { exit 0 } else { exit 2 }
    } else {
        exit 0
    }
} else {
    Write-Host "[SUCCESS] All installer lifecycle qualification tests passed!" -ForegroundColor Green
    exit 0
}
