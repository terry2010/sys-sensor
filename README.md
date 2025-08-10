# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

# sys-sensor Windows 托盘系统监控

一个基于 Tauri (Rust) + Vue3 的 Windows 托盘监控程序，实时显示 CPU 温度/占用、内存占用、网络与磁盘速率等信息。温度与风扇数据由内置的 .NET 8 传感器桥 `sensor-bridge`（使用 LibreHardwareMonitorLib）采集，并通过子进程与 Rust 后端通信。

## 功能概览

- 托盘区“纯文本图标”：32x32 双行文本（上：CPU 温度或 CPU%，下：CPU% 或内存%）。
- Tooltip 与右键菜单信息区：CPU/内存/主板温度/风扇/网络/磁盘实时数据。
- 多窗口页面：详情、设置、关于（从托盘菜单打开）。
- 传感器桥自包含打包：客户机无需安装 .NET 运行时。

## 技术栈

- 前端：Vue 3 + Vite + TypeScript
- 后端：Tauri v2（Rust）
- 传感器：.NET 8 + LibreHardwareMonitorLib 0.9.3（`sensor-bridge/`）

## 目录结构（简要）

- `src/` 前端源代码（多页面路由、详情/设置/关于）
- `src-tauri/src/lib.rs` Tauri 后端：托盘、传感器桥进程、事件广播
- `src-tauri/tauri.conf.json` Tauri 配置与打包设置
- `sensor-bridge/` .NET 8 传感器桥（输出 JSON 到 stdout）
- `doc/progress.md` 进度日志；`doc/项目总结与开发注意事项.md` 开发说明

## 环境要求（Windows 10）

- Node.js 18+（建议 20 LTS）
- Rust 工具链（stable；安装 rustup 与 cargo）
- .NET SDK 8.0（开发/构建时用于发布传感器桥；部署机不需要）

## 开发与运行（Dev）

1) 安装依赖

```powershell
npm install
```

2) 启动开发（前端 + 后端）

```powershell
cargo tauri dev
```

说明：开发态下后端会尝试本地启动 `sensor-bridge`（优先 dll 的 `dotnet <dll>`，其次 exe，最后 `dotnet run --project sensor-bridge`）。

## 构建与发布（Release）

一键构建安装包（会自动发布自包含的桥接并纳入 Tauri 资源）：

```powershell
# 推荐使用 npm 脚本（会自动发布桥接并处理代理）
npm run release:build:nsis:proxy-socks

# 或使用已有 NSIS 的本机环境
npm run release:build:nsis
```

输出：`src-tauri/target/release/bundle/` 下生成安装包（MSI/EXE，具体以 Tauri 配置为准）。

打包细节：

- `src-tauri/tauri.conf.json` 中：
  - `beforeBuildCommand` 会执行：
    `dotnet publish ./sensor-bridge -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true -o ./src-tauri/resources/sensor-bridge && npm run build`
  - `bundle.resources` 包含目录：`resources/sensor-bridge`，确保桥接随包分发。
- 运行时后端优先从 `BaseDirectory::Resource/sensor-bridge/sensor-bridge.exe` 启动桥接；失败时回退到开发路径。

### 一键脚本（推荐）

```powershell
# 1) NSIS 安装包（通过 socks5 代理 127.0.0.1:7890）
npm run release:build:nsis:proxy-socks

# 2) NSIS 安装包（通过 http/https 代理 127.0.0.1:7890）
npm run release:build:nsis:proxy

# 3) NSIS 安装包（本机已预装 NSIS 时）
npm run release:build:nsis

# 4) 绿色便携版 ZIP（无需安装，解压即用）
npm run release:portable
```

产物路径：

- 安装包（NSIS）：`src-tauri/target/release/bundle/nsis/*.exe`
- 便携版 ZIP：`dist-portable/sys-sensor-portable.zip`

若需手动查看产物：

```powershell
npm run bundle:ls
npm run portable:ls
```

## 面向客户的发布与部署（交付指南）

### 1) 前置准备（开发机）

- 必装环境：
  - Node.js 18+（推荐 20 LTS）
  - Rust 工具链（stable，MSVC）
  - .NET SDK 8.0（仅用于构建桥接；客户机无需 .NET）
  - WebView2 Runtime（多数 Win10/11 已自带）
- 清理遗留进程（避免占用）：

```powershell
taskkill /F /IM sys-sensor.exe /IM sensor-bridge.exe /IM dotnet.exe
```

- 如需发布新版本，修改 `src-tauri/tauri.conf.json` 的 `version`。

### 2) 一键打包安装包（开发机执行）

```powershell
cargo tauri build
```

过程会自动：
- `dotnet publish ./sensor-bridge` 到 `src-tauri/resources/sensor-bridge/`，产出自包含单文件 `sensor-bridge.exe`；
- 前端 `npm run build`；
- 将 `resources/sensor-bridge` 随安装包打入。

构建产物目录：`src-tauri/target/release/bundle/`（包含 `.msi` 与/或 `.exe`）。

### 3) 分发到客户机并安装

- 将 `.msi` 或 `.exe` 拷贝到客户电脑，双击安装。
- 首次运行后最小化到托盘。正常日志应见：`[bridge] spawning packaged exe:`（表示从资源目录启动桥接）。

### 4) 客户机前置条件与说明

- 无需安装 .NET（桥接为自包含）。
- 建议安装 WebView2 Runtime（若系统无）。
- 若安全软件拦截 `sensor-bridge.exe`，请加入信任/白名单。
- 某些机型（如 Intel NUC8）读取主板/风扇可能需要“以管理员身份运行”。

### 5) 发布前自测清单（强烈建议）

- 在干净环境（虚拟机或 Windows Sandbox）安装并运行安装包：
  - 托盘数据（CPU/内存/温度/风扇）正常刷新；
  - 日志包含“spawning packaged exe”；
  - 若温度/风扇为空，尝试管理员权限运行。

### 6) 可选：签名与企业部署

- 为消除 SmartScreen 提示，准备代码签名证书并按 Tauri 文档配置 Windows 签名再打包。
- 企业环境可用 SCCM/Intune 等分发 `.msi`/`.exe`。

## 网络/代理与依赖问题处理

- 在线打包时 Tauri 需从 GitHub 下载 NSIS/WiX 等二进制工具，如遇下载失败：
  - 已提供代理脚本：`release:build:nsis:proxy`（http 代理）、`release:build:nsis:proxy-socks`（socks5 代理）。
  - 系统已开启的 System Proxy/TUN 也会被尊重。
- 或者预装依赖后再打包（推荐至少装 NSIS）：

```powershell
# 安装 NSIS（用于 .exe 安装包）
winget install -e --id NSIS.NSIS

# 如切换到 MSI 目标（WiX），再安装 WiX（当前配置默认 NSIS，无需安装 WiX）
winget install -e --id WixToolset.WixToolset
```

## 脚本清单（摘录）

- `clean:proc`：结束遗留的 sys-sensor/sensor-bridge/dotnet 进程。
- `clean:cargo`：结束 cargo/rustc 进程。
- `bridge:publish`：发布自包含桥接到 `src-tauri/resources/sensor-bridge/`。
- `release:build:nsis`：一键清理->发布桥接->Tauri 打包(NSIS)->打开目录。
- `release:build:nsis:proxy`：同上，启用 HTTP(S) 代理 127.0.0.1:7890。
- `release:build:nsis:proxy-socks`：同上，启用 socks5 代理 127.0.0.1:7890。
- `release:portable`：一键产出绿色便携版 ZIP。
- `bundle:ls` / `portable:ls`：列出产物文件。

## 运行与使用

- 首次运行将最小化到托盘；托盘图标每秒刷新文本。
- 右键菜单可打开“显示详情 / 快速设置 / 关于我们”，再次点击同一项会聚焦已有窗口。
- 关闭窗口仅隐藏（托盘“退出”才真正退出）。

## 常用命令

```powershell
# 结束遗留调试进程（避免端口/句柄占用）
taskkill /F /IM sys-sensor.exe /IM sensor-bridge.exe /IM dotnet.exe

# 仅发布桥接（可选，通常由打包流程自动执行）
dotnet publish ./sensor-bridge -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true -o ./src-tauri/resources/sensor-bridge

# 打包安装包（Release 交付）
cargo tauri build

# 检查 Rust 端构建
cargo check

# 前端构建（被 beforeBuildCommand 调用）
npm run build
```

## 文档

- 进度：`doc/progress.md`
- 开发说明：`doc/项目总结与开发注意事项.md`
