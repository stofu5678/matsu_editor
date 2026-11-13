; The installer of MATSU Editor for Windows (ADR-0032), made with Inno Setup 6.3 or later (ARM64 support).
; One installer per target. package.yml compiles it after it has staged the files of the zip:
;
;   ISCC.exe /DTarget=windows-x64 /DArch=x64compatible /DAppVersion=0.13.0 /DStageDir=<dist\matsu-windows-x64> /DOutDir=<dist> matsu.iss
;
; The result is <OutDir>\matsu-<Target>-setup.exe.
;
; The installer installs the files of the zip and nothing else, per user by default
; (%LOCALAPPDATA%\Programs\MATSU Editor, no administrator). "All users" (Program Files) is a choice in the
; first dialog, or /ALLUSERS on the command line (/CURRENTUSER for the default). Silent: /VERYSILENT /SUPPRESSMSGBOXES.
;
; The user's data is never touched (ADR-0032, decision 3): this script creates, changes and deletes no
; matsu.ini, unsaved\, plugins.ini, databases.ini, plugins\ or history, neither in %APPDATA%\matsu-editor nor next to
; matsu.exe. It has no [UninstallDelete] section and no "remove the settings too" choice. Inno Setup's
; uninstaller removes only the files it installed (and their folders when they are empty), so a portable
; matsu.ini the user put next to matsu.exe stays, and so does the folder around it.

#ifndef Target
  #error Pass /DTarget=windows-x64 or /DTarget=windows-arm64.
#endif
#ifndef AppVersion
  #error Pass /DAppVersion=<the version of Cargo.toml>.
#endif
#ifndef Arch
  #error Pass /DArch=x64compatible or /DArch=arm64 (the architecture of the programs).
#endif
#ifndef StageDir
  #error Pass /DStageDir=<the folder with the staged files of the zip>.
#endif
#ifndef OutDir
  #define OutDir "..\..\dist"
#endif

#define AppName "MATSU Editor"
#define AppExe "matsu.exe"

[Setup]
; Fixed for all versions and both targets: an installer finds the installation of an older version by it, and
; updates that one. Never change it.
AppId={{7A716E2B-C55F-4618-92F3-3058FA1AAEEB}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
; The copyright holder in LICENSE-MIT (the same as CompanyName in crates/matsu/matsu.rc).
AppPublisher=stofu5678
AppPublisherURL=https://github.com/stofu5678/matsu_editor
AppSupportURL=https://github.com/stofu5678/matsu_editor/issues
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} setup
VersionInfoProductName={#AppName}
AppCopyright=Copyright (c) 2026 stofu5678

DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
; x64compatible also takes Windows 11 on ARM64, which runs x64 programs.
ArchitecturesAllowed={#Arch}
ArchitecturesInstallIn64BitMode={#Arch}

OutputDir={#OutDir}
OutputBaseFilename=matsu-{#Target}-setup
SetupIconFile=..\..\assets\icons\matsu_editor.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Tell Explorer about the changed PATH and file types.
ChangesEnvironment=yes
ChangesAssociations=yes
; An update closes a running MATSU Editor that holds the files (the editor keeps unsaved documents itself).
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: openwith; Description: "Show {#AppName} in the ""Open with"" list"; GroupDescription: "File types:"
Name: associatetxt; Description: "Offer {#AppName} for .txt files (choose it in Settings > Apps > Default apps)"; GroupDescription: "File types:"; Flags: unchecked
Name: addtopath; Description: "Add the folder of {#AppName} to the &PATH (to start matsu from a terminal)"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
; Everything the zip has (package.yml, "Stage the files"): the two programs, the licenses, the notices, README.txt and icons\.
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"

[Registry]
; "Open with" (task openwith). HKA is HKCU for a per-user install and HKLM for an all-users one.
Root: HKA; Subkey: "Software\Classes\Applications\matsu.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#AppName}"; Flags: uninsdeletekey; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\Applications\matsu.exe\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\Applications\matsu.exe\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: openwith

; .txt (task associatetxt). Windows 10 and later let only the user choose the default program (a hash protects it), so
; this registers MATSU Editor as a program for .txt and lists it under Default apps; it does not take .txt over.
Root: HKA; Subkey: "Software\Classes\MATSU.Editor.txt"; ValueType: string; ValueName: ""; ValueData: "Text Document"; Flags: uninsdeletekey; Tasks: associatetxt
Root: HKA; Subkey: "Software\Classes\MATSU.Editor.txt\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associatetxt
Root: HKA; Subkey: "Software\Classes\MATSU.Editor.txt\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associatetxt
Root: HKA; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "MATSU.Editor.txt"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associatetxt
Root: HKA; Subkey: "Software\{#AppName}"; Flags: uninsdeletekeyifempty; Tasks: associatetxt
Root: HKA; Subkey: "Software\{#AppName}\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#AppName}"; Flags: uninsdeletekey; Tasks: associatetxt
Root: HKA; Subkey: "Software\{#AppName}\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "A text editor that feels like SAKURA Editor"; Tasks: associatetxt
Root: HKA; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".txt"; ValueData: "MATSU.Editor.txt"; Tasks: associatetxt
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#AppName}"; ValueData: "Software\{#AppName}\Capabilities"; Flags: uninsdeletevalue; Tasks: associatetxt

[Code]
// The PATH (task addtopath) lives in the user's environment for a per-user install and in the machine's for an
// all-users one.
function EnvRoot(): Integer;
begin
  if IsAdminInstallMode then
    Result := HKEY_LOCAL_MACHINE
  else
    Result := HKEY_CURRENT_USER;
end;

function EnvSubKey(): String;
begin
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

// The installer removes a PATH entry only if it added it itself: this value (under HKA\Software\MATSU Editor) holds the
// folder it added. An entry that was there before (the user's own) is not recorded, so it stays after the uninstall.
const
  OwnKey = 'Software\{#AppName}';
  OwnValue = 'PathEntryAdded';

procedure AddToPath(Dir: String);
var
  Paths: String;
begin
  // Raw (unexpanded) value: PATH is usually REG_EXPAND_SZ with %USERPROFILE% and the like in it.
  if not RegQueryStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths) then
    Paths := '';
  // Already there (an update, or the user added it): leave it as it is, and the record as it is.
  if Pos(';' + Lowercase(Dir) + ';', ';' + Lowercase(Paths) + ';') > 0 then
    Exit;
  if (Paths <> '') and (Paths[Length(Paths)] <> ';') then
    Paths := Paths + ';';
  RegWriteExpandStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths + Dir);
  RegWriteStringValue(EnvRoot(), OwnKey, OwnValue, Dir);
end;

procedure RemoveFromPath(Dir: String);
var
  Paths, Wrapped, Added: String;
  P: Integer;
begin
  if not RegQueryStringValue(EnvRoot(), OwnKey, OwnValue, Added) then
    Exit;
  RegDeleteValue(EnvRoot(), OwnKey, OwnValue);
  RegDeleteKeyIfEmpty(EnvRoot(), OwnKey);
  if CompareText(Added, Dir) <> 0 then
    Exit;
  if not RegQueryStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths) then
    Exit;
  Wrapped := ';' + Paths + ';';
  P := Pos(';' + Lowercase(Dir) + ';', Lowercase(Wrapped));
  if P = 0 then
    Exit;
  // Take out ';Dir' (the ';' after it stays as the separator), then the ';' added in front and behind.
  Delete(Wrapped, P, Length(Dir) + 1);
  RegWriteExpandStringValue(EnvRoot(), EnvSubKey(), 'Path', Copy(Wrapped, 2, Length(Wrapped) - 2));
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
