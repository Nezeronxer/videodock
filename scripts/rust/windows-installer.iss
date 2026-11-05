#ifndef AppSourceDir
  #error Use package-windows.ps1 to provide the staged application directory.
#endif
#ifndef PackageOutputDir
  #error Use package-windows.ps1 to provide the output directory.
#endif
#ifndef AppVersion
  #error The application version is required.
#endif

[Setup]
AppId={{BA8C3E99-89B9-4C35-B47E-31B8B20147B3}
AppName=VideoDock
AppVersion={#AppVersion}
AppPublisher=Nezeronxer
AppPublisherURL=https://github.com/Nezeronxer/videodock
DefaultDirName={localappdata}\Programs\VideoDock
DefaultGroupName=VideoDock
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
DisableProgramGroupPage=yes
DisableWelcomePage=no
WizardStyle=modern
SetupIconFile=..\..\assets\videodock.ico
UninstallDisplayIcon={app}\videodock.ico
LicenseFile=..\..\LICENSES\GPL-3.0.txt
OutputDir={#PackageOutputDir}
OutputBaseFilename=VideoDock-{#AppVersion}-windows-x64-Setup
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#AppSourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\VideoDock"; Filename: "{app}\VideoDock.exe"; WorkingDir: "{app}"; IconFilename: "{app}\videodock.ico"
Name: "{autodesktop}\VideoDock"; Filename: "{app}\VideoDock.exe"; WorkingDir: "{app}"; IconFilename: "{app}\videodock.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\VideoDock.exe"; Description: "{cm:LaunchProgram,VideoDock}"; Flags: nowait postinstall skipifsilent

; Profiles and downloaded media are outside {app}; uninstall removes installed files only.

[Code]
const
  StartupKey = 'Software\Microsoft\Windows\CurrentVersion\Run';

function IsStartupCommand(const Command, ExePath: String): Boolean;
begin
  Result := (CompareText(Command, '"' + ExePath + '"') = 0) or
    (CompareText(Command, '"\\?\' + ExePath + '"') = 0);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Command: String;
begin
  if (CurStep = ssPostInstall) and
    RegQueryStringValue(HKCU, StartupKey, 'VideoDock', Command) and
    IsStartupCommand(Command, ExpandConstant('{localappdata}\VideoDock\app\VideoDock.exe')) then
    RegWriteStringValue(HKCU, StartupKey, 'VideoDock', '"' + ExpandConstant('{app}\VideoDock.exe') + '"');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Command: String;
begin
  if (CurUninstallStep = usUninstall) and
    RegQueryStringValue(HKCU, StartupKey, 'VideoDock', Command) and
    IsStartupCommand(Command, ExpandConstant('{app}\VideoDock.exe')) then
    RegDeleteValue(HKCU, StartupKey, 'VideoDock');
end;
