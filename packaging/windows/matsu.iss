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
ShowLanguageDialog=no
; An update closes a running MATSU Editor that holds the files (the editor keeps unsaved documents itself).
CloseApplications=yes
RestartApplications=no

[Languages]
; The language of the installer follows the language of Windows (ShowLanguageDialog=no): Japanese on a Japanese
; Windows, English on every other one (the first entry is the fallback). No dialog asks, because the two
; languages are the only ones and the choice is clear from the system. /LANG=ja or /LANG=en forces one.
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"

[CustomMessages]
en.GroupFileTypes=File types:
ja.GroupFileTypes=ファイルの種類:
en.GroupCommandLine=Command line:
ja.GroupCommandLine=コマンドライン:
en.TaskOpenWith=Show {#AppName} in the "Open with" list
ja.TaskOpenWith=「プログラムから開く」の一覧にマツエディタを出す
en.TaskAssociateTxt=Offer {#AppName} for .txt files (choose it in Settings > Apps > Default apps)
ja.TaskAssociateTxt=.txt ファイルを開くプログラムの候補にマツエディタを出す（Windows の設定の「既定のアプリ」で選びます）
en.TaskAddToPath=Add the folder of {#AppName} to the &PATH (to start matsu from a terminal)
ja.TaskAddToPath=マツエディタのフォルダーを &PATH に追加する（ターミナルから matsu を起動するため）

[Tasks]
Name: openwith; Description: "{cm:TaskOpenWith}"; GroupDescription: "{cm:GroupFileTypes}"
Name: associatetxt; Description: "{cm:TaskAssociateTxt}"; GroupDescription: "{cm:GroupFileTypes}"; Flags: unchecked
Name: addtopath; Description: "{cm:TaskAddToPath}"; GroupDescription: "{cm:GroupCommandLine}"; Flags: unchecked

[Files]
; Everything the zip has (package.yml, "Stage the files"): the two programs, the licenses, the notices, README.txt, icons\ and help\ (the help, ADR-0031;
; the uninstaller removes it with the other files).
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
  // The exact text appended to PATH (with or without a ';' in front), so that the uninstall can restore the old value.
  OwnTextValue = 'PathTextAppended';

procedure AddToPath(Dir: String);
var
  Paths, Appended: String;
begin
  // Raw (unexpanded) value: PATH is usually REG_EXPAND_SZ with %USERPROFILE% and the like in it.
  if not RegQueryStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths) then
    Paths := '';
  // Already there (an update, or the user added it): leave it as it is, and the record as it is.
  if Pos(';' + Lowercase(Dir) + ';', ';' + Lowercase(Paths) + ';') > 0 then
    Exit;
  Appended := Dir;
  if (Paths <> '') and (Paths[Length(Paths)] <> ';') then
    Appended := ';' + Dir;
  RegWriteExpandStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths + Appended);
  RegWriteStringValue(EnvRoot(), OwnKey, OwnValue, Dir);
  RegWriteStringValue(EnvRoot(), OwnKey, OwnTextValue, Appended);
end;

procedure RemoveFromPath(Dir: String);
var
  Paths, Wrapped, Added, Appended: String;
  P: Integer;
begin
  if not RegQueryStringValue(EnvRoot(), OwnKey, OwnValue, Added) then
    Exit;
  if not RegQueryStringValue(EnvRoot(), OwnKey, OwnTextValue, Appended) then
    Appended := '';
  RegDeleteValue(EnvRoot(), OwnKey, OwnValue);
  RegDeleteValue(EnvRoot(), OwnKey, OwnTextValue);
  RegDeleteKeyIfEmpty(EnvRoot(), OwnKey);
  if CompareText(Added, Dir) <> 0 then
    Exit;
  if not RegQueryStringValue(EnvRoot(), EnvSubKey(), 'Path', Paths) then
    Exit;
  // The usual case: what was appended is still at the end. Cut it off, and the old value is back as it was.
  if (Appended <> '') and (Length(Paths) >= Length(Appended)) and
     (CompareText(Copy(Paths, Length(Paths) - Length(Appended) + 1, Length(Appended)), Appended) = 0) then
  begin
    RegWriteExpandStringValue(EnvRoot(), EnvSubKey(), 'Path', Copy(Paths, 1, Length(Paths) - Length(Appended)));
    Exit;
  end;
  // The user changed PATH since: take the entry out wherever it is.
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
  if CurStep = ssPostInstall then
  begin
    if WizardIsTaskSelected('addtopath') then
      AddToPath(ExpandConstant('{app}'))
    else
      // An update with the option unchecked takes back what an earlier install added (and only that).
      RemoveFromPath(ExpandConstant('{app}'));
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
