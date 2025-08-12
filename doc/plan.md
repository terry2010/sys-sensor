# sys-sensor 项目计划（Plan）

更新时间：2025-08-12 20:35

## 一、项目技术栈

- 后端运行时与容器
  - Tauri v2（Windows，仅桌面）
  - Rust 1.7x（tokio 异步、serde 序列化、wmi 采集、tauri API）
- 传感器桥接
  - .NET 8（C#）+ LibreHardwareMonitorLib 0.9.4
  - 自愈与日志：支持环境变量控制 summary/dump/log file/周期重建/异常重建
- 前端
  - Vue 3 + TypeScript + Vite + vue-router
  - 事件订阅：`sensor://snapshot` 实时快照广播
- 打包与交付
  - Tauri bundler（NSIS 安装包/绿色便携版）
  - 资源内置：`src-tauri/resources/sensor-bridge/` 自包含单文件桥接
- 其他
  - WMI（温度/风扇、Perf 磁盘/网络计数、系统信息）
  - 命名与映射：桥接 JSON camelCase；Rust 端 snake_case + Serde 对齐；前端 TS 当前沿用 Rust 的 snake_case 字段，避免多点映射

## 二、项目目标（对标 iStat Menus，Windows 侧高可用监控）

- 实时、稳定、低开销地采集并展示：
  - CPU（总/每核心 负载/频率/温度、包功耗、限频状态与原因）
  - GPU（温度/负载/频率/风扇RPM，后续扩展显存/功耗）
  - 内存（使用率，后续细分）
  - 网络（上下行速率、错误率，后续接口基础信息与 Wi‑Fi 指标）
  - 磁盘（读/写速率、IOPS、队列长度，后续容量/SMART 健康/温度）
  - 温度/风扇（CPU/主板/存储等，多源融合与回退）
- 托盘双行文本图标，清晰可读，单位与宽度自适应（回退策略一致）
- 长时稳定运行：桥接自愈、数据新鲜度阈值、WMI 重连、睡眠恢复
- 良好的可观测性：详细日志、dump/summary、前端详情页汇总

## 三、项目工程特点

- 结构清晰：`sensor-bridge/`（C#）↔ `src-tauri/`（Rust）↔ `src/`（Vue）
- 事件驱动：后端每秒组装 `SensorSnapshot` 并 `emit("sensor://snapshot", payload)`
- 数据对齐与兼容：
  - 桥接输出 camelCase；Rust 端 snake_case；Serde 完成映射
  - 前端 TS 类型与 Rust `SensorSnapshot` 对齐（snake_case），UI 对 null 显示“—”
  - 每核心数组采用可选向量+可选元素，容忍部分核心无值
- 稳定性设计：
  - 桥接子进程 stderr 实时输出 + 重启自愈；Rust 端数据新鲜度 30s 阈值
  - WMI 性能计数失败重连与长间隔（睡眠）恢复；EMA 平滑速率
- 打包与运行：
  - 优先使用内置 `sensor-bridge.exe`，无 .NET 运行时也可运行
  - 提供安装包与便携版，两种交付路径
- 平台差异与回退：
  - 已知 NUC8 平台：普通权限下 CPU 温度/风扇多为“—”；管理员下温度可用但 RPM 常无值
  - 回退顺序：CPU 风扇 RPM → 机箱风扇 RPM → 占空比/CPU%（UI 明示平台限制）

## 四、接下来要完成的任务（Roadmap）
 优先级 Tier 1（快速落地，依赖少）
[电池充电状态与剩余/充满耗时]（已完成）
新字段（Rust 
SensorSnapshot
）：ac_line_online?: bool、time_to_empty_min?: i32、time_to_full_min?: i32
数据源：GetSystemPowerStatus 或 WMI Win32_Battery.EstimatedRunTime；充电/放电由 AC 供电与 BatteryStatus 组合判断。
前端：详情页“电池”块新增“AC/充电/放电”“剩余/充满 估时”。
[公网 IP 与 ISP]（已完成）
新字段：public_ip?: String、isp?: String
数据源：HTTP 轻量查询（如 ipify/ip.sb + ip-api/ipinfo）。失败时可为空，不阻塞其他数据。
前端：网络块附加“公网 IP / ISP”。
[每网卡详情与链路参数]（已完成）
新字段：
net_ifs
 已有（名称/速率等），补充：ipv4/ipv6/mac/speed_mbps/duplex/link_up
数据源：WMI Win32_NetworkAdapter + Win32_NetworkAdapterConfiguration
前端：详情页“网络接口”可展开查看。
[Wi‑Fi 细节补充]（已完成）
已有：ssid/signal_pct/link_mbps/band/channel/radio/rssidbm/tx/rx
待补：bssid（已有）基础上增加 channel_width_mhz、security（WPA/WPA2/3）
数据源：netsh wlan show interfaces 解析。
前端：Wi‑Fi 行追加显示。
优先级 Tier 2（需要桥接 LHM 更多传感器）
[主板/CPU 电压与更多风扇]
新字段：voltages?: Vec<SensorKV>、fans_extra?: Vec<SensorKV>（多路风扇）
桥接（C#）读取 LHM 的 Voltage/Fan 传感器统一透出，Rust 映射到 
SensorSnapshot
。
前端：详情页“传感器”新增电压列表、额外风扇 RPM。
[GPU 细分指标]
已有：name/tempC/loadPct/coreMhz/fanRpm、VRAM 使用与功耗（已接入）
待补：mem_clock_mhz、hotspot_temp_c、vram_temp_c、fan_duty_pct、power_limit_w
数据源：LHM 对应 GPU 传感器（NVIDIA/AMD/Intel 视驱动支持）。
前端：GPU 行追加字段，超出两项以“+N”聚合。
[存储健康细节（非仅 OK/Fail）]
已有：SMART 健康（聚合）
待补：关键 SMART 属性简表（如 Reallocated Sectors/Media Wearout/Temperature/Power-On Hours）
数据源：WMI MSStorageDriver_FailurePredictData/
Status
 或桥接扩展。
前端：磁盘详情展开显示“关键 SMART”。
优先级 Tier 3（重度/可选）
[Top 进程（CPU/内存/网络）]
新字段：top_procs?: Vec<ProcSample>（如前 5）
数据源：Windows GetProcessMemoryInfo/NtQuerySystemInformation + 采样差分；或引入 sysinfo crate。
说明：实现复杂、开销较高，可置后。
[网络分主/备测延迟]
已有：ping_rtt_ms 单点近似
待补：可配置多目标 RTT（如 1.1.1.1、8.8.8.8、网关），聚合展示最小/中位、丢包率
说明：增加异步任务与状态管理，次于上面的硬件指标。
字段与实现落点建议
Rust 后端：在 
src-tauri/src/lib.rs
 的采样循环中
补充 Tier 1 WMI/系统 API 读取与赋值。
SensorSnapshot
 增量字段采用 Option<T>，无值不影响序列化（Serde 忽略 null）。
桥接（C# LibreHardwareMonitor）：sensor-bridge/Program.cs
扩展枚举 Voltage/Fan/GPU 其他传感器，输出 camelCase，Rust 端 snake_case 做 Serde 对齐。
前端：
src/main.ts
 同步 
SensorSnapshot
 类型。
src/views/Details.vue
 增加对应展示与格式化函数；遵循“无值显示 —”。
计划与验收（建议顺序）
Step A（本轮直落实现）
电池 AC/剩余/充满耗时（系统 API+WMI）（已完成）
公网 IP/ISP（HTTP 查询，可配置关闭）（已完成）
每网卡详情与 Wi‑Fi 细节（WMI+netsh）（已完成）
Step B 4) 桥接扩展电压/多风扇，Rust/前端打通 5) GPU 细分指标（mem clock/hotspot/VRAM temp）
Step C（可选） 6) SMART 关键属性简表 7) Top 进程 8) 多目标 RTT
每一步：

后端 cargo check、前端 npm run build 验证。
记录进度到 
doc/progress.md
（中文）。
如需管理员验证，我会在 doc/script/ADMIN-TEST-*.md 增补测试点与期望结果。
- 稳定性与诊断
  - 长时跑测（6–12h）与睡眠/断桥注入验证；完善日志与故障指引
  - 文档完善：`README` 与 `doc/项目总结与开发注意事项.md` 持续维护

## 五、当前里程碑状态（摘要）

- 已完成
  - 桥接自包含发布与资源内置；Rust 端启动优先内置桥接
  - GPU 监控全链路（温度/负载/频率/风扇）
  - 第二梯队指标：磁盘 IOPS/队列、网络错误率、RTT 近似
  - CPU 每核心数组落地（负载/频率/温度）并前端展示
  - 构建：`cargo check`、`npm run build` 通过
- 进行中
  - 无
- 待办（见路线图）
  - 内存细分、主板电压与更多风扇、GPU 细分指标（mem clock/hotspot/VRAM temp/fan duty/power limit）、SMART 关键属性简表、Top 进程、多目标 RTT、电池健康
