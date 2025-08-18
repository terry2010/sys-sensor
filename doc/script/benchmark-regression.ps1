param(
  [int]$DurationSec = 60,
  [string]$OutDir = "./doc/script/out"
)

$ErrorActionPreference = "Stop"
if (!(Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }

# 启动 dev:all（若已在运行则跳过）
Write-Host "[bench] 构建前端与后端..."
cmd /c "npm run -s dev:all" | Out-Null
Start-Sleep -Seconds 3

# 记录进程内存与CPU：每秒一次
$procName = "sys-sensor"
$csv = Join-Path $OutDir ("bench-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".csv")
"ts,cpuPercent,workingSetMB,privateMB" | Out-File -FilePath $csv -Encoding UTF8

$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $DurationSec) {
  $p = Get-Process | Where-Object { $_.ProcessName -eq $procName }
  if ($p) {
    $cpu = [Math]::Round($p.CPU, 3)
    $ws  = [Math]::Round($p.WorkingSet64 / 1MB, 2)
    $pm  = [Math]::Round($p.PrivateMemorySize64 / 1MB, 2)
    $line = "{0},{1},{2},{3}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"), $cpu, $ws, $pm
    Add-Content -Path $csv -Value $line
  }
  Start-Sleep -Milliseconds 1000
}
$sw.Stop()

Write-Host "[bench] 完成：$csv"
