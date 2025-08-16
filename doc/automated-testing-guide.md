# sys-sensor 自动化测试指南

本文档详细介绍 sys-sensor 项目的自动化测试系统，包括测试架构、使用方法、测试点说明和故障排除。

---

## 1. 测试系统概述

### 1.1 测试架构

sys-sensor 自动化测试系统采用分层测试架构：

```
┌─────────────────────────────────────────┐
│           便携版测试脚本                │
│        test-portable.ps1               │
└─────────────────┬───────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
┌───────▼────────┐ ┌────────▼────────┐
│  C# 桥接层测试  │ │  Tauri后端测试  │
│  TestRunner.cs  │ │ test_runner.rs  │
└────────────────┘ └─────────────────┘
        │                   │
┌───────▼────────┐ ┌────────▼────────┐
│   硬件传感器    │ │   系统监控      │
│   数据采集      │ │   数据整合      │
└────────────────┘ └─────────────────┘
```

### 1.2 测试分层

- **便携版测试**：无需开发环境，可在客户机器上运行
- **C# 桥接层测试**：测试硬件传感器数据采集功能
- **Tauri 后端测试**：测试系统监控和数据整合功能
- **集成测试**：测试前后端数据流转和序列化

---

## 2. 快速开始

### 2.1 开发环境测试

在有完整开发环境的机器上：

```powershell
# 运行所有测试
.\test-portable.ps1

# 仅运行 C# 桥接层测试
.\test-portable.ps1 -BridgeOnly

# 仅运行 Tauri 后端测试
.\test-portable.ps1 -TauriOnly

# 跳过编译，直接运行测试
.\test-portable.ps1 -SkipBuild

# 详细输出模式
.\test-portable.ps1 -Verbose
```

### 2.2 便携版测试

在客户机器上（无开发环境）：

1. 确保已编译好的程序文件存在
2. 以管理员权限运行 PowerShell
3. 执行测试脚本：

```powershell
# 基本测试
.\test-portable.ps1

# 指定输出目录
.\test-portable.ps1 -OutputDir "C:\TestReports"
```

### 2.3 前端集成测试

在 Tauri 应用中通过前端调用：

```javascript
// 运行 Tauri 后端测试
const tauriResult = await invoke('run_tauri_tests');

// 运行 C# 桥接层测试
const bridgeResult = await invoke('run_bridge_tests');
```

---

## 3. 测试点详细说明

### 3.1 C# 桥接层测试点

| 测试类别 | 测试项目 | 测试内容 | 期望结果 |
|---------|---------|---------|---------|
| **核心功能** | 硬件管理器初始化 | LibreHardwareMonitor 初始化 | 成功初始化，检测到硬件组件 |
| | 数据采集器 | 传感器数据采集 | 成功采集 CPU/GPU/内存等数据 |
| | 传感器监控器 | 多次采样测试 | 连续3次采样成功 |
| | 配置管理器 | 配置文件读写 | 配置加载和保存正常 |
| **硬件传感器** | CPU传感器 | 温度/负载/风扇 | 至少获取温度或负载数据 |
| | GPU传感器 | 显卡信息采集 | 检测GPU数量和基本信息 |
| | 内存传感器 | 内存使用情况 | 获取内存使用量和总量 |
| | 存储传感器 | 存储设备监控 | 基础存储监控功能 |
| | 网络传感器 | 网络接口信息 | 网络监控基础功能 |
| | 电池传感器 | 电池状态检测 | 检测电池存在性（笔记本） |
| | 温度传感器 | 综合温度监控 | 汇总所有温度传感器 |
| **集成测试** | 数据输出格式 | JSON序列化测试 | 数据正确序列化为JSON |
| **可靠性** | 错误处理 | 异常情况处理 | 正确捕获和处理错误 |

### 3.2 Tauri 后端测试点

| 测试类别 | 测试项目 | 测试内容 | 期望结果 |
|---------|---------|---------|---------|
| **核心功能** | 配置系统 | 配置序列化/反序列化 | 配置数据正确处理 |
| | 桥接管理器 | C#进程启动和通信 | 桥接进程正常启动和数据交换 |
| | 系统信息采集 | 基础系统信息 | 获取CPU核心数、内存、系统名称 |
| **硬件监控** | 网络监控 | 网络接口和统计 | 枚举网络接口，获取流量统计 |
| | 磁盘监控 | 磁盘使用和逻辑磁盘 | 获取磁盘读写速率和分区信息 |
| | 温度监控 | WMI温度查询 | 查询系统温度传感器 |
| | 电池监控 | 电池信息获取 | 检测电池状态（如果存在） |
| | GPU监控 | GPU信息采集 | 检测GPU设备数量 |
| | SMART监控 | 硬盘健康状态 | 获取SMART健康数据 |
| **系统监控** | 进程监控 | Top CPU进程 | 获取CPU占用最高的进程 |
| **网络功能** | WiFi监控 | WiFi连接信息 | 获取WiFi连接状态和信号强度 |
| | 公网信息 | 公网IP获取 | 获取公网IP地址（网络可用时） |
| **集成测试** | 数据整合 | 完整数据快照 | 创建完整的SensorSnapshot |
| **性能测试** | 性能基准 | 数据采集性能 | 10次采集的平均耗时 |
| **可靠性** | 错误处理 | 异常处理机制 | 正确处理各种异常情况 |

---

## 4. 测试报告说明

### 4.1 报告文件结构

测试完成后会生成以下报告文件：

```
test-reports/
├── bridge-test-report-YYYY-MM-DD-HH-mm-ss.json  # C#桥接层详细报告
├── tauri-test-report-YYYY-MM-DD-HH-mm-ss.json   # Tauri后端详细报告
└── test-summary-YYYY-MM-DD-HH-mm-ss.json        # 综合测试摘要
```

### 4.2 报告内容解读

#### C# 桥接层报告 (bridge-test-report-*.json)

```json
{
  "testStartTime": "2025-01-17T10:30:00",
  "testEndTime": "2025-01-17T10:32:15",
  "duration": "00:02:15",
  "totalTests": 12,
  "passedTests": 11,
  "failedTests": 1,
  "successRate": 91.7,
  "isAdministrator": true,
  "testResults": [
    {
      "testName": "CPU传感器",
      "category": "Hardware",
      "success": true,
      "duration": "00:00:01.234",
      "details": "CPU温度: 65°C, CPU负载: 25%, CPU风扇: 1200RPM",
      "metrics": {
        "cpu_temp_available": 1,
        "cpu_load_available": 1,
        "cpu_fan_available": 1
      }
    }
  ],
  "environment": {
    "os_version": "Microsoft Windows NT 10.0.19045.0",
    "machine_name": "DESKTOP-ABC123",
    "processor_count": 8
  }
}
```

#### Tauri 后端报告 (tauri-test-report-*.json)

```json
{
  "test_start_time": "2025-01-17T10:32:20",
  "test_end_time": "2025-01-17T10:34:45",
  "duration": 145000000000,  // 纳秒
  "total_tests": 15,
  "passed_tests": 14,
  "failed_tests": 1,
  "success_rate": 93.3,
  "is_administrator": true,
  "test_results": [
    {
      "test_name": "网络监控",
      "category": "Hardware",
      "success": true,
      "duration": 2500000000,  // 纳秒
      "details": "网络接口数量: 3, 统计信息可用: 是",
      "metrics": {}
    }
  ]
}
```

#### 综合测试摘要 (test-summary-*.json)

```json
{
  "TestStartTime": "2025-01-17 10:30:00",
  "TestEndTime": "2025-01-17 10:35:00",
  "Duration": "00:05:00",
  "IsAdministrator": true,
  "BridgeTestResult": "成功",
  "TauriTestResult": "成功",
  "OverallSuccess": true,
  "Environment": {
    "OS": "Microsoft Windows NT 10.0.19045.0",
    "MachineName": "DESKTOP-ABC123",
    "UserName": "Administrator",
    "PowerShellVersion": "5.1.19041.4648"
  }
}
```

---

## 5. 常见问题与故障排除

### 5.1 权限相关问题

**问题**: 温度传感器数据显示为"不可用"
**原因**: 非管理员权限下无法访问硬件传感器
**解决**: 以管理员权限运行测试脚本

```powershell
# 检查当前权限
([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

# 以管理员权限启动PowerShell
Start-Process powershell -Verb RunAs
```

### 5.2 编译问题

**问题**: C# 编译失败
**原因**: 缺少 .NET SDK 或项目依赖
**解决**: 
1. 安装 .NET 6.0 或更高版本 SDK
2. 恢复项目依赖：`dotnet restore`
3. 检查项目文件完整性

**问题**: Rust 编译失败
**原因**: 缺少 Rust 工具链或依赖
**解决**:
1. 安装 Rust: https://rustup.rs/
2. 更新工具链：`rustup update`
3. 清理并重新编译：`cargo clean && cargo build`

### 5.3 运行时问题

**问题**: 桥接进程启动失败
**原因**: 可执行文件不存在或权限不足
**解决**:
1. 确认编译成功：检查 `sensor-bridge/bin/Release/` 目录
2. 检查文件权限
3. 查看错误日志：`sensor-bridge/bridge.err.txt`

**问题**: 网络相关测试失败
**原因**: 网络连接问题或防火墙阻止
**解决**:
1. 检查网络连接
2. 临时关闭防火墙测试
3. 检查代理设置

### 5.4 测试结果异常

**问题**: 某些硬件传感器测试失败
**原因**: 硬件不支持或驱动问题
**解决**:
1. 更新硬件驱动
2. 检查硬件兼容性
3. 查看具体错误信息

**问题**: 性能测试耗时过长
**原因**: 系统负载高或硬件性能限制
**解决**:
1. 关闭其他应用程序
2. 等待系统负载降低后重试
3. 检查硬盘空间和内存使用

---

## 6. 开发者指南

### 6.1 添加新的测试项目

#### 在 C# 桥接层添加测试

1. 在 `TestRunner.cs` 中添加新的测试方法：

```csharp
private async Task TestNewFeature()
{
    var test = new TestResult { 
        TestName = "新功能测试", 
        Category = "Feature" 
    };
    
    var start = Instant.now();
    
    try
    {
        // 测试逻辑
        var result = await SomeNewFeature();
        test.Success = result != null;
        test.Details = $"测试结果: {result}";
    }
    catch (Exception ex)
    {
        test.Success = false;
        test.ErrorMessage = ex.Message;
    }
    
    test.Duration = start.elapsed();
    _testResults.Add(test);
}
```

2. 在 `RunAllTestsAsync()` 方法中调用新测试：

```csharp
await TestNewFeature();
```

#### 在 Tauri 后端添加测试

1. 在 `test_runner.rs` 中添加新的测试方法：

```rust
async fn test_new_feature(&mut self) {
    let mut test = TestResult {
        test_name: "新功能测试".to_string(),
        category: "Feature".to_string(),
        success: false,
        duration: Duration::new(0, 0),
        details: String::new(),
        error_message: None,
        metrics: HashMap::new(),
        timestamp: Local::now(),
    };

    let start = Instant::now();
    
    match self.run_new_feature_test().await {
        Ok(details) => {
            test.success = true;
            test.details = details;
        }
        Err(e) => {
            test.error_message = Some(e.to_string());
        }
    }

    test.duration = start.elapsed();
    self.test_results.push(test);
}
```

2. 在 `run_all_tests()` 方法中调用：

```rust
self.test_new_feature().await;
```

### 6.2 自定义测试配置

可以通过环境变量自定义测试行为：

```powershell
# 设置测试超时时间（秒）
$env:SYS_SENSOR_TEST_TIMEOUT = "30"

# 启用详细日志
$env:SYS_SENSOR_TEST_VERBOSE = "1"

# 跳过网络相关测试
$env:SYS_SENSOR_SKIP_NETWORK_TESTS = "1"
```

### 6.3 CI/CD 集成

在持续集成环境中使用：

```yaml
# GitHub Actions 示例
- name: Run Automated Tests
  run: |
    powershell -ExecutionPolicy Bypass -File test-portable.ps1 -Verbose
  env:
    SYS_SENSOR_TEST_MODE: "ci"
```

---

## 7. 测试最佳实践

### 7.1 测试环境准备

1. **管理员权限**: 确保以管理员权限运行，获取完整硬件信息
2. **网络连接**: 确保网络连接正常，用于公网IP等测试
3. **系统负载**: 测试期间避免高负载操作
4. **杀毒软件**: 临时关闭可能干扰的安全软件

### 7.2 测试执行建议

1. **定期测试**: 建议每次代码变更后运行完整测试
2. **分层测试**: 开发时可分别运行桥接层和后端测试
3. **性能基准**: 记录性能基准，监控性能回归
4. **报告保存**: 保存测试报告用于问题追踪和性能分析

### 7.3 故障诊断流程

1. **查看综合报告**: 先查看 `test-summary-*.json` 了解整体情况
2. **分析详细报告**: 查看具体失败的测试项目和错误信息
3. **检查环境**: 确认系统环境、权限、网络等基础条件
4. **单独测试**: 针对失败项目进行单独测试和调试
5. **日志分析**: 查看应用程序日志和系统事件日志

---

## 8. 附录

### 8.1 测试命令参考

```powershell
# 便携版测试脚本参数
.\test-portable.ps1 [参数]

参数说明:
-BridgeOnly      # 仅运行C#桥接层测试
-TauriOnly       # 仅运行Tauri后端测试  
-SkipBuild       # 跳过编译步骤
-OutputDir       # 指定报告输出目录
-Verbose         # 详细输出模式
```

### 8.2 环境变量参考

```powershell
# 测试相关环境变量
$env:SYS_SENSOR_TEST_MODE = "1"           # 启用测试模式
$env:SYS_SENSOR_TEST_TIMEOUT = "30"       # 测试超时时间（秒）
$env:SYS_SENSOR_TEST_VERBOSE = "1"        # 详细日志输出
$env:SYS_SENSOR_SKIP_NETWORK_TESTS = "1"  # 跳过网络测试
$env:BRIDGE_TEST_MODE = "1"               # C#桥接层测试模式
```

### 8.3 相关文件路径

```
项目根目录/
├── test-portable.ps1                     # 便携版测试脚本
├── sensor-bridge/
│   ├── TestRunner.cs                     # C#测试运行器
│   ├── TestProgram.cs                    # C#测试程序入口
│   └── bin/Release/win-x64/publish/      # 编译输出目录
├── src-tauri/src/
│   ├── test_runner.rs                    # Rust测试运行器
│   └── config_utils.rs                   # 测试命令定义
├── doc/
│   └── automated-testing-guide.md        # 本文档
└── test-reports/                         # 测试报告输出目录
```

---

**文档版本**: 1.0  
**最后更新**: 2025-01-17  
**适用版本**: sys-sensor v1.0+
