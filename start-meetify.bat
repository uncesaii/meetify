@echo off
title MEETIFY Launcher
cd /d "%~dp0"

echo ===================================================
echo               Starting MEETIFY
echo ===================================================
echo.

REM Clean up any dangling processes on port 3118 and old meetify.exe
echo [1/3] Checking for previous instances...
taskkill /IM meetify.exe /F >nul 2>&1
for /f "tokens=5" %%a in ('netstat -aon ^| findstr :3118 2^>nul') do (
    taskkill /PID %%a /F >nul 2>&1
)

echo [2/3] Starting Next.js frontend server on port 3118...
start "MEETIFY Frontend (Next.js)" /min cmd /c "cd /d "%~dp0frontend" && pnpm dev"

echo Waiting for frontend to compile and initialize...
set attempts=0

:wait_loop
ping 127.0.0.1 -n 3 >nul
set /a attempts+=1

REM Check if Next.js actually responds with HTTP 200 (warm and compiled)
powershell -NoProfile -Command "try { $r = Invoke-WebRequest -Uri 'http://localhost:3118' -TimeoutSec 3 -UseBasicParsing -ErrorAction Stop; if ($r.StatusCode -eq 200) { exit 0 } else { exit 1 } } catch { exit 1 }" >nul 2>&1
if not errorlevel 1 goto launch_app

if %attempts% geq 40 (
    echo Frontend took longer than usual, launching app now...
    goto launch_app
)

echo Compiling frontend... please wait a few seconds (%attempts%/40)
goto wait_loop

:launch_app
echo [3/3] Frontend is fully compiled and ready!
echo Launching MEETIFY desktop app...

if exist "%~dp0bin\meetify.exe" (
    cd /d "%~dp0bin"
    start "" "%~dp0bin\meetify.exe"
) else if exist "%~dp0target\debug\meetify.exe" (
    cd /d "%~dp0target\debug"
    start "" "%~dp0target\debug\meetify.exe"
) else (
    echo Binary not found in bin\ or target\debug\.
)

echo.
echo ===================================================
echo           MEETIFY is now running!
echo ===================================================
echo.
echo - Web frontend: http://localhost:3118
echo - Desktop window: Active
echo.
echo Keep this window open while using MEETIFY.
echo To safely close MEETIFY, press any key below.
echo ===================================================
echo.
pause >nul

echo Stopping MEETIFY and frontend server...
taskkill /IM meetify.exe /F >nul 2>&1
for /f "tokens=5" %%a in ('netstat -aon ^| findstr :3118 2^>nul') do (
    taskkill /PID %%a /F >nul 2>&1
)
echo MEETIFY stopped cleanly. Goodbye!
ping 127.0.0.1 -n 2 >nul
exit
