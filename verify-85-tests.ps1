# 验证85个指标测试完整性脚本
Write-Host "=== 验证85个指标测试完整性 ===" -ForegroundColor Green

# 编译项目
Write-Host "1. 编译测试项目..." -ForegroundColor Yellow
cd "C:\code\sys-sensor\sensor-bridge"
$compileResult = dotnet build 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 编译成功" -ForegroundColor Green
} else {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    Write-Host $compileResult
    exit 1
}

# 运行主测试
Write-Host "2. 运行85个指标测试..." -ForegroundColor Yellow
$testOutput = dotnet run --project sensor-bridge.csproj -- --test-main 2>&1
Write-Host "测试输出预览:" -ForegroundColor Cyan
Write-Host ($testOutput | Select-Object -First 10 | Out-String)

# 检查测试报告
Write-Host "3. 检查测试报告..." -ForegroundColor Yellow
$reportFiles = Get-ChildItem -Path "C:\code\sys-sensor\test-reports" -Filter "*main-test*" | Sort-Object LastWriteTime -Descending
if ($reportFiles.Count -gt 0) {
    $latestReport = $reportFiles[0]
    Write-Host "✅ 找到主测试报告: $($latestReport.Name)" -ForegroundColor Green
    
    # 读取并分析报告
    $reportContent = Get-Content $latestReport.FullName -Raw | ConvertFrom-Json
    Write-Host "📊 测试统计:" -ForegroundColor Cyan
    Write-Host "  总测试数: $($reportContent.totalTests)" -ForegroundColor White
    Write-Host "  通过数: $($reportContent.passedTests)" -ForegroundColor Green
    Write-Host "  失败数: $($reportContent.failedTests)" -ForegroundColor Red
    Write-Host "  成功率: $($reportContent.successRate)%" -ForegroundColor Yellow
    
    if ($reportContent.totalTests -ge 85) {
        Write-Host "✅ 测试覆盖85个指标完成" -ForegroundColor Green
    } else {
        Write-Host "⚠️ 测试数量不足85个: $($reportContent.totalTests)" -ForegroundColor Yellow
    }
} else {
    Write-Host "⚠️ 未找到主测试报告，检查所有报告..." -ForegroundColor Yellow
    $allReports = Get-ChildItem -Path "C:\code\sys-sensor\test-reports" | Sort-Object LastWriteTime -Descending | Select-Object -First 3
    foreach ($report in $allReports) {
        Write-Host "  - $($report.Name) ($(Get-Date $report.LastWriteTime -Format 'yyyy-MM-dd HH:mm:ss'))" -ForegroundColor Gray
    }
}

# 验证测试类文件
Write-Host "4. 验证测试类文件..." -ForegroundColor Yellow
$testFiles = @(
    "Tests\CpuTests.cs",
    "Tests\MemoryTests.cs", 
    "Tests\NetworkTests.cs",
    "Tests\StorageTests.cs",
    "Tests\GpuTests.cs",
    "Tests\SystemTests.cs",
    "Tests\MainTestRunner.cs",
    "Tests\TestDataCollector.cs",
    "Tests\TestDataSnapshot.cs"
)

$missingFiles = @()
foreach ($file in $testFiles) {
    $fullPath = Join-Path "C:\code\sys-sensor\sensor-bridge" $file
    if (Test-Path $fullPath) {
        Write-Host "✅ $file" -ForegroundColor Green
    } else {
        Write-Host "❌ $file (缺失)" -ForegroundColor Red
        $missingFiles += $file
    }
}

if ($missingFiles.Count -eq 0) {
    Write-Host "✅ 所有测试文件完整" -ForegroundColor Green
} else {
    Write-Host "❌ 缺失 $($missingFiles.Count) 个测试文件" -ForegroundColor Red
}

Write-Host "=== Verification Complete ===" -ForegroundColor Green
