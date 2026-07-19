[CmdletBinding()]
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet(
        "server-list", "server-add", "server-remove",
        "topic-list", "topic-add", "topic-remove",
        "publish", "poll", "status", "service-start", "service-stop"
    )]
    [string]$Command,
    [string]$Server,
    [string]$Url,
    [string]$Name,
    [string]$Topic,
    [string]$Title,
    [string]$Message,
    [ValidateRange(1, 5)] [int]$Priority = 3,
    [string[]]$Tag = @(),
    [string]$Category,
    [switch]$Popup,
    [switch]$BypassDnd,
    [string]$Since,
    [string]$Token,
    [string]$Device
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$adb = Get-Command adb.exe -ErrorAction Stop | Select-Object -First 1
$arguments = @()
if ($Device) {
    $arguments += @("-s", $Device)
}
$arguments += @(
    "shell", "am", "broadcast", "-a", "com.nsfy.app.CLI",
    "--es", "command", $Command
)

function Add-Extra {
    param([string]$Key, [AllowEmptyString()] [string]$Value)
    if ($Value) {
        $script:arguments += @("--es", $Key, $Value)
    }
}

Add-Extra "server" $Server
Add-Extra "url" $Url
Add-Extra "name" $Name
Add-Extra "topic" $Topic
Add-Extra "title" $Title
Add-Extra "message" $Message
Add-Extra "priority" ([string]$Priority)
Add-Extra "tags" ($Tag -join ",")
Add-Extra "category" $Category
if ($BypassDnd -and -not $Popup) {
    throw "-BypassDnd 必须与 -Popup 一起使用"
}
if ($Popup) { $arguments += @("--ez", "popup", "true") }
if ($BypassDnd) { $arguments += @("--ez", "bypassDnd", "true") }
Add-Extra "since" $Since
if (-not $Token) {
    $Token = $env:NSFY_AUTH_TOKEN
}
Add-Extra "token" $Token

& $adb.Source @arguments
if ($LASTEXITCODE -ne 0) {
    throw "adb CLI 调用失败，退出码：$LASTEXITCODE"
}
