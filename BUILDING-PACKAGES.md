# 构建安装包

在项目根目录运行：

```powershell
.\scripts\build-all-packages.ps1
```

脚本会依次构建：

- Tauri 桌面端：NSIS `.exe` 和 MSI `.msi`
- Tauri 命令行端：独立的 `nsfy-cli-*.exe`，同时嵌入桌面安装包
- Android：签名 release `.apk`
- Slint 桌面端：Inno Setup `.exe`

全部产物统一写入项目根目录的 `.release-packages/`。该目录已被
`.gitignore` 忽略，并包含 `packages-manifest.json` SHA-256 校验清单。

各端也可以单独构建：

```powershell
.\desktop\scripts\build-installer.ps1
.\android\scripts\build-package.ps1
.\slints\scripts\build-installer.ps1
```

可用选项：

- `-SkipBuild`：复用各端现有 release 构建，只收集安装包。
- `-KeepExisting`：总构建时不清理旧的同名产物。
- `-NoBootstrap`：不允许 Slint 脚本自动安装 Inno Setup。
- `-OutputDirectory <路径>`：修改统一产物目录。
- `-SkipTauri`、`-SkipAndroid`、`-SkipSlints`：总构建时跳过指定端。

Android 默认要求本机存在已被 Git 忽略的 `android/keystore.properties`，
否则不会把未签名 APK 当作可安装产物输出。单独调试脚本时可以显式传入
`-AllowUnsigned`。

如果 `-OutputDirectory` 指向当前 Git 仓库内部，脚本会在构建前使用
`git check-ignore` 校验该目录；未被忽略时会直接终止，防止安装包被误提交。
