# sys-sensor Portable Automated Testing Script
# For running comprehensive automated tests on client machines without development environment

param(
    [switch]$BridgeOnly,      # Run C# bridge layer tests only
    [switch]$TauriOnly,       # Run Tauri backend tests only
    [switch]$SkipBuild,       # Skip compilation steps
    [string]$OutputDir = "test-reports",  # Test report output directory
    [switch]$Verbose          # Verbose output
)

$ErrorActionPreference = "Stop"

Write-Host "=== sys-sensor Portable Automated Testing ===" -ForegroundColor Green
Write-Host "Start Time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray
Write-Host ""

# Create test report directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    Write-Host "Created test report directory: $OutputDir" -ForegroundColor Yellow
}

# Check administrator privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "Administrator privileges: $(if($isAdmin){'Yes'}else{'No'})" -ForegroundColor $(if($isAdmin){'Green'}else{'Yellow'})

if (!$isAdmin) {
    Write-Host "Warning: Non-administrator privileges may cause some hardware sensor data to be unavailable" -ForegroundColor Yellow
}

Write-Host ""

# Test results summary
$testResults = @{
    BridgeTests = $null
    TauriTests = $null
    StartTime = Get-Date
    EndTime = $null
    Success = $false
}

try {
    # 1. C# Bridge Layer Tests
    if (!$TauriOnly) {
        Write-Host "=== 1. C# Bridge Layer Tests ===" -ForegroundColor Cyan
        
        $bridgeDir = "sensor-bridge"
        
        # Check if test files exist
        $testRunnerFile = Join-Path $bridgeDir "TestRunner.cs"
        $testProgramFile = Join-Path $bridgeDir "TestProgram.cs"
        
        if (!(Test-Path $testRunnerFile) -or !(Test-Path $testProgramFile)) {
            Write-Host "Error: C# test program source code not found" -ForegroundColor Red
            throw "C# test program does not exist"
        }
        
        # Compile test program (if needed)
        if (!$SkipBuild) {
            Write-Host "Compiling C# test program..." -ForegroundColor Yellow
            
            Push-Location $bridgeDir
            try {
                # Use existing project file
                $buildResult = dotnet build sensor-bridge.csproj -c Release 2>&1
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "C# compilation failed:" -ForegroundColor Red
                    Write-Host $buildResult -ForegroundColor Red
                    throw "C# compilation failed"
                }
                Write-Host "C# compilation completed" -ForegroundColor Green
            }
            finally {
                Pop-Location
            }
        }
        
        # Run C# tests
        Write-Host "Running C# bridge layer tests..." -ForegroundColor Yellow
        
        Push-Location $bridgeDir
        try {
            # Run test program using existing project
            $testOutput = dotnet run --project sensor-bridge.csproj --configuration Release -- --output-dir "../$OutputDir" 2>&1
            $bridgeExitCode = $LASTEXITCODE
            
            if ($bridgeExitCode -eq 0) {
                Write-Host "C# bridge layer tests completed" -ForegroundColor Green
                $testResults.BridgeTests = "Success"
            } else {
                Write-Host "C# bridge layer tests failed" -ForegroundColor Red
                if ($Verbose) {
                    Write-Host "Output: $testOutput" -ForegroundColor Gray
                }
                $testResults.BridgeTests = "Failed"
            }
            
            # Copy test reports
            $reportFiles = Get-ChildItem -Path . -Name "bridge-test-report-*.json" -ErrorAction SilentlyContinue | Sort-Object Name -Descending
            if ($reportFiles) {
                $latestReport = $reportFiles[0]
                $targetDir = Join-Path $PSScriptRoot $OutputDir
                if (-not (Test-Path $targetDir)) {
                    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
                }
                Copy-Item $latestReport $targetDir -Force
                Write-Host "C# test report copied: $latestReport" -ForegroundColor Green
            }
        }
        finally {
            Pop-Location
        }
    }
    
    # 2. Tauri Backend Tests
    if (!$BridgeOnly) {
        Write-Host ""
        Write-Host "=== 2. Tauri Backend Tests ===" -ForegroundColor Cyan
        
        # Check Tauri project
        $tauriDir = "src-tauri"
        $cargoToml = Join-Path $tauriDir "Cargo.toml"
        $testRunnerFile = Join-Path $tauriDir "src\test_runner.rs"
        
        if (!(Test-Path $cargoToml) -or !(Test-Path $testRunnerFile)) {
            Write-Host "Error: Tauri project or test files not found" -ForegroundColor Red
            throw "Tauri project does not exist"
        }
        
        # Compile Tauri test runner (if needed)
        if (!$SkipBuild) {
            Write-Host "Compiling Tauri test runner..." -ForegroundColor Yellow
            
            Push-Location $tauriDir
            try {
                # 直接运行cargo build并捕获输出，忽略警告
                Write-Host "Building Tauri test runner..." -ForegroundColor Gray
                cargo build --bin test_runner --release
                
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "Tauri test runner compilation failed with exit code: $LASTEXITCODE" -ForegroundColor Red
                    throw "Tauri test runner compilation failed"
                }
                
                Write-Host "Tauri test runner compilation completed" -ForegroundColor Green
            }
            finally {
                Pop-Location
            }
        }
        
        # 运行 Tauri 测试
        Write-Host "Running Tauri backend tests..." -ForegroundColor Yellow
        
        Push-Location $tauriDir
        try {
            # 运行测试并直接输出，不捕获stderr
            Write-Host "Running test runner..." -ForegroundColor Gray
            cargo run --bin test_runner --release
            $tauriExitCode = $LASTEXITCODE
            
            if ($tauriExitCode -eq 0) {
                Write-Host "Tauri backend tests completed" -ForegroundColor Green
                $testResults.TauriTests = "Success"
            } else {
                Write-Host "Tauri backend tests failed with exit code: $tauriExitCode" -ForegroundColor Red
                $testResults.TauriTests = "Failed"
            }
            
            # Copy test reports
            $reportFiles = Get-ChildItem -Path . -Name "tauri-test-report-*.json" -ErrorAction SilentlyContinue | Sort-Object Name -Descending
            if ($reportFiles) {
                $latestReport = $reportFiles[0]
                $targetDir = Join-Path $PSScriptRoot $OutputDir
                if (-not (Test-Path $targetDir)) {
                    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
                }
                Copy-Item $latestReport $targetDir -Force
                Write-Host "Tauri test report copied: $latestReport" -ForegroundColor Green
            }
        }
        finally {
            Pop-Location
        }
    }
    
    # Generate comprehensive test report
    $testResults.EndTime = Get-Date
    $testResults.Success = ($testResults.BridgeTests -ne "Failed") -and ($testResults.TauriTests -ne "Failed")
    
    $summaryReport = @{
        TestStartTime = $testResults.StartTime.ToString("yyyy-MM-dd HH:mm:ss")
        TestEndTime = $testResults.EndTime.ToString("yyyy-MM-dd HH:mm:ss")
        Duration = ($testResults.EndTime - $testResults.StartTime).ToString()
        IsAdministrator = $isAdmin
        BridgeTestResult = $testResults.BridgeTests
        TauriTestResult = $testResults.TauriTests
        OverallSuccess = $testResults.Success
        Environment = @{
            OS = "$([System.Environment]::OSVersion)"
            MachineName = $env:COMPUTERNAME
            UserName = $env:USERNAME
            PowerShellVersion = $PSVersionTable.PSVersion.ToString()
        }
    } | ConvertTo-Json -Depth 3
    
    $summaryPath = Join-Path $OutputDir "test-summary-$(Get-Date -Format 'yyyy-MM-dd-HH-mm-ss').json"
    $summaryReport | Out-File -FilePath $summaryPath -Encoding UTF8
    
    Write-Host ""
    Write-Host "=== Testing Completed ===" -ForegroundColor Green
    Write-Host "Total Duration: $(($testResults.EndTime - $testResults.StartTime).ToString())" -ForegroundColor Gray
    Write-Host "C# Bridge Layer: $($testResults.BridgeTests)" -ForegroundColor $(if($testResults.BridgeTests -eq "Success"){'Green'}else{'Red'})
    Write-Host "Tauri Backend: $($testResults.TauriTests)" -ForegroundColor $(if($testResults.TauriTests -eq "Success"){'Green'}else{'Red'})
    Write-Host "Overall Result: $(if($testResults.Success){'Success'}else{'Failed'})" -ForegroundColor $(if($testResults.Success){'Green'}else{'Red'})
    Write-Host "Test Report Directory: $OutputDir" -ForegroundColor Yellow
    
    if ($testResults.Success) {
        Write-Host ""
        Write-Host "✓ All tests passed! System is running normally." -ForegroundColor Green
        exit 0
    } else {
        Write-Host ""
        Write-Host "✗ Some tests failed, please check detailed reports." -ForegroundColor Red
        exit 1
    }
}
catch {
    Write-Host ""
    Write-Host "Test execution error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Stack trace: $($_.ScriptStackTrace)" -ForegroundColor Gray
    exit -1
}
