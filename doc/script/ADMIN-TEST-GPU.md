# 管理员 PowerShell 测试说明（GPU VRAM/PWR）

本说明用于在管理员 PowerShell 环境下对 GPU 显存占用与功耗指标进行端到端验证（C# bridge → Rust → 前端 Vue3/Tauri）。

## 前置条件
- Windows 10/11，管理员 PowerShell。
- 已安装 .NET SDK、Rust、Node.js（`npm run setup` 可检查环境）。
- 源码目录：`C:\code\sys-sensor`。

## 快速开始
1) 释放残留进程与端口（可选，避免 1422 被占用）
```
$names = @("sys-sensor.exe","sensor-bridge.exe","dotnet.exe"); foreach($n in $names){ try { taskkill /IM $n /T /F } catch {} }
Get-NetTCPConnection -State Listen -LocalPort 1422 -ErrorAction SilentlyContinue | ForEach-Object { try { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue } catch {} }
```
2) 一体化联调启动（发布 bridge + 启动 tauri dev + Vite 1422）
```
cd C:\code\sys-sensor
npm run dev:all
```
3) 在弹出的 Tauri 窗口中打开“详情”页（或默认即为详情页），观察 GPU 行展示。

> 注意：直接用浏览器访问 `http://localhost:1422` 仅作 UI 预览，`@tauri-apps/api` 在非 Tauri 环境下不可用，事件订阅会有 warn；真实数据必须在 Tauri 窗口验证。

## 观测要点（GPU 行）
- 展示格式示意：`<GPUName> <TempC>°C <Load>% <Clock>Mhz [Fan <RPM>] VRAM <n> MB PWR <m> W`
- VRAM（MB）与功耗（W）均为可选字段：
  - 若硬件/驱动不支持或无数据，显示“—”。
  - 数值格式：VRAM 取整到 0 位；PWR 保留 1 位小数。
- 多 GPU：最多显示两块，超出部分以“+N”汇总。

## 测试用例与期望
- [单 GPU] 能显示基本字段（温度/负载/频率/风扇）且出现 `VRAM <n> MB` 与 `PWR <m> W`；无对应数据显示“—”。
- [多 GPU] 显示前两块的详情，末尾“+N”统计剩余数量。
- [无功耗传感器] 仅 VRAM 有值；PWR 显示“—”。
- [无显存传感器] 仅 PWR 有值；VRAM 显示“—”。
- [权限不足] 管理员运行可提升可读性；若非管理员，某些传感器可能缺失，显示“—”。
- [稳定性] 连续运行 ≥10 分钟，`sensor://snapshot` 周期广播稳定，无异常退出；CPU/内存/网速等指标正常刷新。

## 日志与诊断
- Bridge 诊断（输出到 `dist-portable/sys-sensor/bridge.log` 或 `src-tauri/resources/sensor-bridge/` 下）：
```
npm run diagnose:bridge
```
- 观察 tauri dev 控制台日志中是否持续输出 `[emit] sensor://snapshot ...`。
- 可设置环境变量增强桥接日志与自愈（示例）：
```
$env:BRIDGE_SUMMARY_EVERY_TICKS = "60"
$env:BRIDGE_DUMP_EVERY_TICKS    = "0"
$env:BRIDGE_LOG_FILE            = "C:\\code\\sys-sensor\\bridge.log"
$env:BRIDGE_SELFHEAL_IDLE_SEC   = "120"
$env:BRIDGE_SELFHEAL_EXC_MAX    = "5"
$env:BRIDGE_PERIODIC_REOPEN_SEC = "1800"
```

## 常见问题
- 端口占用（1422）：
  - 使用上文命令释放；或 `npm run clean:proc` 清理残留进程后重试。
- VRAM/PWR 一直为“—”：
  - 确认管理员运行；
  - 确认显卡与驱动能在 LibreHardwareMonitor 中看到对应传感器；
  - 查看 bridge 日志，确认已识别 `SensorType.Power` 和与 VRAM 相关的传感器名称（vram/memory + used/usage）。

## 退出与清理
```
$names = @("sys-sensor.exe","sensor-bridge.exe","dotnet.exe"); foreach($n in $names){ try { taskkill /IM $n /T /F } catch {} }
```
