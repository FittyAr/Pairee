; installer.iss
#define AppName "Pairee"
#define AppVersion "0.10.0"
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
SetupIconFile=manifests\windows\pairee.ico
WizardImageFile=manifests\windows\wizard_image.png
WizardSmallImageFile=manifests\windows\wizard_small_image.png
UninstallDisplayIcon={app}\{#AppExeName}
; Per-user install ({localappdata}, HKCU): no UAC prompt, and the files land in
; the profile of the user who runs it rather than an elevating administrator's.
PrivilegesRequired=lowest
; Broadcast the PATH change so new terminals find pairee without a sign-out.
ChangesEnvironment=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\pairee.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "lang\*"; DestDir: "{userappdata}\pairee\config\lang"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "help\*"; DestDir: "{userappdata}\pairee\config\help"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
; Prepend the install folder to the user PATH once: NeedsAddPath skips it
; when an update finds it already there. The uninstaller removes it again.
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{app};{olddata}"; Flags: preservestringtype; Check: NeedsAddPath('{app}')

[Code]
const
  EnvKey = 'Environment';

{ ';A;B;' form of the user PATH, upper case, for whole-entry matching. }
function WrappedUserPath(): string;
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths) then
    Paths := '';
  Result := ';' + Uppercase(Paths) + ';';
end;

{ True when Dir (with or without a trailing backslash) is not in the user PATH. }
function NeedsAddPath(Dir: string): Boolean;
var
  Wrapped, Wanted: string;
begin
  Wrapped := WrappedUserPath();
  Wanted := Uppercase(RemoveBackslashUnlessRoot(ExpandConstant(Dir)));
  Result := (Pos(';' + Wanted + ';', Wrapped) = 0) and (Pos(';' + Wanted + '\;', Wrapped) = 0);
end;

{ Removes every entry equal to Dir (earlier versions could add it twice). }
procedure RemoveFromPath(Dir: string);
var
  Paths, Entry, Kept, Wanted: string;
  Sep: Integer;
  Changed: Boolean;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths) then
    exit;
  Wanted := Uppercase(RemoveBackslashUnlessRoot(Dir));
  Kept := '';
  Changed := False;
  Paths := Paths + ';';
  while Paths <> '' do
  begin
    Sep := Pos(';', Paths);
    Entry := Copy(Paths, 1, Sep - 1);
    Delete(Paths, 1, Sep);
    if Uppercase(RemoveBackslashUnlessRoot(Entry)) = Wanted then
      Changed := True
    else if Entry <> '' then
    begin
      if Kept <> '' then
        Kept := Kept + ';';
      Kept := Kept + Entry;
    end;
  end;
  if Changed then
    RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Kept);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
