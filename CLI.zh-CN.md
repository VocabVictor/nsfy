# nsfy 命令行操作

桌面 GUI 和桌面 CLI 共用同一份配置。Android GUI 和通过 ADB 调用的 CLI
也共用服务器、令牌和订阅数据，因此不需要先打开 GUI 才能使用。

## 桌面端

构建：

```powershell
cd desktop\src-tauri
cargo build --release --bin nsfy-cli
```

安装包同时包含 `nsfy-cli.exe`，打包脚本还会在 `.release-packages/`
生成一个可独立使用的 CLI 文件。

```powershell
# 服务器
nsfy-cli server list
$env:NSFY_AUTH_TOKEN = "服务器令牌"
nsfy-cli server add --url https://push.example.com --name Tencent
nsfy-cli server remove Tencent

# 订阅
nsfy-cli topic list
nsfy-cli topic add --server Tencent --topic agents
nsfy-cli topic remove --server Tencent --topic agents

# 发布、拉取和状态
nsfy-cli publish --server Tencent --topic agents `
  --title "Codex" --message "任务完成" --priority 4 `
  --category "开发/Agent/Codex" --tag hook --tag completed
nsfy-cli poll --server Tencent --topic agents
nsfy-cli status --server Tencent
nsfy-cli config-path
```

`--server` 可以使用已保存的名称、URL，也可以直接传入一个临时 URL。未指定时
使用配置中的第一个服务器。`NSFY_AUTH_TOKEN` 会覆盖保存的令牌，适合脚本和 CI。

## Android

Android 不是传统命令行环境。APK 提供了一个仅允许 ADB shell 调用的命令接收器，
并附带 PowerShell 包装脚本：

```powershell
$cli = ".\android\scripts\nsfy-cli.ps1"

& $cli server-list
$env:NSFY_AUTH_TOKEN = "服务器令牌"
& $cli server-add -Url https://push.example.com -Name Tencent
& $cli topic-add -Server Tencent -Topic agents
& $cli publish -Server Tencent -Topic agents -Title Codex `
  -Message "任务完成" -Priority 4 -Category "开发/Agent/Codex" `
  -Tag hook,completed
& $cli poll -Server Tencent -Topic agents
& $cli status -Server Tencent
& $cli service-start
& $cli service-stop
```

不使用包装脚本时，可以直接调用：

```powershell
adb shell am broadcast -a com.nsfy.app.CLI `
  --es command publish `
  --es server Tencent `
  --es topic agents `
  --es message "任务完成" `
  --es category "开发/Agent/Codex"
```

广播结果中的 `data="..."` 是 JSON。接收器要求调用者拥有 Android 的
`DUMP` 权限，普通第三方应用不能借此修改配置或发送消息。

## 多级分类

HTTP 消息中的 `category` 是一个有顺序的字符串数组：

```json
{
  "title": "任务完成",
  "message": "安装包已生成",
  "priority": 4,
  "tags": ["hook", "completed"],
  "category": ["开发", "Agent", "Codex"]
}
```

GUI 和 CLI 输入使用 `/` 分隔层级。桌面端和 Android 时间线可以选择任意父级，
查看该分类及其所有子分类。旧消息没有 `category` 时仍按“未分类”处理。
