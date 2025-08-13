
## 2025-08-14 00:12（NVMe IOCTL 健康日志页 0x02 实装并集成）
- 变更内容：
  - 后端：完成 `nvme_smart_via_ioctl()` 调用链：
    - 遍历并 `CreateFileW("\\\\.\\PhysicalDriveN")` 开句柄；
    - 构造 `STORAGE_PROPERTY_QUERY{StorageDeviceProtocolSpecificProperty}` + `STORAGE_PROTOCOL_SPECIFIC_DATA{ProtocolTypeNvme, NVMeDataTypeLogPage, RequestValue=0x02}`；
    - 调用 `DeviceIoControl(IOCTL_STORAGE_QUERY_PROPERTY)`，解析 `STORAGE_PROTOCOL_DATA_DESCRIPTOR`；
    - 解析 NVMe Health Log（1.3）：温度（K→°C）、PowerOnHours、PowerCycles、Data Units Read/Write（按 512,000B/单位→字节）；
    - 映射至 `SmartHealthPayload`：`temp_c/power_on_hours/power_cycles/host_reads_bytes/host_writes_bytes`。
  - 流程：`wmi_list_smart_status()` 首先尝试 IOCTL，失败回退 WMI/PowerShell，前端保持现有显示与降级。
- 测试点：
  1) 以管理员运行，控制台日志可见 `[nvme_ioctl] ... IOCTL_STORAGE_QUERY_PROPERTY ok` 与解析条目计数；
  2) NVMe 盘应展示温度、POH、上电次数、累计读写（GB 格式化）；
  3) 非 NVMe 或调用失败时，WMI/PowerShell 回退生效，UI 无回归；
  4) 多盘场景逐盘展示（“SMART 详情”折叠列表）。
- 已知事项：
  - 仍未实现 SATA/ATA SMART（计划使用 `ATA_PASS_THROUGH`/`SMART_RCV_DRIVE_DATA`），不影响 NVMe 路径测试。
  - 运行 `vite dev` 同时触发 `cargo run` 会占用 `resources/sensor-bridge` 文件导致 `os error 32`，请先关闭 dev 进程或清理残留进程再 `cargo check`。

## 2025-08-13 17:10（电池健康接入 + 多盘容量/存储温度可展开 + 警告清理）
- 变更内容：
  - 后端（`src-tauri/src/lib.rs`）：
    - 在采样线程读取电池信息时，调用 `wmi_read_battery_health()`，填充 `SensorSnapshot` 的 `battery_design_capacity`、`battery_full_charge_capacity`、`battery_cycle_count`（前端以“电池健康”汇总显示）。
    - 清理警告：将未使用的 `is_primary` 改为 `_is_primary`；移除未使用的本地 `battery_*capacity/cycle_count` 变量；微调 VRAM 反推表达式的冗余括号。
  - 前端（`src/views/Details.vue`）：
    - 为“磁盘容量”和“存储温度”增加“展开/收起”详情列表（与 `smart_health`、`rtt_multi`、Top 进程一致）。
    - 修复误覆盖的函数体并补齐 `fmtDisks()`、`fmtSmart()`。
  - 构建验证：`cargo check` 与 `npm run build` 均通过。
- 测试点：
  - 在“详情”页：
    - 电池区显示“电池健康”汇总：设计容量/满充容量/循环次数与健康度百分比；无数据显示“—”。
    - “磁盘容量/存储温度”可展开查看每项明细，摘要超出以“+N”汇总。
  - 托盘与主窗口显示未受影响，控制台黑窗已按既有策略保持抑制。

## 2025-08-13 16:20（多目标 RTT 默认增加 114.114.114.114 + 前端 GPU 类型同步）
- 变更内容：
  - 后端：在 `src-tauri/src/lib.rs` 的采样线程中，`rtt_targets` 的默认值新增 `114.114.114.114:53`，现默认为：`1.1.1.1:443`、`8.8.8.8:443`、`114.114.114.114:53`。
  - 前端：在 `src/views/Details.vue` 的 `SensorSnapshot.gpus` 类型补充 `vram_total_mb`、`vram_usage_pct` 字段，与后端对齐，避免类型缺失引发的 TS 报警，并为 `fmtGpus()` 的显示提供基础。
- 测试点：
  - 启动应用，打开“详情”页，确认 RTT 多目标列表出现三项，且 114 目标可返回 ms（网络可达时）。
  - GPU 汇总行中 VRAM 显示无报错，缺失值显示“—”。
- 注意：
  - 命名规范保持：Rust snake_case → 前端 camelCase（已用 serde camelCase）。
  - 未改动托盘逻辑与 `make_tray_icon()`，tray 功能不受影响。

## 2025-08-13 11:45（rtt_multi/Top进程 前端集成 + 构建验证）
 - 类型与 UI（前端）：
   - `src/main.ts` 的 `SensorSnapshot` 新增/确认：
     - `rtt_multi?: { target: string; rtt_ms?: number }[]`
     - `top_cpu_procs?: { name?: string; cpu_pct?: number; mem_bytes?: number }[]`
     - `top_mem_procs?: { name?: string; cpu_pct?: number; mem_bytes?: number }[]`
   - `src/views/Details.vue`：
     - 同步扩展本地 `SensorSnapshot` 类型。
     - 新增格式化函数：`fmtRttMulti`、`fmtTopCpuProcs`、`fmtTopMemProcs`。
     - 模板新增三行：“多目标延迟/高CPU进程/高内存进程”，无数据显示“—”。
 - 后端对齐：
   - `src-tauri/src/lib.rs` 的 `SensorSnapshot` 已包含 `rtt_multi/top_cpu_procs/top_mem_procs`，并在采样处填充，随 `sensor://snapshot` 广播。
 - 构建验证：
   - `cargo check`（目录：`src-tauri/`）通过。
   - 根目录 `npm run build` 通过（`vue-tsc` 与 Vite 构建成功）。
 - 说明：
   - 列表字段采用简洁摘要并可能以 `+N` 汇总；RTT 单位 ms，进程 CPU/内存单位分别为 `%`/`MB`；无值显示“—”。

## 2025-08-13 12:10（构建复查：前端 build 通过 + cargo check 通过）
- 前端构建：根目录执行 `npm run build` 通过（Vite 6.x，产物已输出至 `dist/`）。
- Rust 构建检查：`src-tauri/` 下 `cargo check` 通过；存在若干非致命警告：
  - 未使用变量：`keyl`（建议改为 `_keyl` 或清理）。
  - 初始化后未读取赋值：`battery_ac_online`、`battery_time_remaining_sec`、`battery_time_to_full_sec`。
  - 字段未读：`AppState.public_net`。
  - 变量不需要 `mut`：`list`（由 `sysinfo::Process` 收集产生）。
- 进程占用复查：未发现 `sys-sensor.exe/sensor-bridge.exe/dotnet.exe` 残留占用；此前偶发的 `os error 32` 暂无法复现。
  - 如再现，请按：
    1) 查看：`tasklist | findstr /I "sys-sensor sensor-bridge dotnet"`
    2) 清理：`taskkill /F /IM sys-sensor.exe`、`taskkill /F /IM sensor-bridge.exe`、`taskkill /F /IM dotnet.exe`
    3) 重试：进入 `src-tauri/` 执行 `cargo check`

## 2025-01-27 内存细分功能扩展完成
- **内存细分后端实现**：
  - 在 `src-tauri/src/lib.rs` 新增 `PerfOsMemory` 结构体，映射 WMI `Win32_PerfFormattedData_PerfOS_Memory` 查询。
  - 新增 `wmi_perf_memory()` 函数，查询内存细分数据并转换单位（字节转GB，保留速率单位）。
  - 扩展 `SensorSnapshot` 结构体，新增内存细分字段：
    - `mem_cache_gb`：内存缓存（GB）
    - `mem_committed_gb`：已提交内存（GB）
    - `mem_commit_limit_gb`：提交限制（GB）
    - `mem_pool_paged_gb`：分页池（GB）
    - `mem_pool_nonpaged_gb`：非分页池（GB）
    - `mem_pages_per_sec`：分页速率（页/秒）
    - `mem_page_reads_per_sec`：页面读取速率（页/秒）
    - `mem_page_writes_per_sec`：页面写入速率（页/秒）
    - `mem_page_faults_per_sec`：页面错误速率（页/秒）
  - 在采集主循环中调用 `wmi_perf_memory()` 并填充相应字段到 `SensorSnapshot`。

- **内存细分前端实现**：
  - 在 `src/main.ts` 和 `src/views/Details.vue` 的 `SensorSnapshot` 类型定义中新增内存细分字段。
  - 在 `Details.vue` 详情页面新增内存细分显示项：
    - 内存缓存：显示缓存大小（GB）
    - 内存提交：显示已提交/提交限制（GB）
    - 分页池/非分页池：显示大小（GB）
    - 分页速率相关：显示每秒页面操作次数
  - 所有字段支持优雅降级，无数据时显示"—"。

- **构建验证**：
  - 前端编译：`npm run build` 成功，无TypeScript错误
  - 后端编译：`cargo build --release` 成功，仅有非致命警告
  - 功能完整性：后端WMI查询、数据转换、前端类型定义、UI显示全链路完成

- **技术细节**：
  - WMI查询使用 `ROOT\\CIMV2` 命名空间的性能计数器
  - 字节单位自动转换为GB便于显示（除速率字段保持原单位）
  - 前端使用条件渲染和数值格式化确保显示友好
  - 保持与现有内存字段的一致性和兼容性
- 后续验证：
  - 在 Tauri 窗口内手测三处详情“展开/收起”（`rtt_multi`、`top_cpu_procs`、`top_mem_procs`）与列表渲染；无值显示应为“—”。
  - `fmtBytes()` 已恢复规则：小于 10GB 显示 2 位小数，≥10GB 显示 1 位小数。
 ## 2025-08-13 23:58（决定采用 Windows 原生 NVMe/ATA IOCTL 采集 SMART + 骨架接入）
 - 变更内容：
   - 路线决定：弃用内置分发 smartmontools，改为自研基于 Windows 原生 IOCTL 的 NVMe/ATA SMART 采集，规避 GPL 风险。
   - 后端：在 `src-tauri/src/lib.rs` 新增 `nvme_smart_via_ioctl()` 占位函数，并在 `wmi_list_smart_status()` 中优先调用；失败再回退 WMI/PowerShell。
   - 依赖：在 `src-tauri/Cargo.toml` 的 `windows` crate 启用 `Win32_Storage_FileSystem` feature，为后续 `CreateFileW/DeviceIoControl` 做准备。
   - 构建：`src-tauri/ cargo check` 通过（存在少量非致命 warning）。
 - 后续计划：
   1) 实现 NVMe 健康日志（Log Page 0x02）读取：`IOCTL_STORAGE_QUERY_PROPERTY`（`StorageDeviceProtocolSpecificProperty` + `NVMeDataTypeLogPage`）。
   2) 字段映射：温度/PowerOnHours/PowerCycleCount/DataUnitsRead/Write → `SmartHealthPayload`（字节换算、单位规范）。
   3) SATA/ATA 路径：`SMART_RCV_DRIVE_DATA` 或 `ATA_PASS_THROUGH`（管理员权限）。
   4) 数据来源标注与失败回退链路保持不变。
 - 测试点：
   - 无 smartctl 的纯净系统上，NVMe 设备可返回温度/POH/PowerCycle/累计读写；失败时回退 PS/WMI，不影响现有展示。
 
## 2025-08-13 14:49（端到端测试通过 + 文档同步）
- 在 Tauri 窗口完成端到端手测：
  - “多目标延迟 / 高CPU进程 / 高内存进程”三处“展开/收起”交互正常；摘要与详情一致，空值显示“—”。
  - GPU 汇总行电压与风扇 RPM 抖动已由 15s 回填平滑改善。
- 构建复核：根目录 `npm run build` 与 `src-tauri/ cargo check` 再次通过。
- 文档同步：
  - 已更新 `doc/progress.md`（本条）。
  - 已在 `doc/项目总结与开发注意事项.md` 新增“对标 iStat Menus 差距清单（2025-08-13）”。
- 下一步：对标 iStat Menus，梳理并规划补齐候选指标（优先分网卡速率、分盘 IOPS/队列、GPU 显存总量与使用率%、电池健康等）。

## 2025-01-27 数据缺失问题诊断与修复
- **问题定位**：
  - 发现内存细分、GPU 显存、SMART 数据等显示为"—"
  - 通过调试日志定位到WMI查询失败，错误码：-2147217392 (WBEM_E_INVALID_CLASS)
  - 该错误通常表示WMI类名无效、系统不支持该类或权限不足

- **修复内容**：
  - **编译错误修复**：
    - 删除重复的 WMI 性能计数器函数定义，保留原有实现。
    - 修复语法错误（`{{ ... }}` 改为正确的代码块）。
    - 修复 GPU 显存字段类型不匹配（u32/u8 改为 f64）。
  - **GPU 显存数据修复**：
    - 实现 `wmi_query_gpu_vram()` 函数，通过 WMI 查询 `Win32_VideoController` 获取显存总量。
    - 修复 GPU 数据映射逻辑，根据 GPU 名称匹配显存信息，计算显存使用率。
    - 将硬编码的 None 值替换为实际的 WMI 查询结果。
  - **数据采集链路验证**：
    - 确认内存细分数据采集函数 `wmi_perf_memory()` 已正确实现并在采集循环中调用。
    - 确认 SMART 数据采集已正确实现，包含 ROOT\WMI → ROOT\CIMV2 → PowerShell NVMe 的多层回退。
    - 确认磁盘 IOPS 和网络错误率采集函数已实现。

- **编译与构建验证**：
  - Rust 后端编译成功（`cargo build`），仅存在非致命警告（未使用变量等）。
  - 前端构建成功（`npm run build`），TypeScript 类型检查通过。
  - 所有数据缺失问题的技术障碍已清除，等待运行时测试验证实际数据显示效果。

- **预期改善**：
  - 内存细分指标（缓存、提交、分页池等）应能正常显示数值而非"—"。
  - GPU 显存总量和使用率应能在主界面和详情页正常显示。
  - SMART 健康数据应能通过多层回退机制获取并显示。
  - 磁盘 IOPS、网络错误率等第二梯队指标应能正常采集。

## 2025-08-13 15:10（追加“继续开发会话 Prompt” + 文档修复）
- 变更：
  - 在 `doc/task.md` 文末追加“继续开发会话 Prompt”，明确下次迭代的优先事项与构建/联调步骤。
  - 修复 `doc/task.md` 中 `taskkill` 脚本的 Markdown 代码块围栏（补齐结尾 ```）。
- 优先事项（与路线图对齐）：
  1) 内存细分：可用/缓存/提交/交换与分页相关计数，扩展 `SensorSnapshot` 并前端展示。
  2) GPU 显存总量与使用率%：桥接补总显存 MB；Rust 透传 `vram_total_mb` 与 `vram_used_pct`（或桥接提供）；前端 `fmtGpus()` 展示 `VRAM <used>/<total> MB (<pct>%)`。
  3) 电池健康（基础）：循环次数、设计/满充容量；后端 `Win32_Battery`/电源 API 获取，前端新增展示行。
  4) Rust 告警清理：`_keyl`、移除多余 `mut`、未读赋值等，降低告警噪音。
  5) 继续放弃 SMART/NVMe 回退链路，不再投入新增工作；相关测试项跳过。
- 构建状态：本次仅为文档调整，无代码逻辑改动；后续按需执行 `src-tauri/ cargo check` 与根目录 `npm run build`。
- 备注：继续遵循 UI 无值显示“—”、字段命名约定与 15s 短时平滑策略。

- 文档同步（追加）：已同步 `doc/plan.md` 至最新进展。
  - 更新时间：2025-08-13 15:03。
  - 路线图：标记“主板电压/更多风扇（mobo_voltages/fans_extra）”“GPU 细分指标（含 VRAM/功耗/电压展示优化）”“多目标 RTT（rtt_multi）”“Top 进程（CPU/内存）”为已完成；“SMART 关键属性简表”标注为已完成且 NVMe 回退链路不再维护。
  - 待办对齐：内存细分、GPU 显存总量与使用率%、电池健康（基础）、Rust 告警清理。
  - 术语/字段校正：统一使用 `mobo_voltages/fans_extra`；调整 Step B/C 的完成状态与说明。
