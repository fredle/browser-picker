; Browser Picker installer.
;
; Per-user, no admin required. Installs the single exe, then calls its own
; --register flag to do the Windows browser registration and Start Menu
; shortcut (so there is exactly one shortcut, not one from Inno and one from
; the app). Uninstall runs --unregister first, then removes the files.
;
; Build with: iscc installer\setup.iss /DMyAppVersion=1.2.3
; MyAppVersion defaults to 0.0.0 for local/dev builds.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

#define MyAppName "Browser Picker"
#define MyAppPublisher "FBL Consulting Ltd"
#define MyAppExeName "browser_picker.exe"

[Setup]
AppId={{F661E52B-EEDB-43CF-9D18-40B1EC47FFC7}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL=https://leatham.com
AppSupportURL=https://leatham.com/support.html
AppUpdatesURL=https://github.com/fredle/browser-picker/releases
DefaultDirName={localappdata}\Programs\Browser Picker
DisableProgramGroupPage=yes
DisableDirPage=yes
DisableReadyPage=yes
DisableWelcomePage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\assets\browser_picker.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
OutputDir=..\target\installer
OutputBaseFilename=BrowserPickerSetup
VersionInfoVersion={#MyAppVersion}

[Files]
Source: "..\target\release\browser_picker.exe"; DestDir: "{app}"; Flags: ignoreversion

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--register"; Flags: runhidden; StatusMsg: "Registering Browser Picker with Windows..."

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--unregister"; Flags: runhidden; RunOnceId: "UnregisterBrowserPicker"
