[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$OutputDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory)] [string]$Command,
        [Parameter(ValueFromRemainingArguments)] [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "命令执行失败（退出码 $LASTEXITCODE）：$Command $($Arguments -join ' ')"
    }
}

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

if ($env:OS -ne "Windows_NT") {
    throw "Tauri 安装包脚本当前仅支持 Windows。"
}

$desktopRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $desktopRoot
& (Join-Path $repoRoot "scripts\check-source-lines.ps1")
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot ".release-packages"
}
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
Assert-IgnoredOutputDirectory -RepoRoot $repoRoot -OutputPath $outputPath
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

$packageJsonPath = Join-Path $desktopRoot "package.json"
$tauriConfigPath = Join-Path $desktopRoot "src-tauri\tauri.conf.json"
$packageJson = Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json
$tauriConfig = Get-Content -LiteralPath $tauriConfigPath -Raw | ConvertFrom-Json
$version = [string]$tauriConfig.version
if ([string]$packageJson.version -ne $version) {
    throw "desktop/package.json 与 tauri.conf.json 的版本号不一致。"
}

$npm = Get-Command npm.cmd -ErrorAction Stop | Select-Object -First 1
Get-Command cargo.exe -ErrorAction Stop | Out-Null
$packageTarget = Join-Path $desktopRoot "src-tauri\target\package-build"
$oldCargoTarget = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = $packageTarget

Push-Location $desktopRoot
try {
    if (-not $SkipBuild) {
        Write-Host "[Tauri 1/2] 安装前端依赖..." -ForegroundColor Cyan
        Invoke-NativeCommand $npm.Source ci --no-audit --no-fund
        Write-Host "[Tauri 2/2] 编译 GUI、CLI 并生成 NSIS/MSI 安装包..." -ForegroundColor Cyan
        Invoke-NativeCommand -Command $npm.Source -Arguments @(
            "run", "tauri", "--", "build", "--bundles", "nsis,msi"
        )
    }

    $bundleRoot = Join-Path $packageTarget "release\bundle"
    $nsis = Get-ChildItem -LiteralPath (Join-Path $bundleRoot "nsis") -Filter "nsfy_${version}_*-setup.exe" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $msi = Get-ChildItem -LiteralPath (Join-Path $bundleRoot "msi") -Filter "nsfy_${version}_*.msi" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $nsis -or -not $msi) {
        throw "未找到 Tauri $version 的 NSIS 和 MSI 安装包。"
    }

    $nsisOutput = Join-Path $outputPath "nsfy-tauri-$version-windows-x64-setup.exe"
    $msiOutput = Join-Path $outputPath "nsfy-tauri-$version-windows-x64.msi"
    Copy-Item -LiteralPath $nsis.FullName -Destination $nsisOutput -Force
    Copy-Item -LiteralPath $msi.FullName -Destination $msiOutput -Force
    $cliSource = Join-Path $packageTarget "release\nsfy-cli.exe"
    if (-not (Test-Path -LiteralPath $cliSource)) {
        throw "未找到 Tauri CLI：$cliSource"
    }
    $cliOutput = Join-Path $outputPath "nsfy-cli-$version-windows-x64.exe"
    Copy-Item -LiteralPath $cliSource -Destination $cliOutput -Force
    Write-Host "Tauri 安装包：$nsisOutput" -ForegroundColor Green
    Write-Host "Tauri 安装包：$msiOutput" -ForegroundColor Green
    Write-Host "Tauri CLI：$cliOutput" -ForegroundColor Green
} finally {
    Pop-Location
    $env:CARGO_TARGET_DIR = $oldCargoTarget
}
