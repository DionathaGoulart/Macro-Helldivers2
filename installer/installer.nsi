;; Instalador do Macro Helldivers 2 — NSIS/MUI2, por máquina, elevado.
;;
;; Compilar (na raiz do repositório, no Windows):
;;   cargo build --release --target x86_64-pc-windows-msvc
;;   makensis /DVERSION=2.0.0 installer\installer.nsi
;;
;; Defines aceitos:
;;   VERSION       versão exibida, gravada no registro e usada no nome do arquivo
;;   EXE_SOURCE    exe já compilado que vai ser embutido
;;   ASSETS_SOURCE raiz da árvore `assets/` que acompanha o exe
;;   OUTFILE       caminho do instalador gerado
;;
;; O app procura `assets/` ao lado do executável (`util::assets_dir`), então a
;; árvore é instalada dentro de `$INSTDIR` — não em `%APPDATA%`, que é só do
;; usuário e é justamente o que o desinstalador preserva.

Unicode true
ManifestDPIAware true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

;; ---------------------------------------------------------------- parâmetros

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef EXE_SOURCE
  !define EXE_SOURCE "..\target\x86_64-pc-windows-msvc\release\macro-helldivers2.exe"
!endif
!ifndef ASSETS_SOURCE
  !define ASSETS_SOURCE "..\assets"
!endif
!ifndef OUTFILE
  ;; `target/` já é ignorado pelo git e existe depois do build.
  !define OUTFILE "..\target\Macro-Helldivers-2-Setup-${VERSION}.exe"
!endif

!define APP_NAME "Macro Helldivers 2"
!define PUBLISHER "DionathaGoulart"
!define HOMEPAGE "https://github.com/DionathaGoulart/Macro-Helldivers2"
!define EXE_NAME "macro-helldivers2.exe"

;; Classe da janela principal (`ui::window::CLASS_NAME`). É por ela que o
;; instalador descobre se o app está aberto — inclusive quando ele está só na
;; bandeja, porque a janela continua existindo escondida.
!define MAIN_CLASS "MacroHelldivers2Main"

!define UNINST_ROOT "Software\Microsoft\Windows\CurrentVersion\Uninstall"
!define UNINST_KEY "MacroHelldivers2"

;; VIProductVersion exige quatro números; a tag pode ter pré-lançamento
;; ("2.0.0-beta.1"), então o sufixo é descartado só para os metadados do arquivo.
!searchparse /noerrors "${VERSION}" "" VER_MAJOR "." VER_MINOR "." VER_PATCH_RAW
!searchparse /noerrors "${VER_PATCH_RAW}" "" VER_PATCH "-" VER_PRERELEASE
!ifndef VER_PATCH
  !define VER_PATCH "${VER_PATCH_RAW}"
!endif

Name "${APP_NAME}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
RequestExecutionLevel admin
ShowInstDetails show
ShowUninstDetails show

;; Sólido + LZMA: a maior parte do payload são webp já comprimidos, e é o único
;; ajuste que ainda tira algum megabyte do instalador.
SetCompressor /SOLID lzma
SetCompressorDictSize 64

VIProductVersion "${VER_MAJOR}.${VER_MINOR}.${VER_PATCH}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "Instalador do ${APP_NAME}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

;; --------------------------------------------------------------------- páginas

!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"

!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE_NAME}"
!define MUI_FINISHPAGE_RUN_TEXT $(RunAppText)
!define MUI_FINISHPAGE_NOREBOOTSUPPORT

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "English"

LangString RunAppText ${LANG_PORTUGUESEBR} "Executar o ${APP_NAME}"
LangString RunAppText ${LANG_ENGLISH} "Run ${APP_NAME}"

LangString NeedX64 ${LANG_PORTUGUESEBR} "O ${APP_NAME} só roda em Windows 64 bits."
LangString NeedX64 ${LANG_ENGLISH} "${APP_NAME} requires 64-bit Windows."

LangString AppRunning ${LANG_PORTUGUESEBR} "O ${APP_NAME} está aberto.$\n$\nClique em OK para fechá-lo e continuar."
LangString AppRunning ${LANG_ENGLISH} "${APP_NAME} is running.$\n$\nClick OK to close it and continue."

LangString RemovingOldVersion ${LANG_PORTUGUESEBR} "Removendo a versão anterior..."
LangString RemovingOldVersion ${LANG_ENGLISH} "Removing the previous version..."

;; ------------------------------------------------------------------ variáveis

Var LegacyUninstaller
Var LegacyDir

;; -------------------------------------------------------------------- macros

;; Encerra o app antes de mexer nos arquivos.
;;
;; O caminho normal é o updater: ele executa este instalador e o app se encerra
;; sozinho logo depois, então basta esperar a janela sumir. Quem abriu o
;; instalador na mão com o app aberto é avisado antes de o processo ser
;; derrubado. `taskkill` só entra depois da espera porque `WM_CLOSE` não serve:
;; fechar a janela do app esconde ele na bandeja em vez de encerrar.
!macro CloseRunningApp UN
Function ${UN}CloseRunning
  StrCpy $R9 0

  wait:
    FindWindow $R8 "${MAIN_CLASS}" ""
    ${If} $R8 == 0
      Return
    ${EndIf}
    ${If} $R9 >= 10   ; ~5 s
      Goto force
    ${EndIf}
    Sleep 500
    IntOp $R9 $R9 + 1
    Goto wait

  force:
    ${If} ${Silent}
      Goto kill
    ${EndIf}
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "$(AppRunning)" IDOK kill
    Abort

  kill:
    nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM "${EXE_NAME}"'
    Pop $R8
    Sleep 1000
FunctionEnd
!macroend

!insertmacro CloseRunningApp ""
!insertmacro CloseRunningApp "un."

;; Procura a v1 (Electron/electron-builder) numa raiz e visão do registro.
;;
;; A v1 se identificava pelo mesmo DisplayName, mas com uma chave gerada pelo
;; electron-builder — daí a varredura em vez de um caminho fixo. A nossa própria
;; chave é pulada: sem isso, reinstalar por cima rodaria o nosso desinstalador.
;; Uma ocorrência por raiz basta (a v1 se instalava uma vez só) e evita
;; enumerar índices que acabaram de ser removidos.
!macro ScanLegacy ID ROOT VIEW
  SetRegView ${VIEW}
  StrCpy $R0 0

  scan_${ID}:
    EnumRegKey $R1 ${ROOT} "${UNINST_ROOT}" $R0
    StrCmp $R1 "" done_${ID}
    IntOp $R0 $R0 + 1
    StrCmp $R1 "${UNINST_KEY}" scan_${ID}

    ReadRegStr $R2 ${ROOT} "${UNINST_ROOT}\$R1" "DisplayName"
    StrCmp $R2 "${APP_NAME}" 0 scan_${ID}

    ReadRegStr $LegacyUninstaller ${ROOT} "${UNINST_ROOT}\$R1" "UninstallString"
    StrCmp $LegacyUninstaller "" scan_${ID}
    ReadRegStr $LegacyDir ${ROOT} "${UNINST_ROOT}\$R1" "InstallLocation"
    Call RunLegacyUninstaller

  done_${ID}:
!macroend

;; ------------------------------------------------------------------- funções

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "$(NeedX64)"
    Abort
  ${EndIf}

  ;; O app é x64: registro e arquivos vivem sempre na visão de 64 bits, mesmo
  ;; com o instalador rodando como processo de 32 bits.
  SetRegView 64

  ;; Reinstalação por cima (o updater é sempre isto): o diretório escolhido da
  ;; vez passada vira o padrão em vez de $PROGRAMFILES64.
  ReadRegStr $R0 HKLM "${UNINST_ROOT}\${UNINST_KEY}" "InstallLocation"
  ${If} $R0 != ""
    StrCpy $INSTDIR $R0
  ${EndIf}
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

;; Roda o desinstalador da v1 em silêncio, sem derrubar a instalação se falhar.
;;
;; `_?=` faz o desinstalador rodar no lugar em vez de se copiar para o temp — é
;; o que permite ao `ExecWait` de fato esperar o fim. O preço é o executável do
;; desinstalador ficar para trás, removido logo em seguida.
;;
;; Nenhum argumento de limpeza de dados é passado de propósito: as builds e
;; configurações da v1 continuam em `%APPDATA%` e são migradas na primeira
;; execução da v2 (`util::legacy_config_paths`).
Function RunLegacyUninstaller
  DetailPrint "$(RemovingOldVersion)"

  ;; O UninstallString vem entre aspas; `_?=` e `Delete` precisam do caminho cru.
  StrCpy $R2 $LegacyUninstaller 1
  ${If} $R2 == '"'
    StrCpy $LegacyUninstaller $LegacyUninstaller -1 1
  ${EndIf}

  ${If} $LegacyDir == ""
    ${GetParent} "$LegacyUninstaller" $LegacyDir
  ${EndIf}

  ${If} $LegacyDir == ""
    ExecWait '"$LegacyUninstaller" /S'
  ${Else}
    ExecWait '"$LegacyUninstaller" /S _?=$LegacyDir'
    Delete "$LegacyUninstaller"
    RMDir "$LegacyDir"
  ${EndIf}
FunctionEnd

Function UninstallLegacy
  ;; HKLM nas duas visões (a v1 circulou como instalação por máquina) e HKCU
  ;; para o caso de alguém ter recebido uma build por usuário.
  !insertmacro ScanLegacy "hklm64" HKLM 64
  !insertmacro ScanLegacy "hklm32" HKLM 32
  !insertmacro ScanLegacy "hkcu64" HKCU 64

  SetRegView 64
FunctionEnd

;; ---------------------------------------------------------------- instalação

Section "-Instalar"
  ;; Instalação por máquina: atalhos para todos os usuários.
  SetShellVarContext all
  SetRegView 64

  Call CloseRunning
  Call UninstallLegacy

  SetOutPath "$INSTDIR"
  File "${EXE_SOURCE}"
  ;; Recria a pasta `assets/` ao lado do exe, menos o que só serve em tempo de
  ;; build: `icon.ico` já está embutido no executável (build.rs) e é daqui que
  ;; este instalador tira o próprio ícone, e `icons/icon.png` é a arte de 1024px
  ;; de onde saiu o `tray.png`. Juntos são ~800 KB que ninguém lê em execução —
  ;; se algum código passar a abri-los, tire a exclusão correspondente.
  File /r /x ".DS_Store" /x "Thumbs.db" /x "icon.ico" /x "icon.png" "${ASSETS_SOURCE}"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateShortCut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"

  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE_NAME},0"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "URLInfoAbout" "${HOMEPAGE}"
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "${UNINST_ROOT}\${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_ROOT}\${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_ROOT}\${UNINST_KEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINST_ROOT}\${UNINST_KEY}" "EstimatedSize" $0
SectionEnd

;; -------------------------------------------------------------- desinstalação

Section "Uninstall"
  SetShellVarContext all
  SetRegView 64

  Call un.CloseRunning

  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  RMDir /r "$INSTDIR\assets"
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\Uninstall.exe"
  ;; Sem `/r`: o que o usuário tiver deixado na pasta não é nosso para apagar.
  RMDir "$INSTDIR"

  DeleteRegKey HKLM "${UNINST_ROOT}\${UNINST_KEY}"

  ;; `%APPDATA%\Macro Helldivers 2` fica: settings, slots, builds salvas e
  ;; caches sobrevivem à desinstalação e a reinstalar recupera tudo.
SectionEnd
