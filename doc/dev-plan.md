# 开发计划（Windows 系统托盘硬件监控）

## 1. 项目目标
- 基于 Tauri（Rust 后端 + Vue/TS 前端）实现 Windows 托盘常驻监控：
  - 托盘文本显示两项关键指标（默认 CPU 温度 + CPU 占用），1s 刷新。
  - Tooltip/右键菜单上半区展示详细信息。
  - 详情窗口、快速设置、关于我们；退出彻底关闭。
  - 新增：当前网速（上/下/合计）与磁盘读写速率（读/写/合计）。

## 2. 技术选型与依赖
- 后端（Rust / Tauri v2）：
  - tauri, tauri-build
  - sysinfo：系统信息（CPU、内存、进程、网络/磁盘累计字节）
  - serde/serde_json：数据序列化
  - tokio：异步与定时任务
  - image + ab_glyph：文本绘制到 32x32 RGBA 生成托盘图标
- 传感器桥（.NET 8 Console）：
  - LibreHardwareMonitorLib：主板温度、风扇转速
  - 输出 JSON（stdout）；由 Rust 子进程管理与解析
- 前端（Vue + TypeScript + Vite）：
  - 详情页、快速设置、关于页面
  - 与后端通过 Tauri IPC（commands/events）通讯
- 配置：JSON（AppData 路径），持久化托盘显示项、网/盘来源策略等

## 3. 项目结构（规划）
```
C:/code/sys-sensor/
├─ src/                        # Vue 前端
├─ src-tauri/                  # Rust 后端（Tauri）
│  ├─ src/
│  │  ├─ main.rs               # 入口、托盘/菜单/事件、采集调度
│  │  ├─ sensor.rs             # 聚合器：sysinfo + 桥接数据
│  │  ├─ tray.rs               # 托盘文本图标渲染与更新
│  │  ├─ menu.rs               # 右键菜单构建与交互
│  │  ├─ ipc.rs                # 前后端 IPC 命令与事件
│  │  ├─ config.rs             # 配置读写（JSON）
│  │  └─ utils.rs              # 公共工具（EMA、单位换算等）
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ sensor-bridge/              # .NET 8 传感器桥（C#）
│  ├─ Program.cs
│  └─ sensor-bridge.csproj
├─ doc/
│  ├─ product.md               # 产品文档
│  └─ dev-plan.md              # 本开发计划
└─ README.md
```

## 4. 数据模型与刷新逻辑
- 刷新周期：1s（可调）。
- Rust 维护 `SensorSnapshot`：
```jsonc
{
  "timestamp": 0,
  "cpu": {"temp_c": 65.2, "usage_pct": 41.3},
  "memory": {"used_gb": 12.3, "total_gb": 31.8, "usage_pct": 38.7},
  "mobo": {"temp_c": 45.1},
  "fans": [{"name": "CPU Fan", "rpm": 1250}, {"name": "Case Fan1", "rpm": 800}],
  "net": {"rx_bps": 12_300_000, "tx_bps": 2_100_000, "total_bps": 14_400_000},
  "disk": {"read_bps": 8_500_000, "write_bps": 3_200_000, "total_bps": 11_700_000},
  "top_process": {"by_cpu": [...], "by_mem": [...]}
}
```
- 网速/磁盘速率：使用 sysinfo 累计字节做差分 / Δt（秒） = Bps；
  - 平滑：EMA（α=0.3，配置可调）。
  - 单位自适应：< 1024 KB/s 显示 KB/s，否则 MB/s；保留 1 位小数。
- 多设备：
  - 网络：过滤虚拟/非活动接口，默认聚合活动物理接口；支持指定接口。
  - 磁盘：默认系统盘或最繁忙盘；支持聚合或指定盘。

## 5. 托盘与菜单
- 托盘文本：将如 "65℃ 42%" 绘制到 32x32 RGBA，设置为托盘图标。
- Tooltip：按 `SensorSnapshot` 人性化格式化，1s 同步更新。
- 右键菜单：上半区信息（不可点）；下半区操作项：详情 / 快速设置 / 关于我们 / 退出。

## 6. 窗口与前端
- 详情窗口：
  - 分区展示 CPU/内存/主板/风扇、网络速率、磁盘读写、Top 进程（CPU/内存）。
- 快速设置：
  - 托盘两项候选（CPU温度/占用/风扇/内存/主板温度/网速/磁盘吞吐）。
  - 网络/磁盘来源策略选择（自动/指定/聚合）。
  - EMA 平滑参数可选（0.2~0.5）。
- 关于我们：官网、作者微博链接。

## 7. 关键实现点
- 传感器桥子进程：
  - Rust 启动 .NET EXE，固定 1Hz 输出 JSON 行；Rust 异步读取 stdout，解析失败自动重启。
- sysinfo 差分：
  - 保存上次累计计数与时间戳，计算 Δbytes/Δt；首个样本不显示速率。
- DPI 与字体渲染：
  - 根据系统缩放调整字体大小；深浅色主题下确保对比度。
- 配置与持久化：
  - 首启生成默认配置；修改后写回，使用热更新或下次启动生效。
- 错误与恢复：
  - 传感器读取失败/权限不足：UI 显示“不可用”，日志记录并重试。

## 8. 任务拆解（里程碑）
- M1 脚手架与环境（已完成）
  - Tauri (Vue+TS) 项目初始化、依赖确认、开发环境验证。
- M2 后端采集聚合
  - sysinfo 采样、差分速率、EMA；数据模型与事件广播；配置读写。
- M3 托盘/Tooltip/右键菜单
  - 文本图标渲染、1s 刷新；菜单信息区与功能项；退出逻辑。
- M4 传感器桥（.NET）
  - LibreHardwareMonitor 读取主板/风扇；与 Rust 管道通信与合并。
- M5 前端窗口
  - 详情/快速设置/关于；与后端 IPC；设置持久化。
- M6 网速/磁盘细节
  - 多网卡/多磁盘选择与聚合；单位切换与显示策略。
- M7 测试与打包
  - 手测 + 自动化基础；签名/打包；运行验证。

## 9. 测试计划
- 单元测试：单位换算、EMA、差分计算、配置序列化。
- 集成测试：传感器桥通信、采样循环稳定性、菜单/窗口交互。
- 手工测试：
  - 不同网卡/磁盘数量与类型；
  - 断网/高负载/磁盘高 IO；
  - DPI 缩放 100%/125%/150%。

## 10. 开发与运行命令
```powershell
# 安装依赖（已完成）
npm install

# 开发模式
npm run tauri dev

# 构建
npm run build
npx --yes @tauri-apps/cli@latest build
```

## 11. 风险与对策
- 传感器读取权限/兼容性：提供管理员运行提示与白名单说明。
- 多设备策略复杂：默认聚合 + 设置可配，降低边缘情况失败率。
- 字体渲染清晰度：按 DPI 调整字号与描边/阴影提高可读性。

## 12. 交付物
- 可运行安装包、源代码、README 与使用说明。
- `doc/product.md` 与 `doc/dev-plan.md` 同步维护。

## 12.5 iStat Menus 对标结果与缺失项清单（优先级）
 
 基于现有实现（参见 `doc/progress.md` 与代码）：
 
 - CPU
   - 已有：包温度/主板温度、总体占用%、托盘显示、CPU 二级指标（包功耗 `cpu_pkg_power_w`、平均频率 `cpu_avg_freq_mhz`、降频活跃 `cpu_throttle_active`、降频原因 `cpu_throttle_reasons`）。
   - 缺失：
     1) 每核心负载/频率/温度（优先级：高）。
     2) CPU 各类电压/功耗细分（如核心/SoC，优先级：中）。
 - 内存
   - 已有：总量/已用/占用%（UI 已展示）。
   - 缺失：
     1) 可用/缓存/交换区（分页文件）细分（优先级：中）。
     2) 内存温度（部分平台可得，优先级：低）。
 - 磁盘/存储
   - 已有：读/写速率（Bps，含 EMA）、IOPS、队列长度、NVMe/SSD 温度列表（含位置中文化）。
   - 缺失：
     1) 每盘/每分区读写与容量/可用空间（优先级：中）。
     2) SMART 健康/剩余寿命等（优先级：中，依赖磁盘/厂商支持）。
 - 网络
   - 已有：上/下/合计速率、错误包速率（RX/TX）、Ping 近似 RTT。
   - 缺失：
     1) 接口 IP/MAC、连接速率（优先级：中）。
     2) Wi‑Fi SSID/RSSI/链路速率（优先级：中）。
 - 主板/风扇/电源
   - 已有：主板/环境温度、风扇 RPM 统一选择与回退策略（NUC8 说明与管理员建议已文档化）。
   - 缺失：
     1) 主板各路电压/功率（优先级：低）。
 - GPU
   - 已有：温度/负载/核心频率/风扇 RPM；前后端全链路与 UI 展示。
   - 缺失：
     1) 显存占用、GPU 包功耗（优先级：中，依赖 LHM 支持）。
 - 其他
   - 已有：应用/桥接自愈指标（`hb_tick/idle_sec/exc_count/uptime_sec`）、WMI 重连与睡眠自恢复、托盘/Tooltip/详情联动。
   - 缺失：
     1) 电池健康/循环次数/设计容量（笔记本）（优先级：低）。
     2) 系统 Uptime（系统级）（优先级：低，前端可与桥接 uptime 并列展示）。
 
 优先落地清单（按高->中->低）：
 1) 每核心负载/频率/温度（CPU）。
 2) 网络 Wi‑Fi（SSID/RSSI/速率）与接口 IP/MAC 基础信息。
 3) 磁盘每盘/分区容量与可用空间、SMART 健康（可用则展示）。
 4) GPU 显存占用、功耗（可用则展示）。
 5) 内存细分（可用/缓存/交换）、系统 Uptime、主板电压。
 6) 电池健康信息（如为笔记本）。
 
 注：所有新字段遵循命名规范——Rust 端 `snake_case`，桥接/前端 `camelCase`，并确保 `SensorSnapshot` 与前端类型对齐；UI 无值显示“—”。
 
## 12.6 缺失指标结构化清单与补全路线图

为对标 iStat Menus 并提升可观测性，现将缺失指标按“数据源 → 后端/Rust → 桥接/C# → 前端/TS → 验收标准”的格式结构化如下。新增字段遵循命名规范：Rust 端 snake_case，桥接/前端 camelCase；UI 无值显示“—”。

- __CPU 每核心 负载/频率/温度（高）__
  - 数据源：LibreHardwareMonitor（`HardwareType.Cpu` -> per-core `Load/Clock/Temperature`）。
  - 桥接/C#：新增 `cpuCores: [{ id, name, loadPct, coreMhz, tempC }]`。
  - 后端/Rust：`SensorSnapshot.cpu_cores: Option<Vec<{ id: u8, name: String, load_pct: f32, core_mhz: f32, temp_c: Option<f32> }>>`；桥接映射。
  - 前端/TS：在 `SensorSnapshot` 类型新增 `cpu_cores`，`Details.vue` 增加分组展示（最多显示前 8 条，超出以 `+N` 汇总）。
  - 验收：管理员环境下常见桌面 CPU 至少显示每核心 `load/clock`，若温度不可用显示“—”。

- __网络基础信息（IP/MAC/链路速率）（中）__
  - 数据源：WMI `Win32_NetworkAdapter`/`Win32_NetworkAdapterConfiguration`/`MSNdis_LinkSpeed`；或 Rust `windows` crate 调用 `GetAdaptersAddresses`。
  - 桥接/C#：无（建议由 Rust 侧完成，避免双向耦合）。
  - 后端/Rust：新增 `net_ifaces: Option<Vec<{ name, ipv4, mac, link_mbps }>>`，受现有 `net_interfaces` 白名单过滤。
  - 前端/TS：`Details.vue` 新增“网络接口”卡片，显示当前参与统计的接口基本信息。
  - 验收：至少对主要物理接口显示 IPv4、MAC、链路速率（Mb/s），虚拟/禁用接口过滤。

- __Wi‑Fi 指标（SSID/RSSI/连接速率）（中）__
  - 数据源：WLAN API（`WlanOpenHandle/WlanEnumInterfaces/WlanQueryInterface`）；Rust `windows` crate；回退 `netsh wlan show interfaces` 解析。
  - 桥接/C#：无（首选 Rust 直连 WLAN API）。
  - 后端/Rust：新增 `wifi: Option<{ ssid: String, rssi_dbm: i32, link_mbps: u32 }>`（仅当活动接口为 Wi‑Fi 时出现）。
  - 前端/TS：`Details.vue` 新增“Wi‑Fi”行；Tooltip/菜单信息区追加。
  - 验收：连接 Wi‑Fi 时能显示 SSID、RSSI（dBm）、速率（Mb/s）；有线网络时该区块隐藏或显示“—”。

- __磁盘/分区 容量与可用空间（中）__
  - 数据源：WMI `Win32_LogicalDisk`（本地固定盘）；或 Rust `sysinfo` 的分区枚举。
  - 桥接/C#：无。
  - 后端/Rust：新增 `partitions: Option<Vec<{ name, fs, total_gb, used_gb, usage_pct }>>`；与现有磁盘速率并列。
  - 前端/TS：`Details.vue` 新增“分区使用率”表格（最多 6 条，超出折叠）。
  - 验收：系统盘与主要数据盘容量/占用正确，单位换算与排序稳定。

- __磁盘 SMART 健康（中，若可用）__
  - 数据源：LibreHardwareMonitor `HardwareType.Storage` 传感器（如 Remaining Life/Wear Level/Health 等）。
  - 桥接/C#：新增 `storageSmart: [{ name, healthPct, wearPct, powerOnHours }]`（字段按可得性输出）。
  - 后端/Rust：`SensorSnapshot.storage_smart: Option<Vec<...>>`；Tooltip 简要汇总（最多 2 项）。
  - 前端/TS：详情页新增“存储健康”卡片。
  - 验收：NVMe/SSD 平台若 LHM 暴露，显示健康百分比或磨损；未暴露则为空。

- __GPU 显存占用与包功耗（中）__
  - 数据源：LibreHardwareMonitor `HardwareType.Gpu*`（Memory Used/Total，Power）。
  - 桥接/C#：扩展 `gpus[]` 字段，新增 `memUsedMB/memTotalMB/powerW`。
  - 后端/Rust：扩展 `SensorSnapshot.gpus[]` 同步字段。
  - 前端/TS：`Details.vue` 的 GPU 汇总行增加显存与功耗显示。
  - 验收：独显机器显示显存使用与功耗；核显可能无功耗读数则显示“—”。

- __内存细分：可用/缓存/交换（中）__
  - 数据源：WMI `Win32_PerfFormattedData_PerfOS_Memory`（CachedBytes/CommittedBytes/CommitLimit）；`Win32_PageFileUsage`（分页文件）。
  - 桥接/C#：无。
  - 后端/Rust：新增 `mem_available_gb/mem_cached_gb/swap_used_gb/swap_total_gb`。
  - 前端/TS：详情页“内存”分组增加细分行；单位 GiB，1 位小数。
  - 验收：值与任务管理器同量级；不可得字段显示“—”。

- __系统 Uptime（低）__
  - 数据源：Rust `sysinfo::System::uptime()` 或 WinAPI `GetTickCount64`。
  - 桥接/C#：无。
  - 后端/Rust：新增 `sys_uptime_sec: u64`。
  - 前端/TS：`fmtUptime()` 复用，详情与 Tooltip 显示系统运行时长。
  - 验收：与 `systeminfo`/任务管理器一致量级。

- __主板电压（低）__
  - 数据源：LibreHardwareMonitor `HardwareType.Mainboard`（Voltage 传感器）。
  - 桥接/C#：新增 `moboVoltages: [{ name, volts }]`。
  - 后端/Rust：`SensorSnapshot.mobo_voltages` 对应映射。
  - 前端/TS：详情页“主板”分组下以简表展示。
  - 验收：常见主板可见 1–5 项电压；不可得时为空。

- __电池健康（低，笔记本）__
  - 数据源：WMI `Win32_Battery`、`BatteryFullChargedCapacity`、`BatteryStaticData`（root\WMI），WinAPI `GetSystemPowerStatus` 兜底。
  - 桥接/C#：无（Rust 侧完成以减少依赖）。
  - 后端/Rust：新增 `battery: Option<{ percent: u8, health_pct: Option<u8>, cycle_count: Option<u32> }>`。
  - 前端/TS：详情页新增“电池”卡片；无电池时隐藏。
  - 验收：带电池设备显示电量；若能取到设计/满充容量则给出健康估算与循环次数。

### 实施里程碑（建议顺序）

1) 高优先（一期）：
   - CPU 每核心；Wi‑Fi（SSID/RSSI/速率）；网络接口基础信息；GPU 显存/功耗。
2) 中优先（二期）：
   - 分区容量与使用率；磁盘 SMART 健康；内存细分；系统 Uptime。
3) 低优先（三期）：
   - 主板电压；电池健康。

### 通用实现要求

- __一致性__：Rust 与前端类型严格对齐；桥接字段 camelCase 与 Rust 端 `serde(rename_all = "camelCase")` 映射保持一致。
- __可用性__：所有新增字段允许缺省/空；UI 一律以“—”渲染，不影响其他指标。
- __性能__：采样线程保持 1 Hz；WMI/WinAPI 查询集中化与连接复用；必要时引入 EMA 或节流。
- __日志与诊断__：新增字段在首次出现与不可用时各打印 1 条摘要日志；长时稳定性沿用现有自愈/重连策略。

## 13. 新增指标优先级（三梯队）与实施计划
 
 - __第一梯队（立即）__
  - 主板/系统环境温度（`moboTempC`，已接入）。
  - NVMe/SSD 温度（每盘设备温度列表）。
  - 桥接健康指标（心跳 tick、空闲秒数、连续异常次数、上次重建至今秒数、桥接运行秒数）。
  - 网/盘统计细化与聚合稳定性（已具备 Bps，持续优化）。

{{ ... }}
  - CPU 包功耗、频率、降频/热限标志。
  - 磁盘 IOPS、队列长度。
  - 网络丢包/错误计数与基本延迟探测。

- __第三梯队（后续）__
  - GPU 温度/负载/频率/风扇（按用户要求降级至第三梯队）。
  - Wi‑Fi SSID / RSSI。
  - 电池健康（如为笔记本）。

### 13.1 最小任务清单（MVP 扩展）
- __存储温度（NVMe/SSD）__：桥接开启 `IsStorageEnabled`，输出 `storageTemps: [{ name, tempC }]`；Rust 端反序列化并在 Tooltip/详情页展示（初期仅 Tooltip）。
- __桥接健康指标__：桥接每秒在 JSON 中附带 `hbTick`、`hbIdleSec`、`hbExcCount`、`hbSinceReopenSec`、`hbUptimeSec`，便于后端/前端判断“桥接离线/卡死/需自愈”。
- __自愈默认值与脚本__：启动脚本默认 `BRIDGE_SELFHEAL_IDLE_SEC=300`、`BRIDGE_SELFHEAL_EXC_MAX=5`，并支持 `BRIDGE_PERIODIC_REOPEN_SEC`（默认 0）。
- __NUC8 诊断提示__：在风扇 RPM 不可用时，Tooltip 与详情页显式提示“NUC 平台多不公开风扇 RPM，已回退占空比/CPU%”。

### 13.2 验收标准
- 存储温度：在常见 NVMe/SSD 设备上可见温度（单位 ℃，异常值过滤），无设备时 UI 显示“—”。
- 桥接健康：在长时间运行（>6 小时）后，仍能自动恢复主板温度/风扇读数（通过自愈重建），Tooltip 能显示最近空闲秒数与异常计数。
- 文档与脚本：`doc/progress.md` 有更新记录；`doc/script/start-sys-sensor.ps1` 含自愈与日志环境变量。

### 13.3 已知问题与对策（NUC8/桌面 Win10 若数小时后温度/风扇消失）
- 可能原因：EC/驱动句柄失效或权限变化导致传感器树失活。
- 对策：
  - 桥接在“空闲超过阈值/连续异常达到阈值/周期到达”时自动重建 `Computer`。
  - 后端对桥接数据设置过期判定（>5s 过期），并在 UI 提示“桥接暂不可用，正在自愈…”。
  - 建议以管理员运行以最大化传感器可用性；NUC8 平台风扇 RPM 多不可用属预期。
