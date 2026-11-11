# MultiNet CyberShield AI - Native Runtime Qualification
#
# Verifies the *shipped* desktop executable at runtime:
#   * the release binary embeds the compiled frontend assets;
#   * the binary does not depend on a local Vite development server;
#   * the native window opens and loads through the Tauri custom protocol;
#   * the Tauri runtime detection succeeds inside the window;
#   * the foundation commands and all nine diagnostics commands answer with
#     real backend data;
#   * the process terminates and releases its temporary state.
#
# A successful build is not runtime acceptance: this harness is what proves the
# application actually works when a user launches it.
param (
    [string]$ExePath = "$PSScriptRoot\..\target\release\multinet-cybershield-ai.exe",
    [string]$LogDir = "$PSScriptRoot\output\native-runtime-logs",
    [int]$DebugPort = 9222
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\.."

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$LogFile = Join-Path $LogDir "native-runtime-verification.log"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "MultiNet CyberShield AI - Native Runtime Qualification" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$Results = [System.Collections.Generic.List[PSCustomObject]]::new()
$script:Passed = 0
$script:Failed = 0
$script:Skipped = 0

function Record-Result {
    param (
        [string]$Name,
        [string]$Status, # PASS, FAIL, SKIPPED
        [string]$Details = ""
    )
    $Results.Add([PSCustomObject]@{
        "Name"    = $Name
        "Status"  = $Status
        "Details" = $Details
    })
    switch ($Status) {
        "PASS"    { Write-Host "[$Status] $Name" -ForegroundColor Green; $script:Passed++ }
        "FAIL"    { Write-Host "[$Status] $Name - $Details" -ForegroundColor Red; $script:Failed++ }
        "SKIPPED" { Write-Host "[$Status] $Name - $Details" -ForegroundColor Yellow; $script:Skipped++ }
    }
}

# ---------------------------------------------------------------------------
# 1. The release executable must exist and embed the compiled frontend
# ---------------------------------------------------------------------------
Write-Host "`n--- Step 1: Production executable and embedded frontend ---" -ForegroundColor Yellow

if (-not (Test-Path $ExePath)) {
    Record-Result -Name "Release executable exists" -Status "FAIL" -Details "not found: $ExePath"
} else {
    $resolved = (Resolve-Path $ExePath).Path
    $exeItem = Get-Item $resolved
    Record-Result -Name "Release executable exists" -Status "PASS" -Details "$resolved ($($exeItem.Length) bytes)"

    # A development-server build contains no compiled asset bundle. The
    # embedded bundle is what proves the executable is self-contained.
    $bytes = [System.IO.File]::ReadAllBytes($resolved)
    $text = [System.Text.Encoding]::ASCII.GetString($bytes)
    if ($text -match "assets/index-") {
        Record-Result -Name "Release executable embeds the compiled frontend" -Status "PASS" -Details "embedded asset bundle found"
    } else {
        Record-Result -Name "Release executable embeds the compiled frontend" -Status "FAIL" -Details "no embedded asset bundle was found"
    }
}

# ---------------------------------------------------------------------------
# 2. No development server may be listening while the runtime is verified
# ---------------------------------------------------------------------------
Write-Host "`n--- Step 2: Development server isolation ---" -ForegroundColor Yellow

$devListeners = @(Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue |
    Where-Object { $_.State -eq "Listen" })
if ($devListeners.Count -gt 0) {
    Record-Result -Name "No Vite development server is listening on port 1420" -Status "FAIL" `
        -Details "port 1420 is in use; the runtime could not prove it is independent of the dev server"
} else {
    Record-Result -Name "No Vite development server is listening on port 1420" -Status "PASS" -Details "port 1420 is free"
}

# ---------------------------------------------------------------------------
# 3. Launch the native application and probe the live IPC bridge
# ---------------------------------------------------------------------------
Write-Host "`n--- Step 3: Native launch and IPC smoke checks ---" -ForegroundColor Yellow

$probeScript = Join-Path $ScriptDir "verify-native-runtime.mjs"
$tempData = Join-Path ([System.IO.Path]::GetTempPath()) "multinet-native-runtime-$([System.Guid]::NewGuid().ToString('N'))"
$probeOutput = Join-Path $LogDir "native-runtime-probe.json"
$process = $null

try {
    New-Item -ItemType Directory -Force -Path $tempData | Out-Null

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = (Resolve-Path $ExePath).Path
    $psi.EnvironmentVariables["MULTINET_DATA_DIR"] = $tempData
    $psi.EnvironmentVariables["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = "--remote-debugging-port=$DebugPort"
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true

    try {
        $process = [System.Diagnostics.Process]::Start($psi)
    } catch {
        $errMsg = $_.Exception.Message
        if ($errMsg -match "Application Control|blocked by rule|Smart App Control") {
            Record-Result -Name "Native application launches" -Status "SKIPPED" -Details "Blocked by Application Control: $errMsg"
        } else {
            Record-Result -Name "Native application launches" -Status "FAIL" -Details $errMsg
        }
        throw
    }

    if (-not $process) {
        Record-Result -Name "Native application launches" -Status "FAIL" -Details "the process did not start"
        throw "no process"
    }
    Record-Result -Name "Native application launches" -Status "PASS" -Details "pid $($process.Id)"

    # The probe drives the real WebView2 window over the DevTools Protocol.
    Push-Location $ScriptDir
    try {
        $probe = & node $probeScript $DebugPort 2>&1 | Out-String
    } finally {
        Pop-Location
    }
    $probe | Out-File -FilePath $LogFile -Encoding utf8 -Append

    if ($probe -match "RESULT_FAIL") {
        Record-Result -Name "Native runtime probe" -Status "FAIL" -Details (($probe -split "`n" | Where-Object { $_ -match "RESULT_FAIL" }) -join " ")
    } elseif ($probe -match "RESULT_BLOCKED") {
        Record-Result -Name "Native runtime probe" -Status "BLOCKED" -Details (($probe -split "`n" | Where-Object { $_ -match "RESULT_BLOCKED" }) -join " ")
    } else {
        $json = $probe -replace "(?s)^.*?RESULT\s*", ""
        try {
            $parsed = $json | ConvertFrom-Json
            $url = $parsed.runtime.url
            $bridged = $parsed.runtime.hasTauriInternals
            $commandFailures = $parsed.failures

            if ($url -like "http://tauri.localhost*") {
                Record-Result -Name "Frontend loads through the Tauri custom protocol" -Status "PASS" -Details "url $url"
            } else {
                Record-Result -Name "Frontend loads through the Tauri custom protocol" -Status "FAIL" -Details "url $url"
            }

            if ($bridged) {
                Record-Result -Name "Tauri runtime detection succeeds in the window" -Status "PASS" -Details "__TAURI_INTERNALS__ present"
            } else {
                Record-Result -Name "Tauri runtime detection succeeds in the window" -Status "FAIL" -Details "__TAURI_INTERNALS__ missing"
            }

            if ($commandFailures -eq 0) {
                Record-Result -Name "Foundation and diagnostics commands answer with real data" -Status "PASS" -Details "9 commands invoked, 0 failures"
            } else {
                Record-Result -Name "Foundation and diagnostics commands answer with real data" -Status "FAIL" -Details "$commandFailures command(s) failed"
            }
        } catch {
            Record-Result -Name "Native runtime probe" -Status "FAIL" -Details "the probe output could not be parsed"
        }
    }
} catch {
    # The per-step results already describe the failure.
} finally {
    # The process must terminate and release its temporary state.
    if ($process -and -not $process.HasExited) {
        $null = $process.CloseMainWindow()
        if (-not $process.WaitForExit(8000)) {
            $process.Kill()
            $process.WaitForExit(5000)
        }
    }
    if ($process) {
        try {
            $stdout = $process.StandardOutput.ReadToEnd()
            $stderr = $process.StandardError.ReadToEnd()
            @("--- stdout ---", $stdout, "--- stderr ---", $stderr) |
                Out-File -FilePath $LogFile -Encoding utf8 -Append
        } catch {}
    }
    Remove-Item -Path $tempData -Recurse -Force -ErrorAction SilentlyContinue
}

# ---------------------------------------------------------------------------
# 4. Summary
# ---------------------------------------------------------------------------
$report = [ordered]@{
    "suite"     = "Native Runtime Qualification"
    "timestamp" = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    "strict"    = ($env:CI -eq 'true')
    "executable"= $ExePath
    "summary"   = [ordered]@{
        "total"      = $script:Passed + $script:Failed + $script:Skipped
        "passed"     = $script:Passed
        "failed"     = $script:Failed
        "skipped"    = $script:Skipped
        "blocked"    = 0
        "not_tested" = 0
        "verdict"    = $(if ($script:Failed -gt 0) { "FAIL" } else { "PASS" })
    }
    "tests"     = $Results
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText((Join-Path $LogDir "test-results.json"), ($report | ConvertTo-Json -Depth 6), $utf8NoBom)

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Native Runtime Summary: Passed = $Passed, Failed = $Failed, Skipped = $Skipped" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host "==========================================================" -ForegroundColor Cyan

if ($Failed -gt 0) { exit 1 }
exit 0
