@echo off
rem Undoes install.bat: .jlb files will no longer open with jlb.exe.
reg delete "HKCU\Software\Classes\.jlb" /f >nul 2>&1
reg delete "HKCU\Software\Classes\jlb.program" /f >nul 2>&1
ie4uinit.exe -show >nul 2>&1
echo .jlb files are no longer linked to jlb.exe.
pause
