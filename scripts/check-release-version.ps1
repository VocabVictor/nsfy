[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$ExpectedVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$version = $ExpectedVersion -replace '^v', ''
if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
    throw "无效的发布版本：$ExpectedVersion"
}

$repoRoot = Split-Path -Parent $PSScriptRoot

function Read-CargoVersion {
    param([Parameter(Mandatory)] [string]$Path)

    $content = Get-Content -LiteralPath $Path -Raw
    if ($content -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw "Cargo 清单缺少 package.version：$Path"
    }
    return $Matches[1]
}

function Read-AndroidVersion {
    param([Parameter(Mandatory)] [string]$Path)

    $content = Get-Content -LiteralPath $Path -Raw
    if ($content -notmatch 'versionName\s*=\s*"([^"]+)"') {
        throw "Android 清单缺少 versionName：$Path"
    }
    return $Matches[1]
}

$packageJson = Get-Content -LiteralPath (Join-Path $repoRoot "desktop/package.json") -Raw | ConvertFrom-Json
$packageLock = Get-Content -LiteralPath (Join-Path $repoRoot "desktop/package-lock.json") -Raw | ConvertFrom-Json -AsHashtable
$tauriConfig = Get-Content -LiteralPath (Join-Path $repoRoot "desktop/src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json

$checks = [ordered]@{
    "server/Cargo.toml" = Read-CargoVersion (Join-Path $repoRoot "server/Cargo.toml")
    "desktop/package.json" = [string]$packageJson.version
    "desktop/package-lock.json" = [string]$packageLock["version"]
    "desktop/package-lock.json packages root" = [string]$packageLock["packages"][""]["version"]
    "desktop/src-tauri/Cargo.toml" = Read-CargoVersion (Join-Path $repoRoot "desktop/src-tauri/Cargo.toml")
    "desktop/src-tauri/tauri.conf.json" = [string]$tauriConfig.version
    "android/app/build.gradle.kts" = Read-AndroidVersion (Join-Path $repoRoot "android/app/build.gradle.kts")
    "slints/Cargo.toml" = Read-CargoVersion (Join-Path $repoRoot "slints/Cargo.toml")
}

$mismatches = @($checks.GetEnumerator() | Where-Object Value -ne $version)
if ($mismatches.Count -gt 0) {
    $details = $mismatches | ForEach-Object { "$($_.Key): $($_.Value)" }
    throw "Tag 版本 $version 与项目版本不一致：`n$($details -join "`n")"
}

Write-Host "Release version check passed: $version"
