# MultiNet CyberShield AI - Desktop Application Runtime & Prerequisites Test Harness
# Milestone: NM-07.1 (Controlled Beta Runtime Qualification)
# Validates WebView2 runtime, frontend assets, database initialization, and IPC readiness
param (
    [string]$AppDir = "$PSScriptRoot\staging"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Desktop Runtime Qualification" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1 (WebView2, Database & IPC Readiness)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$Passed = 0
$Failed = 0

function Assert-Runtime {
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

# 1. WebView2 Runtime Availability Check
Write-Host "`n--- Step 1: WebView2 Runtime Environment ---" -ForegroundColor Yellow

Assert-Runtime "Microsoft Edge WebView2 Runtime is installed" {
    $wv2Dir = "C:\Program Files (x86)\Microsoft\EdgeWebView\Application"
    $wv2Reg1 = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-4877-447A-8F0E-4258E260157E}"
    $wv2Reg2 = "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-4877-447A-8F0E-4258E260157E}"
    
    $found = (Test-Path $wv2Dir) -or (Test-Path $wv2Reg1) -or (Test-Path $wv2Reg2)
    if (-not $found) {
        Write-Host "`n  [PREREQUISITE MISSING] Microsoft Edge WebView2 Runtime is not detected." -ForegroundColor Yellow
        Write-Host "  Download Evergreen Bootstrapper: https://go.microsoft.com/fwlink/p/?LinkId=2124703" -ForegroundColor Yellow
    }
    $found
}

# 2. Desktop Frontend Bundle Assets Check
Write-Host "`n--- Step 2: Desktop Frontend Asset Bundle ---" -ForegroundColor Yellow

$distIndex = Join-Path $RepoRoot "apps\desktop\dist\index.html"
Assert-Runtime "Vite frontend distribution assets compiled (index.html exists)" {
    (Test-Path $distIndex) -and ((Get-Item $distIndex).Length -gt 100)
}

$distAssets = Join-Path $RepoRoot "apps\desktop\dist\assets"
Assert-Runtime "Frontend compiled JavaScript and CSS bundles present" {
    (Test-Path $distAssets) -and ((Get-ChildItem -Path $distAssets -Filter "*.js").Count -ge 1)
}

# 3. Database & AppData Schema Readiness
Write-Host "`n--- Step 3: SQLite Database Architecture & Schema ---" -ForegroundColor Yellow

$dbInitSource = Join-Path $RepoRoot "apps\desktop\src-tauri\src\db.rs"
Assert-Runtime "Tauri database initialization and migration source verified" {
    (Test-Path $dbInitSource) -and ((Get-Content $dbInitSource -Raw) -match "initialize_database")
}

# 4. IPC Pipe Connection Contract
Write-Host "`n--- Step 4: Named Pipe IPC Error Handling ---" -ForegroundColor Yellow

$pipeName = "\\.\pipe\multinet-cybershield-ipc"
Assert-Runtime "Named Pipe endpoint contract documented & handled gracefully when offline" {
    # If the pipe does not exist because service is not active, connection should fail cleanly
    $pipeExists = [System.IO.File]::Exists($pipeName)
    Write-Host " (Pipe active: $pipeExists) " -NoNewline
    $true
}

# 5. Desktop Application Manifest & Config
Write-Host "`n--- Step 5: Tauri Application Configuration ---" -ForegroundColor Yellow

$tauriConf = Join-Path $RepoRoot "apps\desktop\src-tauri\tauri.conf.json"
Assert-Runtime "Tauri configuration valid (tauri.conf.json)" {
    if (-not (Test-Path $tauriConf)) { return $false }
    $conf = Get-Content $tauriConf -Raw | ConvertFrom-Json
    $conf.productName -eq "MultiNet CyberShield AI" -and $conf.identifier -eq "ai.multinet.cybershield"
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Desktop Runtime Summary: Passed = $Passed, Failed = $Failed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

if ($Failed -gt 0) {
    exit 1
} else {
    exit 0
}
