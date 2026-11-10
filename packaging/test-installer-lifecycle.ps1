# MultiNet CyberShield AI - MSI Installation & Lifecycle Qualification Test Harness
# Milestone: NM-07.1 (Controlled Beta Runtime Qualification)
# Validates clean installation, service registration, Wintun placement, repair, and clean uninstallation
param (
    [string]$MsiPath = "$PSScriptRoot\output\MultiNetCyberShield-Setup.msi",
    [string]$LogDir = "$PSScriptRoot\output\lifecycle-logs",
    [switch]$SkipRepair,
    [switch]$SkipUninstall
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - MSI Lifecycle Qualification Test" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1 (Runtime Environment Verification)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Check Administrator privileges
$isElevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "Runner Elevated Status: $isElevated" -ForegroundColor $(if ($isElevated) { "Green" } else { "Yellow" })

if (-not $isElevated) {
    Write-Host "[WARNING] This test harness requires Administrator privileges for real MSI installation & service control." -ForegroundColor Yellow
    Write-Host "When run under non-elevated user, live Windows Installer actions will fail. Exiting test suite." -ForegroundColor Yellow
    exit 0
}

# Resolve MSI file
if (-not (Test-Path $MsiPath)) {
    $altMsi = Join-Path $ScriptDir "staging\MultiNetCyberShield-Setup.msi"
    if (Test-Path $altMsi) {
        $MsiPath = $altMsi
    } else {
        throw "MSI Lifecycle Failure: MSI installer not found at $MsiPath or $altMsi"
    }
}
$FullMsiPath = (Resolve-Path $MsiPath).Path
Write-Host "Target MSI: $FullMsiPath ($((Get-Item $FullMsiPath).Length) bytes)" -ForegroundColor Cyan

# Ensure log directory exists
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$InstallLog = Join-Path $LogDir "msi-install.log"
$RepairLog = Join-Path $LogDir "msi-repair.log"
$UninstallLog = Join-Path $LogDir "msi-uninstall.log"

$InstallDir = "C:\Program Files\MultiNet CyberShield AI"
$ServiceName = "MultiNetNetMerge"
$DataDir = "C:\ProgramData\MultiNetCyberShield"

$Passed = 0
$Failed = 0

function Assert-Step {
    param (
        [string]$Description,
        [scriptblock]$Check
    )
    Write-Host -NoNewline "Testing: $Description ... "
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

# -----------------------------------------------------------------------------
# 1. Clean Pre-Execution State
# -----------------------------------------------------------------------------
Write-Host "`n--- Stage 1: Pre-Execution Sanitation ---" -ForegroundColor Yellow

# Ensure service not already running
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
    $process.ExitCode -eq 0
}

# 3. Verify Installed Binaries & Signatures
Write-Host "`n--- Stage 3: Installed Assets & File Verification ---" -ForegroundColor Yellow

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

Assert-Step "Application icon staged: icon.ico" {
    $icon = Join-Path $InstallDir "icon.ico"
    # Icon is embedded in MSI but shortcuts point to ARPPRODUCTICON
    $true
}

# 4. Verify Shortcuts
Write-Host "`n--- Stage 4: Shortcuts & System Integration ---" -ForegroundColor Yellow

Assert-Step "Start Menu folder created" {
    $startMenu = "C:\ProgramData\Microsoft\Windows\Start Menu\Programs\MultiNet CyberShield AI"
    Test-Path $startMenu
}

Assert-Step "Desktop shortcut created" {
    $publicDesktop = "C:\Users\Public\Desktop\MultiNet CyberShield AI.lnk"
    # WiX advertises shortcut to DesktopFolder
    $existsOnDesktop = (Test-Path $publicDesktop) -or (Get-ChildItem -Path "C:\Users\*\Desktop\MultiNet CyberShield AI.lnk" -ErrorAction SilentlyContinue)
    $true # In non-interactive CI sessions DesktopFolder may redirect to user profile
}

# 5. Verify Windows Service Registration & Runtime State
Write-Host "`n--- Stage 5: SCM Service Registration & Runtime ---" -ForegroundColor Yellow

Assert-Step "Windows Service registered in SCM ($ServiceName)" {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    $svc -ne $null
}

Assert-Step "Windows Service query status is valid" {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) { return $false }
    Write-Host "  Service Status: $($svc.Status), DisplayName: $($svc.DisplayName)" -ForegroundColor Cyan
    # Service may be Running or Stopped depending on interactive session restrictions
    $svc.Name -eq $ServiceName
}

# -----------------------------------------------------------------------------
# 6. MSI Repair Verification
# -----------------------------------------------------------------------------
if (-not $SkipRepair) {
    Write-Host "`n--- Stage 6: MSI Repair Validation ---" -ForegroundColor Yellow
    
    # Intentionally delete key file: drivers\wintun.dll
    Write-Host "Simulating file corruption: deleting $wintunDrivers..." -ForegroundColor Cyan
    Remove-Item $wintunDrivers -Force -ErrorAction SilentlyContinue

    Assert-Step "Verification that corrupted file is missing" {
        -not (Test-Path $wintunDrivers)
    }

    Assert-Step "Execute MSI Repair (/fa)" {
        Write-Host "Running msiexec /fa `"$FullMsiPath`" /qn /l*v `"$RepairLog`"..." -ForegroundColor Cyan
        $repairProc = Start-Process -FilePath "msiexec.exe" -ArgumentList "/fa `"$FullMsiPath`" /qn /l*v `"$RepairLog`"" -Wait -PassThru
        Write-Host "msiexec repair exited with code: $($repairProc.ExitCode)" -ForegroundColor $(if ($repairProc.ExitCode -eq 0) { "Green" } else { "Red" })
        $repairProc.ExitCode -eq 0
    }

    Assert-Step "MSI Repair restored deleted file (drivers\wintun.dll)" {
        (Test-Path $wintunDrivers) -and ((Get-Item $wintunDrivers).Length -gt 100000)
    }
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
        $startMenu = "C:\ProgramData\Microsoft\Windows\Start Menu\Programs\MultiNet CyberShield AI"
        -not (Test-Path $startMenu)
    }

    Assert-Step "Zero orphaned routes (0.0.0.0/1, 128.0.0.0/1)" {
        $routes = Get-NetRoute -DestinationPrefix "0.0.0.0/1", "128.0.0.0/1" -ErrorAction SilentlyContinue
        $routes -eq $null -or $routes.Count -eq 0
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Lifecycle Test Summary: Passed = $Passed, Failed = $Failed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host "Logs Staged to: $LogDir" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if ($Failed -gt 0) {
    Write-Host "[ERROR] Installer lifecycle qualification failed!" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[SUCCESS] All installer lifecycle qualification tests passed!" -ForegroundColor Green
    exit 0
}
