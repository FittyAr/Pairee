; installer.iss
#define AppName "Pairee"
#define AppVersion "0.5.1"
#define AppPublisher "FittyAr"
#define AppURL "https://github.com/FittyAr/Pairee"
#define AppExeName "pairee.exe"

#ifndef SourceDir
  #define SourceDir "target\x86_64-pc-windows-msvc\release"
#endif

[Setup]
AppId={{D37E8417-C08D-43EC-4FE5-87673A12B57F}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={localappdata}\Programs\pairee
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=target\release
OutputBaseFilename=pairee-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "{#SourceDir}\pairee.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "lang\*"; DestDir: "{userappdata}\pairee\config\lang"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "help\*"; DestDir: "{userappdata}\pairee\config\help"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "docs\*"; DestDir: "{userappdata}\pairee\config\docs"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"

[Registry]
; Safely append to user PATH environment variable
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{localappdata}\Programs\pairee;{olddata}"; Flags: preservestringtype










