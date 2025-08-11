# sys-sensor 项目计划（Plan）

更新时间：2025-08-11 20:11

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

- 高优先级
  - 网络基础信息与 Wi‑Fi 指标：
    - 接口 IP/MAC、链路速率、介质类型
    - Wi‑Fi SSID/RSSI/协商速率（如可用）
  - 磁盘/分区容量与 SMART 健康：
    - 每盘/分区容量、可用空间；可用时展示 SMART 健康/温度
- 中优先级
  - GPU 扩展：显存占用、功耗（LHM 可用则接入）
  - 内存细分：可用/缓存/交换；系统 Uptime 已有基础字段，r进一步完善展示
  - 主板电压（可用则展示）
  - 电池健康信息（笔记本）
- 体验与可视化
  - 每核心指标可视化增强（迷你条形/热图/展开面板）
  - 详情页信息分组与折叠
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
  - 端到端联调每核心指标稳定性与单位/取整策略
- 待办（见路线图）
  - 网络基础信息与 Wi‑Fi 指标、磁盘容量/SMART、GPU 显存/功耗、内存细分、主板电压、电池健康
