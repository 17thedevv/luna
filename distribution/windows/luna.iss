#ifndef GeneratedDir
  #error GeneratedDir must point to the output of build_installer.py
#endif
#include GeneratedDir + "\payloads.iss"

[Setup]
AppId=Luna.Toolchain
AppName=Luna
AppVersion=0.1.0-alpha.1
AppVerName=Luna 0.1.0-alpha.1
AppPublisher=17thedevv
AppPublisherURL=https://github.com/17thedevv/luna
AppSupportURL=https://github.com/17thedevv/luna/issues
DefaultDirName={localappdata}\Programs\Luna
DefaultGroupName=Luna
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.17763
DisableProgramGroupPage=yes
DisableDirPage=no
WizardStyle=modern
InfoBeforeFile=installer-notices.txt
OutputDir={#OutputDir}
OutputBaseFilename=lunainstaller
Compression=lzma2
SolidCompression=yes
ArchiveExtraction=full
UninstallDisplayIcon={app}\bin\luna.exe
ChangesEnvironment=yes
SetupLogging=yes
ExtraDiskSpaceRequired=1500000000

[Tasks]
Name: addtopath; Description: "Add Luna to my user PATH (new terminals)"; Flags: checkedonce

[Files]
Source: "{#GeneratedDir}\luna.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#GeneratedDir}\luna-cc.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "luna-shell.cmd"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#GeneratedDir}\installer-lock.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "installer-notices.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#LunaUrl}"; DestName: "luna-package.zip"; DestDir: "{app}\packages"; Hash: "{#LunaSHA256}"; ExternalSize: {#LunaUnpacked}; Flags: external download extractarchive recursesubdirs createallsubdirs ignoreversion
Source: "{#LLVMUrl}"; DestName: "luna-llvm18.zip"; DestDir: "{app}"; Hash: "{#LLVMSHA256}"; ExternalSize: {#LLVMUnpacked}; Flags: external download extractarchive recursesubdirs createallsubdirs ignoreversion
Source: "{#GNUUrl}"; DestName: "winlibs.zip"; DestDir: "{app}\toolchains\gnu"; Hash: "{#GNUSHA256}"; ExternalSize: {#GNUUnpacked}; Flags: external download extractarchive recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\Luna Command Prompt"; Filename: "{cmd}"; Parameters: "/K """"{app}\bin\luna-shell.cmd"""""; WorkingDir: "{userdocs}"; IconFilename: "{app}\bin\luna.exe"
Name: "{group}\Luna documentation"; Filename: "https://lunalang.netlify.app/getting-started/"

[Code]
const
  StateKey = 'Software\Luna\Installer';
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Luna.Toolchain_is1';
var
  OwnPath: Boolean;
  PreviousPathPresent: Boolean;
  PreviousPath, InstalledPath, PathEntry: String;

function NormalPath(Value: String): String;
begin
  Result := Trim(Value);
  if (Length(Result) > 1) and (Result[1] = '"') and (Result[Length(Result)] = '"') then
    Result := Copy(Result, 2, Length(Result) - 2);
  while (Length(Result) > 3) and (Result[Length(Result)] = '\') do
    Delete(Result, Length(Result), 1);
  Result := Lowercase(Result);
end;

function ContainsPath(Value, Entry: String): Boolean;
var Token: String; Separator: Integer;
begin
  Result := False;
  repeat
    Separator := Pos(';', Value);
    if Separator = 0 then begin Token := Value; Value := ''; end
    else begin Token := Copy(Value, 1, Separator - 1); Delete(Value, 1, Separator); end;
    if NormalPath(Token) = NormalPath(Entry) then begin Result := True; Exit; end;
  until Value = '';
end;

function WithoutPath(Value, Entry: String): String;
var Token: String; Separator: Integer; First: Boolean;
begin
  Result := ''; First := True;
  repeat
    Separator := Pos(';', Value);
    if Separator = 0 then begin Token := Value; Value := ''; end
    else begin Token := Copy(Value, 1, Separator - 1); Delete(Value, 1, Separator); end;
    if NormalPath(Token) <> NormalPath(Entry) then begin
      if not First then Result := Result + ';';
      Result := Result + Token; First := False;
    end;
  until (Separator = 0);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var OldDir: String;
begin
  Result := True;
  if (CurPageID = wpSelectDir) and RegQueryStringValue(HKCU, UninstallKey, 'Inno Setup: App Path', OldDir) then
    if (OldDir <> '') and (NormalPath(OldDir) <> NormalPath(ExpandConstant('{app}'))) then begin
      MsgBox('Uninstall the existing Luna installation before choosing a different directory.', mbError, MB_OK);
      Result := False;
    end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var CurrentPath: String; Added: Cardinal; Code: Integer;
begin
  if CurStep <> ssPostInstall then Exit;
  if not Exec(ExpandConstant('{app}\bin\luna.exe'), '--version', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
    RaiseException('Installed Luna failed its version check. See the setup log; user PATH was not changed.');
  if not WizardIsTaskSelected('addtopath') then Exit;
  PathEntry := ExpandConstant('{app}\bin');
  if RegQueryDWordValue(HKCU64, StateKey, 'OwnPath', Added) then
    if Added = 1 then Exit;
  PreviousPathPresent := RegQueryStringValue(HKCU, 'Environment', 'Path', CurrentPath);
  if ContainsPath(CurrentPath, PathEntry) then Exit;
  PreviousPath := CurrentPath;
  if (CurrentPath <> '') and (CurrentPath[Length(CurrentPath)] <> ';') then CurrentPath := CurrentPath + ';';
  InstalledPath := CurrentPath + PathEntry;
  if not (RegWriteDWordValue(HKCU, StateKey, 'PreviousPathPresent', Ord(PreviousPathPresent)) and
          RegWriteStringValue(HKCU, StateKey, 'PreviousPath', PreviousPath) and
          RegWriteStringValue(HKCU, StateKey, 'InstalledPath', InstalledPath) and
          RegWriteStringValue(HKCU, StateKey, 'PathEntry', PathEntry) and
          RegWriteDWordValue(HKCU, StateKey, 'OwnPath', 1)) then
    RaiseException('Cannot record PATH ownership; user PATH was not changed.');
  if not RegWriteExpandStringValue(HKCU, 'Environment', 'Path', InstalledPath) then begin
    RegDeleteValue(HKCU, StateKey, 'OwnPath');
    RaiseException('Cannot update the user PATH.');
  end;
end;

function InitializeUninstall(): Boolean;
var Value: Cardinal;
begin
  Result := True;
  OwnPath := False;
  if RegQueryDWordValue(HKCU64, StateKey, 'OwnPath', Value) then OwnPath := Value = 1;
  PreviousPathPresent := False;
  if RegQueryDWordValue(HKCU64, StateKey, 'PreviousPathPresent', Value) then PreviousPathPresent := Value = 1;
  RegQueryStringValue(HKCU64, StateKey, 'PreviousPath', PreviousPath);
  RegQueryStringValue(HKCU64, StateKey, 'InstalledPath', InstalledPath);
  RegQueryStringValue(HKCU64, StateKey, 'PathEntry', PathEntry);
  Log(Format('Luna PATH ownership: %d; entry: %s', [Ord(OwnPath), PathEntry]));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var CurrentPath: String;
begin
  if CurUninstallStep <> usPostUninstall then Exit;
  if OwnPath and RegQueryStringValue(HKCU64, 'Environment', 'Path', CurrentPath) then begin
    if CurrentPath = InstalledPath then begin
      if PreviousPathPresent then RegWriteExpandStringValue(HKCU, 'Environment', 'Path', PreviousPath)
      else RegDeleteValue(HKCU, 'Environment', 'Path');
    end else RegWriteExpandStringValue(HKCU, 'Environment', 'Path', WithoutPath(CurrentPath, PathEntry));
  end;
  Log('Luna PATH cleanup completed.');
  RegDeleteValue(HKCU, StateKey, 'OwnPath');
  RegDeleteValue(HKCU, StateKey, 'PreviousPathPresent');
  RegDeleteValue(HKCU, StateKey, 'PreviousPath');
  RegDeleteValue(HKCU, StateKey, 'InstalledPath');
  RegDeleteValue(HKCU, StateKey, 'PathEntry');
  RegDeleteKeyIfEmpty(HKCU, StateKey);
end;
