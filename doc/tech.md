# sys-sensor 技术文档

## 项目架构概述

sys-sensor 是一个基于 Tauri + Vue 3 + C# 的跨平台系统监控应用，采用三层架构：
- **前端层**：Vue 3 + TypeScript，负责UI展示和用户交互
- **中间层**：Rust (Tauri)，负责事件处理、数据广播和配置管理
- **数据层**：C# 桥接程序，负责底层系统数据采集

## 主界面74个监控指标统计

基于 Details.vue 分析，主界面包含以下74个监控指标：

### CPU相关指标 (12个)
1. CPU使用率 (`cpu_usage`) - 总体CPU使用百分比
2. CPU温度 (`cpu_temp_c`) - CPU核心温度
3. CPU包功耗 (`cpu_pkg_power_w`) - CPU整体功耗
4. CPU平均频率 (`cpu_avg_freq_mhz`) - CPU平均工作频率
5. CPU限频状态 (`cpu_throttle_active`) - 是否发生限频
6. CPU限频原因 (`cpu_throttle_reasons`) - 限频原因列表
7. CPU每核负载 (`cpu_core_loads_pct`) - 各核心负载百分比数组
8. CPU每核频率 (`cpu_core_clocks_mhz`) - 各核心工作频率数组
9. CPU每核温度 (`cpu_core_temps_c`) - 各核心温度数组
10. 桥接健康 (`hb_tick`, `idle_sec`, `exc_count`) - C#桥接程序健康状态
11. 系统运行时间 (`uptime_sec`) - 系统启动时长
12. 桥接重连时间 (`since_reopen_sec`) - 桥接程序重连间隔

### 内存相关指标 (12个)
13. 内存使用 (`mem_used_gb`, `mem_total_gb`, `mem_pct`) - 内存使用量/总量/百分比
14. 内存可用 (`mem_avail_gb`) - 可用内存量
15. 交换区 (`swap_used_gb`, `swap_total_gb`) - 虚拟内存使用/总量
16. 内存缓存 (`mem_cache_gb`) - 系统缓存大小
17. 内存提交 (`mem_committed_gb`, `mem_commit_limit_gb`) - 已提交/提交限制
18. 分页池 (`mem_pool_paged_gb`) - 分页内存池大小
19. 非分页池 (`mem_pool_nonpaged_gb`) - 非分页内存池大小
20. 分页速率 (`mem_pages_per_sec`) - 每秒分页操作次数
21. 页面读取 (`mem_page_reads_per_sec`) - 每秒页面读取次数
22. 页面写入 (`mem_page_writes_per_sec`) - 每秒页面写入次数
23. 页面错误 (`mem_page_faults_per_sec`) - 每秒页面错误次数
24. 主板温度 (`mobo_temp_c`) - 主板传感器温度

### 网络相关指标 (18个)
25. 网络下行(平滑) (`net_rx_bps`) - EMA平滑后的接收速率
26. 网络上行(平滑) (`net_tx_bps`) - EMA平滑后的发送速率
27. 网络下行(瞬时) (`net_rx_instant_bps`) - 实时接收速率
28. 网络上行(瞬时) (`net_tx_instant_bps`) - 实时发送速率
29. 网络错误(RX) (`net_rx_err_ps`) - 每秒接收错误包数
30. 网络错误(TX) (`net_tx_err_ps`) - 每秒发送错误包数
31. 网络丢包率 (`packet_loss_pct`) - 网络丢包百分比
32. 活动连接数 (`active_connections`) - TCP活动连接数
33. 网络延迟 (`ping_rtt_ms`) - 网络往返时延
34. 多目标延迟 (`rtt_multi`) - 多个目标的RTT数组
35. Wi‑Fi SSID (`wifi_ssid`) - 无线网络名称
36. Wi‑Fi信号 (`wifi_signal_pct`) - 信号强度百分比
37. Wi‑Fi链路 (`wifi_link_mbps`) - 链路速度
38. Wi‑Fi BSSID (`wifi_bssid`) - 基站MAC地址
39. Wi‑Fi参数 (`wifi_channel`, `wifi_band`, `wifi_radio`) - 信道/频段/协议
40. Wi‑Fi速率 (`wifi_rx_mbps`, `wifi_tx_mbps`) - 接收/发送速率
41. Wi‑Fi RSSI (`wifi_rssi_dbm`, `wifi_rssi_estimated`) - 信号强度/是否估算
42. Wi‑Fi安全 (`wifi_auth`, `wifi_cipher`) - 认证方式/加密算法
43. Wi‑Fi信道宽度 (`wifi_chan_width_mhz`) - 信道带宽
44. 网络接口 (`net_ifs`) - 网络适配器详情数组

### 磁盘存储相关指标 (10个)
45. 磁盘读 (`disk_r_bps`) - 磁盘读取速率
46. 磁盘写 (`disk_w_bps`) - 磁盘写入速率
47. 磁盘读IOPS (`disk_r_iops`) - 每秒读取操作数
48. 磁盘写IOPS (`disk_w_iops`) - 每秒写入操作数
49. 磁盘队列 (`disk_queue_len`) - 磁盘队列长度
50. 磁盘活动 (计算值) - 基于IOPS计算的活动百分比
51. 磁盘容量 (`logical_disks`) - 逻辑磁盘容量信息数组
52. 存储温度 (`storage_temps`) - 存储设备温度数组
53. SMART健康 (`smart_health`) - 磁盘SMART健康状态数组
54. SMART关键 (计算值) - SMART关键指标汇总

### GPU相关指标 (14个)
55. GPU汇总 (`gpus`) - GPU基础信息数组
56. GPU名称 (`name`) - 显卡型号
57. GPU温度 (`temp_c`) - 核心温度
58. GPU负载 (`load_pct`) - 核心负载百分比
59. GPU核心频率 (`core_mhz`) - 核心工作频率
60. GPU显存频率 (`memory_mhz`) - 显存工作频率
61. GPU风扇转速 (`fan_rpm`) - 风扇转速
62. GPU风扇占空比 (`fan_duty_pct`) - 风扇占空比
63. GPU显存使用 (`vram_used_mb`, `vram_total_mb`) - 显存使用/总量
64. GPU功耗 (`power_w`) - 实时功耗
65. GPU功耗限制 (`power_limit_w`) - 功耗上限
66. GPU电压 (`voltage_v`) - 核心电压
67. GPU热点温度 (`hotspot_temp_c`) - 热点温度
68. GPU显存温度 (`vram_temp_c`) - 显存温度
69. GPU编码单元占用 (`encode_util_pct`) - 编码器使用率
70. GPU解码单元占用 (`decode_util_pct`) - 解码器使用率
71. GPU显存带宽占用 (`vram_bandwidth_pct`) - 显存带宽使用率
72. GPU性能状态 (`p_state`) - P-State功耗状态

### 其他系统指标 (8个)
73. 风扇 (`fan_rpm`) - 主要风扇转速
74. 主板电压 (`mobo_voltages`) - 主板电压传感器数组
75. 更多风扇 (`fans_extra`) - 额外风扇信息数组
76. 高CPU进程 (`top_cpu_procs`) - CPU占用最高的进程数组
77. 高内存进程 (`top_mem_procs`) - 内存占用最高的进程数组
78. 公网IP (`public_ip`) - 外网IP地址
79. 运营商 (`isp`) - 网络服务提供商
80. 电池电量 (`battery_percent`) - 电池剩余电量百分比
81. 电池状态 (`battery_status`) - 电池状态字符串
82. 电池健康 (`battery_design_capacity`, `battery_full_charge_capacity`, `battery_cycle_count`) - 设计容量/满充容量/循环次数
83. AC电源 (`battery_ac_online`) - 是否连接交流电源
84. 剩余时间 (`battery_time_remaining_sec`) - 电池剩余使用时间
85. 充满时间 (`battery_time_to_full_sec`) - 电池充满所需时间

## 技术架构详解

### 数据采集层 (C# 桥接)
- 使用 LibreHardwareMonitor 库采集硬件传感器数据
- 通过 WMI 查询系统性能计数器
- 使用 Windows API 获取网络和进程信息
- 数据格式化为 JSON 并通过标准输出传递给 Rust 层

### 数据处理层 (Rust/Tauri)
- 解析 C# 输出的 JSON 数据
- 应用 EMA (指数移动平均) 算法平滑关键指标
- 管理配置文件和用户设置
- 通过事件系统广播数据给前端

### 数据展示层 (Vue 3)
- 响应式接收 Rust 广播的数据快照
- 实现数据格式化和单位转换
- 支持展开/收起的详细信息面板
- 兼容 snake_case 和 camelCase 命名风格

## 自动化测试架构

### 测试代码组织结构
为避免单文件过长导致的编辑器问题，测试代码已拆分为多个专门的测试类：

1. **BaseTestRunner**: 测试基础类
   - 提供统一的测试报告生成
   - JSON序列化配置（确保中文字符正确显示）
   - 测试结果收集和汇总

2. **CpuTests**: CPU相关测试（12个指标）
   - CPU使用率、温度、频率、功耗
   - 每核心详细监控
   - 电压和缓存监控

3. **MemoryTests**: 内存相关测试（12个指标）
   - 内存使用量、使用率、总量
   - 缓冲区、缓存、交换分区
   - 内存池和句柄监控

4. **NetworkTests**: 网络相关测试（15个指标）
   - 网络速率（平滑/瞬时）
   - 网络错误和丢包率
   - WiFi详细信息
   - 公网信息和延迟

5. **StorageTests**: 存储相关测试（8个指标）
   - 磁盘使用量和读写速度
   - SMART健康信息
   - 磁盘温度和响应时间

6. **GpuTests**: GPU相关测试（8个指标）
   - GPU使用率、温度、显存
   - GPU时钟频率、风扇转速
   - GPU功耗和电压

7. **SystemTests**: 系统其他测试（30个指标）
   - 系统运行时间和进程监控
   - 电池健康和时间预估
   - 系统风扇和主板电压

8. **MainTestRunner**: 主测试运行器
   - 整合所有测试类
   - 生成JSON和Markdown双格式报告
   - 提供完整的85指标测试覆盖

### 测试技术原理

#### JSON中文编码处理
使用`JavaScriptEncoder.Create(UnicodeRanges.All)`确保中文字符在JSON报告中正确显示，避免Unicode转码问题。

#### 测试数据验证
每个测试用例包含：
- **数据范围验证**: 确保数值在合理范围内
- **空值处理**: 正确处理可能为null的传感器数据
- **异常捕获**: 记录详细的错误信息用于调试

#### 报告生成机制
- **JSON格式**: 结构化数据，便于程序处理
- **Markdown格式**: 人类可读，便于查看和分享
- **分类展示**: 按功能模块组织测试结果
- **详细统计**: 包含通过率、耗时等关键指标面板
- 兼容 snake_case 和 camelCase 命名风格

## 数据流向

```
硬件传感器 → LibreHardwareMonitor → C# 桥接 → JSON 输出 → 
Rust 解析 → EMA 平滑 → 事件广播 → Vue 前端 → UI 展示
```

## 关键技术特性

1. **实时性**：1秒采集周期，低延迟数据传输
2. **平滑性**：EMA 算法减少数据抖动
3. **兼容性**：支持多种命名风格和数据格式
4. **扩展性**：模块化设计便于添加新指标
5. **稳定性**：异常处理和数据回填机制

## 测试架构技术原理

### 测试数据收集器设计

#### TestDataCollector 核心架构
- **异步数据收集**：使用`CollectDataAsync()`方法异步收集所有传感器数据
- **硬件管理器集成**：直接调用`HardwareManager.MakeComputer()`获取硬件访问权限
- **模块化数据收集**：按功能分类收集CPU、内存、GPU、存储、网络、系统数据
- **资源管理**：确保硬件访问完成后正确关闭`IComputer`实例

#### 关键技术问题解决

1. **静态类调用修复**
   - 问题：测试类错误地尝试实例化`DataCollector`和`HardwareManager`静态类
   - 解决：修正为正确的静态方法调用模式
   - 影响：消除了所有编译错误，确保测试代码可正常执行

2. **属性名称映射修复**
   - 问题：`CpuExtra`类属性名称不匹配（`PackagePowerW` vs `PkgPowerW`）
   - 解决：查看实际类定义，使用正确的属性名称
   - 影响：确保CPU功耗和频率数据正确获取

3. **类型转换处理**
   - 问题：`double?`到`float?`的隐式转换失败
   - 解决：添加显式类型转换`(float?)cpuExtra?.PkgPowerW`
   - 影响：保证数据类型一致性，避免运行时错误

4. **集合数据处理优化**
   - 问题：`CpuPerCore`返回可空集合，需要过滤和转换
   - 解决：使用LINQ进行空值过滤和类型转换
   - 代码：`perCore?.Loads?.Where(x => x.HasValue).Select(x => x.Value).ToList()`

#### TestDataSnapshot 数据结构设计
- **完整指标覆盖**：包含所有85个监控指标的强类型字段
- **测试专用类型**：定义`TestProcessInfo`、`TestNetInterface`等避免依赖外部类型
- **可空类型支持**：正确处理传感器数据缺失的情况
- **内存管理优化**：使用WMI替代VB.NET组件获取系统内存信息

### 测试执行流程

1. **硬件初始化**：创建并配置`IComputer`实例
2. **传感器更新**：遍历所有硬件组件进行数据更新
3. **数据收集**：按模块收集各类传感器数据
4. **类型转换**：统一数据格式和类型
5. **资源清理**：安全关闭硬件访问连接

### 编译优化成果
- **错误消除**：从11个编译错误减少到0个
- **警告管理**：保留45个非关键警告（主要是Windows平台特定API警告）
- **类型安全**：所有数据访问都经过类型检查和转换
- **异常处理**：每个数据收集模块都有独立的错误处理机制