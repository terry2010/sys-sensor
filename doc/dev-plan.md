# 开发计划（Windows 系统托盘硬件监控）

## 1. 项目目标
- 基于 Tauri（Rust 后端 + Vue/TS 前端）实现 Windows 托盘常驻监控：
  - 托盘文本显示两项关键指标（默认 CPU 温度 + CPU 占用），1s 刷新。
  - Tooltip/右键菜单上半区展示详细信息。
  - 详情窗口、快速设置、关于我们；退出彻底关闭。
  - 新增：当前网速（上/下/合计）与磁盘读写速率（读/写/合计）。

## 2. 技术选型与依赖
- 后端（Rust / Tauri v2）：
  - tauri, tauri-build
  - sysinfo：系统信息（CPU、内存、进程、网络/磁盘累计字节）
  - serde/serde_json：数据序列化
  - tokio：异步与定时任务
  - image + ab_glyph：文本绘制到 32x32 RGBA 生成托盘图标
- 传感器桥（.NET 8 Console）：
  - LibreHardwareMonitorLib：主板温度、风扇转速
  - 输出 JSON（stdout）；由 Rust 子进程管理与解析
- 前端（Vue + TypeScript + Vite）：
  - 详情页、快速设置、关于页面
  - 与后端通过 Tauri IPC（commands/events）通讯
- 配置：JSON（AppData 路径），持久化托盘显示项、网/盘来源策略等

## 3. 项目结构（规划）
```
C:/code/sys-sensor/
├─ src/                        # Vue 前端
├─ src-tauri/                  # Rust 后端（Tauri）
│  ├─ src/
│  │  ├─ main.rs               # 入口、托盘/菜单/事件、采集调度
│  │  ├─ sensor.rs             # 聚合器：sysinfo + 桥接数据
│  │  ├─ tray.rs               # 托盘文本图标渲染与更新
│  │  ├─ menu.rs               # 右键菜单构建与交互
│  │  ├─ ipc.rs                # 前后端 IPC 命令与事件
│  │  ├─ config.rs             # 配置读写（JSON）
│  │  └─ utils.rs              # 公共工具（EMA、单位换算等）
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ sensor-bridge/              # .NET 8 传感器桥（C#）
│  ├─ Program.cs
│  └─ sensor-bridge.csproj
├─ doc/
│  ├─ product.md               # 产品文档
│  └─ dev-plan.md              # 本开发计划
└─ README.md
```

## 4. 数据模型与刷新逻辑
- 刷新周期：1s（可调）。
- Rust 维护 `SensorSnapshot`：
```jsonc
{
  "timestamp": 0,
  "cpu": {"temp_c": 65.2, "usage_pct": 41.3},
  "memory": {"used_gb": 12.3, "total_gb": 31.8, "usage_pct": 38.7},
  "mobo": {"temp_c": 45.1},
  "fans": [{"name": "CPU Fan", "rpm": 1250}, {"name": "Case Fan1", "rpm": 800}],
  "net": {"rx_bps": 12_300_000, "tx_bps": 2_100_000, "total_bps": 14_400_000},
  "disk": {"read_bps": 8_500_000, "write_bps": 3_200_000, "total_bps": 11_700_000},
  "top_process": {"by_cpu": [...], "by_mem": [...]}
}
```
- 网速/磁盘速率：使用 sysinfo 累计字节做差分 / Δt（秒） = Bps；
  - 平滑：EMA（α=0.3，配置可调）。
  - 单位自适应：< 1024 KB/s 显示 KB/s，否则 MB/s；保留 1 位小数。
- 多设备：
  - 网络：过滤虚拟/非活动接口，默认聚合活动物理接口；支持指定接口。
  - 磁盘：默认系统盘或最繁忙盘；支持聚合或指定盘。

## 5. 托盘与菜单
- 托盘文本：将如 "65℃ 42%" 绘制到 32x32 RGBA，设置为托盘图标。
- Tooltip：按 `SensorSnapshot` 人性化格式化，1s 同步更新。
- 右键菜单：上半区信息（不可点）；下半区操作项：详情 / 快速设置 / 关于我们 / 退出。

## 6. 窗口与前端
- 详情窗口：
  - 分区展示 CPU/内存/主板/风扇、网络速率、磁盘读写、Top 进程（CPU/内存）。
- 快速设置：
  - 托盘两项候选（CPU温度/占用/风扇/内存/主板温度/网速/磁盘吞吐）。
  - 网络/磁盘来源策略选择（自动/指定/聚合）。
  - EMA 平滑参数可选（0.2~0.5）。
- 关于我们：官网、作者微博链接。

## 7. 关键实现点
- 传感器桥子进程：
  - Rust 启动 .NET EXE，固定 1Hz 输出 JSON 行；Rust 异步读取 stdout，解析失败自动重启。
- sysinfo 差分：
  - 保存上次累计计数与时间戳，计算 Δbytes/Δt；首个样本不显示速率。
- DPI 与字体渲染：
  - 根据系统缩放调整字体大小；深浅色主题下确保对比度。
- 配置与持久化：
  - 首启生成默认配置；修改后写回，使用热更新或下次启动生效。
- 错误与恢复：
  - 传感器读取失败/权限不足：UI 显示“不可用”，日志记录并重试。

## 8. 任务拆解（里程碑）
- M1 脚手架与环境（已完成）
  - Tauri (Vue+TS) 项目初始化、依赖确认、开发环境验证。
- M2 后端采集聚合
  - sysinfo 采样、差分速率、EMA；数据模型与事件广播；配置读写。
- M3 托盘/Tooltip/右键菜单
  - 文本图标渲染、1s 刷新；菜单信息区与功能项；退出逻辑。
- M4 传感器桥（.NET）
  - LibreHardwareMonitor 读取主板/风扇；与 Rust 管道通信与合并。
- M5 前端窗口
  - 详情/快速设置/关于；与后端 IPC；设置持久化。
- M6 网速/磁盘细节
  - 多网卡/多磁盘选择与聚合；单位切换与显示策略。
- M7 测试与打包
  - 手测 + 自动化基础；签名/打包；运行验证。

## 9. 测试计划
- 单元测试：单位换算、EMA、差分计算、配置序列化。
- 集成测试：传感器桥通信、采样循环稳定性、菜单/窗口交互。
- 手工测试：
  - 不同网卡/磁盘数量与类型；
  - 断网/高负载/磁盘高 IO；
  - DPI 缩放 100%/125%/150%。

## 10. 开发与运行命令
```powershell
# 安装依赖（已完成）
npm install

# 开发模式
npm run tauri dev

# 构建
npm run build
npx --yes @tauri-apps/cli@latest build
```

## 11. 风险与对策
- 传感器读取权限/兼容性：提供管理员运行提示与白名单说明。
- 多设备策略复杂：默认聚合 + 设置可配，降低边缘情况失败率。
- 字体渲染清晰度：按 DPI 调整字号与描边/阴影提高可读性。

## 12. 交付物
- 可运行安装包、源代码、README 与使用说明。
- `doc/product.md` 与 `doc/dev-plan.md` 同步维护。
