# MultiNet CyberShield AI - Desktop Application Runtime & Prerequisites Test Harness
# Milestone: NM-07.1.2 (Controlled Beta Runtime Qualification)
# Validates WebView2 runtime, frontend assets, database initialization, live execution, and IPC readiness
param (
    [string]$AppDir = "$PSScriptRoot\staging",
    [string]$LogDir = "$PSScriptRoot\output\desktop-runtime-logs",
    [switch]$AllowSkip,
    [switch]$Strict = ($env:CI -eq 'true')
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Desktop Runtime Qualification" -ForegroundColor Cyan
Write-Host "Milestone: NM-07.1.2 (WebView2, Database & Live Runtime)" -ForegroundColor Cyan
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
        "suite"     = "Desktop Application Runtime Qualification"
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

function Assert-Runtime {
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
# 1. WebView2 Runtime Availability Check
# -----------------------------------------------------------------------------
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

# -----------------------------------------------------------------------------
# 2. Desktop Frontend Bundle Assets Check
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 2: Desktop Frontend Asset Bundle ---" -ForegroundColor Yellow

$distIndex = Join-Path $RepoRoot "apps\desktop\dist\index.html"
Assert-Runtime "Vite frontend distribution assets compiled (index.html exists)" {
    (Test-Path $distIndex) -and ((Get-Item $distIndex).Length -gt 100)
}

$distAssets = Join-Path $RepoRoot "apps\desktop\dist\assets"
Assert-Runtime "Frontend compiled JavaScript and CSS bundles present" {
    (Test-Path $distAssets) -and ((Get-ChildItem -Path $distAssets -Filter "*.js").Count -ge 1)
}

# -----------------------------------------------------------------------------
# 3. Database & AppData Schema Readiness
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 3: SQLite Database Architecture & Schema ---" -ForegroundColor Yellow

$dbInitSource = Join-Path $RepoRoot "apps\desktop\src-tauri\src\db.rs"
Assert-Runtime "Tauri database initialization and migration source verified" {
    (Test-Path $dbInitSource) -and ((Get-Content $dbInitSource -Raw) -match "initialize_database")
}

# -----------------------------------------------------------------------------
# 4. Live Desktop Application Runtime Execution (P1)
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 4: Live Desktop Application Execution & SQLite Inspection ---" -ForegroundColor Yellow

$desktopCandidates = @(
    (Join-Path $RepoRoot "target\release\multinet-cybershield-ai.exe"),
    (Join-Path $RepoRoot "packaging\staging\multinet-cybershield-ai.exe"),
    (Join-Path $RepoRoot "packaging\output\multinet-cybershield-ai.exe"),
    "C:\Program Files\MultiNet CyberShield AI\multinet-cybershield-ai.exe"
)
$desktopExe = $desktopCandidates | Where-Object { (Test-Path $_) -and ((Get-Item $_).Length -gt 1000000) } | Select-Object -First 1

if ($desktopExe) {
    Assert-Runtime "Live desktop execution initializes SQLite DB with migrations" {
        $tempProfile = Join-Path ([System.IO.Path]::GetTempPath()) "multinet-runtime-test-$([System.Guid]::NewGuid().ToString('N'))"
        $tempAppData = Join-Path $tempProfile "AppData\Roaming"
        $tempDataDir = Join-Path $tempAppData "ai.multinet.cybershield"
        $proc = $null
        
        try {
            New-Item -ItemType Directory -Force -Path $tempDataDir | Out-Null
            
            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = $desktopExe
            $psi.EnvironmentVariables["MULTINET_DATA_DIR"] = $tempDataDir
            $psi.EnvironmentVariables["APPDATA"] = $tempAppData
            $psi.UseShellExecute = $false
            
            try {
                $proc = [System.Diagnostics.Process]::Start($psi)
            } catch {
                $errMsg = $_.Exception.Message
                if ($errMsg -match "Application Control|blocked by rule|Smart App Control") {
                    return "Blocked by Application Control: $errMsg"
                }
                throw
            }
            
            if (-not $proc) { return "Failed to start desktop process" }
            
            $candidatePaths = @(
                (Join-Path $tempDataDir "multinet-cybershield-ai.db"),
                (Join-Path $tempAppData "ai.multinet.cybershield\multinet-cybershield-ai.db"),
                (Join-Path $tempAppData "multinet-cybershield-ai.db"),
                (Join-Path $env:APPDATA "ai.multinet.cybershield\multinet-cybershield-ai.db")
            )
            $dbPath = $null
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            while ($sw.ElapsedMilliseconds -lt 25000) {
                foreach ($cand in $candidatePaths) {
                    if ((Test-Path $cand) -and ((Get-Item $cand).Length -gt 0)) {
                        $dbPath = $cand
                        break
                    }
                }
                if ($dbPath) { break }
                if ($proc.HasExited) {
                    foreach ($cand in $candidatePaths) {
                        if ((Test-Path $cand) -and ((Get-Item $cand).Length -gt 0)) {
                            $dbPath = $cand
                            break
                        }
                    }
                    break
                }
                Start-Sleep -Milliseconds 500
            }
            
            # Gracefully stop process
            $gracefulShutdown = $false
            if (-not $proc.HasExited) {
                $closed = $proc.CloseMainWindow()
                if ($closed) {
                    $gracefulShutdown = $proc.WaitForExit(5000)
                }
                if (-not $proc.HasExited) {
                    Write-Host " (Process did not exit after CloseMainWindow, terminating) " -NoNewline
                    $proc.Kill()
                    $proc.WaitForExit(3000)
                }
            } else {
                $gracefulShutdown = $true
            }
            
            if (-not $dbPath) {
                $exitInfo = if ($proc.HasExited) { "Process exited early with code $($proc.ExitCode)" } else { "Process still running" }
                return "Database file not created within timeout. $exitInfo. Checked paths: $($candidatePaths -join ', ')"
            }
            
            # Inspect SQLite database schema & migrations using python
            $pyScript = @"
import sqlite3, sys
db = sys.argv[1]
conn = sqlite3.connect(db)
cur = conn.cursor()
cur.execute("SELECT COUNT(*) FROM _sqlx_migrations")
mig_count = cur.fetchone()[0]
if mig_count < 3:
    print(f"FAILED: migration count {mig_count} < 3")
    sys.exit(1)
cur.execute("SELECT name FROM sqlite_master WHERE type IN ('table', 'view')")
names = set(r[0] for r in cur.fetchall())
required = ['incidents', 'security_events']
for req in required:
    if req not in names:
        print(f"FAILED: missing required table {req}")
        sys.exit(2)
if 'audit_events' not in names and 'audit_logs' not in names:
    print("FAILED: missing audit_events/audit_logs table")
    sys.exit(3)
if 'traffic_snapshots' not in names and 'traffic_samples' not in names:
    print("FAILED: missing traffic_snapshots/traffic_samples table")
    sys.exit(4)
print(f"OK: {mig_count} migrations, verified required tables")
sys.exit(0)
"@
            $pyResult = python -c $pyScript "$dbPath" 2>&1
            if ($LASTEXITCODE -ne 0) {
                return "SQLite DB inspection failed: $pyResult"
            }
            
            $true
        } finally {
            if ($proc -and -not $proc.HasExited) {
                try { $proc.Kill(); $proc.WaitForExit(2000) } catch {}
            }
            if (Test-Path $tempProfile) {
                Remove-Item -Path $tempProfile -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
} else {
    Record-TestResult -Name "Live desktop execution initializes SQLite DB with migrations" -Status "SKIPPED" -Details "multinet-cybershield-ai.exe not found in build outputs"
}

# -----------------------------------------------------------------------------
# 5. IPC Pipe Connection Contract
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 5: Named Pipe IPC Connection Contract ---" -ForegroundColor Yellow

Assert-Runtime "Named Pipe client handles offline service gracefully without crash" {
    $pipeClient = New-Object System.IO.Pipes.NamedPipeClientStream(".", "multinet-netmerge-v1", [System.IO.Pipes.PipeDirection]::InOut)
    $graceful = $false
    try {
        $pipeClient.Connect(100)
        $pipeClient.Close()
        $graceful = $true
    } catch [System.TimeoutException] {
        $graceful = $true
    } catch [System.IO.FileNotFoundException] {
        $graceful = $true
    } catch [System.IO.IOException] {
        $graceful = $true
    } catch {
        return "Unexpected offline pipe exception ($($_.Exception.GetType().FullName)): $($_.Exception.Message)"
    } finally {
        $pipeClient.Dispose()
    }
    $graceful
}

# -----------------------------------------------------------------------------
# 6. Desktop Application Manifest & Config
# -----------------------------------------------------------------------------
Write-Host "`n--- Step 6: Tauri Application Configuration ---" -ForegroundColor Yellow

$tauriConf = Join-Path $RepoRoot "apps\desktop\src-tauri\tauri.conf.json"
Assert-Runtime "Tauri configuration valid (tauri.conf.json)" {
    if (-not (Test-Path $tauriConf)) { return "tauri.conf.json not found" }
    $conf = Get-Content $tauriConf -Raw | ConvertFrom-Json
    $conf.productName -eq "MultiNet CyberShield AI" -and $conf.identifier -eq "ai.multinet.cybershield"
}

# -----------------------------------------------------------------------------
# Summary & Output
# -----------------------------------------------------------------------------
Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Desktop Runtime Summary: Passed = $Passed, Failed = $Failed, Skipped = $Skipped, Blocked = $Blocked" -ForegroundColor $(if ($Failed -eq 0 -and $Blocked -eq 0) { "Green" } elseif ($Blocked -gt 0 -and $Failed -eq 0) { "DarkYellow" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

Save-Results

if ($Failed -gt 0) {
    exit 1
} elseif ($Blocked -gt 0 -or $Skipped -gt 0) {
    if ($Strict) {
        Write-Host "[ERROR] Strict mode active: Skipped ($Skipped) or Blocked ($Blocked) tests cause failure." -ForegroundColor Red
        exit 1
    }
    if ($Blocked -gt 0) {
        if ($AllowSkip) { exit 0 } else { exit 2 }
    } else {
        exit 0
    }
} else {
    exit 0
}
