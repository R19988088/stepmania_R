@echo off
setlocal
cd /d "%~dp0"

set CHART=%~1
set DIFF=%~2
set KEEP_OPEN=1

set RUST_BACKTRACE=1
set CARGO_INCREMENTAL=0
set CARGO_BUILD_JOBS=1

if exist "mini_stepmania_rust\Cargo.toml" (
  set MANIFEST=mini_stepmania_rust\Cargo.toml
) else if exist "Cargo.toml" (
  set MANIFEST=Cargo.toml
) else (
  echo.
  echo Cannot find Cargo.toml.
  pause
  exit /b 1
)

if "%CHART%"=="" (
  cargo run --manifest-path "%MANIFEST%" --
) else if "%DIFF%"=="" (
  cargo run --manifest-path "%MANIFEST%" -- "%CHART%"
) else (
  cargo run --manifest-path "%MANIFEST%" -- "%CHART%" "%DIFF%"
)

if errorlevel 1 (
  echo.
  echo Run failed. See error above.
  if "%KEEP_OPEN%"=="1" pause
  exit /b %errorlevel%
)

if "%KEEP_OPEN%"=="1" (
  echo.
  echo Run finished.
  pause
)
