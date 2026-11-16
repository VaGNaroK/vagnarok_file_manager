; Inno Setup script for Vagnarok File Manager
#ifndef AppVersion
  #define AppVersion "0.1.0-alpha.1"
#endif
#ifndef NumericVersion
  #define NumericVersion "0.1.0.0"
#endif

[Setup]
AppId={{9E8A42D3-7D1F-4B1C-8A54-6D8C0F4E2911}
AppName=Vagnarok File Manager
AppVersion={#AppVersion}
AppVerName=Vagnarok File Manager v{#AppVersion}
AppPublisher=Vagnarok File Manager (em desenvolvimento)
DefaultDirName={autopf}\Vagnarok File Manager
DefaultGroupName=Vagnarok File Manager
AllowNoIcons=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=vagnarok-file-manager-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\assets\icons\vagnarok.ico
UninstallDisplayIcon={app}\vagnarok-file-manager.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
VersionInfoVersion={#NumericVersion}
VersionInfoProductName=Vagnarok File Manager
VersionInfoProductVersion={#NumericVersion}
VersionInfoProductTextVersion={#AppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "contextmenu"; Description: "Add 'Open in Vagnarok File Manager' to folder context menu"; Flags: unchecked

[Files]
Source: "..\..\target\release\vagnarok-file-manager.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Vagnarok File Manager"; Filename: "{app}\vagnarok-file-manager.exe"
Name: "{group}\{cm:UninstallProgram,Vagnarok File Manager}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Vagnarok File Manager"; Filename: "{app}\vagnarok-file-manager.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\Directory\shell\VagnarokFileManager"; ValueType: string; ValueData: "Open in Vagnarok File Manager"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\VagnarokFileManager"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\vagnarok-file-manager.exe"",0"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\VagnarokFileManager\command"; ValueType: string; ValueData: """{app}\vagnarok-file-manager.exe"" ""%1"""; Tasks: contextmenu

[Run]
Filename: "{app}\vagnarok-file-manager.exe"; Description: "{cm:LaunchProgram,Vagnarok File Manager}"; Flags: nowait postinstall skipifsilent
