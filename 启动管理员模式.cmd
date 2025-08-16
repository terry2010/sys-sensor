@echo off
chcp 65001 >nul
echo 正在以管理员权限启动 Sys-Sensor...
cd /d "%~dp0"
powershell -Command "Start-Process cmd -ArgumentList '/c chcp 65001 && cd /d \"%~dp0\" && npm run dev:all && pause' -Verb RunAs"
pause
