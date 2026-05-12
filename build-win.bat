@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   MMY Todo Windows Build Tool
echo ========================================
echo.

:: Get script directory
set "SCRIPT_DIR=%~dp0"
set "DESKTOP_DIR=%SCRIPT_DIR%desktop"

:: Check desktop directory
if not exist "%DESKTOP_DIR%" (
    echo [ERROR] desktop directory not found
    echo Path: %DESKTOP_DIR%
    pause
    exit /b 1
)

:: Enter desktop directory
cd /d "%DESKTOP_DIR%"
echo Current directory: %CD%
echo.

:: Check node
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js not found, please install Node.js first
    pause
    exit /b 1
)

:: Check npm
where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] npm not found, please install Node.js first
    pause
    exit /b 1
)

:: Install dependencies
echo [1/2] Installing dependencies...
call npm install
if %errorlevel% neq 0 (
    echo [ERROR] Failed to install dependencies
    pause
    exit /b 1
)
echo       Dependencies installed!
echo.

:: Build Portable (--no-bundle)
echo [2/3] Building portable exe...
call npm run tauri build -- --no-bundle
if %errorlevel% neq 0 (
    echo [ERROR] Portable build failed
    pause
    exit /b 1
)
echo       Portable exe built!
echo.

:: Immediately save portable exe before NSIS build overwrites it
set "RELEASE_DIR=%~dp0desktop\src-tauri\target\release"
set "OUT_DIR=%~dp0dist"
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

copy "%RELEASE_DIR%\mmy_todo_app.exe" "%OUT_DIR%\MMY TodoList Portable.exe"
echo       Saved: MMY TodoList Portable.exe
echo.

:: Build NSIS Installer
echo [3/3] Building NSIS installer...
call npm run build:win
if %errorlevel% neq 0 (
    echo [ERROR] NSIS build failed
    pause
    exit /b 1
)
echo       NSIS installer built!
echo.

:: Copy NSIS installer to dist
set "NSIS_DIR=%~dp0desktop\src-tauri\target\release\bundle\nsis"
for %%F in ("%NSIS_DIR%\*setup.exe") do (
    copy "%%F" "%OUT_DIR%\MMY TodoList_x64-setup.exe" >nul
)
echo       Saved: MMY TodoList_x64-setup.exe

:: Output result
echo ========================================
echo   Build completed!
echo ========================================
echo.
echo   Portable exe: MMY TodoList Portable.exe
echo   Installer:    MMY TodoList_x64-setup.exe
echo.
echo Final output:
echo   %OUT_DIR%
echo.
echo Opening output directory...
start "" "%OUT_DIR%"

pause