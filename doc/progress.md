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

## 2025-08-10 16:35
- README 增补“面向客户的发布与部署（交付指南）”板块：包含前置准备、打包步骤、产物目录、客户机安装/运行、客户机前置条件与说明、自测清单、签名与企业部署建议。
- 修正 README 打包细节中 `bundle.resources` 的描述为目录路径：`resources/sensor-bridge`。
- 常用命令新增 `cargo tauri build` 作为 Release 交付入口，便于一键打包。

## 2025-08-10 16:37
- 新增 npm scripts（`package.json`）：
  - `env:check` 环境检查；`clean:proc` 结束遗留进程；`bridge:publish` 发布桥接到 `src-tauri/resources/sensor-bridge/`。
  - `tauri:dev`、`tauri:build`；`dev:all`（先发布桥接再 dev）。
  - `release:build`（一键清理->发布桥接->Tauri 打包->打开产物目录），`open:bundle` 打开产物目录。
  - 目的：统一用 `npm run` 执行开发/打包/交付动作。

## 2025-08-10 17:05
- 应用户提供代理（127.0.0.1:7890），新增：
  - `tauri:build:nsis:proxy`、`release:build:nsis:proxy`（启用 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY）
  - 绿色便携版脚本：`portable:build`/`portable:stage`/`portable:zip` 以及 `release:portable`
- 多次通过代理重试 NSIS 打包：编译成功，但在下载 NSIS 工具阶段失败：
  - 先前报 `timeout: global`，代理启用后报 `protocol: http response missing version`
  - 说明：编译与资源打包均 OK，失败发生于安装器依赖在线下载阶段。
- 已改走“绿色便携版”打包，成功产出 `dist-portable/sys-sensor-portable.zip`，可直接分发。
- 后续建议：
  1) 使用 `winget install NSIS.NSIS` 预装 NSIS（推荐），再执行 `npm run release:build:nsis`
  2) 或继续仅使用便携版进行交付。

## 2025-08-10 17:19
- 新增 socks5 代理脚本：`tauri:build:nsis:proxy-socks` 与 `release:build:nsis:proxy-socks`
- 通过 socks5 代理重试 NSIS 打包成功，产物：
  - 安装包：`src-tauri/target/release/bundle/nsis/sys-sensor_0.1.0_x64-setup.exe`（约 23.6 MB，实际 24,757,366 字节）
  - 便携版：`dist-portable/sys-sensor-portable.zip`（约 31.6 MB）
- 修复 `open:bundle` PowerShell 引号问题，避免终止符错误。
- 接下来：补充 README 文档，说明一键脚本与代理/离线依赖安装方案。

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

## 2025-08-10 18:09
- 托盘底行模式与风扇RPM显示：
  - 后端 `src-tauri/src/lib.rs`：
    - `AppConfig` 新增 `tray_bottom_mode: "cpu"|"mem"|"fan"`，并保留兼容字段 `tray_show_mem`（true 等价 mem，false 等价 cpu）。
    - 托盘图标渲染重构：`make_tray_icon(top, bottom)` 接收两行文本，保持大字号优先与宽度自适应策略（必要时移除 `C`/`%` 并降级字号）。
    - 采样线程依据配置生成底行文本：`cpu%` / `mem%` / `fanRPM`（无 RPM 时回退 `cpu%`），顶行优先显示温度（整数+`C`），否则 `cpu%`。
  - 前端 `src/views/Settings.vue`：新增“托盘第二行显示”下拉（CPU%/内存%/风扇RPM），保存 `tray_bottom_mode`，并写回兼容字段 `tray_show_mem`。
  - Tooltip/菜单上半区风扇信息维持原逻辑，无需调整。
- 后续验证计划：
  - 本地执行 `cargo check` 与 `npm run build` 验证类型与序列化；跨机观察 32x32 托盘图标在不同缩放下的可读性与 RPM 宽度适配。

## 2025-08-10 18:52
- 风扇 RPM 显示一致性修复：
  - 统一选择逻辑：优先 CPU 风扇 RPM，其次机箱风扇 RPM；若 RPM 不可用再回退显示占空比（%）。
  - 引入 `fan_best`（`Option<u32>`）用于托盘底行“风扇RPM”模式与前端 `SensorSnapshot.fan_rpm`，确保托盘、Tooltip 与主界面一致。
  - 修复并补全托盘菜单信息行：`temp_line`/`fan_line`/`net_line`/`disk_line`，并通过 `info_*_c.set_text()` 每秒刷新。
- 隐藏桥接黑色控制台窗口：
  - 所有 Windows 进程启动路径均加 `CREATE_NO_WINDOW`（`creation_flags(0x08000000)`），覆盖：打包 exe、便携 exe、dotnet dll、fallback `dotnet run`。
- 打包脚本清理：
  - 从 `package.json` 移除过期的 `portable:copy:z` 与 `release:portable:copy:z`，避免构建产物拷贝到 Z 盘。
- 构建验证：
  - `cargo check` 通过；`npm run build` 通过。
  - 待现场验证：
    1) 运行应用观察托盘顶/底行与详情页风扇 RPM 是否一致；
    2) 启动桥接时不再出现黑色 cmd 窗口；
    3) 便携版与安装包流程不再触发 Z 盘拷贝。

## 2025-08-10 19:09
- 便携版压缩阶段报 `sensor-bridge.exe` 被占用导致 `Compress-Archive` 失败：
  - 新增 `portable:unlock` 脚本：在 `portable:zip` 前，定向结束占用 `dist-portable/sys-sensor` 目录内文件的 `sys-sensor`/`sensor-bridge`/关联 `dotnet` 进程。
  - 更新 `release:portable`：在 `portable:stage` 之后插入 `portable:unlock`，确保压缩前文件句柄释放。
  - 如仅需复用已有 stage 结果：可直接执行 `npm run portable:unlock && npm run portable:zip && npm run portable:ls`。

## 2025-08-10 21:52
- 现场诊断与桥接升级：
  - 升级 `LibreHardwareMonitorLib` 至 `0.9.4` 并重建发布自包含：`dotnet publish -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true`。
  - 新增管理员一键脚本：`sensor-bridge/run-bridge-admin-exe.ps1`（运行 `bin/Release/win-x64/publish/sensor-bridge.exe`，采样 12 次并输出 `bridge.admin.*` 日志）。
  - 非管理员运行结果（发布版 exe）：多次输出 `{"isAdmin":false,"hasTemp":true,"hasTempValue":false,"hasFan":false,"hasFanValue":false}`；stderr 传感器树中 CPU 负载为 100%，但所有温度读数为空，风扇无条目。
  - WMI 探测：`MSAcpi_ThermalZoneTemperature` 返回 `PermissionDenied (0x80041003)`，`Win32_Fan` 存在条目但 `DesiredSpeed/Speed` 无值（多数 NUC 平台常见，RPM 不经 WMI 暴露）。
- 初步结论：
  - 非管理员态下 MSR/EC 读数受限，LHM 无法读取 CPU 温度与风扇 RPM；需管理员以加载底层驱动。
  - NUC8 主板风扇 RPM 很可能不通过标准接口公开（EC/OEM 限制），即使管理员也可能仅获得温度而无 RPM。
- 待办/验证：
  1) 在“管理员 PowerShell”中执行 `run-bridge-admin-exe.ps1`，确认 `bridge.admin.out.jsonl` 是否出现 `hasTempValue=true`，并检查风扇是否仍缺失。
  2) 若管理员下温度恢复而 RPM 仍无，则应用内继续采用“温度优先 + RPM 不可用回退占空比/CPU%”策略，并在 Tooltip 标注“NUC 平台 RPM 不支持/未公开”。
  3) 在 `README` 与 `doc/项目总结与开发注意事项.md` 补充兼容性说明与诊断指南（含管理员需求与常见返回空值现象）。

## 2025-08-10 22:04
- 管理员态验证（NUC8BEB / i7-8559U）：
  - `bridge.admin.out.jsonl` 多次输出示例：`{"cpuTempC":60,"isAdmin":true,"hasTemp":true,"hasTempValue":true,"hasFan":false,"hasFanValue":false}`。
  - 结论：管理员权限下 CPU 温度恢复为有值；风扇 RPM 依然无值（平台/EC 限制）。
  - `stderr` 传感器树可见 CPU 温度条目，风扇条目缺失为常态。
- 应用与文档更新：
  - README：
    - 技术栈更正为 LibreHardwareMonitorLib 0.9.4。
    - 新增《平台兼容性与诊断（NUC8 实测）》板块，包含管理员诊断步骤与常见问题。
  - `doc/项目总结与开发注意事项.md`：
    - 新增 5.4《管理员与 NUC8 诊断步骤》。
    - 在第 6 节“已知限制”补充 NUC8 兼容性与 WMI 限制说明。
- UI 策略确认：
  - 托盘/Tooltip/详情页统一优先 CPU 风扇 RPM；无则机箱风扇 RPM；再无则回退占空比或 CPU%。Tooltip 将提示“NUC 平台 RPM 不支持/未公开”。

## 2025-08-10 22:30
- 长时运行后“温度/风扇为 —”的过期误判修复：
  - 在 `src-tauri/src/lib.rs` 采样线程读取桥接数据处，将过期阈值由 5s 提高到 30s（仅当超过 30s 未更新才视为过期）。
  - 原因：桥接在短暂重启/系统休眠/杀软拦截等情况下，stdout 间隔可能 >5s，过低阈值导致丢弃桥接数据，且 WMI 常无值，UI 显示“—”。
  - 影响：提升长时间运行稳定性；NUC8 平台 RPM 受限结论不变。
  - 验证建议：手动暂停/重启桥接或让系统短暂休眠，恢复后 30s 内应继续使用最近读数；超过 30s 才回退。

## 2025-08-10 22:45
- 桥接自愈机制（`sensor-bridge/Program.cs`）
  - 新增 `MakeComputer()` 和 `ReadEnvInt()`，统一创建/配置 `LibreHardwareMonitor.Computer` 与读取环境变量。
  - 每秒刷新后根据读数状态与异常情况判断是否重建 `Computer`：
    1) 空闲阈值：`BRIDGE_SELFHEAL_IDLE_SEC`（默认 300s）内无有效温度/风扇读数；
    2) 连续异常：`BRIDGE_SELFHEAL_EXC_MAX`（默认 5 次）；
    3) 周期重建：`BRIDGE_PERIODIC_REOPEN_SEC`（默认 0=关闭）。
  - 触发时会输出 `[bridge][selfheal]` 日志到 stderr；重建后立即继续采样，无黑窗。
  - 目标：解决运行数小时后主板温度/风扇读数消失的“枚举卡死/句柄失效”问题。
- 相关联调整：Rust 侧已将桥接数据过期阈值提高到 30s，避免短暂中断时误判过期。
- 验证建议：
  - 长时运行并模拟睡眠/安全软件短暂拦截；观察是否能自动恢复温度/风扇读数。
  - 如需更保守，可设置 `BRIDGE_PERIODIC_REOPEN_SEC=1800` 实现半小时周期性重建。

## 2025-08-10 23:10
- 后端诊断增强（实时日志）：
  - 将桥接子进程的 `stderr` 改为“逐行实时读取并打印”，覆盖所有启动分支：打包 exe、便携 exe、`dotnet <dll>` 与 fallback `dotnet run`。
  - 采样线程新增“桥接数据新鲜/过期”状态转换日志：当最近一次桥接输出在 30s 内/外分别打印
    `[bridge][status] data became FRESH` / `[bridge][status] data became STALE`，用于定位现场何时开始丢失读数。
- 目的：
  - 便于客户机长时间运行时实时捕获桥接自愈与传感器读数丢失的关键时间点；无需等待子进程退出即可看到日志。
- 配置与使用建议：
  - 桥接（C#）端可配合开启：`BRIDGE_SUMMARY_EVERY_TICKS`、`BRIDGE_DUMP_EVERY_TICKS`、`BRIDGE_LOG_FILE`；
    自愈相关：`BRIDGE_SELFHEAL_IDLE_SEC`（默认300）、`BRIDGE_SELFHEAL_EXC_MAX`（默认5）、`BRIDGE_PERIODIC_REOPEN_SEC`（默认0）。
  - Rust 端现已实时输出桥接 `stderr` 到控制台/日志收集器；如需进一步排查，建议同时保留 `BRIDGE_LOG_FILE` 到本地文件。
- 验证建议：
  1) 正常运行数小时，确认无“误判过期”导致的“—”；
  2) 人为断桥/重启/系统短暂休眠，观察 `[bridge][status]` 的 FRESH/STALE 切换与 C# 端 `[bridge][selfheal]` 是否对应；
  3) 收集现场日志包（stdout+stderr+桥接 log 文件）回传分析。

## 2025-08-10 23:30
- Rust 后端采样扩展：
  - 在 `src-tauri/src/lib.rs` 采样线程中读取桥接新增字段：`storageTemps`（NVMe/SSD 温度）与健康指标（`hbTick`、`idleSec`、`excCount`、`uptimeSec`）。
  - 构造并广播 `SensorSnapshot` 时新增同名字段：`storage_temps`、`hb_tick`、`idle_sec`、`exc_count`、`uptime_sec`，前端可直接消费。
- 结构体对齐：
  - 确认 `SensorSnapshot` 与 `StorageTempPayload` 的字段与类型已存在且命名一致（Rust 端 snake_case 与桥接 camelCase 通过 `serde` 对齐）。
- 构建验证：
  - `cargo check` 通过（目录：`src-tauri/`）。

## 2025-08-10 23:58
- 托盘菜单与 Tooltip 扩展：
  - 在 `src-tauri/src/lib.rs` 新增信息行 `存储:` 与 `桥接:`，实时展示 NVMe/SSD 温度列表与桥接健康摘要（`hb`、`idle`、`exc`、`up`）。
  - Tooltip 同步新增两行，与托盘信息区保持一致；存储温度最多展示 3 项，超出以 `+N` 汇总。
- 前端详情页扩展：
  - 在 `src/views/Details.vue` 的 `SensorSnapshot` 类型新增：`storage_temps[]`、`hb_tick`、`idle_sec`、`exc_count`、`uptime_sec`。
  - 新增展示项“存储温度”“桥接健康”，并编写 `fmtStorage()`、`fmtUptime()`、`fmtBridge()` 进行友好格式化。
- 构建验证：
  - `cargo check` 通过；`npm run build` 通过（无类型错误）。

## 2025-08-10 23:59
- 桥接编译错误修复与发布：
  - 修复 `sensor-bridge/Program.cs` 将 `class StorageTemp` 与 `CollectStorageTemps()` 误置于 `Main()` 内导致的语法错误，现已移至类作用域；补充 `using System.Collections.Generic;`。
  - 修复空引用告警（CS8602）：构建 JSON 负载时对 `fans` 做空值检查，避免 `fans.Count` 在空时解引用。
  - `dotnet build -c Release` 通过；验证运行 3 次循环（`BRIDGE_TICKS=3`）成功输出心跳/健康字段。
  - 当前环境下 `storageTemps` 未出现（值为 null，因 JSON 忽略 null），推测为非管理员或平台未暴露；后续建议以管理员运行或开启 `BRIDGE_DUMP_EVERY_TICKS` 检查 `Storage` 节点是否存在温度传感器。
  - 已发布自包含单文件桥接至 `src-tauri/resources/sensor-bridge/`（`dotnet publish -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true`）。

## 2025-08-11 02:38
- 存储温度显示优化：
  - 新增 `Program.cs` 中 `MapStorageTempName()`，将通用名 `Temperature/Temperature 1/Temperature 2` 等映射为具体位置：`复合/控制器/闪存/盘体`，UI 将直接显示中文位置。
  - 在 `CollectStorageTemps()` 处使用映射；并移除按名称分组去重，避免多盘或多位置被合并丢失读数。
  - 构建与发布：`dotnet publish -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true -o src-tauri/resources/sensor-bridge`；可直接打包联调。

## 2025-08-11 03:20
- 稳定性验证准备：
  - 核实并确认桥接（`sensor-bridge/Program.cs`）已实现并读取以下环境变量：`BRIDGE_SUMMARY_EVERY_TICKS`、`BRIDGE_DUMP_EVERY_TICKS`、`BRIDGE_LOG_FILE`、`BRIDGE_SELFHEAL_IDLE_SEC`、`BRIDGE_SELFHEAL_EXC_MAX`、`BRIDGE_PERIODIC_REOPEN_SEC`；`doc/script/start-sys-sensor.ps1` 与便携脚本均已设置默认值。
- 建议验证方案（可分阶段执行）：
  1) 基线长跑（6-12h）：使用默认参数运行，观察托盘/Tooltip/详情页持续更新；`logs/bridge.log` 每分钟有 summary；无长时间“—”。
  2) 睡眠/断桥注入：让系统睡眠约 5 分钟再唤醒，或手动结束 `sensor-bridge` 进程；期望 30s 内恢复；Rust 侧出现 `[bridge][status] FRESH/STALE` 切换，C# 侧在满足阈值时出现 `[bridge][selfheal]`。
  3) 周期重建：将 `BRIDGE_PERIODIC_REOPEN_SEC` 设为 `1800` 运行数小时，确认每 30 分钟一次重建且读数无闪断。
  4) NUC8 权限对比：普通/管理员分别运行，确认 CPU 温度与 RPM 表现符合文档结论，UI 显示提示。
  5) 存储温度：管理员下将 `BRIDGE_DUMP_EVERY_TICKS` 设为 `600`，检查 dump 中 `Storage` 节点与温度条目，前端“存储温度”行是否出现。
- 产出物：收集 `logs/bridge.log` 与（如使用开发模式）控制台 stderr/stdout，记录关键时间点并回传分析。

## 2025-08-11 04:20
- 第二梯队指标（后端采集与前端展示）落地：
  - Rust 端（`src-tauri/src/lib.rs`）：
    - 新增 WMI 性能计数查询：
      - `Win32_PerfFormattedData_PerfDisk_PhysicalDisk` 汇总磁盘 `DiskReadsPerSec`/`DiskWritesPerSec`（读/写 IOPS）与 `CurrentDiskQueueLength`（平均队列长度，排除 `_Total`）。
      - `Win32_PerfFormattedData_Tcpip_NetworkInterface` 汇总 `PacketsReceivedErrors`/`PacketsOutboundErrors`（网络错误包/秒，排除 `_Total`）。
    - 新增延迟近似：`tcp_rtt_ms()` 通过 `TcpStream::connect_timeout` 到 `1.1.1.1:443` 计算往返时间（超时 300ms）。
    - 扩展 `SensorSnapshot` 并在采样循环内赋值广播：`disk_r_iops/disk_w_iops/disk_queue_len/net_rx_err_ps/net_tx_err_ps/ping_rtt_ms`。
  - 前端：
    - `src/main.ts` 的 `SensorSnapshot` 类型同步扩展上述字段。
    - `src/views/Details.vue` 新增格式化函数与展示条目：
      - 磁盘读/写 IOPS（`fmtIOPS`）、磁盘队列（`fmtQueue`）。
      - 网络错误 RX/TX（`fmtPktErr`）、网络延迟（`fmtRtt`）。
  - 构建验证：`cargo check` 通过（清除 1 个未用导入告警），前端类型检查/构建将继续验证。
  - 说明：WMI 性能计数不需管理员权限；NUC8 平台温度/RPM 限制不影响该部分指标。

## 2025-08-11 04:45
- 长时稳定性增强（Rust 端 WMI 重连与睡眠自恢复）：
  - 在 `src-tauri/src/lib.rs` 采样线程加入“长间隔”检测：当两次采样间隔 `dt > 5s`（可能由于系统睡眠/挂起），将重置速率基线（`has_prev=false`，EMA 下一轮重建）并分别重建三类 WMI 连接（温度 `ROOT\\WMI`、风扇/Perf `ROOT\\CIMV2`）。打印日志：`[wmi][reopen] due to long gap ...`。
  - 对 WMI Perf（磁盘/网卡计数器）引入失败计数：若连续 3 次查询结果全部为 None，则重建 Perf 连接；并加入 1800 秒的周期性重开保护。打印日志：`[wmi][reopen] perf conn recreated (fail_cnt=..., periodic=...)`。
  - 目的：解决系统睡眠/短暂 WMI 故障后长期不恢复的问题，避免 IOPS/错误率等指标长时间缺失；同时防止长间隔导致的速率尖峰。
- 构建验证：`cargo check` 通过（目录：`src-tauri/`）。
- 验证建议：
  1) 运行 6h+ 并让系统睡眠 5 分钟再唤醒；观察 `[wmi][reopen]` 日志与 UI 指标是否在 1-2 个采样周期内恢复。
  2) 手动阻断 WMI（或模拟异常）验证“连续失败 3 次重建”是否触发且恢复正常。
  3) 检查恢复后首轮速率无异常尖峰（EMA 基线已重置）。

## 2025-08-11 05:20
- GPU 监控全链路接入（温度/负载/频率/风扇）：
  - 桥接（C# `sensor-bridge/Program.cs`）：输出 `gpus[]` 字段（`name/tempC/loadPct/coreMhz/fanRpm`，camelCase）。
  - Rust 端（`src-tauri/src/lib.rs`）：
    - 新增 `BridgeGpu`（`serde(rename_all="camelCase")`）与前端负载结构 `GpuPayload`。
    - `BridgeOut` 添加 `gpus: Option<Vec<BridgeGpu>>`，解析并映射到 `SensorSnapshot.gpus`。
    - `SensorSnapshot` 新增 `gpus: Option<Vec<GpuPayload>>` 并在广播时带上。
  - 前端：
    - `src/views/Details.vue` 与 `src/main.ts` 的 `SensorSnapshot` 类型新增 `gpus[]`。
    - 新增 `fmtGpus()` 并在详情页网格中展示 GPU 汇总（名称/温度/负载/频率/风扇RPM，最多两项，超出以 `+N` 汇总）。
- 兼容性：如平台不公开风扇 RPM，`fan_rpm` 可能为空，UI 将显示“—”。
- 构建与验证计划：
  1) `cargo check` 验证 Rust 端；
  2) `npm run build` 验证前端类型与编译；
  3) `npm run dev:all` 或已运行的 dev 实例中观察 GPU 行出现与刷新；
  4) 如无 GPU 或权限不足，`gpus` 可能为 null（JSON 忽略 null）。

## 2025-08-11 06:00
- 文档更新：
  - `doc/dev-plan.md` 新增“12.6 缺失指标结构化清单与补全路线图”，明确数据源、后端/桥接/前端改动与验收标准，并给出里程碑与通用要求。
  - 修复“12.5 优先落地清单”误删项，补回“3) 磁盘每盘/分区容量与可用空间、SMART 健康（可用则展示）”。
- 后续动作：
  - 按里程碑推进：优先落地“CPU 每核心指标”“Wi‑Fi 指标”“网络接口基础信息”“GPU 显存/功耗”。

## 2025-08-11 06:40
- CPU 每核心指标全链路打通（桥接→后端→前端）：
  - 桥接（C# `sensor-bridge/Program.cs`）：已输出 camelCase 数组字段 `cpuCoreLoadsPct`/`cpuCoreClocksMhz`/`cpuCoreTempsC`（此前已存在，无需变更）。
  - 后端（Rust `src-tauri/src/lib.rs`）：
    - `BridgeOut` 与 `SensorSnapshot` 扩展可选数组字段：`cpu_core_loads_pct: Option<Vec<Option<f32>>>`、`cpu_core_clocks_mhz: Option<Vec<Option<f64>>>`、`cpu_core_temps_c: Option<Vec<Option<f32>>>`。
    - 采样/广播线程在快照组装时将上述数组赋值并随 `sensor://snapshot` 事件发往前端。
  - 前端：
    - `src/main.ts` 的 `SensorSnapshot` 类型新增三个可选数组：`cpu_core_loads_pct?`、`cpu_core_clocks_mhz?`、`cpu_core_temps_c?`（元素允许 null）。
    - `src/views/Details.vue`：
      - 类型同步扩展，同名三数组。
      - 新增 `fmtCoreLoads`/`fmtCoreClocks`/`fmtCoreTemps` 三个格式化函数（最多预览前 8 个核心，超出以 `+N` 汇总）。
      - 详情页网格新增三行：“CPU每核负载/CPU每核频率/CPU每核温度”。
- 构建验证：
  - `cargo check`（目录：`src-tauri/`）通过。
  - `npm run build`（根目录）通过（`vue-tsc --noEmit` 与 Vite 构建均成功）。
- 说明与兼容：
  - 每核心数组采用“可选向量+可选元素”以容忍缺失核心读数；UI 对缺失项显示“—”。
  - 字段命名：桥接 camelCase，Rust 端 snake_case 通过 Serde 对齐；前端字段与 Rust 保持一致（snake_case）。
- 下一步：
  1) 启动应用进行端到端联调，观察每核心数组长度与逻辑核心数一致性。
  2) 在 NUC8 等平台对比普通/管理员权限差异（配合已知“NUC RPM 不公开”结论）。
  3) 视需要在前端新增每核心图表/展开面板（后续迭代）。

## 2025-08-11 20:11
 - 新增文档：`doc/plan.md`
  - 内容包含：项目技术栈、项目目标、工程特点、接下来要完成的任务（路线图/优先级）。
  - 命名/对齐规则：桥接 camelCase、Rust snake_case（Serde 映射）、前端与 Rust 同步；UI 无值显示“—”。

## 2025-08-11 20:52
- 管理员测试文档：新增 `doc/script/ADMIN-TEST-GPU.md`，包含管理员 PowerShell 下 GPU VRAM/PWR 端到端验证步骤、观测要点、测试用例与期望、日志/诊断与清理方法。
- 前端展示校验：`src/views/Details.vue` 的 `fmtGpus()` 确认 `VRAM` 取整（0 位）、`PWR` 保留 1 位小数；缺失值显示“—”；展示格式为 `VRAM <n> MB PWR <m> W`，与设计一致。
- 构建验证：`npm run build` 通过；`cargo check`（`src-tauri/`）通过。
- 下一步：在 Tauri 窗口内观察 GPU 行实时值（无数据则为“—”），并结合管理员运行进一步验证显存/功耗传感器可用性；如发现缺失，按链路（C# 采集→Rust 映射→前端展示）逐段排查。

## 2025-08-12 03:52
- 托盘增强：在 `src-tauri/src/lib.rs` 增加 `info_gpu` 菜单项与 GPU 汇总行，tooltip 同步包含 GPU 行，便于快速观察显存/功耗。
- 展示规则：最多展示 2 块 GPU，单项格式：`<Name> VRAM <n> MB PWR <m> W`；缺失值为“—”，超出以 `+N` 汇总。
- 构建验证：`cargo check` 通过；`npm run dev:all` 已在运行，Tauri dev 会自动热重建。
- 期望效果：托盘菜单与鼠标悬浮提示均可看到 GPU VRAM/PWR 关键指标，与详情页一致。

## 2025-08-11 21:20
- GPU 指标扩展（显存/功耗）全链路打通：
  - 桥接（C# `sensor-bridge/Program.cs`）：`GpuInfo` 新增可选字段 `VramUsedMb`、`PowerW`，`CollectGpus()` 识别并采集 VRAM 使用与功耗（多关键字匹配，单位换算与异常值过滤）。
  - 后端（Rust `src-tauri/src/lib.rs`）：`BridgeGpu`/`GpuPayload` 新增 `vram_used_mb`、`power_w`，扩展 `BridgeOut.gpus -> SensorSnapshot.gpus` 映射，保持 Serde 对齐（桥接 camelCase → Rust snake_case）。
  - 前端（Vue3）：`src/main.ts` 与 `src/views/Details.vue` 的 `SensorSnapshot.gpus[]` 类型新增 `vram_used_mb`、`power_w`；`fmtGpus()` 增加 VRAM（MB）与功耗（W）展示。
- 构建与验证计划：
  1) `cargo check` 验证 Rust 端；
  2) `npm run build` 验证前端类型与构建；
  3) 本机运行观察“GPU”行显示 VRAM 与 PWR 字段；不存在/不支持时显示“—”。
 
## 2025-08-11 21:28
- 构建验证结果：
  - Rust 后端 `cargo check` 通过（`src-tauri/`）。
  - 前端 `npm run build` 通过（Vite 产物已生成至 `dist/`）。

## 2025-08-11 21:32
- 启动前端开发服务器：执行 `npm run dev`（Vite），端口 `1422`（`vite.config.ts` 中 `server.port=1422` 且 `strictPort=true`）。
- 计划：通过浏览器预览访问 `http://localhost:1422`，验证 `Details.vue` 的 GPU 字段展示（VRAM MB / PWR W），无值显示“—”。

## 2025-08-12 03:24
- 端到端联调：
  - 释放被占用的端口 `1422`（定位 PID=27348 并终止）。
  - 执行 `npm run tauri:dev`（Vite + Rust + C# bridge）：Vite `ready` on 1422；Rust `sys-sensor.exe` 运行；bridge 启动成功，日志持续 `[emit] sensor://snapshot ...`。
  - 说明：直接用浏览器访问 1422 时，`@tauri-apps/api` 在非 Tauri 环境下不可用，`Details.vue` 会记录 warn，但 UI 不受影响；真实数据请在 Tauri 窗口中查看。
- 下一步：在 Tauri 窗口核对 `GPU` 行新增字段展示（`VRAM <n> MB  PWR <n> W`），无数据显示“—”；如缺失则排查桥接字段/映射与前端格式化。

## 2025-08-11 20:30
- 构建与类型验证：
  - 后端 `cargo check` 通过（`src-tauri/`）。
  - 前端 `npm run build` 通过（`vue-tsc --noEmit` 与 Vite 构建均成功）。
- 新字段连通性确认：
  - 后端 `SensorSnapshot` 已包含 `logical_disks: Option<Vec<LogicalDiskPayload>>`、`smart_health: Option<Vec<SmartHealthPayload>>`，并随 `sensor://snapshot` 序列化广播。
  - 前端 `src/main.ts` 的 `SensorSnapshot` 类型与 `src/views/Details.vue` 已对齐并渲染。
- UI 表现：
  - 详情页网格已显示“磁盘容量”“SMART健康”；缺失/空值时按既定格式显示“—”。
- 说明：本次仅进行静态构建与类型校验；建议后续在运行态继续观察上述两项在目标机上的实际数据展示。
- 作为“项目总览与路线图”入口，后续随功能推进持续更新。
- 后续：继续端到端联调 CPU 每核心指标；推进“网络基础信息与 Wi‑Fi 指标”“磁盘容量/SMART”“GPU 显存/功耗”等高优先级任务。
- 前端展示：`Details.vue` 网格新增三行：Wi‑Fi SSID、Wi‑Fi信号、Wi‑Fi链路（Mbps）。
- 构建验证：根目录执行 `npm run build` 通过（`vue-tsc` 与 Vite 构建成功）。
- 说明与兼容：Wi‑Fi 字段均为可选；无连接/解析失败时前端显示“—”。
 - 下一步：
  1) 运行应用进行端到端联调，观察 Wi‑Fi 指标在不同语言系统下的解析兼容性。
  2) 视需要优化 `netsh` 输出解析与错误处理；评估是否补充 .NET 桥接侧实现（目前不依赖）。

## 2025-08-11 21:10
- 字段梳理与缺口对比（iStat 对齐基线）：
  - 已支持（Rust `src-tauri/src/lib.rs` 中 `SensorSnapshot` 与前端 `src/main.ts`/`src/views/Details.vue` 同名类型）：
    `cpu_usage`、`mem_used_gb/mem_total_gb/mem_pct`、`net_rx_bps/net_tx_bps`、
    `wifi_ssid/wifi_signal_pct/wifi_link_mbps`、
    `net_ifs{name/mac/ips/link_mbps/media_type}`、
    `disk_r_bps/disk_w_bps`、`cpu_temp_c/mobo_temp_c/fan_rpm`、
    `storage_temps[]`、`logical_disks[]`、`smart_health[]`、
    `hb_tick/idle_sec/exc_count/uptime_sec`、
    `cpu_pkg_power_w/cpu_avg_freq_mhz/cpu_throttle_active/cpu_throttle_reasons/since_reopen_sec`、
    `cpu_core_loads_pct/cpu_core_clocks_mhz/cpu_core_temps_c`、
    `disk_r_iops/disk_w_iops/disk_queue_len`、`net_rx_err_ps/net_tx_err_ps/ping_rtt_ms`、
    `gpus{name/temp_c/load_pct/core_mhz/fan_rpm}`、`timestamp_ms`。
- 与 iStat Menus 相比的优先缺口（Windows 可行性）：
  1) Wi‑Fi 进阶：信道/频段（2.4/5/6GHz）/带宽、Radio 类型（802.11ac/ax）、BSSID、独立 RX/TX 速率、RSSI dBm、加密类型。数据源：`netsh wlan show interfaces`。优先级：高。
  2) 内存细分：可用/缓存/提交/交换（含使用率与分页指标）。数据源：WMI Perf“Memory”、`Win32_OperatingSystem`。优先级：高。
  3) GPU 扩展：显存占用（MB/%）、功耗（W）、显存频率。数据源：LibreHardwareMonitor。优先级：高。
  4) 电池健康（笔记本）：循环次数、设计/满充容量、当前电量/剩余时间。数据源：`Win32_Battery`/Power API。优先级：中。
  5) 主板电压/更多风扇：+12V/+5V/+3.3V、Vcore、各风扇转速/占空比。数据源：LibreHardwareMonitor。优先级：中。
  6) 磁盘 SMART 细项：通电时长/重映射/坏块/寿命% 等重点属性。数据源：WMI/MSFT_* 或厂商工具，先择要。优先级：中。
  7) 网络细分：按网卡的上下行速率/错误/丢包/MTU/双工，默认路由/外网 IP。数据源：WMI Perf、`nslookup`/HTTP。优先级：中。
- 下一步实施计划：
  - 第1步（当前迭代）：扩展 Wi‑Fi 解析，新增字段 `wifi_bssid`、`wifi_channel`、`wifi_radio`、`wifi_band`、`wifi_rx_mbps`、`wifi_tx_mbps`、`wifi_rssi_dbm`；同步前端类型与 `Details.vue` 展示。
  - 第2步：内存细分（物理/缓存/提交/交换）与 UI 分组。
  - 第3步：桥接新增 GPU 显存/功耗，Rust 透传，前端汇总展示。
  - 第4步：电池/电压与磁盘 SMART 细项（按平台可用性启用，UI 友好降级）。
- 验收：每步完成后更新 `doc/progress.md`/`doc/plan.md`，并以 `npm run dev:all` 联调验证；浏览器预览保持静默降级。

## 2025-08-11 21:15
- SensorSnapshot 字段梳理与 iStat 对齐：
  - 已梳理并对齐的字段：`cpu_usage`、`mem_used_gb/mem_total_gb/mem_pct`、`net_rx_bps/net_tx_bps`、
    `wifi_ssid/wifi_signal_pct/wifi_link_mbps`、
    `net_ifs{name/mac/ips/link_mbps/media_type}`、
    `disk_r_bps/disk_w_bps`、`cpu_temp_c/mobo_temp_c/fan_rpm`、
    `storage_temps[]`、`logical_disks[]`、`smart_health[]`、
    `hb_tick/idle_sec/exc_count/uptime_sec`、
    `cpu_pkg_power_w/cpu_avg_freq_mhz/cpu_throttle_active/cpu_throttle_reasons/since_reopen_sec`、
    `cpu_core_loads_pct/cpu_core_clocks_mhz/cpu_core_temps_c`、
    `disk_r_iops/disk_w_iops/disk_queue_len`、`net_rx_err_ps/net_tx_err_ps/ping_rtt_ms`、
    `gpus{name/temp_c/load_pct/core_mhz/fan_rpm}`、`timestamp_ms`。
  - 下一步：
    1) 扩展 Wi‑Fi 解析，新增字段 `wifi_bssid`、`wifi_channel`、`wifi_radio`、`wifi_band`、`wifi_rx_mbps`、`wifi_tx_mbps`、`wifi_rssi_dbm`；同步前端类型与 `Details.vue` 展示。
    2) 内存细分（物理/缓存/提交/交换）与 UI 分组。
    3) 桥接新增 GPU 显存/功耗，Rust 透传，前端汇总展示。
    4) 电池/电压与磁盘 SMART 细项（按平台可用性启用，UI 友好降级）。

## 2025-08-11 20:45
 - 修复桥接 C# 正则无效转义问题：`sensor-bridge/Program.cs` 使用逐字字符串（@）修正 `Regex.Match` 模式中的 `\s`/`\d` 转义。
 - 重新发布桥接并启动端到端联调：根目录执行 `npm run dev:all` 成功，Vite Dev 端口 `http://localhost:1422/` 就绪。
 - 下一步：在详情页验证 Wi‑Fi SSID/信号/链路的实时显示；完成后使用 `npm run clean:proc` 主动清理进程。

## 2025-08-11 21:05
- 前端类型：
  - `src/main.ts` 的 `SensorSnapshot` 类型新增可选字段：
    - `net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string }[]`
    - `smart_health?: { device?: string; predict_fail?: boolean }[]`
- 详情页 UI：
  - `src/views/Details.vue` 同步扩展类型，并新增格式化函数：`fmtBytes`/`fmtNetIfs`/`fmtDisks`/`fmtSmart`。
  - 详情页网格新增三行展示：网络接口、磁盘容量、SMART 健康（含 +N 汇总策略）。
- 构建验证：
  - `cargo check`（目录：`src-tauri/`）通过。
  - 根目录 `npm run build` 通过（`vue-tsc` 与 Vite 构建成功）。
- 说明与兼容：
  - 前端字段与 Rust 端保持 snake_case 对齐；均为可选，缺失显示“—”。
  - 列表字段仅预览前若干项，并以 `+N` 汇总剩余，避免 UI 拖长。
- 下一步：
  1) 启动应用进行端到端联调，确认网卡 IP/MAC、链路速率与介质类型显示；
  2) 在多盘环境验证逻辑盘容量与可用空间；
  3) 验证 SMART 预警在不同品牌与接口（SATA/NVMe/USB 转接）下的可用性与权限要求；
  4) 视需要在 `Details.vue` 分组与折叠显示网络/存储信息。

## 2025-08-11 22:05
- 预览环境挂载异常修复：
  - 在 `src/views/Details.vue` 的 `onMounted` 中增加 Tauri 环境检测与 try/catch。
  - 非 Tauri（浏览器预览）场景下跳过 `@tauri-apps/api/event` 订阅，避免“mounted hook 未处理错误”。
- 构建验证：
  - 根目录执行 `npm run build` 通过（`vue-tsc` 与 Vite 构建成功）。
- 影响范围：仅前端挂载与事件订阅逻辑；不改变数据结构与 UI 展示（网络接口/磁盘容量/SMART 健康等仍保持可选与容错）。
- 下一步：
  1) 启动 `npm run dev:all` 进行端到端实时联调，确认浏览器预览不再报错、Tauri 环境可正常接收 `sensor://snapshot` 事件。
  2) 在真实硬件上验证网卡 IP/MAC/链路速率、逻辑盘容量与可用空间、SMART 预警显示。

## 2025-08-11 22:30
- 端到端联调：根目录执行 `npm run dev:all`，Vite Dev 端口 `http://localhost:1422/` 启动，Tauri 后端启动并拉起桥接。
- 桥接与事件广播：
  - 后端新增调试日志，确认每 1 秒 `emit("sensor://snapshot")` 成功，示例：
    - `[emit] sensor://snapshot ts=... cpu=48% mem=54% net_rx=0 net_tx=0`
    - `[emit] sensor://snapshot ts=... cpu=44% mem=53% net_rx=19202 net_tx=205573`
  - 桥接状态：`[bridge][status] data became FRESH`，表明桥接输出有效。
- 浏览器预览说明：
  - 通过代理预览页面控制台见到 `Details.vue` 的提示：`[Details] Tauri API 不可用：运行于普通浏览器预览，禁用事件订阅`。
  - 由于非 Tauri 环境，前端不会订阅事件，UI 将显示“—”（预期行为）。
- 结论：后端事件与桥接数据正常；需要在 Tauri 应用窗口内观察传感器数值是否从“—”变为实时数值。
- 下一步：
  1) 在 Tauri 窗口内确认前端是否收到并渲染快照（CPU/内存/网络/温度/风扇等）。
  2) 若仍显示“—”，将在 `Details.vue` 的订阅回调内临时增加 `console.debug` 并核对字段命名/类型。
  3) 视需要对 `src/main.ts` 顶层订阅补充 Tauri 环境检测以提升浏览器预览的健壮性。

## 2025-08-11 23:55
- 配置命令与设置页联调核验：
  - 后端 `src-tauri/src/lib.rs` 已实现并注册 Tauri 命令：`get_config()`、`set_config(app, state, new_cfg)`、`list_net_interfaces()`；`set_config` 会持久化到 `AppConfig/config.json` 并 `emit("config://changed")`。
  - 前端 `src/views/Settings.vue` 在挂载时调用 `get_config` 与 `list_net_interfaces`，保存时以 `await invoke("set_config", { newCfg: new_cfg })` 传参（camelCase → snake_case 映射已对齐）。
  - UI 绑定：`trayBottomMode`（"cpu"|"mem"|"fan"）与 `selectedNics`；兼容旧字段 `tray_show_mem`（便于旧版本读取）。
- 结论：命令与参数命名端到端一致，配置持久化路径与事件广播已确认。
- 下一步：
  1) 在 Tauri 窗口内更改设置并保存，观察托盘第二行与网卡聚合是否按配置生效。
  2) 视需要在前端监听 `config://changed` 给予保存成功提示或刷新逻辑。
  3) 持续扩展网络/Wi‑Fi/存储 SMART 等指标的 UI 展示与容错。

## 2025-08-11 23:58
- 前端设置页改进：
  - 在 `src/views/Settings.vue` 中增加 `config://changed` 事件监听（`@tauri-apps/api/event.listen`）。保存后自动刷新配置；在 `onUnmounted` 中清理监听句柄。
  - 非 Tauri 环境监听失败将静默降级并打印告警，避免浏览器预览报错。
- 联调：
  - 已执行 `npm run dev:all`，Vite Dev 地址 `http://localhost:1422/`，Tauri 后端启动并每秒 `emit("sensor://snapshot")` 正常；桥接状态 FRESH。
- 下一步：
  1) 在 Tauri 窗口内切换“托盘第二行显示/网卡聚合”，点击保存，确认变更即时生效（可观察托盘与速率来源）。
  2) 如需，设置页可加入保存成功提示（toast）与禁用状态反馈。

## 2025-08-11 23:59
- Wi‑Fi 扩展前端对齐：
  - `src/main.ts` 的 `SensorSnapshot` 新增：`wifi_bssid`、`wifi_channel`、`wifi_radio`、`wifi_band`、`wifi_rx_mbps`、`wifi_tx_mbps`、`wifi_rssi_dbm`。
  - `src/views/Details.vue` 同步类型、增加格式化函数 `fmtWifiMeta`/`fmtWifiRates`/`fmtWifiRssi`，并在网格新增 BSSID/参数/速率/RSSI 行。
- 构建验证：
  - `src-tauri/` 下 `cargo check` 通过；为旧函数 `read_wifi_info()` 添加 `#[allow(dead_code)]` 抑制未使用告警。
  - 根目录 `npm run build` 通过。
- 下一步：
  1) 在 Tauri 窗口运行 `npm run dev:all` 验证 Wi‑Fi 扩展字段的实时渲染。
  2) 如需，在 `Details.vue` 订阅回调中临时加入 `console.debug` 以核对字段到 UI 的映射。

## 2025-08-11 23:59（补充）
- 完善 `.gitignore`：
  - 忽略 Rust/Tauri 构建产物：`/target`、`/src-tauri/target`、`/src-tauri/target/**/bundle/**`。
  - 忽略 .NET 桥接输出：`/sensor-bridge/bin/`、`/sensor-bridge/obj/`、`.vs/.idea`。
  - 忽略打包资源：`/src-tauri/resources/sensor-bridge/*`，但保留 `.gitkeep` 以满足打包通配符。
  - 忽略便携包目录：`/dist-portable`。
- 目的：避免大体积/临时构建产物入库，保持仓库干净、可重复构建。
- 验证：`git status` 不再包含上述构建产物；后续提交仅包含源码/配置/文档。

## 2025-08-12 01:33（Wi‑Fi 解析稳健性增强）
- 新增 `encoding_rs` 依赖，并在 `src-tauri/src/lib.rs` 引入 `decode_console_bytes()`，优先 UTF‑8，失败回退 GBK，最后损失性 UTF‑8，解决中文 Windows 上 `netsh` 输出乱码导致字段为 None 的问题。
- 扩展关键词匹配：
  - 信号：支持“信号”“信号质量”。
  - 信道：支持“channel/信道/通道/频道”。
  - 仍保留英/中文“接收速率/传输速率 (Mbps)”。
- Debug 下，当关键字段均为 None 时打印 `[wifi][raw]` 原始 `netsh wlan show interfaces` 文本，便于比对实际标签。
- 预期：`[wifi][parsed]` 中的 `signal%/ch/radio/band/rx/tx/bssid/rssi` 不再为 None；band 可由信道推断（1-14 -> 2.4GHz，32-177 -> 5GHz）。
- 测试建议：运行 `npm run tauri dev`，观察控制台日志；若仍为 None，请贴出 `[wifi][raw]` 片段以便进一步适配。

## 2025-08-12 02:24（SMART 健康显示“—”的修复）
- 现象：管理员 PowerShell 运行应用时，`SMART健康` 显示为 `—`。
- 根因分析：
  - 部分 NVMe/SSD 在 `ROOT\\WMI` 下的 `MSStorageDriver_FailurePredictStatus` 无实例返回（或驱动不提供），导致后端 `smart_health` 为 `None`。
  - 但 `Get-PhysicalDisk` 可见磁盘且 `HealthStatus = Healthy`。
- 修复方案（回退）：
  - 优先使用 `MSStorageDriver_FailurePredictStatus`（命名空间：`ROOT\\WMI`）。
  - 若无数据，则回退使用 `Win32_DiskDrive.Status`（命名空间：`ROOT\\CIMV2`）作为近似健康。
  - 实现：
    - 新增 `Win32DiskDrive` 结构与 `wmi_fallback_disk_status()`。
    - 在快照构建处：先取 `wmi_list_smart_status()`，若 `None` 再取 `wmi_fallback_disk_status()`。
  - 编译验证：`src-tauri/` 下 `cargo check` 通过。
- 前端表现：
  - 只要列表非空，`fmtSmart()` 将显示 `OK (N)`；若存在 `predict_fail === true` 的项，将显示 `预警 X`。
- 建议验证：
  1) 运行应用（或 `npm run tauri dev`），观察“SMART健康”是否变为 `OK (N)` 或 `预警 X`。
  2) 同机对比 PowerShell：`Get-PhysicalDisk | Select FriendlyName,HealthStatus`。
  3) 如仍为 `—`，请贴出调试日志中 `snapshot.smart_health` 片段以便进一步分析。

## 2025-08-12 02:41（SMART 运行态验证）
- 结果：界面显示 `SMART健康  OK (1)`。
- 含义解释：前端 `src/views/Details.vue` 中 `fmtSmart()` 在无 `predict_fail` 的情况下显示 `OK (N)`，其中 `N` 为后端 `smart_health` 列表长度，即检测到的磁盘条目数。本机为 `1` 表示有 1 个磁盘并且没有预警。
- 预期：如有多块磁盘且均健康，会显示 `OK (2)`、`OK (3)` 等；若任何盘 `predict_fail === true`，则显示 `预警 X`。
- 结论：与设计一致，回退方案生效。

## 2025-08-12 03:10（公网 IP/ISP 前端集成）
- 类型对齐：
  - 在 `src/main.ts` 的 `SensorSnapshot` 新增可选字段：`public_ip?: string`、`isp?: string`。
  - 在 `src/views/Details.vue` 的本地 `SensorSnapshot` 同步新增上述字段。
- UI 展示：
  - 在 `src/views/Details.vue` 模板网格新增两行：`公网IP`、`运营商`，无数据时显示 `—`。
  - 该数据随 `sensor://snapshot` 事件更新；若后端轮询尚未成功或配置关闭，将显示 `—`。
- 后端现状回顾：
  - `src-tauri/src/lib.rs` 已实现公网 IP/ISP 缓存、后台轮询（主 `ip-api.com`，回退 `ipinfo.io`），并将 `public_ip/isp` 纳入 `SensorSnapshot` 广播与托盘展示。
- 构建与验证计划：
  1) 在 `src-tauri/` 下执行 `cargo check` 验证 Rust 端编译。
  2) 在仓库根目录执行 `npm run build` 验证前端类型与打包。
  3) 运行应用（或 `npm run dev:all`/`npm run tauri dev`）观察详情页“公网IP/运营商”是否正确显示；托盘 tooltip 亦应包含该信息。
- 后续改进（可选）：
  - 前端在公网查询失败时可显示轻量提示（例如 `暂无公网信息`）或悬浮说明（`配置已关闭/暂未拉取成功`）。

## 2025-08-12 04:15（电池 AC/剩余/充满耗时 前端对齐与后端修复）
- 类型对齐（前端）：
  - 在 `src/main.ts` 的 `SensorSnapshot` 新增可选字段：`battery_ac_online?: boolean`、`battery_time_remaining_sec?: number`、`battery_time_to_full_sec?: number`。
  - 在 `src/views/Details.vue` 的本地 `SensorSnapshot` 同步新增上述字段。
- UI 展示：
  - `Details.vue` 新增格式化函数：`fmtBatAC()`（显示“接通/电池”）与 `fmtDuration()`（按 `h/m/s` 简洁显示）。
  - 网格新增三行：`AC电源`、`剩余时间`、`充满耗时`；无数据显示“—”。
- 后端修复与对齐（Rust `src-tauri/src/lib.rs`）：
  - 修复 `read_power_status()`：`GetSystemPowerStatus(&mut sps).as_bool()` 改为 `.is_ok()`，解决编译错误（E0599）。
  - 采样循环整合 WinAPI 与 WMI：
    - AC 接入取自 WinAPI；`battery_time_remaining_sec` 优先 WMI，回退 WinAPI；`battery_time_to_full_sec` 取自 WMI（WinAPI 无该值）。
  - 将 `battery_ac_online/battery_time_remaining_sec/battery_time_to_full_sec` 填充进 `SensorSnapshot` 广播。
- 构建验证：
  - `src-tauri/` 下 `cargo check` 通过（存在若干非致命告警，属初始化赋值后被覆盖的提示）。
  - 根目录 `npm run build` 通过。
- 运行验证建议（需管理员权限）：
  1) 执行 `npm run dev:all` 或 `npm run tauri dev` 启动应用（建议以管理员 PowerShell）。
  2) 在 Tauri 窗口的详情页观察：
     - `AC电源` 应显示“接通/电池”。
     - `剩余时间/充满耗时` 显示为 `xhym`/`xm ys`/`zs`；无数据显示“—”。
  3) 若笔记本在充电，`充满耗时` 可能由 WMI 提供；若为 `—` 属设备未提供/驱动不支持的常见情况。

## 2025-08-12 05:10（Wi‑Fi 安全/信道宽度与网卡细节前端对齐）
- 类型对齐：
  - 在前端 `src/main.ts` 的 `SensorSnapshot` 新增可选字段：`wifi_auth?: string`、`wifi_cipher?: string`、`wifi_chan_width_mhz?: number`。
  - 扩展 `net_ifs[]` 项：`gateway?: string[]`、`dns?: string[]`、`dhcp_enabled?: boolean`、`up?: boolean`。
- 详情页 UI：
  - `src/views/Details.vue` 本地 `SensorSnapshot` 同步新增上述字段。
  - 新增格式化函数：`fmtWifiSec(auth, cipher)`、`fmtWifiWidth(mhz)`；`fmtNetIfs()` 扩展拼接 `UP/DOWN`、`DHCP/静态`、`GW <x>`、`DNS <x>`。
  - 模板网格新增两行：`Wi‑Fi安全`、`Wi‑Fi信道宽度`；`网络接口`行将显示状态/DHCP/网关/DNS 聚合摘要。
- 后端对齐回顾：
  - Rust `src-tauri/src/lib.rs` 先前已扩展 `WifiInfoExt` 解析 `auth/cipher/chan_width_mhz`，并在快照中映射为 `wifi_auth/wifi_cipher/wifi_chan_width_mhz`。
  - WMI 网卡扩展已在 `NetIfPayload` 中加入 `gateway/dns/dhcp_enabled/up`，并随快照广播。
- 构建与验证计划：
  1) `cargo check`（`src-tauri/`）验证 Rust 编译。
  2) 根目录 `npm run build` 验证前端类型与打包。
  3) 运行 `npm run dev:all`，在 Tauri 窗口验证新字段显示；无数据时应显示“—”。
  4) 多语言系统下核对 `netsh wlan show interfaces` 解析兼容性（Wi‑Fi 安全/加密/带宽）。

## 2025-08-12 17:13（Wi‑Fi 信道宽度解析修复）
- 背景：测试发现 `Wi‑Fi信道宽度` 显示为 `—`，其余指标正常。
- 根因分析：`read_wifi_info_ext()` 中对“信道/Channel”的放宽匹配（`contains("channel"|"信道"...)`）会先于“信道宽度/Channel width”命中，导致带宽行被“信道”分支提前消费，`chan_width_mhz` 未被填充。
- 修复方案（`src-tauri/src/lib.rs`）：
  - 在循环中先解析冒号左侧键名 `key/keyl`，对“信道”仅以键名精确匹配：`Channel/信道/通道/频道`。
  - 对“信道宽度/带宽”以键名匹配：`Channel width/Channel bandwidth`，以及中文同义词：`信道宽度/通道宽度/频道宽度/信道带宽/通道带宽/频道带宽`。
  - 仍以冒号右侧提取数值，形如 `80 MHz` 抽取为 `80` 并写入 `chan_width_mhz`。
- 结果：
  - `cargo check` 通过（仅若干非致命告警）。
  - 预期 Tauri 界面“Wi‑Fi信道宽度”显示为如 `80 MHz`（无数据时显示 `—`）。
- 验证建议（需管理员权限启动应用）：
  1) 运行 `npm run dev:all` 或 `npm run tauri dev`。
  2) 在 `详情` 页观察“Wi‑Fi信道宽度”，应出现 `N MHz`；切换不同 AP/频段验证 20/40/80/160MHz。
  3) 在中文/英文系统对比 `netsh wlan show interfaces` 输出，以确认同义词匹配兼容性。

## 2025-08-12 18:20（SensorSnapshot 前后端一致性复核）
 - 结论：前端 `SensorSnapshot` 与后端 `SensorSnapshot` 字段完全一致，无需改动。
 - 对齐范围：
   - 电池：`battery_percent/battery_status/battery_ac_online/battery_time_remaining_sec/battery_time_to_full_sec`（均为可选，单位秒已注明）。
   - 公网：`public_ip/isp`（可选）。
   - GPU：`gpus[]{name/temp_c/load_pct/core_mhz/fan_rpm/vram_used_mb/power_w}`（可选数值容忍 `null`）。
   - 网络接口：`net_ifs[]{name/mac/ips/link_mbps/media_type/gateway/dns/dhcp_enabled/up}`（类型与可选性一致）。
   - 其它：`cpu_* / mem_* / disk_* / storage_temps / logical_disks / smart_health / 每核数组 / 错误率 / ping / hb/idle/exc/uptime` 均与 Rust 端匹配；`timestamp_ms` 为必填。
 - 依据：
   - 后端定义：`src-tauri/src/lib.rs` 内 `struct SensorSnapshot` 与 `GpuPayload/NetIfPayload/...`
   - 前端定义：`src/main.ts` 与 `src/views/Details.vue` 顶部本地 `type SensorSnapshot`
 - 后续验证（管理员运行）：
   1) `npm run dev:all` 启动端到端联调，在 Tauri 窗口观察“公网IP/运营商”“电池 AC/剩余/充满耗时”渲染与托盘展示。
   2) 如遇空值，检查后台公网轮询日志与电源 API/WMI 可用性；前端控制台 `console.debug` 已输出 `snapshot` 便于对照。

## 2025-08-12 19:55（构建验证与电池/公网数据核验）
- 构建：`cargo check`（src-tauri/）通过；`npm run build` 通过。
- 告警：Rust 存在少量非致命告警（未使用变量/初始化后覆盖）；不影响运行，后续清理。
- 测试点（建议管理员运行）：
  1) 详情页“AC电源/剩余时间/充满耗时”显示是否正确（无数据显示“—”）；充电中如 WMI 提供 `time_to_full_sec` 将显示。
  2) 详情页“公网IP/运营商”是否显示；如配置关闭或查询未成功则为“—”。
  3) 托盘 tooltip 含 GPU 与 公网信息行。

## 2025-08-12 19:30（网络接口详情展开查看 UI）
 - 变更内容（前端 `src/views/Details.vue`）：
  - 新增响应式开关 `showIfs` 与方法 `toggleIfs()`，控制网络接口详情展开/收起。
  - “网络接口”行追加“展开/收起”链接，当存在 `snap.net_ifs` 时可切换。
  - 新增详情区块 `.netifs-list`：逐项展示 `name/up/link_mbps/media_type/mac/ips/dhcp_enabled/gateway/dns`。
  - 新增相关样式：`.item .link`、`.netifs-list`、`.netif-card` 等，适配深浅色主题。
- 后端影响：无。沿用现有 `SensorSnapshot.net_ifs[]` 字段，空值在 UI 显示为“—”。
- 构建与验证：
  1) 在 `src-tauri/` 下执行 `cargo check` 验证 Rust 端；
  2) 在仓库根目录执行 `npm run build` 验证前端打包；
  3) 运行 `npm run dev:all`（建议管理员 PowerShell），在“详情”页点击“网络接口 → 展开”，应显示各网卡详细信息；再次点击“收起”恢复摘要。
- 预期结果：
  - 概要行保持原有两项聚合展示（含 UP/DOWN、DHCP/静态、GW/DNS 摘要）。
  - 展开后可见全部接口及其各字段；缺失数据按“—”显示。

## 2025-08-12 20:30（快照组装与广播确认 + 构建验证）
- 位置确认（后端）：
  - 在 `src-tauri/src/lib.rs` 采样循环尾部完成快照组装与广播：
    - 快照构建：`let snapshot = SensorSnapshot { ... };`（约在 2143 行附近）。
    - 事件广播：`app_handle_c.emit("sensor://snapshot", snapshot);`（约在 2212 行）。
  - 已核对 `SensorSnapshot` 填充项包含：
    - 电池：`battery_percent/battery_status/battery_ac_online/battery_time_remaining_sec/battery_time_to_full_sec`。
    - 公网：`public_ip/isp`（由后台线程轮询并缓存）。
    - 其它：`wifi_* / net_ifs / logical_disks / smart_health / gpus / disk_* / 每核数组 / 错误率 / ping / hb/idle/exc/uptime / timestamp_ms` 等。
- 托盘信息：
  - 更新行包括 `public_line/gpu_line/storage_line/bridge_line` 等；tooltip 已包含“公网/GPU/存储温度/桥接”等信息。
- 构建结果：
  - 根目录 `npm run build`：通过（vite 6.x）。
  - `src-tauri/` 下 `cargo check`：通过，出现 7 条非致命告警：
    - 未使用变量：`keyl`（多处 Wi‑Fi 解析处提示）。
    - 初始化后覆盖（unused_assignments）：`battery_ac_online/time_remaining_sec/time_to_full_sec`。
    - 结构体字段未读：`AppState.public_net`。
  - 以上不影响运行，后续清理或以 `_` 前缀抑制。
- 运行测试点（建议管理员 PowerShell）：
  1) 运行 `npm run dev:all` 或 `npm run tauri dev`，打开“详情”页。
  2) 验证“AC电源/剩余时间/充满耗时”字段显示（无数据为“—”），切换充放电场景观察变化。
  3) 验证“公网IP/运营商”显示；tooltip 中应出现“公网”与“GPU/存储/桥接”行。
  4) 打开 DevTools 观察 `sensor://snapshot` 事件频率与字段值是否随时间更新。

## 2025-08-12 20:36（计划文档 Roadmap 标注为“已完成”）
- 修改 `doc/plan.md`：
  - 将 Tier 1 四项功能标注为“已完成”：
    1) 电池充电状态与剩余/充满耗时。
    2) 公网 IP 与 ISP。
    3) 每网卡详情与链路参数。
    4) Wi‑Fi 细节补充（安全/信道宽度等）。
  - 将 Step A 的三项任务标注为“已完成”。
  - 更新“待办（见路线图）”：保留后续项（内存细分、主板电压与更多风扇、GPU 细分指标、SMART 关键属性简表、Top 进程、多目标 RTT、电池健康）。
- 构建状态：编译与打包测试均已通过（见上文 20:30 小节）。
- 下一步建议：按 `plan.md` 进入 Tier 2/3 功能项的分解与实现，并在每步后追加进展与测试点。

## 2025-08-12 20:55（GPU 电压监测全链路）
- 目标：为 GPU 指标新增“电压（V）”，并贯通 桥接（C#）→ Rust → 前端（Vue3）。
- 桥接（C# `sensor-bridge/Program.cs`）：
  - `GpuInfo` 新增可空字段 `VoltageV`。
  - `CollectGpus()` 筛选电压类传感器，关键字包含 `core`/`vddc`/`gfx`；过滤异常值，仅保留 0.2–2.5V 区间。
  - 以 camelCase 序列化为 `voltageV` 并随 `gpus[]` 输出到桥接 JSON。
- 后端（Rust `src-tauri/src/lib.rs`）：
  - 扩展结构体：`BridgeGpu` 与 `GpuPayload` 新增 `voltage_v: Option<f64>`；`#[serde(rename_all = "camelCase")]` 对齐桥接字段。
  - 在 GPU 映射处透传：`voltage_v: x.voltage_v`（与 `vram_used_mb`、`power_w` 同步）。
- 前端（Vue3 + TS）：
  - `src/main.ts` 的 `SensorSnapshot.gpus[]` 新增可选字段 `voltage_v?: number`。
  - `src/views/Details.vue`：`fmtGpus()` 增加电压展示，保留 3 位小数、单位 `V`；缺失显示 `—`。
- 构建与验证：
  - dotnet：`dotnet build sensor-bridge/sensor-bridge.csproj` 通过（仅跨平台可用性警告）。
  - Rust：`cargo check` 通过（若干非致命告警）。
  - 前端：根目录 `npm run build` 通过。
  - 运行建议：`npm run dev:all` 或 `npm run tauri dev`，在“详情 → GPU”观察电压 `V <x.xxx> V`；无数据显示 `—`。
  - 兼容性：不同显卡/驱动可能不暴露核心电压，字段为空属预期，UI 已优雅降级。

## 2025-08-12 21:20（GPU 电压显示修正 + 平滑；隐藏子进程窗口）
- 目标：
  - 修复 GPU 汇总行在无电压数据时出现“V —”的视觉问题；减轻 GPU 风扇转速与电压间歇性空值导致的 UI 抖动。
  - 彻底避免启动期间或后台操作触发的命令行窗口闪烁（客户反馈）。
- 前端（`src/views/Details.vue`）：
  - 调整 `fmtGpus()`：仅当 `voltage_v` 有效时追加诸如 `1.000 V` 的片段；无值时不再拼接“V —”。
  - 新增轻量“短时平滑”机制：在 15s 窗口内，如当前快照 `fan_rpm/voltage_v` 缺失，则回填上一帧的有效值（按 `name` 对齐），减少 UI 闪烁。
  - 订阅处理由直接赋值改为 `smoothSnapshot(lastSnap, curr)`，并维持 `lastSnap` 以便回填。
- 后端（Rust `src-tauri/src/lib.rs`）：
  - 将所有辅助子进程统一设置为隐藏窗口（`CREATE_NO_WINDOW`）：
    - `netsh wlan show interfaces`（Wi‑Fi 解析）。
    - `powershell`（管理员检测与自提权启动）。
    - `taskkill`（必要时结束残留桥接进程）。
  - 方式：在 Windows 下使用 `std::os::windows::process::CommandExt::creation_flags(0x08000000)`。
- 构建与验证：
  - 建议在管理员 PowerShell 执行：
    1) 根目录 `npm run build`；
    2) `src-tauri/` 下 `cargo check`；
    3) 运行 `npm run dev:all` 打开“详情”页，观察 GPU 汇总：
       - 若无电压：不应出现“V —”。
       - 断续空值场景（桥接重启/短暂延迟）：风扇/电压应稳定，短时可回填上一帧。
    4) 启动全程及后台操作（Wi‑Fi 查询/自提权判定/清理进程）不应弹出/闪烁命令行窗口。
  - 兼容性：平滑仅在 15s 内生效，避免长期掩盖真实空值；可按需调整 `SMOOTH_TTL_MS`。
\n## 2025-08-12 22:02（GPU 细分指标扩展 + 三端构建验证）
- 目标：为 GPU 增加 memory_mhz、hotspot_temp_c、vram_temp_c，全链路打通并通过本地构建验证。
- 代码变更：
  - C# sensor-bridge/Program.cs：GpuInfo 新增 MemoryMhz/HotspotTempC/VramTempC；CollectGpus() 采集显存时钟/热点/VRAM 温度并 camelCase 输出。
  - Rust src-tauri/src/lib.rs：BridgeGpu/GpuPayload 增加 memory_mhz/hotspot_temp_c/vram_temp_c（#[serde(rename_all =  camelCase)]），映射桥接字段并随 snapshot 广播。
  - 前端 src/main.ts 与 src/views/Details.vue：SensorSnapshot.gpus[] 类型扩展；mtGpus() 新增显示显存时钟、Hotspot 温度与 VRAM 温度。
- 构建验证：
  - 前端 
pm run build：通过（Vite 6.x，产物生成正常）。
  - Rust cargo check：通过；警告 7 条（未使用变量 keyl 建议改为 _keyl；attery_* 未读赋值；AppState.public_net 未读）。
  - C# dotnet build sensor-bridge/sensor-bridge.csproj -c Release：通过；CA1416 Windows 平台 API 使用提示若干，符合预期。
- 注意：
  - 字段命名约定：桥接 JSON camelCase；Rust 内部 snake_case 由 serde 映射；前端使用 camelCase。
  - UI 对缺失数据显示 —；显存时钟自动选择 MHz/GHz 单位。
- 下一步：
  1) 以管理员权限运行 
pm run dev:all 或 
pm run tauri dev 做端到端手测，检查 GPU 卡片是否出现 Mem Clock/Hotspot/VRAM 三项。
  2) 复核多 GPU（>2）时的显示汇总与+N统计。
  3) 根据硬件差异校验传感器可用性与桥接日志。
  4) 视需要清理上述非功能性警告（可先以 _ 前缀做降噪）。
