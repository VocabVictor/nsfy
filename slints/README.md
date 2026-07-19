# nsfy Slint desktop prototype

This is an isolated native UI prototype. It does not replace or modify the
Tauri desktop client in `../desktop`.

The prototype now includes native Slint system-tray integration, reconnecting
WebSocket subscriptions, and Windows toast notifications for messages with
priority 4 or 5. Closing the window hides it to the tray; use the tray menu to
open or quit the app.

```powershell
cargo run --release
```

Runtime settings are deliberately kept outside the original desktop client:

```powershell
$env:NSFY_SERVER = "http://localhost:8080"
$env:NSFY_TOPICS = "alerts,backups,certificates"
$env:NSFY_AUTH_TOKEN = "optional-server-token"
$env:NSFY_NOTIFICATIONS = "1"
cargo run --release
```

The WebSocket client reconnects with a bounded exponential backoff, suppresses
duplicate message IDs across reconnects, and keeps at most 500 messages in UI
memory. Cached old messages are loaded without replaying stale system toasts.

The UI follows `../nsfy设计稿.pdf`: pale blue-gray navigation, white content
canvas, cyan actions, compact typography, split-topic and unified-timeline
layouts, and restrained notification priority colors.

## Windows installer

Run the PowerShell script from this directory to compile the optimized binary
and create a per-user Windows installer:

```powershell
.\scripts\build-installer.ps1
```

The result is written to
`../.release-packages/nsfy-slint-<version>-windows-x64-setup.exe`.
The script uses Inno Setup 6 and installs it through `winget` automatically when
it is missing. Pass `-NoBootstrap` to disable that behavior, `-SkipBuild` to
reuse an existing release binary, or `-OutputDirectory <path>` to select a
different output directory.

The installer adds Start Menu integration and offers optional desktop shortcut
and sign-in auto-start tasks. It installs for the current user without requiring
administrator privileges.
