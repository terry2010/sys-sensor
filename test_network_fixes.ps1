# 网络监控修复验证脚本
# 测试活动连接数和网络速度显示准确性

Write-Host "=== 网络监控修复验证测试 ===" -ForegroundColor Green

# 1. 测试PowerShell活动连接数查询
Write-Host "`n1. 测试活动连接数查询:" -ForegroundColor Yellow
try {
    $connections = (Get-NetTCPConnection -State Established).Count
    Write-Host "当前活动TCP连接数: $connections" -ForegroundColor Cyan
} catch {
    Write-Host "PowerShell查询失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 2. 启动sys-sensor并监控日志
Write-Host "`n2. 启动sys-sensor进行实时监控..." -ForegroundColor Yellow
Write-Host "请观察以下指标:" -ForegroundColor White
Write-Host "- 活动连接数是否正确显示（不再为0）" -ForegroundColor White
Write-Host "- 网络速度是否快速响应（EMA alpha=0.3）" -ForegroundColor White
Write-Host "- 日志是否包含详细的时间戳" -ForegroundColor White

# 3. 建议测试步骤
Write-Host "`n3. 建议测试步骤:" -ForegroundColor Yellow
Write-Host "a) 启动一个大文件下载（如40M+文件）" -ForegroundColor White
Write-Host "b) 观察网络下行速度是否接近实际传输速度" -ForegroundColor White
Write-Host "c) 检查活动连接数是否显示正确数值" -ForegroundColor White
Write-Host "d) 查看控制台日志的时间戳格式" -ForegroundColor White

Write-Host "`n按任意键启动sys-sensor..." -ForegroundColor Green
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

# 启动应用
Set-Location "C:\code\sys-sensor"
npm run tauri dev
