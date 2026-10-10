@echo off
rem Makes .jlb files run when you double-click them (just for your Windows account; no admin needed).
rem Keep this folder where it is; if you move it, run install.bat again.
setlocal
set "EXE=%~dp0jlb.exe"
if not exist "%EXE%" set "EXE=%~dp0..\target\release\jlb.exe"
if not exist "%EXE%" (
    echo Couldn't find jlb.exe. Put it next to install.bat, or build it first with:
    echo     cargo build --release
    pause
    exit /b 1
)
for %%I in ("%EXE%") do set "EXE=%%~fI"

reg add "HKCU\Software\Classes\.jlb" /ve /d "jlb.program" /f >nul
reg add "HKCU\Software\Classes\jlb.program" /ve /d ".jlb program" /f >nul
reg add "HKCU\Software\Classes\jlb.program\DefaultIcon" /ve /d "\"%EXE%\",0" /f >nul
reg add "HKCU\Software\Classes\jlb.program\shell\open\command" /ve /d "\"%EXE%\" \"%%1\"" /f >nul
reg add "HKCU\Software\Classes\jlb.program\shell\edit" /ve /d "Edit" /f >nul
reg add "HKCU\Software\Classes\jlb.program\shell\edit\command" /ve /d "notepad.exe \"%%1\"" /f >nul

rem Tell Windows to refresh file icons and associations.
ie4uinit.exe -show >nul 2>&1

echo Done! .jlb files now run with:
echo     %EXE%
echo Double-click any .jlb file to run it, or right-click it and choose Edit.
pause
