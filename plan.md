# sys-sensor 后端模块化拆分计划（lib.rs 全量拆分）

更新时间：2025-08-14 21:04

## 目标
- 将 `src-tauri/src/lib.rs` 拆分为多个子模块，提升可维护性与清晰度。
- 绝不影响托盘图标绘制与主界面指标显示：`make_tray_icon()` 与 `run()` 保持原位与逻辑不变。
- 保持现有 IPC 接口与对前端的 `SensorSnapshot` 负载格式不变。
- 兼容现有 NVMe Pass-through 模块 `nvme_pass`（已拆分）。

## 保留在 `lib.rs` 的内容
- `make_tray_icon()`（托盘图标渲染）
- `pub fn run()`（Tauri 启动、菜单/托盘、配置、主循环与桥接子进程）
- 传感器快照构造、托盘 tooltip 与菜单文字更新逻辑
- 现有 IOCTL NVMe 直连函数 `nvme_smart_via_ioctl()`（被 `wmi_list_smart_status()` 首选调用）

## 新增模块与职责
1) `src-tauri/src/payloads.rs`
- 前端快照相关载荷结构体（被 `SensorSnapshot` 引用）：
  - `VoltagePayload`/`FanPayload`/`StorageTempPayload`/`GpuPayload`
  - `SmartHealthPayload`（包含 NVMe 四项与 F1/F2→字节字段）
  - `NetIfPayload`/`LogicalDiskPayload`/`CpuPayload`/`MemoryPayload`/`DiskPayload`
- 在 `lib.rs` 通过 `pub(crate) use payloads::SmartHealthPayload;` 重导出以保持 `crate::SmartHealthPayload` 可用。

2) `src-tauri/src/wifi.rs`
- `WifiInfoExt` 与 `read_wifi_info_ext()`（Windows 解析 `netsh wlan show interfaces`）。
- 可选保留 `read_wifi_info()` 旧接口以兼容调用。

3) `src-tauri/src/wmi/`（多子模块）
- `perf_disk.rs`：`wmi_perf_disk()` 与 `PerfDiskPhysical` 定义。
- `gpu_vram.rs`：`wmi_read_gpu_vram()` 与 `Win32VideoController` 定义；保留 `wmi_query_gpu_vram()`（若使用）。
- `battery.rs`：`wmi_read_battery_health()` / `wmi_read_battery()` / `wmi_read_battery_time()` 与 `Win32Battery`。
- `perf_net.rs`：`wmi_perf_net_err()` 与 `PerfTcpipNic`。
- `memory.rs`：`wmi_perf_memory()` 与 `PerfOsMemory` 及嵌套的 `Win32OS` 回退结构。
- `temps.rs`：`wmi_read_cpu_temp_c()` / `wmi_read_fan_rpm()` 与 `MSAcpiThermalZoneTemperature`、`Win32Fan`。
- `netifs.rs`：`wmi_list_net_ifs()` 与 `Win32NetworkAdapter`、`Win32NetworkAdapterConfiguration`。
- `logical_disks.rs`：`wmi_list_logical_disks()` 与 `Win32LogicalDisk`。

4) `src-tauri/src/smart/`（多子模块，含平台条件）
- `wmi.rs`：`wmi_list_smart_status()`（仍优先 `nvme_smart_via_ioctl()`），`wmi_fallback_disk_status()`，以及相关结构体 `MsStorageDriverFailurePredict*`、`Win32DiskDrive`。
- `nvme_ps.rs`：`nvme_storage_reliability_ps()` 与非 Windows stub。
- `smartctl.rs`：`smartctl_collect()` 与非 Windows stub。

5) `src-tauri/src/utils.rs`
- 控制台字节解码等通用工具：`decode_console_bytes()`；如存在其他格式化工具（例如 bps/bytes），也集中到此。

## `lib.rs` 调整
- 顶部增加：
  - `mod payloads;`
  - `mod wifi;`
  - `mod wmi;`
  - `mod smart;`
  - `mod utils;`
- 选择性重导出在 `lib.rs` 内部调用到的函数/类型：
  - `pub(crate) use payloads::SmartHealthPayload;`
  - `pub(crate) use wmi::perf_disk::wmi_perf_disk;`
  - `pub(crate) use wmi::gpu_vram::wmi_read_gpu_vram;`
  - `pub(crate) use wmi::battery::{wmi_read_battery_health, wmi_read_battery, wmi_read_battery_time};`
  - `pub(crate) use wmi::perf_net::wmi_perf_net_err;`
  - `pub(crate) use wmi::memory::wmi_perf_memory;`
  - `pub(crate) use wmi::temps::{wmi_read_cpu_temp_c, wmi_read_fan_rpm};`
  - `pub(crate) use wmi::netifs::wmi_list_net_ifs;`
  - `pub(crate) use wmi::logical_disks::wmi_list_logical_disks;`
  - `pub(crate) use smart::wmi::{wmi_list_smart_status, wmi_fallback_disk_status};`
  - `pub(crate) use smart::nvme_ps::nvme_storage_reliability_ps;`
  - `pub(crate) use smart::smartctl::smartctl_collect;`
  - `pub(crate) use utils::decode_console_bytes;`
- 其余使用点改为 `use crate::<module>::...` 或依赖上述重导出。

## 拆分步骤（本轮按用户要求一次性提交）
1. 新建上述模块文件并迁移对应结构体与函数实现。
2. 更新 `lib.rs`：引入 `mod`、移除被迁移实现、添加必要的 `pub(crate) use` 重导出；保留 `make_tray_icon()` 与 `run()` 与 `nvme_smart_via_ioctl()` 原地不动。
3. 运行 `cargo check` 验证编译。
4. 在 `doc/progress.md` 末尾记录此次拆分内容与编译结果。

## 风险与对策
- 可见性：模块内部类型/函数统一 `pub(crate)`；根部做重导出，避免外部 API 变动。
- 平台分支：`smartctl_collect()` 与 `nvme_storage_reliability_ps()` 保持 `#[cfg(windows)]` 与非 Windows stub。
- NVMe Pass-through：`nvme_pass` 已拆分完成；本次不调整其调用方式。
- 托盘与 UI：严格不改动 `make_tray_icon()` 与 `run()` 及相关调用逻辑。

## 后续
- 管理员环境下进行 NVMe + SATA/USB 交叉测试（参考 `doc/script/ADMIN-TEST-SMART.md`）。
- 清理编译警告（不必要括号、未使用字段等）。
