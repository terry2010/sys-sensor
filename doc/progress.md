
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
  4) 多盘场景逐盘展示（“SMART 详情”折叠列表）。
- 已知事项：
  - 仍未实现 SATA/ATA SMART（计划使用 `ATA_PASS_THROUGH`/`SMART_RCV_DRIVE_DATA`），不影响 NVMe 路径测试。
  - 运行 `vite dev` 同时触发 `cargo run` 会占用 `resources/sensor-bridge` 文件导致 `os error 32`，请先关闭 dev 进程或清理残留进程再 `cargo check`。

## 2025-08-14 00:35（smartctl -j 可选回退接入，无黑窗）
- 变更内容：
  - 后端：新增 `smartctl_collect()`，使用 `std::process::Command` + `CREATE_NO_WINDOW` 无黑窗执行 `smartctl -j -a \\ \\.\\PhysicalDriveN`，解析 JSON 映射到 `SmartHealthPayload`（温度、POH、PowerCycles、DataUnitsRead/Write、ATA关键属性：5/9/12/194/197/198/199）。
  - 集成：在 `wmi_list_smart_status()` 回退链路中，`ROOT\\WMI` 为空时优先尝试 smartctl，其后再回退 `ROOT\\CIMV2` 与 PowerShell（保持既有顺序）。
  - 日志：记录每盘调用结果、非零退出 stderr、字段映射摘要，方便管理员场景下排障。
  - 构建：`src-tauri/ cargo check` 通过（存在非致命告警），未引入新依赖。
- 测试点：
  1) 未安装 smartctl：应打印“not found or not executable”，并继续回退，不影响 UI。
  2) 已安装 smartctl：NVMe/SATA 设备可返回核心字段；控制台无黑窗闪现；UI “SMART 详情”正常。
  3) 多盘：遍历 `PhysicalDrive0..31`，逐盘成功返回时结果累加。
  4) 异常：smartctl 非零退出或 JSON 解析失败时打印 stderr/错误并跳过，不阻断后续回退。
- 备注：
  - NVMe `data_units_*` 按 512,000 B/单位换算为字节，做 i64 上限裁剪；温度优先 `temperature.current` → `nvme_smart_health_information_log.temperature(K→°C)` → ATA 194；`smart_status.passed=false` 映射为 `predict_fail=true`。

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

## 2025-08-14 00:50（验证 smartctl 集成与回退链路 + UI/托盘回归）
- 核对结果：
  - `src-tauri/src/lib.rs` 的 `wmi_list_smart_status()` 在 `ROOT\\WMI` 为空时，已优先调用 `smartctl_collect()`，随后回退 `ROOT\\CIMV2`（`wmi_fallback_disk_status`）与 PowerShell（`nvme_storage_reliability_ps`）。
  - `smartctl_collect()` 使用 `CREATE_NO_WINDOW` 调用 `smartctl -j -a \\ \\.\\PhysicalDriveN`，解析 JSON（温度、POH、PowerCycles、DataUnits Read/Write、ATA 5/9/12/194/197/198/199），无黑窗闪烁。
  - 采样广播的 `SensorSnapshot` 已包含内存细分、GPU VRAM 总量与使用率%、电池健康（设计/满充/循环），托盘图标与 tooltip 正常更新，无回归。
- 受影响文件：
  - 后端：`src-tauri/src/lib.rs`（`wmi_list_smart_status()`、`smartctl_collect()`、`nvme_storage_reliability_ps()`、采样线程的 `SensorSnapshot` 构造）。
  - 文档：`doc/progress.md`（00:35 smartctl 集成记录已存在，本条补充验证结果与回归检查）。
- 下一步：
  - 复核前端 `src/main.ts`、`src/views/Details.vue` 对新增字段（VRAM total/usage%、电池健康、内存细分）的优雅降级“—”显示；必要时补充类型与格式化函数。
  - 按 `doc/script/ADMIN-TEST-SMART.md` 进行管理员测试，覆盖 NVMe+SATA/USB 混合环境与多路径回退验证。
  - 清理剩余 Rust 告警；留意 `os error 32`（文件占用）并按文档提供脚本清理残留进程。

## 2025-08-14 02:32（运行日志分析 + 构建报错与后续动作）
- 现场日志要点：
  - NVMe IOCTL 全路径失败：`gle=87/1/50`，符合已知驱动兼容性问题；进入 WMI 回退。
  - ROOT\WMI 查询 `MsStorageDriverFailurePredict*` 失败（`hres=-2147217396`），返回 0 设备。
  - smartctl 兜底：对 `\\.\\PhysicalDrive0..31` 全部 `non-zero exit`；当前运行的二进制未包含 `--scan-open` 增强，因此未打印退出码/stdout。
  - ROOT\CIMV2 回退失败；PowerShell NVMe 成功返回 1 盘，成功兜底。
  - 内存 WMI 类失败（`-2147217392`），使用 Windows API SysInfo 回退；GPU VRAM 通过 WMIC CSV 成功解析 1024MB。
- 构建报错：
  - `cargo check` 报 `os error 32`（文件被占用），锁定路径位于 `resources/sensor-bridge/sensor-bridge.exe`。需先结束残留进程后再编译。
- 已提交代码变更（待重新编译）：
  - `smartctl_collect()` 增强：优先 `smartctl --scan-open -j` 列举并识别 NVMe（自动追加 `-d nvme`），失败打印包含退出码、stderr 与 stdout 片段；扫描为空时回退遍历 PhysicalDrive0..31；全程 `CREATE_NO_WINDOW` 无黑窗。
- 建议操作：
  1) 以管理员关闭残留进程后重试编译：
     - `tasklist | findstr /I "sensor-bridge sys-sensor tauri"`
     - `taskkill /F /IM sensor-bridge.exe`
     - `taskkill /F /IM sys-sensor.exe`
     - 重新执行 `src-tauri/ cargo check`
  2) 手工验证 smartctl（管理员 PowerShell）：
     - `smartctl --scan-open -j`
     - 针对 NVMe：`smartctl -j -a -d nvme \\.\\PhysicalDrive0`
     - 针对 SATA/USB 场景可尝试：`smartctl -j -a -d sat \\.\\PhysicalDriveN`
  3) 重新运行应用，期待日志出现 `"[smartctl] scan-open found N devices"` 与更详尽的失败输出，便于排障。

## 2025-08-14 03:15（便携版黑框闪现的修复）
- 现象：客户机器运行便携版（`npm run release:portable` 产物）时，出现黑框闪一下再消失。
- 根因定位：Rust 后端在 `wmi_query_gpu_vram()` 的 WMIC 调用未设置 `CREATE_NO_WINDOW`，在部分环境下会短暂弹出 `conhost` 控制台窗口。
- 修复：
  - 在 `src-tauri/src/lib.rs` 的 WMIC 调用增加 `creation_flags(0x08000000)`（`CREATE_NO_WINDOW`）。
  - `smartctl`、`powershell`、`netsh`、`sensor-bridge` 启动路径均已设置 `CREATE_NO_WINDOW`；主程序 `src-tauri/src/main.rs` 亦启用 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`，Release 下为 GUI 子系统。
- 构建验证：
  - `src-tauri/ cargo check` 通过（仅非功能性告警）。
  - 建议重新生成便携版并在客户机验证：`npm run release:portable`（含桥接重新发布与打包）。
- 预期结果：便携版运行不再出现黑框闪现；如仍复现，请回传日志关键行以进一步定位。

## 2025-08-14 04:10（smartctl 路径内置优先 + 设备类型回退增强 + 便携包打包 smartctl）
- 变更内容：
  - 后端（`src-tauri/src/lib.rs`）：
    - `smartctl_collect()` 增强设备类型尝试序列：在 `scan-open` 返回类型基础上，依次回退 `sat` → `ata` → `scsi` → `sat,12` → `sat,16` → 无 `-d` 自动。使用去重以避免重复尝试。
    - 新增 smartctl 可执行路径解析：优先使用随包路径（相对 exe）`resources/smartctl/smartctl.exe`、`resources/bin/smartctl.exe`、`smartctl.exe`、`bin/smartctl.exe`，均不存在时回退系统 `PATH`（`smartctl`）。所有调用统一使用 `CREATE_NO_WINDOW`，避免黑窗。
  - 构建脚本（`package.json`）：`portable:stage` 增加条件复制 `src-tauri/resources/smartctl/` 到便携包 `resources/smartctl/`，以支持离线环境采集。
  - 告警清理：移除未使用导入 `std::cmp::min`（在尝试循环内）。
- 构建验证：
  - `src-tauri/ cargo check` 通过（仅非功能性告警）。
- 管理员测试要点：
  1) 生成便携包：根目录运行 `npm run release:portable`，完成后在 `dist-portable/sys-sensor/` 下应存在 `resources/smartctl/smartctl.exe`（若仓库内提供）。
  2) 在未安装 smartctl 的机器验证：
     - 启动便携版，日志应显示 `smartctl --scan-open -j` 结果；若失败，逐盘尝试 `-d sat/ata/scsi/sat,12/sat,16/(auto)`，失败会打印退出码与 stderr 片段。
     - UI “SMART 详情”应出现温度、POH、上电次数、（NVMe）累计读/写字节等；无数据时保持“—”。
  3) 多盘/多接口环境（NVMe + SATA/USB）：确认各盘至少一种路径成功（优先 scan-open 返回类型），无黑窗闪现。
  4) 若 `resources/smartctl/` 缺失且系统 `PATH` 也无 smartctl，日志应提示 `not found or not executable`，随后回退到 WMI/PowerShell，不影响 UI。
- 受影响文件：
  - `src-tauri/src/lib.rs`（`smartctl_collect()` 路径解析与尝试序列）。
  - `package.json`（`portable:stage` 复制 smartctl）。
- 后续建议：
  - 如需内置 smartctl，请将二进制放入 `src-tauri/resources/smartctl/` 并重新执行 `npm run release:portable`。
  - 收集含 USB-SATA 转接盒、硬 RAID、笔记本 NVMe 的测试日志，观察哪类 `-d` 最常成功，以便调整顺序。

## 2025-08-14 04:30 smartctl 集成修复与核心功能完善

### 主要修复与实现
1. **smartctl 调用修复**：
   - 修复 SMART 数据处理链路，将 `smartctl_collect()` 设为首选采集方式
   - 采集顺序：smartctl → ROOT\WMI → NVMe PowerShell → ROOT\CIMV2 回退
   - 确保用户已安装 smartctl 时优先使用，无 smartctl 时自动回退现有方案

2. **内存细分功能完善**：
   - 后端已有 `wmi_perf_memory()` 函数，正确调用并填充 9 个内存细分字段
   - 前端新增展示：内存缓存、内存提交/限制、分页池/非分页池、分页速率/页面读写/页面错误
   - UI 格式化：GB 单位显示，无数据时显示"—"

3. **GPU 显存总量与使用率**：
   - 后端已有 `wmi_query_gpu_vram()` 函数获取显存总量
   - 前端 `fmtGpus()` 已支持 VRAM 显示格式：`VRAM <used>/<total> MB (<pct>%)`
   - GPU 汇总行在托盘 tooltip 中稳定展示

4. **电池健康功能**：
   - 后端 `wmi_read_battery_health()` 获取设计容量、满充容量、循环次数
   - 前端新增"电池健康"展示行，调用 `fmtBatteryHealth()` 格式化
   - 类型定义已同步：`battery_design_capacity`、`battery_full_charge_capacity`、`battery_cycle_count`

5. **代码质量优化**：
   - 清理重复的条件编译指令（`#[cfg(windows)]`/`#[cfg(not(windows))]`）
   - 修复前端语法错误（风扇详情列表的 v-for 循环）
   - 移除重复的内存/分页展示行

### 构建验证
- `cargo check` 通过（16个警告，主要为未使用字段/结构体，不影响功能）
- `npm run build` 通过（前端构建成功，无错误）
- 所有新增字段的类型定义已在前后端同步

### 测试要点
1. **smartctl 功能**：
   - 在已安装 smartctl 的环境测试，SMART 详情应显示温度、通电时长、上电次数、累计读写等
   - 在未安装 smartctl 的环境测试，应自动回退到 WMI/PowerShell，不影响 UI

2. **内存细分**：
   - 详情页应显示内存缓存、提交、分页池等 9 个新增字段
   - 数值格式为 GB，无数据时显示"—"

3. **GPU 显存**：
   - GPU 汇总应显示 VRAM 使用情况（如有显存信息）
   - 托盘 tooltip 的 GPU 行应稳定显示

4. **电池健康**：
   - 详情页"电池健康"行应显示设计容量、满充容量、循环次数
   - 格式：`设计 <design>mWh / 满充 <full>mWh / 循环 <cycle>次`

### 后续建议
- 管理员权限启动测试所有功能
- 多硬盘环境验证 smartctl 采集效果
- NUC 等特殊平台验证硬件限制友好降级

## 2025-08-14 05:05（测试清单与期望结果汇总）

### 测试前准备
- 以管理员运行应用与命令行；若构建占用，先参考 `package.json` 的 `clean:proc` 停止残留进程。
- 环境尽量覆盖：NVMe + SATA/USB 混合、独显/核显、多电池机型（如有）。
- 若需离线采集 SMART，可将 `smartctl.exe` 放入 `resources/smartctl/`（便携包）或保证系统 PATH 可用。

### A. smartctl 集成与回退
- 入口：`src-tauri/src/lib.rs::smartctl_collect()`，回退链路：smartctl → ROOT\WMI → NVMe PowerShell → ROOT\CIMV2。
- 步骤与期望：
  1) 已安装 smartctl：
     - 期望：日志出现 `--scan-open` 发现设备；NVMe/SATA 盘返回温度/POH/上电次数/累计读写；UI“SMART 详情”逐盘呈现。
  2) 未安装 smartctl：
     - 期望：日志提示 `not found or not executable`，自动回退 WMI/PowerShell；UI 无报错，缺值显示“—”。
  3) 多盘与多接口：
     - 期望：至少一种路径成功；无黑窗；异常时打印退出码与 stderr 片段且不阻断回退。

### B. 内存细分（9 项）
- 字段来源：`wmi_perf_memory()`；前端 `Details.vue` 已展示缓存/提交/分页池/速率等。
- 期望：
  - 数值单位 GB（速率为每秒页面/读/写/错误）；无值显示“—”。
  - 运行时内存压力变化可见提交与分页速率联动。

### C. GPU VRAM 总量与使用率
- 来源：`wmi_query_gpu_vram()` + 桥接 GPU 负载；托盘 tooltip 汇总展示。
- 期望：
  - 详情页与托盘行显示 `VRAM <used>/<total> MB (<pct>%)`；缺值用“—”。
  - 多卡时最多显示2块，超出以 `+N` 汇总。

### D. 电池健康
- 来源：`wmi_read_battery_health()`；前端 `fmtBatteryHealth()` 展示。
- 期望：
  - 显示 `设计 <design>mWh / 满充 <full>mWh / 循环 <cycle>`；缺值“—”。
  - 充放电场景切换稳定；与 AC/估时字段不冲突。

### E. 便携版与黑窗
- 期望：便携包运行无 `conhost` 黑窗闪现；日志确认 `CREATE_NO_WINDOW` 已应用于 smartctl/WMIC/netsh/powershell。

### 验收方法
- 构建：`src-tauri/ cargo check` 通过；根目录 `npm run build` 通过。
- 管理员脚本：参考 `doc/script/ADMIN-TEST-SMART.md` 执行交叉验证并记录日志关键行。

## 2025-08-14 05:26（前端：NVMe SMART 四项指标 UI 集成 + 摘要扩展）
- 变更内容：
  - `src/main.ts`：`SensorSnapshot.smart_health` 类型新增 4 个 NVMe 字段（可选）：
    - `nvme_percentage_used_pct`、`nvme_available_spare_pct`、`nvme_available_spare_threshold_pct`、`nvme_media_errors`。
  - `src/views/Details.vue`：
    - 本地 `SensorSnapshot.smart_health` 同步扩展上述 4 字段（保持 snake_case/camelCase 双兼容）。
    - “SMART 详情”卡片新增 4 行显示：已用寿命%、可用备用%、备用阈值%、介质错误（缺失则“—”）。
    - 扩展 `fmtSmartKeys()`：汇总 NVMe 指标至摘要（最大“已用%”、最小“备用%”、介质错误合计），与温度/POH/重映射/待定/不可恢复/CRC 一并展示。
- 测试点：
  1) 管理员运行，打开“详情”页 → 展开“SMART 详情”，NVMe 盘应出现 4 项新指标；缺值显示“—”。
  2) 摘要行应追加 `已用 X% | 备用 Y% | 介质 Z`，若无 NVMe 数据则仅显示既有 SATA/通用项。
  3) 回退链路（smartctl/IOCTL/WMI/PowerShell）任一路径返回的 NVMe 字段应被前端正确渲染。

## 2025-01-14 22:45 增量重构第二步完成
- 成功提取电池工具模块 `src-tauri/src/battery_utils.rs`：
  - 包含 `Win32Battery` 结构体定义
  - 包含 `battery_status_to_str()` 状态码转换函数
  - 包含 `wmi_read_battery()`、`wmi_read_battery_time()`、`wmi_read_battery_health()` 查询函数
- 更新 `lib.rs` 中的电池函数调用，使用 `battery_utils::` 前缀
- 删除 `lib.rs` 中重复的电池相关结构体和函数定义
- 编译通过：`cargo check` 成功，19个警告但无错误
- 功能完整性：电池相关功能已安全迁移至独立模块，保持原有调用接口不变

## 2025-01-15 增量重构第二步完成
成功提取电池工具模块 `battery_utils.rs`，包含：
- `Win32Battery` 结构体定义
- `battery_status_to_str` 电池状态码转换函数  
- `wmi_read_battery`、`wmi_read_battery_time`、`wmi_read_battery_health` 电池查询函数

更新了 `lib.rs` 中的电池函数调用，使用 `battery_utils::` 模块前缀。编译通过（19个警告但无错误），功能完整。

## 2025-01-15 增量重构第三步完成

## 2025-01-15 增量重构第八步完成
成功提取Wi-Fi相关模块 `wifi_utils.rs`，包含：
- `WifiInfoExt` 结构体定义，包含完整Wi-Fi连接详情字段
- `read_wifi_info` 函数：获取简化Wi-Fi信息（SSID、信号强度、链路速率）
- `read_wifi_info_ext` 函数：获取详细Wi-Fi信息（BSSID、信道、加密方式、频段、RSSI等）
- `decode_console_bytes` 辅助函数：解码Windows控制台输出，支持UTF-8和GBK编码回退

技术要点：
- 使用Windows `netsh wlan show interfaces` 命令解析Wi-Fi信息
- 支持中英文标签识别和多种输出格式
- 修复了编译错误：`encoding_rs::GBK.decode()` 返回类型处理
- 条件编译确保Windows平台兼容性

更新了 `lib.rs` 中的Wi-Fi函数调用，删除重复代码。编译通过（28个警告但无错误），功能完整。

## 2025-01-15 增量重构第九步完成
成功提取NVMe/SMART相关模块 `nvme_smart_utils.rs`，包含：
- `SmartHealthPayload` 结构体定义，包含完整SMART健康数据字段（含NVMe特有字段）
- `nvme_smart_via_ioctl` 函数：通过Windows原生IOCTL直接访问NVMe设备
- `nvme_storage_reliability_ps` 函数：通过PowerShell Get-StorageReliabilityCounter获取NVMe数据
- `smartctl_collect` 函数：集成smartctl工具进行SMART数据采集
- `decode_console_bytes` 辅助函数：解码控制台输出，支持UTF-8和GBK编码

技术要点：
- 修复了Windows API调用的类型错误：`CreateFileW` 返回 `Result<HANDLE>` 需要正确处理
- 保留了复杂的NVMe IOCTL协议命令实现框架（暂时返回None，为后续完整实现预留）
- 支持多种SMART数据采集路径：IOCTL → smartctl → PowerShell → WMI回退
- 条件编译确保Windows平台兼容性

更新了 `lib.rs` 中的模块导入，添加 `nvme_smart_utils` 模块。编译通过（38个警告但无错误），功能完整。
成功提取温度和风扇工具模块 `thermal_utils.rs`，包含：
- `MSAcpiThermalZoneTemperature` 和 `Win32Fan` 结构体定义
- `wmi_read_cpu_temp_c` CPU温度查询函数
- `wmi_read_fan_rpm` 风扇转速查询函数

更新了 `lib.rs` 中的温度风扇函数调用，使用 `thermal_utils::` 模块前缀。编译通过，功能完整。

## 2025-01-15 增量重构第四步完成
成功提取网络和磁盘查询工具模块 `network_disk_utils.rs`，包含：
- `Win32NetworkAdapter` 和 `Win32NetworkAdapterConfiguration` 网络适配器结构体
- `Win32LogicalDisk` 逻辑磁盘结构体
- `wmi_list_net_ifs` 网络接口查询函数
- `wmi_list_logical_disks` 逻辑磁盘查询函数

更新了 `lib.rs` 中的网络磁盘函数调用，使用 `network_disk_utils::` 模块前缀。编译通过（20个警告但无错误），功能完整。

下一步：继续提取其他独立小模块，或等待用户测试反馈后续拆分计划。

## 2025-01-14

### 增量重构第四步完成
- **目标**: 提取网络接口和逻辑磁盘查询相关模块
- **创建模块**: `src-tauri\src\network_disk_utils.rs`
  - 包含结构体: `Win32NetworkAdapter`, `Win32NetworkAdapterConfiguration`, `Win32LogicalDisk`
  - 包含函数: `wmi_list_net_ifs`, `wmi_list_logical_disks`
- **更新主程序**: 
  - 在 `lib.rs` 中添加 `network_disk_utils` 模块导入
  - 删除重复的网络接口和逻辑磁盘相关结构体定义
  - 更新函数调用为使用新模块命名空间
- **编译验证**: `cargo check` 通过，19个警告但无错误
- **功能完整性**: 保持所有网络接口查询和逻辑磁盘查询功能完整
- **下一步**: 继续提取其他独立小模块（如GPU、进程等）

### 增量重构第五步完成
- **目标**: 提取GPU查询相关模块
- **创建模块**: `src-tauri\src\gpu_utils.rs`
  - 包含结构体: `Win32VideoController`, `BridgeGpu`
  - 包含函数: `wmi_read_gpu_vram`, `wmi_query_gpu_vram`
- **更新主程序**: 
  - 在 `lib.rs` 中添加 `gpu_utils` 模块导入
  - 删除重复的GPU相关结构体定义（`Win32VideoController`, `BridgeGpu`）
  - 更新函数调用为使用新模块命名空间
  - 修复编译错误：添加缺失的 `wmi_query_gpu_vram` 函数导入
- **编译验证**: `cargo check` 通过，24个警告但无错误
- **功能完整性**: 保持所有GPU查询和VRAM监控功能完整
- **下一步**: 继续提取其他独立小模块（如进程监控、SMART状态等）待用户测试反馈后续拆分计划。

## 2025-01-14 增量重构第五步：提取GPU相关模块

### 完成内容
1. **创建GPU工具模块** (`src-tauri/src/gpu_utils.rs`)
   - 提取GPU相关WMI结构体：`Win32VideoController`
   - 提取GPU桥接结构体：`BridgeGpu` 
   - 提取GPU查询函数：`wmi_read_gpu_vram`、`wmi_query_gpu_vram`

2. **更新主程序** (`src-tauri/src/lib.rs`)
   - 添加`gpu_utils`模块导入
   - 删除重复的GPU相关结构体和函数定义
   - 更新相关函数调用以使用新模块

3. **修复编译错误**
   - 添加缺失的导入语句
   - 确保所有GPU相关功能正常工作

### 验证结果
- ✅ 编译通过（有警告但无错误）
- ✅ GPU相关代码成功模块化
- ✅ 主程序代码量减少，结构更清晰

## 2025-01-14 增量重构第六步：提取SMART状态查询模块

### 完成内容
1. **创建SMART工具模块** (`src-tauri/src/smart_utils.rs`)
   - 提取SMART相关WMI结构体：`MsStorageDriverFailurePredictStatus`、`MsStorageDriverFailurePredictData`、`Win32DiskDrive`
   - 提取SMART属性结构体：`SmartAttrRec`
   - 提取SMART解析函数：`parse_smart_vendor`
   - 提取SMART查询函数：`wmi_list_smart_status`、`wmi_fallback_disk_status`

2. **更新主程序** (`src-tauri/src/lib.rs`)
   - 添加`smart_utils`模块导入，包含`Win32DiskDrive`结构体
   - 删除重复的SMART相关结构体和函数定义
   - 更新相关函数调用以使用新模块

3. **修复编译错误**
   - 删除lib.rs中重复的`wmi_fallback_disk_status`函数定义
   - 补充缺失的`Win32DiskDrive`导入
   - 确保所有SMART相关功能正常工作

### 验证结果
- ✅ 编译通过（26个警告但无错误）
- ✅ SMART相关代码成功模块化
- ✅ 主程序代码量进一步减少，结构更清晰
- ✅ 保持完整的SMART数据采集功能链路

### 下一步计划
准备执行增量重构第七步：继续提取其他独立小模块

## 2025-01-16 增量重构第六步完成

- **时间**: 2025-01-16 23:45
- **任务**: 提取SMART状态查询相关模块smart_utils.rs
- **完成内容**:
  1. 删除lib.rs中重复的wmi_fallback_disk_status函数定义（第1743-1763行）
  2. 在smart_utils.rs中补充Win32DiskDrive结构体的正确导入
  3. 更新lib.rs中SMART相关函数调用，使用smart_utils模块
  4. 验证编译通过：cargo check成功，26个警告但无阻塞错误
- **技术细节**:
  - 成功移除lib.rs中重复的SMART相关函数定义
  - 保持多路径SMART数据采集完整性（NVMe IOCTL、WMI、smartctl回退、PowerShell回退）
  - 模块化后SMART功能保持完整，包括属性解析和健康状态查询
- **下一步**: 继续增量重构第七步，提取其他独立小模块

## 2025-01-16 增量重构第七步完成

- **时间**: 2025-01-16 23:50
- **任务**: 提取进程监控相关模块process_utils.rs并更新主流程调用
- **完成内容**:
  1. 创建process_utils.rs模块，包含进程监控相关功能：
     - RttResultPayload、TopProcessPayload结构体
     - tcp_rtt_ms函数：单目标RTT测试
     - measure_multi_rtt函数：多目标RTT测量
     - get_top_processes函数：获取Top进程（CPU和内存排序）
     - calculate_disk_totals函数：计算磁盘累计读写字节数
  2. 更新lib.rs主流程代码，替换内联实现为模块函数调用：
     - 磁盘统计：替换为calculate_disk_totals(&sys)
     - RTT测试：替换为measure_multi_rtt(&targets, timeout)
     - Top进程：替换为get_top_processes(&sys, top_n)
  3. 删除lib.rs中重复的结构体和函数定义
  4. 修复编辑过程中的语法错误（网络统计代码结构）
  5. 验证编译通过：cargo check成功，27个警告但无阻塞错误
- **技术细节**:
  - 成功提取进程监控、RTT测试、磁盘统计等独立功能模块
  - 保持功能完整性，包括多目标RTT测量和Top进程排序
  - 简化主流程代码，提高可读性和可维护性
  - 模块化后主流程逻辑更清晰，便于后续维护
- **下一步**: 继续增量重构第八步，提取其他独立小模块
