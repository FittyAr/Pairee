; installer.iss
;
; Built by .github/scripts/package.sh, which passes the release version, the
; folder of pairee.exe and its architecture:
;   iscc /DAppVersion=X.Y.Z /DSourceDir=target\<triple>\release /DAppArch=x64|arm64 installer.iss
; The defaults below are for a local build of the x64 release binary.
#define AppName "Pairee"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef AppArch
  #define AppArch "x64"
#endif
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
DefaultDirName={localappdata}\Programs\FittyAr\Pairee
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
; An ARM64 build runs only on ARM64 Windows; x64 builds also run there.
#if AppArch == "arm64"
ArchitecturesAllowed=arm64
#else
ArchitecturesAllowed=x64compatible
#endif

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\pairee.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "lang\*"; DestDir: "{code:ConfigDir}\lang"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "help\*"; DestDir: "{code:ConfigDir}\help"; Flags: ignoreversion recursesubdirs createallsubdirs

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

{ Pairee's settings folder: %APPDATA%\FittyAr\Pairee, or the folder from
  before the shared FittyAr layout (%APPDATA%\pairee) while it could not be
  moved; the same rule as src/config/paths.rs. }
function DataRoot(): string;
begin
  Result := ExpandConstant('{userappdata}\FittyAr\Pairee');
  if not DirExists(Result) and DirExists(ExpandConstant('{userappdata}\pairee')) then
    Result := ExpandConstant('{userappdata}\pairee');
end;

function ConfigDir(Param: string): string;
begin
  Result := DataRoot() + '\config';
end;

{ Renames %APPDATA%\pairee into place before the translations and help are
  copied (copying them first would create the new folder and leave the
  settings behind). Nothing is deleted; when the folder is in use it stays
  where it is and Pairee moves it on a later start. }
procedure MovePreviousSettings();
var
  OldDir, NewDir: string;
begin
  OldDir := ExpandConstant('{userappdata}\pairee');
  NewDir := ExpandConstant('{userappdata}\FittyAr\Pairee');
  if DirExists(OldDir) and not DirExists(NewDir) then
  begin
    ForceDirectories(ExtractFileDir(NewDir));
    if RenameFile(OldDir, NewDir) then
      Log('Moved ' + OldDir + ' to ' + NewDir)
    else
      Log('Could not move ' + OldDir + '; it keeps being used');
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
    MovePreviousSettings();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
