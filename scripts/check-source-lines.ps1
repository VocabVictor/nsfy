[CmdletBinding()]
param(
    [ValidateRange(1, 10000)]
    [int]$MaximumLines = 300
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$extensions = [Collections.Generic.HashSet[string]]::new(
    [string[]]@(
        ".css", ".gradle", ".html", ".iss", ".java", ".js", ".jsx", ".kt", ".kts",
        ".ps1", ".rs", ".sh", ".slint", ".ts", ".tsx", ".svelte"
    ),
    [StringComparer]::OrdinalIgnoreCase
)
$ignoredPathPattern = '(^|[\\/])(node_modules|target|build|dist|gen|\.gradle)([\\/]|$)'
$violations = [Collections.Generic.List[object]]::new()
$rg = Get-Command rg -ErrorAction Stop | Select-Object -First 1

Push-Location $repoRoot
try {
    & $rg.Source --files | ForEach-Object {
        $relativePath = $_
        if ($relativePath -match $ignoredPathPattern) {
            return
        }
        if (-not $extensions.Contains([IO.Path]::GetExtension($relativePath))) {
            return
        }

        $lineCount = (Get-Content -LiteralPath $relativePath).Count
        if ($lineCount -gt $MaximumLines) {
            $violations.Add([PSCustomObject]@{
                Lines = $lineCount
                File = $relativePath
            })
        }
    }
} finally {
    Pop-Location
}

if ($violations.Count -gt 0) {
    $details = $violations |
        Sort-Object Lines -Descending |
        ForEach-Object { "$($_.File): $($_.Lines) lines" }
    throw "Source files exceed the $MaximumLines-line limit:`n$($details -join "`n")"
}

Write-Host "Source line limit passed: every hand-written source file is at most $MaximumLines lines."
