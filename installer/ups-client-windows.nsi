; =============================================================================
; NSIS (Nullsoft Scriptable Install System) Setup Script for OpenUPS Client for Windows
; Open-source Windows .exe installer definition
; =============================================================================

!include "MUI2.nsh"
!include "x64.nsh"

!ifndef VERSION
  !define VERSION "1.0.0"
!endif

!ifndef OUTFILE
  !define OUTFILE "..\dist\UPS-Client-Windows-v${VERSION}-x86_64-setup.exe"
!endif

!define APP_NAME "OpenUPS Client for Windows"
!define COMP_NAME "Werner Garcia (virlatinus)"
!define WEB_SITE "https://github.com/virlatinus/UPS-Client-Windows"
!define MAIN_APP_EXE "ups-client-windows.exe"
!define UNINSTALL_REG_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\UPS-Client-Windows"

Name "${APP_NAME} v${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\virlatinus\UPS-Client-Windows" "Install_Dir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
Unicode True

; -----------------------------------------------------------------------------
; Modern UI Configuration
; -----------------------------------------------------------------------------
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${MAIN_APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; -----------------------------------------------------------------------------
; Version Information embedded in Setup .exe
; -----------------------------------------------------------------------------
VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "${COMP_NAME}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "${APP_NAME} Setup Installer"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${VERSION}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "MIT License - Copyright (c) 2026 ${COMP_NAME}"

; -----------------------------------------------------------------------------
; Installer Section
; -----------------------------------------------------------------------------
Section "Install" SecMain
  SetRegView 64
  SetOutPath "$INSTDIR"

  ; Install main application binary and docs
  File "..\target\release\${MAIN_APP_EXE}"
  File "..\LICENSE"
  File "..\README.md"

  ; Store installation folder
  WriteRegStr HKLM "Software\virlatinus\UPS-Client-Windows" "Install_Dir" "$INSTDIR"

  ; Create Uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Create Start Menu & Desktop shortcuts
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${MAIN_APP_EXE}" "" "$INSTDIR\${MAIN_APP_EXE}" 0
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${MAIN_APP_EXE}" "" "$INSTDIR\${MAIN_APP_EXE}" 0

  ; Register in Windows Add/Remove Programs (Installed Apps)
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "Publisher" "${COMP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "URLInfoAbout" "${WEB_SITE}"
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "DisplayIcon" "$INSTDIR\${MAIN_APP_EXE}"
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKLM "${UNINSTALL_REG_KEY}" "QuietUninstallString" "$\"$INSTDIR\Uninstall.exe$\" /S"
  WriteRegDWORD HKLM "${UNINSTALL_REG_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_REG_KEY}" "NoRepair" 1
SectionEnd

; -----------------------------------------------------------------------------
; Uninstaller Section
; -----------------------------------------------------------------------------
Section "Uninstall"
  SetRegView 64

  ; Remove installed files
  Delete "$INSTDIR\${MAIN_APP_EXE}"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  ; Remove shortcuts
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  ; Remove startup registry entry if present
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "UPS-Client-Windows"

  ; Remove installation & uninstall registry keys
  DeleteRegKey HKLM "${UNINSTALL_REG_KEY}"
  DeleteRegKey HKLM "Software\virlatinus\UPS-Client-Windows"
SectionEnd
