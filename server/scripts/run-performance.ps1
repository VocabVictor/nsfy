[CmdletBinding()]
param([switch]$Quick)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$serverRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $serverRoot
$resultRoot = Join-Path $repoRoot ".performance-results"
New-Item -ItemType Directory -Path $resultRoot -Force | Out-Null
$profile = if ($Quick) { "quick" } else { "full" }
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$resultPath = Join-Path $resultRoot "server-$profile-$timestamp.log"
$oldProfile = $env:NSFY_PERF_PROFILE
$env:NSFY_PERF_PROFILE = $profile

Push-Location $serverRoot
try {
    "NSFY server performance profile: $profile" | Tee-Object -FilePath $resultPath
    "Started: $([DateTime]::UtcNow.ToString('o'))" | Tee-Object -FilePath $resultPath -Append
    "OS: $([Environment]::OSVersion)" | Tee-Object -FilePath $resultPath -Append
    "CPU count: $([Environment]::ProcessorCount)" | Tee-Object -FilePath $resultPath -Append
    $output = & cargo test --release `
        --test performance_http --test performance_stream --test performance_tls `
        -- --ignored --nocapture --test-threads=1 2>&1
    $exitCode = $LASTEXITCODE
    $output | Tee-Object -FilePath $resultPath -Append
    if ($exitCode -ne 0) {
        throw "Performance suite failed with exit code $exitCode"
    }
} finally {
    Pop-Location
    $env:NSFY_PERF_PROFILE = $oldProfile
}

Write-Host "Performance report: $resultPath"
