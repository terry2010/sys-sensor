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
