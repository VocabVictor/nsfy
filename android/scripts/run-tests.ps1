[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$androidRoot = Split-Path -Parent $PSScriptRoot
$gradle = Get-ChildItem -Path "$env:USERPROFILE\.gradle\wrapper\dists" `
    -Filter gradle.bat -File -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match 'gradle-[0-9.]+\\bin\\gradle\.bat$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $gradle) {
    throw "No local Gradle distribution found. Run build-package.ps1 once first."
}

$workingRoot = $androidRoot
$mappedDrive = $null
if ($androidRoot.ToCharArray() | Where-Object { [int]$_ -gt 127 }) {
    $used = (Get-PSDrive -PSProvider FileSystem).Name
    $letter = [char[]](90..68) | Where-Object { $_ -notin $used } | Select-Object -First 1
    if (-not $letter) {
        throw "No free drive letter is available for the Unicode-safe Gradle test path."
    }
    $mappedDrive = "$letter`:"
    & subst.exe $mappedDrive $androidRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create the temporary Gradle test drive."
    }
    $workingRoot = "$mappedDrive\"
}

try {
    Push-Location $workingRoot
    & $gradle.FullName testDebugUnitTest --no-daemon --console=plain
    if ($LASTEXITCODE -ne 0) {
        throw "Android unit tests failed."
    }
    Pop-Location
} finally {
    if ((Get-Location).Path -eq $workingRoot) {
        Pop-Location
    }
    if ($mappedDrive) {
        & subst.exe $mappedDrive /D
    }
}
