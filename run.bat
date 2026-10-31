@echo off
taskkill /F /IM mouseblur.exe >nul 2>&1
if exist "target\release\mouseblur.exe" (
    start "" "%~dp0target\release\mouseblur.exe" %*
) else (
    cargo run --release -- %*
)
