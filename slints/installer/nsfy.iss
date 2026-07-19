#ifndef AppVersion
  #error AppVersion must be provided by build-installer.ps1
#endif
#ifndef VersionInfoVersion
  #error VersionInfoVersion must be provided by build-installer.ps1
#endif
#ifndef SourceExe
  #error SourceExe must be provided by build-installer.ps1
#endif
#ifndef SourceIcon
  #error SourceIcon must be provided by build-installer.ps1
#endif
#ifndef OutputDirectory
  #error OutputDirectory must be provided by build-installer.ps1
#endif

#define AppName "信鸽"
#define AppPublisher "nsfy"
#define AppExeName "nsfy-desktop-slint.exe"

[Setup]
AppId={{93A817E3-A938-40C1-89BD-910796284E63}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
VersionInfoVersion={#VersionInfoVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} Windows installer
VersionInfoProductName={#AppName}
DefaultDirName={localappdata}\Programs\nsfy
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDirectory}
OutputBaseFilename=nsfy-slint-{#AppVersion}-windows-x64-setup
SetupIconFile={#SourceIcon}
UninstallDisplayIcon={app}\nsfy.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=no
RestartApplications=no
SetupLogging=yes
UsePreviousAppDir=yes
UsePreviousTasks=yes

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "快捷方式："; Flags: unchecked
Name: "startup"; Description: "登录 Windows 后自动启动"; GroupDescription: "自动启动："; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion
Source: "{#SourceIcon}"; DestDir: "{app}"; DestName: "nsfy.ico"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\nsfy.ico"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\nsfy.ico"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "nsfy"; ValueData: """{app}\{#AppExeName}"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#AppExeName}"; Description: "启动 {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
function StopRunningApp(): Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec(
    ExpandConstant('{sys}\taskkill.exe'),
    '/F /IM "{#AppExeName}"',
    '',
    SW_HIDE,
    ewWaitUntilTerminated,
    ResultCode
  );
  Result := Result and ((ResultCode = 0) or (ResultCode = 128));
  if not Result then
    Log(Format('Unable to stop {#AppExeName}; taskkill exit code: %d', [ResultCode]));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if not StopRunningApp() then
    Result := '无法关闭正在运行的信鸽，请手动退出后重试。';
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    StopRunningApp();
end;
