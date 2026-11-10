; Mostafa-converting app - NSIS hooks: register the Explorer context menu on install and remove
; every per-user registry entry the app creates on uninstall. All keys live under HKCU.

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Classes\*\shell\MostafaConverter" "MUIVerb" "Convert with Mostafa-converting app"
  WriteRegStr HKCU "Software\Classes\*\shell\MostafaConverter" "Icon" '"$INSTDIR\${MAINBINARYNAME}.exe",0'
  WriteRegStr HKCU "Software\Classes\*\shell\MostafaConverter" "MultiSelectModel" "Player"
  WriteRegStr HKCU "Software\Classes\*\shell\MostafaConverter\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" --open "%1"'
  WriteRegStr HKCU "Software\Classes\Directory\shell\MostafaConverter" "MUIVerb" "Compress with Mostafa-converting app"
  WriteRegStr HKCU "Software\Classes\Directory\shell\MostafaConverter" "Icon" '"$INSTDIR\${MAINBINARYNAME}.exe",0'
  WriteRegStr HKCU "Software\Classes\Directory\shell\MostafaConverter" "MultiSelectModel" "Player"
  WriteRegStr HKCU "Software\Classes\Directory\shell\MostafaConverter\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" --open "%1"'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\MostafaConverter"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\MostafaConverter"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Mostafa-converting app"
!macroend
