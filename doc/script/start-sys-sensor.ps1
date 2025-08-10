# 创建日志目录
$logDir = Join-Path $PSScriptRoot "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

# 配置桥接日志与诊断级别（按需调整）
$env:BRIDGE_LOG_FILE = (Join-Path $logDir "bridge.log")                 # 桥接日志文件
$env:BRIDGE_SUMMARY_EVERY_TICKS = "60"                                   # 每 60 次采样输出一条汇总日志（约 1 分钟）
$env:BRIDGE_DUMP_EVERY_TICKS    = "0"                                    # 传感器树全量 dump（0=关闭，建议现场先关）
$env:BRIDGE_SELFHEAL_IDLE_SEC   = "300"                                  # 300s 内无有效读数则自愈
$env:BRIDGE_SELFHEAL_EXC_MAX    = "5"                                    # 连续 5 次异常则自愈
$env:BRIDGE_PERIODIC_REOPEN_SEC = "0"                                    # 周期重建（0=关闭，可按需设 1800）

# 以管理员运行主程序（推荐）
Start-Process -Verb RunAs -FilePath (Join-Path $PSScriptRoot "sys-sensor.exe")