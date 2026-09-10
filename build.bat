@echo off
echo Building Browser Picker...
echo.

pip install --quiet pyinstaller

echo [1/3] Building picker (browser_picker.exe)...
pyinstaller --onefile --noconsole --name browser_picker browser_picker.py --noconfirm >nul 2>&1
if not exist dist\browser_picker.exe (echo FAILED & exit /b 1)
echo       Done.

echo [2/3] Building daemon (browser_picker_daemon.exe)...
pyinstaller --onefile --noconsole --name browser_picker_daemon browser_picker_daemon.py --noconfirm >nul 2>&1
if not exist dist\browser_picker_daemon.exe (echo FAILED & exit /b 1)
echo       Done.

echo [3/3] Building launcher (browser_picker_launcher.exe)...
pyinstaller --onefile --noconsole --name browser_picker_launcher browser_picker_launcher.py --noconfirm >nul 2>&1
if not exist dist\browser_picker_launcher.exe (echo FAILED & exit /b 1)
echo       Done.

echo.
echo All builds successful. Installing to %LOCALAPPDATA%\BrowserPicker...

set DEST=%LOCALAPPDATA%\BrowserPicker
if not exist "%DEST%" mkdir "%DEST%"

copy /y dist\browser_picker.exe          "%DEST%\" >nul
copy /y dist\browser_picker_daemon.exe   "%DEST%\" >nul
copy /y dist\browser_picker_launcher.exe "%DEST%\" >nul
copy /y install.py                       "%DEST%\" >nul
copy /y uninstall.py                     "%DEST%\" >nul

echo Copied to %DEST%
echo.
echo Running install.py...
cd /d "%DEST%"
python install.py
