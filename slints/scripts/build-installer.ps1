[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$NoBootstrap,
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

function Find-InnoCompiler {
    $fromPath = Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($fromPath) {
        return $fromPath.Source
    }

    $roots = @(
        (Join-Path $env:LOCALAPPDATA "Programs"),
        ${env:ProgramFiles(x86)},
        $env:ProgramFiles
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }

    foreach ($root in $roots) {
        $compiler = Get-ChildItem -LiteralPath $root -Directory -Filter "Inno Setup*" -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "ISCC.exe" } |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
        if ($compiler) {
            return $compiler
        }
    }
    return $null
}

if ($env:OS -ne "Windows_NT") {
    throw "安装包脚本仅支持 Windows。"
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $projectRoot
& (Join-Path $repoRoot "scripts\check-source-lines.ps1")
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot ".release-packages"
}
$manifestPath = Join-Path $projectRoot "Cargo.toml"
$installerScript = Join-Path $projectRoot "installer\nsfy.iss"
$iconPath = Join-Path $projectRoot "..\desktop\src-tauri\icons\icon.ico"

foreach ($requiredFile in @($manifestPath, $installerScript, $iconPath)) {
    if (-not (Test-Path -LiteralPath $requiredFile -PathType Leaf)) {
        throw "缺少安装包构建文件：$requiredFile"
    }
}

$cargo = Get-Command cargo.exe -ErrorAction Stop | Select-Object -First 1
$metadataJson = & $cargo.Source metadata --manifest-path $manifestPath --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) {
    throw "无法读取 Cargo 项目元数据。"
}
$metadata = $metadataJson | ConvertFrom-Json
$package = $metadata.packages | Where-Object name -eq "nsfy-desktop-slint" | Select-Object -First 1
if (-not $package) {
    throw "Cargo.toml 中未找到 nsfy-desktop-slint 包。"
}

$version = [string]$package.version
if ($version -notmatch '^(\d+)\.(\d+)\.(\d+)') {
    throw "Cargo 包版本无法转换为 Windows 版本号：$version"
}
$versionInfo = "$($Matches[1]).$($Matches[2]).$($Matches[3]).0"
$targetDirectory = [string]$metadata.target_directory
$sourceExe = Join-Path $targetDirectory "release\nsfy-desktop-slint.exe"

if ([IO.Path]::IsPathRooted($OutputDirectory)) {
    $outputPath = [IO.Path]::GetFullPath($OutputDirectory)
} else {
    $outputPath = [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
}
Assert-IgnoredOutputDirectory -RepoRoot $repoRoot -OutputPath $outputPath
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

Push-Location $projectRoot
try {
    if (-not $SkipBuild) {
        Write-Host "[1/3] 编译 release 可执行文件..." -ForegroundColor Cyan
        Invoke-NativeCommand $cargo.Source build --release --locked --manifest-path $manifestPath
    }
    if (-not (Test-Path -LiteralPath $sourceExe -PathType Leaf)) {
        throw "未找到 release 可执行文件：$sourceExe"
    }

    Write-Host "[2/3] 查找 Inno Setup 编译器..." -ForegroundColor Cyan
    $iscc = Find-InnoCompiler
    if (-not $iscc) {
        if ($NoBootstrap) {
            throw "未安装 Inno Setup 6。去掉 -NoBootstrap 后脚本可通过 winget 自动安装。"
        }
        $winget = Get-Command winget.exe -ErrorAction Stop | Select-Object -First 1
        Write-Host "未检测到 Inno Setup，正在通过 winget 安装..." -ForegroundColor Yellow
        Invoke-NativeCommand $winget.Source install --id JRSoftware.InnoSetup --exact --source winget --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
        $iscc = Find-InnoCompiler
        if (-not $iscc) {
            throw "Inno Setup 安装完成，但仍未找到 ISCC.exe。"
        }
    }

    Write-Host "[3/3] 生成 Windows 安装包..." -ForegroundColor Cyan
    Invoke-NativeCommand $iscc "/DAppVersion=$version" "/DVersionInfoVersion=$versionInfo" "/DSourceExe=$sourceExe" "/DSourceIcon=$iconPath" "/DOutputDirectory=$outputPath" $installerScript

    $installerPath = Join-Path $outputPath "nsfy-slint-$version-windows-x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
        throw "Inno Setup 已结束，但未找到预期安装包：$installerPath"
    }
    $sizeMiB = [math]::Round((Get-Item -LiteralPath $installerPath).Length / 1MB, 2)
    Write-Host "安装包已生成：$installerPath ($sizeMiB MiB)" -ForegroundColor Green
} finally {
    Pop-Location
}
