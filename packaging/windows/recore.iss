; ReCore's Windows installer. scripts/package-windows.py compiles it with
; Inno Setup 7 under Wine, from the same files as the portable ZIP.
;
; It installs for the person running it, without administrator rights, and can
; start ReCore in the tray when they sign in. That is the Run key value
; Settings > Advanced > "Start when you sign in" writes (src/autostart.rs), so
; the installer and the setting turn the same thing on and off.

#ifndef AppVersion
  #error Compile through scripts/package-windows.py, which defines AppVersion, VersionNumber and SourceDir
#endif
; The full preview defines Edition and FileSuffix; the Discord release neither.
; Both share the AppId, so installing one replaces the other.
#ifdef Edition
  #define DisplayName "ReCore (" + Edition + ")"
#else
  #define DisplayName "ReCore"
  #define FileSuffix ""
#endif

[Setup]
; Windows knows an installed ReCore by this; never change it.
AppId={{19FF8D53-8D7E-4E66-91BB-E2F5BE307E9A}
AppName=ReCore
AppVersion={#AppVersion}
AppVerName={#DisplayName} {#AppVersion}
AppPublisher=ReCore
AppPublisherURL=https://github.com/leriart/ReCore
AppSupportURL=https://github.com/leriart/ReCore/issues
AppUpdatesURL=https://github.com/leriart/ReCore/releases
VersionInfoVersion={#VersionNumber}
; Per user: %LOCALAPPDATA%\Programs\ReCore, with no UAC prompt.
PrivilegesRequired=lowest
DefaultDirName={autopf}\ReCore
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=ReCore-{#AppVersion}{#FileSuffix}-windows-x64-setup
SetupIconFile=recore.ico
UninstallDisplayIcon={app}\recore.exe
UninstallDisplayName={#DisplayName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; ReCore keeps running in the tray with its window closed, so an update or
; uninstall closes it rather than asking.
CloseApplications=force
RestartApplications=no
#ifdef LicenseFile
LicenseFile={#LicenseFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartOnSignIn=Start ReCore automatically when Windows starts
english.StartOnSignInGroup=Startup:

[Tasks]
Name: "startonsignin"; Description: "{cm:StartOnSignIn}"; GroupDescription: "{cm:StartOnSignInGroup}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; README.txt explains the ZIP; the installer needs no instructions.
Source: "{#SourceDir}\*"; Excludes: "README.txt"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\ReCore"; Filename: "{app}\recore.exe"
Name: "{autodesktop}\ReCore"; Filename: "{app}\recore.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ReCore"; ValueData: """{app}\recore.exe"" --background"; Tasks: startonsignin
; Chosen here, it starts even if Task Manager had turned it off before.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "ReCore"; Flags: deletevalue dontcreatekey; Tasks: startonsignin
; ReCore's own links open here; claiming Discord's stays the person's choice.
Root: HKCU; Subkey: "Software\Classes\recore"; ValueType: string; ValueName: ""; ValueData: "URL:ReCore Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\recore"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\recore\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\recore.exe"",0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\recore\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\recore.exe"" --open-link ""%1"""; Flags: uninsdeletekey
; Settings keeps the app discord:// links opened before claiming them here;
; uninstalling gives them back (below) and drops what was kept.
Root: HKCU; Subkey: "Software\ReCore"; Flags: uninsdeletekeyifempty dontcreatekey
Root: HKCU; Subkey: "Software\ReCore\DiscordProtocol"; Flags: uninsdeletekey dontcreatekey
; Each start registers ReCore's name and icon for its notifications
; (src/notify.rs); uninstalling forgets them.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\io.github.leriart.ReCore"; Flags: uninsdeletekey dontcreatekey
; Uninstalling stops starting at sign-in, however it was turned on.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "ReCore"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "ReCore"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\recore.exe"; Description: "{cm:LaunchProgram,ReCore}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Before the files go: discord:// links open in the app that had them again,
; if Settings claimed them for this ReCore.
Filename: "{app}\recore.exe"; Parameters: "--release-discord-links"; Flags: runhidden waituntilterminated; RunOnceId: "ReleaseDiscordLinks"
