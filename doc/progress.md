## 2025-08-10 15:55
- 托盘文本图标可读性修复：
  - 文本绘制新增 `gap` 参数：当字号为 `scale=2` 时使用 `gap=0`，最大化横向可用像素；仅在仍溢出时降到 `scale=1` 并使用 `gap=1`。
  - 顶/底行在判定超宽时，优先尝试移除单位字符（顶行去掉 `C`，底行去掉 `%`）以保留大字号；例如 `100C` -> `100`，`85%` -> `85`。
  - 结果：常见值如 `70C`、`85%`、`100` 可在 32x32 内以大字号清晰显示。
- 设置页保存报错修复：
  - `Settings.vue` 调用 `set_config` 的参数名改为 `newCfg`（原 `new_cfg` 导致 Tauri v2 参数映射不匹配）。
- 编译告警清理：
  - 去除了 `net_rx_total/net_tx_total` 绑定的多余 `mut`，消除了 `unused_mut` 警告。
- 构建验证：`cargo check` 通过。
 - 新增文档：`doc/项目总结与开发注意事项.md`（作为后续开发的基础信息与注意事项清单）。
# 开发进度

本文档用于记录每次功能开发/验证后的里程碑与变更摘要（持续更新）。
 
## 2025-08-10 16:20
- 打包资源与桥接启动调整：
  - 在 `src-tauri/tauri.conf.json` 增加 `bundle.resources: ["resources/sensor-bridge/**"]`。
  - 将 `beforeBuildCommand` 改为先发布自包含单文件 `.NET` 桥接：
    `dotnet publish ./sensor-bridge -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true -o ./src-tauri/resources/sensor-bridge && npm run build`
  - 这样打包后的应用可在无 .NET 运行时的客户机上直接运行。
- 后端优先从资源目录启动桥接：
  - 在 `src-tauri/src/lib.rs` 计算 `BaseDirectory::Resource/sensor-bridge/sensor-bridge.exe`，若存在则优先启动，并保持 stderr 日志与自动重启。
  - 保留开发态回退：按顺序尝试 `dotnet <dll>`、`exe`、`dotnet run --project sensor-bridge`。
- 注意与测试建议：
  - 构建前先清理遗留进程：`taskkill /F /IM sys-sensor.exe /IM sensor-bridge.exe /IM dotnet.exe`
  - 本地打包：`cargo tauri build`（会自动执行上面的 `dotnet publish` 并将桥接纳入资源）。
 - 安装包运行后，在托盘日志中应出现 `[bridge] spawning packaged exe:` 日志，温度/风扇应在 NUC8 上显示（若仍无，建议以管理员运行以测试权限影响）。

## 2025-08-10 16:22
- 修复 dev 构建时报错 `glob pattern resources/sensor-bridge/** path not found`：
  - 新增占位文件：`src-tauri/resources/sensor-bridge/.gitkeep`，确保通配符至少匹配一个文件。
  - 保持 release 流程不变（`beforeBuildCommand` 发布自包含桥接并随包分发）。
  - 建议：如需在 dev 启动前就使用发布版桥接，可手动执行一次 `dotnet publish ... -o ./src-tauri/resources/sensor-bridge`。

## 2025-08-10 16:26
- 调整 `src-tauri/tauri.conf.json` 的 `bundle.resources`：由 `resources/sensor-bridge/**` 改为 `resources/sensor-bridge`，避免某些环境下 glob 校验在 dev 期仍报未匹配。
- 当前 dev 启动成功，后端日志显示：优先从本地 `sensor-bridge/bin/Release/net8.0/sensor-bridge.dll` 以 `dotnet` 启动；打包版会优先从 `BaseDirectory::Resource/sensor-bridge/sensor-bridge.exe` 启动。

## 2025-08-10 03:45
- 实现托盘文本图标（32x32，双行显示 CPU%/内存%），每秒动态刷新。
- 后端事件广播通道：周期性 `emit("sensor://snapshot", payload)`。
- 前端在 `src/main.ts` 订阅实时快照事件并打印到控制台。
- Tooltip 和右键菜单上半区展示 CPU、内存、网络上下行和磁盘读写速率。

## 2025-08-10 03:51
- 启动“温度/风扇”采集开发：
  - 方案：优先使用 WMI（`ROOT\\WMI` 的 `MSAcpi_ThermalZoneTemperature`）获取温度（单位：开尔文×10 转摄氏度）。
  - 风扇转速：先尝试 WMI 可用项；若不可用，保持占位“—”。
  - 计划将温度/风扇集成至托盘菜单、Tooltip 与事件广播负载。

> 注：本文档将伴随后续每次功能完成后继续更新。

## 2025-08-10 03:55
- 完成“温度/风扇”采集最小可用实现：
  - 使用 WMI（`ROOT\\WMI` -> `MSAcpi_ThermalZoneTemperature`）读取温度，按 `K×10 - 273.15` 转换为摄氏度并做异常值过滤。
  - 风扇通过 `ROOT\\CIMV2` -> `Win32_Fan.DesiredSpeed` 尝试获取；若系统/驱动不提供则显示“—”。
  - 集成托盘菜单与 Tooltip，事件负载新增 `cpu_temp_c`、`fan_rpm` 字段。

## 2025-08-10 13:31
- 创建 `.NET 8` 传感器桥 `sensor-bridge/`（`LibreHardwareMonitorLib`），每秒输出 JSON：`cpuTempC`、`moboTempC`、`fans[]`。
- Rust 端集成桥接子进程并优先使用桥接数据：
  - 新增 JSON 反序列化结构 `BridgeOut`，在 `setup()` 中启动桥接并读取 stdout。
  - 向上递归定位项目根目录，可靠查找 `sensor-bridge/sensor-bridge.csproj`，避免路径不一致导致无法启动。
  - 增加 `stderr`/非 JSON 行日志，便于定位桥接失败原因。
  - 为桥接子进程增加自动重启（退出后 3 秒重启），提升健壮性。
- 菜单展示：温度优先显示 CPU 温度，并在可用时附加主板温度；风扇优先显示 CPU 风扇 RPM，机箱风扇存在时并列显示。
- 前端 `SensorSnapshot` 类型新增 `mobo_temp_c` 字段。

## 2025-08-10 13:36
- 修复 `sensor-bridge/Program.cs` 少 `using System.Linq;` 导致 LINQ 编译错误。
- 增补桥接子进程的路径查找、日志与自动重启逻辑，避免出现“温度/风扇均为 —”但无明显错误的情况。
- 提供 PowerShell 一键清理/编译命令：`taskkill` 结束遗留进程，`dotnet restore && build -c Release` 先编译出 exe 以便 Rust 端直接启动。

## 2025-08-10 13:39
- 即将执行：结束遗留调试进程并编译桥接 Release 版。
  - 结束进程：`sys-sensor`、`sensor-bridge`、`dotnet`。
  - 编译命令：`dotnet restore .\sensor-bridge`、`dotnet build .\sensor-bridge -c Release`。
  - 目的：确保 Rust 端可直接启动 `sensor-bridge.exe`，避免 fallback 的 `dotnet run` 受限导致“温度/风扇为 —”。

## 2025-08-10 13:42
- Rust 桥接进程启动逻辑增强：若存在 `sensor-bridge.dll` 则优先使用 `dotnet <dll>` 启动，其次尝试启动 exe，最后才回退到 `dotnet run`。
- 目的：适配 Release 默认生成 dll 的情况，提升桥接进程启动成功率。

待验证与注意：
- 首次运行需编译桥接：建议 `sensor-bridge` 目录执行 `dotnet restore && dotnet build -c Release`；否则将回退到 `dotnet run`。
- 某些主板/EC 传感器需要管理员权限；若仍为“—”，请尝试以管理员身份运行。
- 编辑器存在调试进程未自动结束问题：需要使用 `taskkill` 主动结束 `sys-sensor.exe`/`sensor-bridge.exe`/关联 `dotnet.exe`。

## 2025-08-10 14:41
- 托盘菜单联动窗口：实现“显示详情/快速设置/关于我们”菜单点击后的窗口创建/聚焦逻辑（`src-tauri/src/lib.rs`）。
  - 使用 `WebviewWindowBuilder` 创建命名窗口（`details`/`settings`/`about`），再次点击时聚焦已有窗口。
  - 窗口尺寸：详情 900x600（可调），设置 640x520（可调），关于 420x360（固定）。
- Tooltip 与菜单上半区已展示 CPU/内存/温度/风扇/网络/磁盘的实时信息；托盘图标双行文本每秒刷新。
- 前端暂使用默认 `App.vue` 作为窗口内容（后续将分别实现 详情/设置/关于 页面与路由）。

## 2025-08-10 14:46
- 前端接入多页面路由与窗口内容：
  - 安装 `vue-router@4`，新增 `src/router/index.ts`，以 hash 路由承载多窗口页面。
  - 新增页面：`src/views/Details.vue`、`src/views/Settings.vue`、`src/views/About.vue`；`App.vue` 改为仅渲染 `<router-view>`。
  - `src/main.ts` 引入并使用 `router`，保持对 `sensor://snapshot` 的订阅日志。
  - `Details.vue` 订阅 `sensor://snapshot` 并展示 CPU/内存/温度/风扇/网速/磁盘速率，带单位格式化；修正 ref 用法避免 TS 报错。
  - `About.vue` 使用 `@tauri-apps/api/app#getVersion()` 获取版本号，替代不可用的编译期常量，修复 Lint。
- 后端修复编译：
  - 在 `src-tauri/src/lib.rs` 引入 `tauri::Manager`，解决 `app.get_webview_window(..)` 编译错误。
  - `cargo check` 通过；前端 `npm run build` 通过。
- 路由与窗口对应：Rust 端创建窗口的 URL 分别为 `index.html#/details`、`#/settings`、`#/about`，与前端路由完全一致，托盘菜单可打开并显示对应页面。

## 2025-08-10 14:58
- 修复窗口交互问题：
  - 托盘菜单点击【显示详情/快速设置/关于我们】时，若窗口已存在则 `show()+set_focus()`，不再重复创建；仅首次点击才会创建窗口（`src-tauri/src/lib.rs`）。
  - 拦截窗口关闭事件并改为隐藏：对 `main`/`details`/`settings`/`about` 窗口注册 `WindowEvent::CloseRequested`，执行 `hide()` 且 `api.prevent_close()`，避免点击窗口右上角 X 直接退出程序。
  - 托盘【退出】菜单保留真实退出行为，调用 `app.exit(0)`。
- 事件广播与订阅：后端使用 `app.emit("sensor://snapshot", payload)` 全局广播，前端在 `src/main.ts` 与 `src/views/Details.vue` 订阅；新窗口将自动接收下一次快照数据。
- 构建验证：`cargo check` 通过。

## 2025-08-10 15:40
- 配置持久化与设置页面联动：
  - 后端新增 `AppConfig`（JSON）：`tray_show_mem`（托盘第二行显示内存%或CPU%）、`net_interfaces`（网卡白名单，空=聚合全部）。
  - 新增 Tauri 命令：`get_config`、`set_config`、`list_net_interfaces`，并通过 `AppState(Arc<Mutex<_>>)` 注入全局状态；配置保存在 `AppConfig` 目录下 `config.json`。
  - 采样线程按配置过滤网卡聚合；托盘底行按配置显示 CPU% 或 内存%。
  - 前端 `src/views/Settings.vue` 接入：加载/保存配置，提供网卡多选（为空表示统计全部），以及“托盘第二行显示内存”开关。
  - 变更后无需重启：保存后下一轮刷新即生效；同时广播 `config://changed`（目前未在前端使用）。
- 构建验证：`cargo check` 通过（有轻微 `unused_mut` 警告，不影响运行）。

## 2025-08-10 15:15
- 复用主窗口并路由导航：
  - 托盘【显示详情/快速设置/关于】不再创建新窗口，而是复用启动时的主窗口（label: `main`），执行 `show()+set_focus()` 并通过 `win.eval("location.hash='#/xxx'")` 切换到对应页面。
  - 解决“启动后点击显示详情会新建一个无数据窗口（窗口2）且未复用窗口1”的问题；现在始终只使用主窗口。
- 关闭行为沿用：点击窗口右上角 X 仍为隐藏（`WindowEvent::CloseRequested` -> `hide()`），托盘【退出】才真正退出。
- 构建验证：`cargo check` 通过。

## 2025-08-10 15:24
- 托盘“纯文本图标”实现与刷新：
  - 在 `src-tauri/src/lib.rs` 新增 `make_tray_icon()` 文本绘制：上行显示 CPU 温度（如 `70C`，若无温度则回退为 CPU%），下行显示 CPU%。
  - 动态按行选择缩放比例（2 或 1），避免 32x32 图标文字溢出；每秒根据最新数据重绘并 `tray.set_icon()` 刷新。
- Tooltip 与信息区联动：
  - 托盘菜单顶部信息区（CPU/内存/温度/风扇/网络/磁盘）与 Tooltip 多行文本每秒同步更新，来源与详情页一致。
- 传感器桥接与回退：
  - 后台线程尝试启动 `.NET sensor-bridge`（优先 dll，其次 exe，最后 `dotnet run`），解析 JSON 输出带入 CPU/主板温度与风扇（含占空比）。
  - 并行提供 WMI 回退：温度来自 `ROOT\\WMI` 的 `MSAcpiThermalZoneTemperature`；风扇尝试 `Win32_Fan` 的 `DesiredSpeed`（可能缺失）。
  - 加入过期策略（>5s 视为过期）与管理员/可用性标志透出，用于 Tooltip 友好提示（“需管理员/无读数/不支持”）。
- 网络/磁盘速率：
  - 采用累计字节差分求速率，加入 `alpha=0.3` 的 EMA 平滑；单位自适应 KB/s 或 MB/s。
  - `SensorSnapshot` 已包含 `net_rx_bps/net_tx_bps/disk_r_bps/disk_w_bps`，前端 `Details.vue` 展示无改动即可生效。
- 构建验证：`cargo check` 通过。
