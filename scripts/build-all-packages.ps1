[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$NoBootstrap,
    [switch]$SkipTauri,
    [switch]$SkipAndroid,
    [switch]$SkipSlints,
    [switch]$KeepExisting,
    [string]$OutputDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-IgnoredOutputDirectory {
    param([string]$RepoRoot, [string]$OutputPath)

    $probe = Join-Path $OutputPath ".git-ignore-probe"
    $relativeProbe = [IO.Path]::GetRelativePath($RepoRoot, $probe)
    if (-not $relativeProbe.StartsWith("..")) {
        $git = Get-Command git.exe -ErrorAction Stop | Select-Object -First 1
        & $git.Source -C $RepoRoot check-ignore -q -- $relativeProbe
        if ($LASTEXITCODE -ne 0) {
            throw "仓库内的安装包目录必须被 Git 忽略：$OutputPath"
        }
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot "check-source-lines.ps1")
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot ".release-packages"
}
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
Assert-IgnoredOutputDirectory -RepoRoot $repoRoot -OutputPath $outputPath
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

if (-not $KeepExisting) {
    $patterns = @("nsfy-tauri-*", "nsfy-cli-*", "nsfy-android-*", "nsfy-slint-*", "packages-manifest.json")
    foreach ($pattern in $patterns) {
        Get-ChildItem -LiteralPath $outputPath -Filter $pattern -File -ErrorAction SilentlyContinue |
            Remove-Item -Force
    }
}

$sharedArguments = @{ OutputDirectory = $outputPath }
if ($SkipBuild) {
    $sharedArguments.SkipBuild = $true
}

if (-not $SkipTauri) {
    Write-Host "=== 构建 Tauri 桌面安装包 ===" -ForegroundColor Magenta
    & (Join-Path $repoRoot "desktop\scripts\build-installer.ps1") @sharedArguments
}
if (-not $SkipAndroid) {
    Write-Host "=== 构建 Android 安装包 ===" -ForegroundColor Magenta
    & (Join-Path $repoRoot "android\scripts\build-package.ps1") @sharedArguments
}
if (-not $SkipSlints) {
    Write-Host "=== 构建 Slint 桌面安装包 ===" -ForegroundColor Magenta
    $slintArguments = $sharedArguments.Clone()
    if ($NoBootstrap) {
        $slintArguments.NoBootstrap = $true
    }
    & (Join-Path $repoRoot "slints\scripts\build-installer.ps1") @slintArguments
}

$packages = Get-ChildItem -LiteralPath $outputPath -File |
    Where-Object { $_.Extension -in ".exe", ".msi", ".apk" } |
    Sort-Object Name
if (-not $packages) {
    throw "没有生成任何安装包。"
}

$manifest = [ordered]@{
    generatedAtUtc = [DateTime]::UtcNow.ToString("o")
    packages = @($packages | ForEach-Object {
        [ordered]@{
            file = $_.Name
            size = $_.Length
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
    })
}
$manifestPath = Join-Path $outputPath "packages-manifest.json"
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifestPath -Encoding utf8

Write-Host "=== 全部安装包已生成 ===" -ForegroundColor Green
$packages | ForEach-Object {
    $sizeMiB = [math]::Round($_.Length / 1MB, 2)
    Write-Host "$($_.Name) ($sizeMiB MiB)"
}
Write-Host "产物目录：$outputPath" -ForegroundColor Green
Write-Host "校验清单：$manifestPath" -ForegroundColor Green
