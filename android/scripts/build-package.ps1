[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$AllowUnsigned,
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

function Get-LocalGradle {
    param([Parameter(Mandatory)] [string]$AndroidRoot)

    $wrapperProperties = Join-Path $AndroidRoot "gradle\wrapper\gradle-wrapper.properties"
    $urlLine = Get-Content -LiteralPath $wrapperProperties |
        Where-Object { $_ -like "distributionUrl=*" } | Select-Object -First 1
    if (-not $urlLine) {
        throw "gradle-wrapper.properties 中缺少 distributionUrl。"
    }
    $distributionUrl = $urlLine.Substring("distributionUrl=".Length).Replace("\:", ":")
    $archiveName = Split-Path ([Uri]$distributionUrl).AbsolutePath -Leaf
    if ($archiveName -notmatch '^gradle-([0-9.]+)-bin\.zip$') {
        throw "无法识别 Gradle 发行版：$archiveName"
    }

    $version = $Matches[1]
    $distributionRoot = Join-Path $AndroidRoot ".gradle\distribution"
    $gradle = Join-Path $distributionRoot "gradle-$version\bin\gradle.bat"
    if (Test-Path -LiteralPath $gradle -PathType Leaf) {
        return $gradle
    }

    New-Item -ItemType Directory -Path $distributionRoot -Force | Out-Null
    $archive = Join-Path $distributionRoot $archiveName
    if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        $partial = "$archive.partial"
        Write-Host "正在下载 Gradle $version..." -ForegroundColor Yellow
        Invoke-WebRequest -Uri $distributionUrl -OutFile $partial
        Move-Item -LiteralPath $partial -Destination $archive -Force
    }
    Write-Host "正在解压 Gradle $version..." -ForegroundColor Yellow
    Expand-Archive -LiteralPath $archive -DestinationPath $distributionRoot -Force
    if (-not (Test-Path -LiteralPath $gradle -PathType Leaf)) {
        throw "Gradle 解压后未找到 gradle.bat。"
    }
    return $gradle
}

if ($env:OS -ne "Windows_NT") {
    throw "Android 安装包脚本当前仅支持 Windows。"
}

$androidRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $androidRoot
& (Join-Path $repoRoot "scripts\check-source-lines.ps1")
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot ".release-packages"
}
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
Assert-IgnoredOutputDirectory -RepoRoot $repoRoot -OutputPath $outputPath
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

$buildFile = Join-Path $androidRoot "app\build.gradle.kts"
$buildText = Get-Content -LiteralPath $buildFile -Raw
if ($buildText -notmatch 'versionName\s*=\s*"([^"]+)"') {
    throw "无法读取 Android versionName。"
}
$version = $Matches[1]

$keystoreProperties = Join-Path $androidRoot "keystore.properties"
$hasReleaseKey = Test-Path -LiteralPath $keystoreProperties -PathType Leaf
if (-not $hasReleaseKey -and -not $AllowUnsigned) {
    throw "缺少 android/keystore.properties，无法生成可安装的签名 APK。"
}

$localProperties = Join-Path $androidRoot "local.properties"
$sdkLine = Get-Content -LiteralPath $localProperties -ErrorAction Stop |
    Where-Object { $_ -like "sdk.dir=*" } | Select-Object -First 1
if (-not $sdkLine) {
    throw "android/local.properties 中缺少 sdk.dir。"
}
$sdkPath = $sdkLine.Substring("sdk.dir=".Length).Replace("\:", ":").Replace("\\", "\")
if (-not (Test-Path -LiteralPath $sdkPath -PathType Container)) {
    throw "Android SDK 不存在：$sdkPath"
}

$java = Get-Command java.exe -ErrorAction Stop | Select-Object -First 1
$javaVersionText = (& $java.Source -version 2>&1) | Out-String
if ($javaVersionText -notmatch 'version "(\d+)' -or [int]$Matches[1] -lt 17) {
    throw "Android 构建需要 JDK 17 或更高版本。"
}
$oldAndroidHome = $env:ANDROID_HOME
$oldAndroidSdkRoot = $env:ANDROID_SDK_ROOT
$oldGradleUserHome = $env:GRADLE_USER_HOME
$oldJavaHome = $env:JAVA_HOME
$env:ANDROID_HOME = $sdkPath
$env:ANDROID_SDK_ROOT = $sdkPath
$env:GRADLE_USER_HOME = Join-Path $androidRoot ".gradle\user-home"
$env:JAVA_HOME = Split-Path -Parent (Split-Path -Parent $java.Source)

Push-Location $androidRoot
try {
    if (-not $SkipBuild) {
        Write-Host "[Android] 编译 release APK..." -ForegroundColor Cyan
        $gradle = Get-LocalGradle -AndroidRoot $androidRoot
        Invoke-NativeCommand $gradle --no-daemon --console=plain assembleRelease
    }

    $apkName = if ($hasReleaseKey) { "app-release.apk" } else { "app-release-unsigned.apk" }
    $apk = Join-Path $androidRoot "app\build\outputs\apk\release\$apkName"
    if (-not (Test-Path -LiteralPath $apk -PathType Leaf)) {
        throw "未找到 Android APK：$apk"
    }

    if ($hasReleaseKey) {
        $apksigner = Get-ChildItem -LiteralPath (Join-Path $sdkPath "build-tools") -Directory |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "apksigner.bat" } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
        if (-not $apksigner) {
            throw "Android SDK 中未找到 apksigner.bat。"
        }
        $verificationOutput = & $apksigner verify $apk 2>&1
        if ($LASTEXITCODE -ne 0) {
            $verificationOutput | Write-Host
            throw "APK 签名校验失败。"
        }
    }

    $suffix = if ($hasReleaseKey) { "release" } else { "unsigned" }
    $apkOutput = Join-Path $outputPath "nsfy-android-$version-$suffix.apk"
    Copy-Item -LiteralPath $apk -Destination $apkOutput -Force
    Write-Host "Android 安装包：$apkOutput" -ForegroundColor Green
} finally {
    Pop-Location
    $env:ANDROID_HOME = $oldAndroidHome
    $env:ANDROID_SDK_ROOT = $oldAndroidSdkRoot
    $env:GRADLE_USER_HOME = $oldGradleUserHome
    $env:JAVA_HOME = $oldJavaHome
}
