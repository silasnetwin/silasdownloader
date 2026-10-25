; Wartet beim Selbst-Update, bis die laufende App beendet ist.
;
; Der Updater startet den Installer und beendet die App unmittelbar danach.
; Der Installer prüft anschließend über den Restart Manager, ob noch eine Instanz
; läuft. Trifft er die App beim Beenden an, schlägt das Beenden fehl ("Failed to
; kill SilasDownloader"). Deshalb wird hier vor dieser Prüfung gewartet, bis die
; EXE nicht mehr gesperrt ist. Nach 20 Sekunden übernimmt wieder die normale
; Prüfung des Installers.

!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
    Push $R8
    Push $R9
    StrCpy $R9 0
    silas_wait_for_exit:
    IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 silas_wait_done
    ClearErrors
    ; Im Anhängemodus öffnen: Die laufende EXE ist gesperrt, eine beendete nicht.
    FileOpen $R8 "$INSTDIR\${MAINBINARYNAME}.exe" a
    IfErrors 0 silas_exe_released
    IntOp $R9 $R9 + 1
    IntCmp $R9 80 silas_wait_done
    Sleep 250
    Goto silas_wait_for_exit
    silas_exe_released:
    FileClose $R8
    silas_wait_done:
    Pop $R9
    Pop $R8
  ${EndIf}
!macroend
