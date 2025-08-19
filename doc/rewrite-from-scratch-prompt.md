# 从零重写 sys-sensor 的完整提示（New Session Prompt）

以下提示用于在一个全新会话中，从零开始重写 sys-sensor 项目，使之功能完整、鲁棒、可维护，并避免现有实现中出现过的所有问题。请严格遵循本提示，最终交付一个可运行、可测试、可维护的完整工程。

## 1. 目标与范围（Goals & Scope）
- 提供系统传感器的统一采集、聚合与广播能力，覆盖：
  - RTT（多目标）
  - 网卡 NetIf（吞吐、错误率、连接数、丢包率等）
  - 逻辑磁盘 LDisk（读/写吞吐、IOPS、队列长度）
  - SMART（磁盘健康：统计计数、最近成功/失败时间戳等）
- 后端以 Rust + Tauri(2.x) 实现，提供：
  - 可配置的调度器（Scheduler）按 tick 驱动各 Runner 执行
  - 统一状态仓库（StateStore）维护 TickTelemetry 与 Aggregated 聚合
  - 热更新配置（不重启）
  - 事件广播：`sensor://agg`（聚合）与 `sensor://smart`（SMART）
  - 命令接口（Tauri commands）用于前端读取状态、控制任务、更新配置
- 前端以 Vue3 + Vite + TS 实现调试页（Debug），展示 KPI、控制任务、查看状态、查看 SMART。
- Windows 10 - Windows 11 全面支持（采集实现以 Win10 为主，兼容 Win11 差异）。

## 2. 非目标（Non-Goals）
- 不实现分布式与跨主机收集（单机内采集为主）。
- 不引入大型图表库（优先轻量实现）。

## 3. 指标目录（85+ 指标，要求全部可用且有单测）
- CPU（≥20 项）
  - 总体：`cpu_usage_pct`、`cpu_avg_freq_mhz`、`cpu_pkg_power_w`、`cpu_throttle_active`、`cpu_throttle_reasons`
  - 按核心：`cpu_core_loads_pct[]`、`cpu_core_clocks_mhz[]`、`cpu_core_temps_c[]`
  - 温度与功耗：`cpu_temp_c`、`soc_temp_c`、`tdp_limit_w`（可选）
  - 其他：`since_reopen_sec`、`uptime_sec`、`boot_time_ms`
- 内存（≥6 项）
  - `mem_used_mb`、`mem_total_mb`、`mem_pct`、`swap_used_mb`、`swap_total_mb`、`commit_limit_mb`
- GPU（≥10 项）
  - `gpu_count`、每卡：`name`、`tempC`、`loadPct`、`coreMhz`、`fanRpm`、`vram_total_mb`、`vram_used_mb`、`power_w`
- 网络 NetIf（≥10 项）
  - 聚合：`net_rx_bps`、`net_tx_bps`、`net_rx_err_ps`、`net_tx_err_ps`、`packet_loss_pct`、`active_connections`
  - 分接口（可选）：每接口 rx/tx bps、错误、丢弃、MTU、链路状态
- 逻辑磁盘 LDisk（≥10 项）
  - `disk_r_bps`、`disk_w_bps`、`disk_r_iops`、`disk_w_iops`、`disk_queue_len`、（可选）平均读/写时延、每卷剩余空间
- SMART（≥15 项 + NVMe 关键4项）
  - 统计：`smart_ok_count`、`smart_fail_count`、`smart_consecutive_failures`、`smart_last_ok_ms`、`smart_last_fail_ms`
  - 逐盘摘要：温度、通电时长、上电次数、重分配、CRC 错误、坏道计数、健康度
  - SSD 专项：`host_reads_bytes`、`host_writes_bytes`
  - NVMe 专项：`nvme_percentage_used_pct`、`nvme_available_spare_pct`、`nvme_available_spare_threshold_pct`、`nvme_media_errors`
- 电池（可选 ≥5 项）
  - `battery_percent`、循环次数、健康度估计、充放电状态、剩余时间估计
- 主板/机箱（可选 ≥8 项）
  - `mobo_voltages{}`、`fans_extra[]`、若平台不支持需给出回退与占位策略

说明：以上清单总数 >85；实现时以 Option 字段为主，平台不支持→None，不得阻塞主循环。

### 3.6 指标补充与单位约定（补齐清单）
- CPU 补充：
  - `cpu_pkg_power_w`、`cpu_core_power_w`、`cpu_soc_power_w`（若可得）
  - `cpu_voltage_v`（包电压/核心电压，平台相关）
  - `cpu_tctl_temp_c`（AMD 平台）
  - `cpu_e_core_usage_pct[]`、`cpu_p_core_usage_pct[]`（异构核可选）
- Memory 补充：
  - `mem_cached_mb`、`mem_available_mb`、`mem_free_mb`、`commit_used_mb`
- GPU 补充：
  - `encPct`、`decPct`、`power_limit_w?`、`pstate?`、`fanPct?`、`vram_bus_util_pct?`
- NetIf 补充：
  - 每接口：`link_up`、`mtu`、`rx_discards_ps`、`tx_discards_ps`、`rx_unicast_ps`、`tx_unicast_ps`
- LDisk 补充：
  - `disk_r_latency_ms`、`disk_w_latency_ms`（可选）
  - 每卷：`fs_total_mb`、`fs_free_mb`、`fs_used_pct`
- SMART 补充：
  - SSD：`nand_writes_bytes`、`program_fail_count`、`erase_fail_count`
  - NVMe：`nvme_data_units_read`、`nvme_data_units_written`、`nvme_controller_busy_time_ms`、`nvme_power_cycles`
- Battery 补充：
  - `battery_cycle_count`、`battery_health_pct`、`battery_time_to_empty_min`、`battery_time_to_full_min`
- Mobo/Chassis 补充：
  - `vrm_temp_c?`、`pch_temp_c?`、`pump_rpm?`、`chassis_fans[]`、`rails_voltages{+3.3V,+5V,+12V}`
- 单位约定：
  - 温度：摄氏度 `*_c`
  - 频率：MHz `*_mhz`
  - 功率：瓦 `*_w`
  - 电压：伏 `*_v`
  - 速率：`bps`；IOPS：`iops`；时延：`ms`；容量：`mb`
  - 百分比：`*_pct`，范围 0-100

## 3. 技术栈（Tech Stack）
- Backend: Rust（stable），Tauri 2.x
- Frontend: Vue 3 + Vite + TypeScript
- 打包/运行：Tauri Dev/Build，Node 18+
- 日志：简单结构化日志（env 控制级别）

## 4. 目录结构（Proposed Structure）
 
/ (repo root)
  README.md
  package.json
  package-lock.json
  vite.config.ts
  tsconfig.json
  tsconfig.node.json
  index.html
  /src                      # 前端（Vue3 + Vite + TS）
    /assets
    /views
      Debug.vue
      About.vue
      Settings.vue
      Details.vue
    /router
      index.ts
    App.vue
    main.ts
  /src-tauri                 # 后端（Rust + Tauri 2）
    Cargo.toml
    tauri.conf.json
    /src
      lib.rs                 # 入口；注册 commands、事件与主循环
      scheduler.rs           # 调度器
      state_store.rs         # TickTelemetry / Aggregated
      config.rs              # 配置模型与热更新
      api.rs                 # Tauri commands 实现
      bus.rs                 # 事件总线封装（对 tauri emit 的薄封装）
      runners/
        mod.rs
        rtt_runner.rs
        netif_runner.rs
        ldisk_runner.rs
        smart_runner.rs
        cpu_runner.rs
        gpu_runner.rs
        memory_runner.rs
        battery_runner.rs
        mobo_runner.rs
      smart_worker.rs        # SMART 背景刷新（如需要）
      utils/
        wmi.rs               # Windows WMI 适配（mockable）
        pdh.rs               # 性能计数器适配（mockable）
        nvme.rs              # NVMe IOCTL/SMART 读取（mockable）
        disk.rs              # 逻辑磁盘与 IO 统计
        netif.rs             # 网卡统计
    /resources
      /sensor-bridge         # C# Bridge 资源或二进制（如随包分发）
        README.md
  /sensor-bridge             # C# 采集桥（独立进程，stdout 输出行式 JSON）
    sensor-bridge.csproj
    Program.cs
    ConfigurationManager.cs
    HardwareManager.cs
    DataCollector.cs
    DataModels.cs
    SensorUtils.cs
    LogHelper.cs
    TestRunner.cs
    appsettings.json
  /tests
    /backend                 # Rust 后端单测/集成测试
    /frontend                # 前端组件与 e2e（可选）
  /doc
    progress.md              # 每次变更追加记录
    rewrite-from-scratch-prompt.md
    plan.md
    plan-async-scheduler.md
  /scripts
    dev.ps1                  # 本地开发快捷脚本（Win10）
    build.ps1                # 构建/打包脚本
    test.ps1                 # 运行测试与报告

说明：
- C# Bridge 通过 stdout 提供实时 JSON；Rust 后端作为子进程管理其生命周期，并转发为 `sensor://agg`/`sensor://smart` 事件。
- `utils/*` 与系统接口解耦并可注入 mock，以便 Windows CI 可重复测试。
- `tests/` 目录可按需细分为单测、集成与端到端层次；前端可使用最小 e2e 校验事件订阅与命令调用。

## 5. 后端核心设计（Backend Design）
### 5.1 Runner 抽象（可扩展、版本化）
- Trait `Runner`: `should_run(tick)->bool`, `run()->Result<Snapshot>`，`last_ok_ms()`, `age_ms(now)`。
- 各 Runner 负责：读取配置、执行采集、缓存最新快照（非阻塞获取）、必要时独立线程（如 SMART）。
- Runner 注册表（Registry）：唯一 `kind`，幂等注册，支持查询与热插拔（为后续插件化留接口）。
- Snapshot/事件 Schema 版本化：`schema_ver` 字段；新增字段仅追加，不破坏旧语义。

### 5.2 Scheduler（节拍/节流/启停/一次触发，主进程注册防重复）
- 配置：`interval_ms`（最小 300ms，建议 500-1500ms），各任务 `*_every`（单位：tick）。
- 状态：`tick`、`tick_cost_ms`、`frame_skipped`、各任务 `*_is_running`、`*_enabled`、`*_every`、`*_last_ok_ms`、`*_age_ms`。
- 行为：
  - 每 tick（由 `interval_ms` 驱动）检查各 Runner `should_run`。
  - `mark_start/ok/finish` 记录任务状态与时间。
  - 一次性触发命令 `trigger_task(kind)` 立即置位一次。
- 主进程防重复注册：采用 `OnceCell`/`Lazy` + 进程内全局互斥 + `TaskRegistry` 唯一约束，确保 Scheduler、事件监听、命令注册仅初始化一次；重复初始化返回显式错误并记录日志，绝不静默覆盖。

### 5.3 StateStore（聚合与快照，历史窗口）
- `TickTelemetry`：`tick`、`tick_cost_ms`、`frame_skipped`。
- `Aggregated` 字段（全部 Option 以保持兼容）：
  - 时间：`timestamp_ms: i64`
  - CPU/MEM：`cpu_usage: Option<f32>`、`mem_pct: Option<f32>`
  - NetIf：`net_rx_bps: Option<f64>`、`net_tx_bps: Option<f64>`、`net_rx_err_ps: Option<f64>`、`net_tx_err_ps: Option<f64>`、`packet_loss_pct: Option<f64>`、`active_connections: Option<u32>`
  - LDisk：`disk_r_bps: Option<f64>`、`disk_w_bps: Option<f64>`、`disk_r_iops: Option<f64>`、`disk_w_iops: Option<f64>`、`disk_queue_len: Option<f64>`
  - Ping：`ping_rtt_ms: Option<f32>`（单点 ping）
  - 电量：`battery_percent: Option<f32>`（若不支持则 None）
  - GPU：`gpu_count: Option<u32>`（若不支持则 None）
  - RTT 聚合（多目标）：`rtt_avg_ms`、`rtt_min_ms`、`rtt_max_ms`、`rtt_success_ratio`、`rtt_success_count`、`rtt_total_count`
  - SMART 聚合：`smart_ok_count`、`smart_fail_count`、`smart_consecutive_failures`、`smart_last_ok_ms`、`smart_last_fail_ms`
- 提供接口：`update_tick()`、`update_agg()`、`get_tick()`、`get_agg()`，以及（可选）最近 N 条历史 ring buffer（例如 60 条）。
 - 历史窗口：各关键 KPI（RTT avg、IOPS、NetIf 速率等）固定 N=60 条，用于前端微图；内存占用受限。

### 5.4 事件与命令（API，UI 开发次序）
- 事件：
  - `sensor://agg`：负载为 `Aggregated`
  - `sensor://smart`：负载为 SMART 最新快照 JSON（含 `stats`）
- 命令（Tauri commands）：
  - `get_config()`：获取当前配置
  - `cmd_cfg_update({ patch })`：热更新配置并持久化
  - `get_scheduler_state()`：获取 Scheduler 状态
  - `set_task_enabled(kind, enabled)`：启停任务（`kind in ['rtt','netif','ldisk','smart']`）
  - `set_task_every(kind, every)`：更新节拍倍数
  - `trigger_task(kind)`：一次性触发
  - `get_state_store_tick()` / `get_state_store_agg()`：读取状态仓库
  - `smart_get_last()` / `smart_refresh()`：SMART 专用接口
 - 开发流程约束：先完成采集器（含单测与 CLI 验证）→ 打通事件广播 → 最后对接 Tauri UI，UI 不得反向阻塞采集主循环。

### 5.5 并发与鲁棒性
- 各 Runner 内部非阻塞缓存快照；Scheduler 只读取快照，不长时间阻塞主循环。
- 严格避免所有权/借用问题：
  - 聚合前先复制/计算需要的统计，再 move 到快照结构。
- 错误处理：
  - Runner 出错不影响主循环；记录 `last_error`，并在前端展示徽章。
  - 事件广播失败不 panic（忽略或重试）。

## 6. 前端 Debug 页（Vue3）
- 区块：
  - 配置查看/热更新（预设按钮：low/normal/high）
  - 调度器状态（启停、every 设置、一次性触发）
  - StateStore TickTelemetry / Aggregated（KPI 展示）
  - SMART 健康（最新快照+统计）
- KPI 展示：
  - NetIf：速率、错误率、丢包率、连接数
  - Disk：读/写吞吐、IOPS、队列长度
  - RTT：avg/min/max/成功率（成功/总数）
  - SMART：ok/fail/连续失败/最近 ok/fail
- 轻量历史可选：近 60 条 RTT avg、IOPS 小折线图（不引大型图表库）。
 - 兼容性：Win10/Win11 下字段可用性差异要在 UI 标注（灰显/提示）。

### 6.5 前端交互逻辑与状态流转
- 状态源：
  - 实时：订阅 `sensor://agg`/`sensor://smart` 更新展示。
  - 请求式：通过命令拉取 `get_state_store_*` 与 `get_config`/`get_scheduler_state`。
- 典型交互：
  - 启停任务：调用 `set_task_enabled(kind, enabled)` → 成功后 UI 立即反映，同时等待下一次 `get_scheduler_state` 或事件心跳确认。
  - 调整频率：`set_task_every(kind, every)` → 立即更新状态面板与后续事件。
  - 一次触发：`trigger_task(kind)` → 在“最近一次运行时间/耗时”字段更新。
  - 配置热更新：`cmd_cfg_update({ patch })` → 成功 toast + 本地缓存更新。
- 异常与告警：
  - 当 `last_error`/连续失败出现时，在 UI 显示徽章与提示；不阻塞其他区域渲染。
- 伪代码（TypeScript）：
  ```ts
  onMounted(() => {
    tauri.listen('sensor://agg', (e) => state.agg = e.payload)
    tauri.listen('sensor://smart', (e) => state.smart = e.payload)
    refreshConfigAndSched()
  })
  async function refreshConfigAndSched() {
    state.config = await cmd.get_config();
    state.sched = await cmd.get_scheduler_state();
  }
  ```

## 7. 配置（Config）
- 结构：
  - `interval_ms`（>=300）
  - `pace_rtt_multi_every`、`pace_net_if_every`、`pace_logical_disk_every`、`pace_smart_every`
  - `rtt_timeout_ms`、rtt 目标列表
  - 可扩展项：阈值（RTT 成功率、磁盘队列长度、SMART 连续失败）
- 热更新：PATCH 合并 + 持久化（JSON/TOML），更新后立刻生效。
 - 版本：`config_ver` 字段；兼容策略为“新字段可缺省，旧字段不变更语义”。

## 8. 测试（Testing，采集器优先与每指标单测）
- 单元测试：
  - 每个 Runner 为每个公开指标提供最小单测：
    - 值域/单位正确性（ms/bps/iops/% 等）
    - 空平台/权限不足的 None/回退路径覆盖
    - 异常不阻塞主循环（错误隔离）
  - 聚合统计函数（RTT 平均/最小/最大/成功率，IOPS 汇总等）
  - 配置补丁合并与校验
- 集成测试（可脚本化）：
  - 模拟 Runner 快照与事件广播，前端通过命令验证状态与事件。
- 边界：
  - 空数据、超时、NaN/Inf、异常路径
 - Mock 策略：
   - WMI/PDH/IOCTL 等系统接口用 trait 抽象并注入 mock 实现，CI 中用 mock 保证可重复测试。
 - CI 门禁：
   - 每新增指标/字段必须带单测（阈值：diff 覆盖率不下降），Windows 上 cargo check/测试通过；前端 typecheck 与 build 必须通过。

## 9. 性能与稳定性（Perf & Stability）
- 聚合与事件广播在 500-1500ms tick 内稳定完成
- 低内存占用（历史缓存上限、按需分配）
- 错误隔离、可恢复
 - 任务注册与释放：确保重复 init/teardown 的幂等（主进程与子任务）。

## 10. 交付件（Deliverables）
- 完整可运行项目（前后端）
- README：
  - 开发/运行/打包指令
  - 配置说明、API/事件说明、字段单位
  - 常见问题与故障排查
- 文档：`doc/progress.md` 用于记录每次进展
- 自动化脚本（可选）：一键构建与测试

## 11. 验收标准（Acceptance Criteria）
- 本地启动后：
  - 调试页面可显示所有 KPI，并可进行任务启停、频率设置、一次性触发
  - `sensor://agg` 每个 tick 正常推送，包含上述 `Aggregated` 全部字段（缺省为 None）
  - SMART 定期采集并推送 `sensor://smart`；手动刷新可用
  - 配置热更新生效，且持久化
  - 任何 Runner 故障不影响主循环运行
- 代码质量：通过编译与基本测试；模块清晰，注释充分
 - 指标覆盖：上述指标目录中的必选项均实现且单测通过；可选项在不支持平台上返回 None，并在 UI 标注。

## 12. 实施步骤（建议，采集器优先）
1) 采集器阶段：先实现 Runner 抽象与 Registry、各系统接口适配层（WMI/PDH/IOCTL/WinAPI），逐项完成 CPU/Memory/GPU/NetIf/LDisk/SMART/Battery/Mobo 指标及其单测
2) 核心后端：StateStore/聚合、Scheduler（含防重复注册）、Bus/事件、Config/热更新
3) 事件验收：提供 CLI 或最小 Tauri 后端，仅通过命令与事件核对聚合输出
4) 前端对接：实现 Debug.vue，订阅 `sensor://agg` 与 `sensor://smart`，完成 KPI 展示与控制
5) 历史与阈值：加入 ring buffer 与告警样式（可配置阈值）
6) 文档与CI：完善 README/进度记录，开启覆盖率门禁

## 13. 质量与规范（Conventions）
- 字段单位在注释中明确（ms、bps、iops、%）
- 所有聚合字段使用 `Option<T>` 以保持兼容与缺省安全
- 避免在 move 后再借用（先计算/clone，再 move）
- 错误处理不 panic，记录日志并继续
  - 主进程不可重复注册任务：任何注册点必须通过 Registry/OnceCell 守护；重复调用返回错误并打点日志。

## 14. 名词与术语（Glossary）
- `Runner`：采集器抽象，负责按需产出快照。
- `Scheduler`：按 tick 调度 Runner 执行与节流的组件。
- `tick`：主循环节拍，单位 ms，由 `interval_ms` 控制。
- `Snapshot`：一次采集的原始结果（各 Runner 专属 Schema）。
- `Aggregated`：跨 Runner 聚合后的统一视图，供事件广播与前端展示。
- `should_run`/`*_every`：按 N 个 tick 的倍频运行策略。
- `once trigger`：一次性触发机制，不改变倍频设置。
- `self-heal`（自愈）：在异常/空闲/周期场景下重建采集资源，避免长期失效。
- `Option<T>`：可缺省字段的约定，前端需容忍 None。
- 单位简称：`ms` 毫秒、`bps` 比特每秒、`iops` 每秒 I/O、`pct` 百分比、`mb` 兆字节、`mhz` 兆赫、`w` 瓦、`v` 伏。

---
请基于上述要求从零实现完整工程，并在每个关键里程碑更新 `doc/progress.md`。实现过程中，如需调整字段或新增能力，请保持 `sensor://agg` 事件的兼容性（新增字段仅追加，不修改既有字段语义）。

## 附录 A：C# Sensor Bridge 模块说明（现存实现参考）

本附录用于梳理当前仓库中 `sensor-bridge/` 的 C# 采集桥接实现，帮助在重写 Rust 方案时对齐能力边界与行为语义。以下内容来自对如下文件的代码审阅与运行行为总结：
- `sensor-bridge/Program.cs.current`
- `sensor-bridge/SensorMonitor.cs`
- `sensor-bridge/DataCollector.cs`
- `sensor-bridge/DataModels.cs`
- `sensor-bridge/HardwareManager.cs`
- `sensor-bridge/SensorUtils.cs`
- `sensor-bridge/ConfigurationManager.cs`
- `sensor-bridge/LogHelper.cs`
- `sensor-bridge/TestRunner.cs`

### A.1 模块职责
- 通过 LibreHardwareMonitor 打开并维护硬件监控句柄（Computer）。
- 每秒更新硬件树，采集 CPU/主板/GPU/风扇/电压/存储温度等指标。
- 将快照序列化为 JSON 并写入 stdout（供上游进程消费）。
- 异常与空闲超时的“自愈”重建；可选周期性重建。
- 结构化日志输出到 stderr 和可选文件，含节流与错误计数。

### A.2 架构与主循环
- 入口：`Program.Main()` 或 `SensorMonitor.Run()` 封装主循环。
- 硬件：`HardwareManager.MakeComputer()` 启用 CPU/Gpu/Storage/Mainboard/SuperIO。
- 采集：`DataCollector.*` 负责各类指标汇集与过滤、名称映射与启发式选择。
- 工具：`SensorUtils.*` 做传感器存在性判断、核心索引解析、风扇控制识别、存储温度友好名映射。
- 配置与日志：`ConfigurationManager`、`LogHelper` 负责 env 读取、日志落地与节流。
- 自愈策略：见 A.5。

主循环（每 ~1s）：
1) `computer.Accept(UpdateVisitor)` 刷新传感器树
2) 采集：CPU 温度/每核负载/频率、主板温度、风扇、存储温度、GPU 组、CPU 额外信息、主板电压
3) 构建 JSON 负载并 `Console.WriteLine()`
4) 记录摘要日志（按 tick 间隔）、必要时 dump 传感器信息
5) 根据异常计数/空闲阈值/周期重开策略重建 Computer

### A.3 对外接口与输出
- 进程对外唯一接口：标准输出的逐秒 JSON 行（stdout line-oriented）。
- 标准错误（stderr）用于日志；可选写入文件（见 `BRIDGE_LOG_FILE`）。
- 进程退出码：异常未吞掉时按 .NET 默认；平稳运行时常驻。

样例 JSON（字段按可用性为可空/可缺省）：
```json
{
  "cpuTempC": 62.1,
  "moboTempC": 48.5,
  "sinceReopenSec": 17,
  "gpus": [
    { "name": "RTX 3070", "tempC": 65.2, "loadPct": 34.4, "coreMhz": 1650,
      "fanRpm": 1200, "powerW": 140.5, "vramUsedMb": 2048, "vramTotalMb": 8192 }
  ],
  "fans": [ { "name": "CPU Fan", "rpm": 980 } ],
  "storageTemps": [ { "name": "Samsung SSD 970", "tempC": 43.0 } ],
  "cpuPerCore": {
    "loadsPct": [12.2, 7.9, 5.1, 4.7],
    "clocksMhz": [4200, 4200, 4100, 4100],
    "tempsC": [62.0, 58.0, 57.0, 56.5]
  },
  "cpuExtra": { "pkgPowerW": 38.4, "avgCoreFreqMhz": 4150, "throttleActive": false, "throttleReasons": [] },
  "moboVoltages": [ { "name": "+12V", "voltage": 12.08 } ]
}
```

说明：字段来源于 `DataModels.cs` 与 `DataCollector.cs` 的组合，缺失/不支持时省略或置空。

### A.4 数据模型（`sensor-bridge/DataModels.cs`）
- `StorageTemp { name, tempC }`
- `CpuPerCore { loadsPct[], clocksMhz[], tempsC[] }`
- `FanInfo { name, rpm }`
- `VoltageInfo { name, voltage }`
- `GpuInfo { name, tempC, loadPct, coreMhz, fanRpm, powerW, voltageV?, vramUsedMb?, vramTotalMb?, encPct?, decPct? }`
- `CpuExtra { pkgPowerW?, avgCoreFreqMhz?, throttleActive?, throttleReasons[]? }`

字段大多为可选，具体取决于平台/驱动传感器可见性。

### A.5 自愈与运行时配置（环境变量）
- `BRIDGE_SELFHEAL_IDLE_SEC`（默认 300，范围 30-3600）：在“长时间未产生有效数据/无变化”时触发重建。
- `BRIDGE_SELFHEAL_EXC_MAX`（默认 5，范围 1-100）：连续异常计数上限，超限重建。
- `BRIDGE_PERIODIC_REOPEN_SEC`（默认 0 关闭，范围 0-86400）：到期强制重建。
- `BRIDGE_SUMMARY_EVERY_TICKS`（默认 60）：摘要日志节流（每 N tick 1 次）。
- `BRIDGE_DUMP_EVERY_TICKS`（默认 0 关闭）：周期性 dump 传感器树到 stderr。
- `BRIDGE_LOG_FILE`：可选日志文件路径；若设置则双写 stdout/stderr 之外的文件日志。

行为要点：
- 自愈动作为关闭并重新创建 `Computer`；重置异常计数与时间基准。
- 周期重建与异常/空闲触发互斥按先后次序评估，避免抖动。
- 所有重建均记录 Info 级摘要与 Debug 细节。

### A.6 日志（`LogHelper.cs`/`ConfigurationManager.cs`）
- 级别：Debug/Info/Warning/Error/Fatal；支持节流（防刷屏）。
- 介质：stderr 为主，文件为辅（若配置）。
- 计数：异常次数、上次重建时间、摘要心跳等关键指标定期输出。

### A.7 测试要点（`TestRunner.cs`）
- 覆盖：硬件初始化、数据采集、监控主循环、配置读取、传感器类型用例。
- 产物：带成功/失败/错误详情与耗时的 JSON 报告。
- 目标：验证异常隔离与自愈有效，字段单位/取值域正确，不支持平台路径返回 None/省略。

### A.8 端到端流程（E2E）
1) 进程启动 → 读取 env → 初始化日志/阈值
2) 创建 `Computer` → 启用硬件类型 → 首次 `Update` 全量刷新
3) 进入 1s tick：更新 → 采集 → 构建 JSON → 写 stdout
4) 按节流输出摘要日志；如配置，定期 dump 传感器树
5) 若异常累计超阈/空闲超时/周期到期 → 关闭并重建 `Computer`
6) 常驻运行，直至被外部终止

### A.9 与重写版对齐建议
- 字段语义尽量保持一致；新增字段仅追加，不改变旧字段含义。
- 以 Option/None 兼容不可用平台，避免阻塞主循环。
- 自愈与日志阈值映射到 Rust 配置项，默认值与范围与现实现对齐。

## 15. 现状对照表（Repository vs Proposed）
下表对比当前仓库与“目录结构（第4节）”提案，标注“已存在/缺失/需调整”。仅列关键项：
- 前端 `src/`
  - 已存在：`/views/About.vue`、`/views/Details.vue`、`/views/Settings.vue`、`App.vue`、`router/index.ts`
  - 缺失：`/views/Debug.vue`（需新增调试页）、`main.ts`（若存在请确认初始化逻辑是否完备）
- 后端 `src-tauri/`
  - 已存在：`Cargo.toml`、`tauri.conf.json`、`icons/`、`capabilities/default.json`、`resources/sensor-bridge/`、一个 `*.rs` 主文件
  - 缺失：`src/` 下模块化文件：`lib.rs`（或 `main.rs` 与模块拆分）、`scheduler.rs`、`state_store.rs`、`config.rs`、`api.rs`、`bus.rs`、`runners/*`、`smart_worker.rs`、`utils/*`
  - 需调整：若当前仅 `main.rs`，请按第4节拆分为模块化结构
- C# Bridge `sensor-bridge/`
  - 已存在：`ConfigurationManager.cs`、`DataCollector.cs`、`DataModels.cs`、`HardwareManager.cs` 等核心文件与 `*.csproj`
  - 缺失：无（按附录 A 视为功能完整，后续仅需与 Rust 接口对齐）
- 文档 `doc/`
  - 已存在：`plan.md`、`plan-async-scheduler.md`、`progress.md`、`rewrite-from-scratch-prompt.md`
  - 缺失：可选 `sensor-bridge-module.md`（更详尽字段与样本）
- 脚本 `doc/script/`（现有）→ 建议迁移/补充为根目录 `/scripts/`：`dev.ps1`、`build.ps1`、`test.ps1`
- 测试 `tests/`
  - 缺失：建议新增 `/tests/backend` 与 `/tests/frontend`

迁移建议：优先完成 `src-tauri/src` 的模块化拆分与 `src/views/Debug.vue`，以便打通端到端数据流与调试能力。

## 16. UI/系统托盘与窗口/动画规范
为适配 Win10 开发与运行，前端需实现以下能力，并在 Tauri 后端配套：
- 系统托盘（System Tray）
  - 要求：任务栏区域常驻图标，左键打开主窗口/悬浮窗，右键打开菜单（启动/暂停采集、刷新 SMART、打开设置、退出）。
  - 后端：`src-tauri/src/tray.rs` 定义托盘与菜单，事件回调分发到前端或直接调用 commands。
  - 前端：监听 `tray://*` 自定义事件或通过命令拉取状态并更新 UI。
- 贴边窗口（Edge-Anchored Window）
  - 要求：可吸附屏幕边缘，进入“贴边收起/展开”两态；可在配置中选择边（上/下/左/右）。
  - 后端：在 `tauri.conf.json` 或运行时设置无边框、置顶、可拖拽；提供命令 `set_window_edge(side)`、`toggle_edge_collapsed()`。
  - 前端：`/views/Debug.vue` 或独立窗口 `edge` 实例，渲染精简 KPI，支持鼠标悬停/点击展开动画。
- 悬浮状态窗口（Floating Status Window）
  - 要求：小型 Always-On-Top 无边框窗体，可拖拽；展示关键 KPI（CPU、NetIf、LDisk、RTT、SMART 汇总）。
  - 后端：`create_window('floating', {...})` 按需创建；暴露命令 `show/hide/move/resize`。
  - 前端：`/views/Floating.vue`，订阅 `sensor://agg` 并以极简渲染；支持透明背景。
- 事件触发动画（Event-driven Animations）
  - 要求：所有窗口均支持根据事件状态触发动画（如阈值告警、任务启停、SMART 连续失败）。
  - 前端：
    - 动画实现采用 CSS 动画 + Web Animations API；统一事件总线将状态变化映射为动画触发器。
    - 约定事件：`ui://alert/high`、`ui://task/start|stop`、`ui://smart/fail` 等，对应添加/移除 CSS class 或调用 `element.animate(...)`。
- 自定义脚本触发动画（User Scripts）
  - 要求：允许用户以受限脚本（例如沙箱化 JS 片段）订阅特定事件并触发动画/样式更改。
  - 前端：提供“脚本管理”设置页，存储于本地（如 `tauri::path::app_config_dir`）；运行时在受限沙箱执行，暴露有限 API：`on(event, handler)`、`setStyle(selector, css)`、`animate(selector, opts)`。
  - 安全：严格禁止网络/文件系统访问；超时与异常需隔离不影响主循环。
- 窗口与交互文件规划
  - 前端新增：
    - `src/views/Debug.vue`（调试主窗）
    - `src/views/Floating.vue`（悬浮窗）
    - `src/views/EdgePanel.vue`（贴边窗体，可与 Debug 合并为多窗口实例）
    - `src/store/ui.ts`（事件→动画映射、用户脚本沙箱）
    - `src/styles/animations.css`（统一动画样式）
  - 后端新增：
    - `src-tauri/src/tray.rs`（系统托盘初始化与事件）
    - `src-tauri/src/windows.rs`（多窗口创建/管理，置顶/透明/无边框配置）
    - `src-tauri/src/cmd_ui.rs`（窗口控制 commands：显示/隐藏/移动/尺寸/贴边收起）
  - 配置：
    - `config.ui` 段：`tray_enabled`、`floating_enabled`、`edge_enabled`、`edge_side`、`edge_auto_hide`、`anim_enabled`、`user_scripts_enabled`。
## 17. 数据流向（End-to-End Data Flow）
- 采集链路（周期可配）：
  1) C# Bridge（`sensor-bridge`）按配置周期采样硬件（`config.sampling.intervalMs`，默认 1000ms，可 5000/10000ms）→ 输出一行 JSON 到 stdout。
  2) Rust 后端（`src-tauri`）子进程读取 stdout → 解析为 `BridgeOut`/`SensorSnapshot`。
  3) Scheduler 每 tick 聚合来自各 Runner 的快照 → 写入 `StateStore.Aggregated`。
  4) 后端通过事件 `sensor://agg` 推送聚合 → （若有）`sensor://smart` 推送 SMART 更新。
  5) 前端（Vue）订阅事件更新页面；必要时通过 commands 拉取 `get_state_store_*` 与配置/调度状态。
  - 任务链路：
    - 前端操作（启停/倍频/一次触发/热更新）→ 调用 Tauri commands → 后端更新 `Scheduler`/`Config` → 下一 tick 生效并体现在事件与状态读取中。
  - 采样策略（两种可选，按实现权衡性能与实时性）：
    - 策略A：Bridge 固定较快采样（如 1000ms），后端以 `intervalMs` 聚合/抽样对外广播（例如 UI 设 5000ms 时，每5条聚合一次）。优点：高实时性可选；缺点：CPU/IO 更高。
    - 策略B：Bridge 直接采用 UI/配置周期（如 5000ms），后端按同周期广播。优点：资源占用低；缺点：细粒度波动不可见。
  - 采集形态说明：
    - 当前实现为“独立 C# 进程经 stdout 输出”，由 Tauri/Rust 管理子进程并解析；不是 DLL 直调。
    - 如需 DLL 直调，可将 Bridge 编译为 Class Library 并通过 Tauri 插件/FFI 调用，但需处理托管/非托管边界、STA/MTA、异常隔离与发布体积等问题（本项目当前未采用）。
  - 错误与自愈：
    - Bridge 内部：异常/空闲/周期达到 → 重建 `Computer`；不中断进程。
    - Bridge 子进程退出：后端退避重启；在 `get_scheduler_state()` 与日志中反映。
    - Runner/聚合错误：记录 `last_error` 并跳过，不阻塞主循环；UI 显示徽章。
  - 背压与节流：
    - 事件发送频率受 `interval_ms` 约束；如阻塞，事件可丢弃或合并。
    - 历史 ring buffer 固定容量（默认 60），防止内存膨胀。
  - 序列化风格：
    - 后端对前端事件/命令结果统一 `camelCase`（serde `rename_all = "camelCase"`），便于 TS 直接消费。

## 18. 代码编写与命名规范（Conventions）
- Rust（后端）：
  - 文件/模块：`snake_case`（如 `state_store.rs`、`smart_worker.rs`）。类型/trait：`PascalCase`；函数/变量：`snake_case`。
  - JSON/事件：结构体使用 `#[serde(rename_all = "camelCase")]`；时间戳用 `*_ms`（i64 毫秒），比特率 `*_bps`（f64），百分比 `*_pct`（0-100）。
{{ ... }}
  - 事件命名：`sensor://agg`、`sensor://smart`、UI 导航或托盘可用 `ui://*` 自定义命名空间。
  - 错误处理：`anyhow::Result`/自定义错误枚举+`thiserror`；不得 `panic!` 影响主循环。
  - 日志：`tracing` 框架（建议），级别 Debug/Info/Warn/Error；关键路径记录耗时与计数。
- C#（Bridge）：
  - 类/属性：`PascalCase`；局部变量：`camelCase`；JSON 输出字段统一 `camelCase`（与前端一致）。
  - 自愈与日志：环境变量 `BRIDGE_*`；异常不传播到顶层（尽量吞并并记录）。
- TypeScript/Vue（前端）：
  - 接口与字段：`camelCase`；类型/组件名 `PascalCase`（如 `Floating.vue`）。
  - 事件总线与动画：统一在 `src/store/ui.ts` 管理；CSS 类名 `kebab-case`；关键动画集中于 `src/styles/animations.css`。
  - 目录与文件：视图放 `src/views/`；共享工具/格式化放 `src/lib/` 或 `src/utils/`；避免在组件中直接写复杂逻辑。
- 单位与命名后缀（统一约定）：
  - `*_ms` 毫秒、`*_bps` 比特每秒、`*_iops` 每秒 I/O、`*_pct` 百分比、`*_mb` 兆字节、`*_mhz` 兆赫、`*_w` 瓦、`*_v` 伏。
- 提交与分支（建议）：
  - commit 消息：`feat: ...`、`fix: ...`、`docs: ...`、`refactor: ...`、`test: ...`、`chore: ...`；附带影响范围（backend/frontend/bridge/doc）。
  - 分支：`feature/<topic>`、`fix/<topic>`；PR 描述引用对应文档与章节（如“见第 17 节数据流向”）。
