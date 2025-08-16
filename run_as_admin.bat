@echo off
echo 正在以管理员权限启动 Sys-Sensor...
cd /d "C:\code\sys-sensor"
powershell -Command "Start-Process cmd -ArgumentList '/c cd /d C:\code\sys-sensor && npm run dev:all && pause' -Verb RunAs"
