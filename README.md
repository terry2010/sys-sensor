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
cargo tauri build
```

输出：`src-tauri/target/release/bundle/` 下生成安装包（MSI/EXE，具体以 Tauri 配置为准）。

打包细节：

- `src-tauri/tauri.conf.json` 中：
  - `beforeBuildCommand` 会执行：
    `dotnet publish ./sensor-bridge -c Release -r win-x64 -p:PublishSingleFile=true -p:SelfContained=true -o ./src-tauri/resources/sensor-bridge && npm run build`
  - `bundle.resources` 包含：`resources/sensor-bridge/**`，确保桥接随包分发。
- 运行时后端优先从 `BaseDirectory::Resource/sensor-bridge/sensor-bridge.exe` 启动桥接；失败时回退到开发路径。

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

# 检查 Rust 端构建
cargo check

# 前端构建（被 beforeBuildCommand 调用）
npm run build
```

## 文档

- 进度：`doc/progress.md`
- 开发说明：`doc/项目总结与开发注意事项.md`
