@echo off
setlocal EnableExtensions EnableDelayedExpansion
cd /d "%~dp0"

set SRC=mini_stepmania_rust
set OUTROOT=mini_stepmania_backups

if not exist "%SRC%" (
  echo Source folder not found: %SRC%
  exit /b 1
)

for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd_HHmmss"') do set TS=%%i
set DEST=%OUTROOT%\mini_stepmania_min_!TS!

mkdir "%DEST%" >nul 2>nul
if errorlevel 1 (
  echo Failed to create backup folder: %DEST%
  exit /b 1
)

robocopy "%SRC%\src" "%DEST%\src" /E /NFL /NDL /NJH /NJS /NP >nul
if errorlevel 8 (
  echo Failed to copy src
  exit /b 1
)

if exist "%SRC%\assets" (
  robocopy "%SRC%\assets" "%DEST%\assets" /E /NFL /NDL /NJH /NJS /NP >nul
  if errorlevel 8 (
    echo Failed to copy assets
    exit /b 1
  )
)

copy /Y "%SRC%\Cargo.toml" "%DEST%\" >nul
if exist "%SRC%\Cargo.lock" copy /Y "%SRC%\Cargo.lock" "%DEST%\" >nul
if exist "%SRC%\README.md" copy /Y "%SRC%\README.md" "%DEST%\" >nul
if exist "debug.bat" copy /Y "debug.bat" "%DEST%\" >nul
copy /Y "backup_min_code.bat" "%DEST%\" >nul

echo Backup created: %DEST%
