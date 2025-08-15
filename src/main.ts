import { createApp } from "vue";
import App from "./App.vue";
import { listen } from "@tauri-apps/api/event";
import { router } from "./router";

// 订阅后端广播的实时数据快照
type SensorSnapshot = {
  cpu_usage: number;
  mem_used_gb: number;
  mem_total_gb: number;
  mem_pct: number;
  // 内存细分（可用/交换）
  mem_avail_gb?: number;
  swap_used_gb?: number;
  swap_total_gb?: number;
  // 内存细分扩展（缓存/提交/分页池/分页速率）
  mem_cache_gb?: number;
  mem_committed_gb?: number;
  mem_commit_limit_gb?: number;
  mem_pool_paged_gb?: number;
  mem_pool_nonpaged_gb?: number;
  mem_pages_per_sec?: number;
  mem_page_reads_per_sec?: number;
  mem_page_writes_per_sec?: number;
  mem_page_faults_per_sec?: number;
  net_rx_bps: number;
  net_tx_bps: number;
  disk_r_bps: number;
  disk_w_bps: number;
  // Wi‑Fi
  wifi_ssid?: string;
  wifi_signal_pct?: number;
  wifi_link_mbps?: number;
  // Wi‑Fi 扩展
  wifi_bssid?: string;
  wifi_channel?: number;
  wifi_radio?: string;
  wifi_band?: string;
  wifi_rx_mbps?: number;
  wifi_tx_mbps?: number;
  wifi_rssi_dbm?: number;
  wifi_rssi_estimated?: boolean;
  // Wi‑Fi 扩展2
  wifi_auth?: string;
  wifi_cipher?: string;
  wifi_chan_width_mhz?: number;
  // 网络接口/磁盘容量/SMART 健康
  net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string; gateway?: string[]; dns?: string[]; dhcp_enabled?: boolean; up?: boolean; packet_loss_pct?: number; active_connections?: number }[];
  // 兼容两种磁盘容量形态：
  // - 旧版：drive/size_bytes/free_bytes（字节）
  // - 新版：name/total_gb/free_gb（GB）
  logical_disks?: { drive?: string; size_bytes?: number; free_bytes?: number; name?: string; total_gb?: number; free_gb?: number; fs?: string }[];
  smart_health?: { device?: string; predict_fail?: boolean; temp_c?: number; power_on_hours?: number; reallocated?: number; pending?: number; uncorrectable?: number; crc_err?: number; power_cycles?: number; host_reads_bytes?: number; host_writes_bytes?: number; life_percentage_used_pct?: number; nvme_percentage_used_pct?: number; nvme_available_spare_pct?: number; nvme_available_spare_threshold_pct?: number; nvme_media_errors?: number }[];
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  // 扩展字段
  mobo_voltages?: { name?: string; volts?: number }[];
  fans_extra?: { name?: string; rpm?: number; pct?: number }[];
  storage_temps?: { name?: string; tempC?: number }[];
  gpus?: {
    name?: string;
    temp_c?: number;
    load_pct?: number;
    core_mhz?: number;
    memory_mhz?: number;
    fan_rpm?: number;
    fan_duty_pct?: number;
    vram_used_mb?: number;
    vram_total_mb?: number;
    vram_usage_pct?: number;
    power_w?: number;
    power_limit_w?: number;
    voltage_v?: number;
    hotspot_temp_c?: number;
    vram_temp_c?: number;
    // GPU深度监控新增字段 - 兼容snake_case和camelCase命名
    encode_util_pct?: number;    // 编码单元使用率
    decode_util_pct?: number;    // 解码单元使用率
    vram_bandwidth_pct?: number; // 显存带宽使用率
    p_state?: string;            // P-State功耗状态
    // camelCase命名风格字段
    encodeUtilPct?: number;      // 编码单元使用率
    decodeUtilPct?: number;      // 解码单元使用率
    vramBandwidthPct?: number;   // 显存带宽使用率
    pState?: string;             // P-State功耗状态
  }[];
  hb_tick?: number;
  idle_sec?: number;
  exc_count?: number;
  uptime_sec?: number;
  // 第二梯队 CPU 指标
  cpu_pkg_power_w?: number;
  cpu_avg_freq_mhz?: number;
  cpu_throttle_active?: boolean;
  cpu_throttle_reasons?: string[];
  since_reopen_sec?: number;
  // 每核心：负载/频率/温度
  cpu_core_loads_pct?: (number | null)[];
  cpu_core_clocks_mhz?: (number | null)[];
  cpu_core_temps_c?: (number | null)[];
  // 第二梯队：磁盘/网络/延迟
  disk_r_iops?: number;
  disk_w_iops?: number;
  disk_queue_len?: number;
  net_rx_err_ps?: number;
  net_tx_err_ps?: number;
  ping_rtt_ms?: number;
  // 多目标 RTT（可选）
  rtt_multi?: { target: string; rtt_ms?: number }[];
  // Top 进程（可选）
  top_cpu_procs?: { name?: string; cpu_pct?: number; mem_bytes?: number }[];
  top_mem_procs?: { name?: string; cpu_pct?: number; mem_bytes?: number }[];
  // 电池
  battery_percent?: number;
  battery_status?: string;
  battery_design_capacity?: number;
  battery_full_charge_capacity?: number;
  battery_cycle_count?: number;
  battery_ac_online?: boolean;
  battery_time_remaining_sec?: number;
  battery_time_to_full_sec?: number;
  // 公网
  public_ip?: string;
  isp?: string;
  timestamp_ms: number;
};

// 创建应用实例并挂载
console.log("🚀 [main] sys-sensor 前端启动中...");
console.log("🚀 [main] 当前时间:", new Date().toLocaleString());
console.log("🚀 [main] Tauri环境检测:", typeof window !== 'undefined' && (window as any).__TAURI__ != null ? "✅ Tauri环境" : "❌ 浏览器环境");

createApp(App).use(router).mount("#app");
console.log("🚀 [main] Vue应用已挂载");

// 在应用挂载后设置事件监听器，确保路由器已初始化
// 强制设置事件监听器，无论是否在Tauri环境
console.log("🔧 [main] 开始强制设置事件监听器...");

// 延迟设置事件监听器，确保路由器完全初始化
setTimeout(() => {
  console.log("🔧 [main] 延迟执行事件监听器设置，路由器状态:", router ? "✅ 已初始化" : "❌ 未初始化");
  
  // 检查Tauri环境 - 使用更宽松的检测条件
  const isTauri = typeof window !== 'undefined' && 
                  (typeof (window as any).__TAURI__ !== 'undefined' || 
                   typeof (window as any).isTauri !== 'undefined' ||
                   window.location.protocol === 'tauri:');
  console.log("🔧 [main] Tauri环境检测:", isTauri ? "✅ Tauri环境" : "❌ 浏览器环境");
  console.log("🔧 [main] 调试信息 - __TAURI__:", typeof (window as any).__TAURI__);
  console.log("🔧 [main] 调试信息 - protocol:", window.location.protocol);
  
  // 强制启用事件监听器进行测试
  console.log("🔧 [main] 🚨 强制启用事件监听器进行调试...");

  try {
    // 传感器数据监听
    listen<SensorSnapshot>("sensor://snapshot", (e) => {
      console.debug("📊 [sensor] snapshot", e.payload);
    });
    console.log("🔧 [main] ✅ 传感器事件监听器已设置");
    
    // 托盘菜单事件监听 - 快速设置
    listen("navigate-to-settings", () => {
      console.log("🎯 [tray] ✅ 接收到 navigate-to-settings 事件！");
      console.log("🎯 [tray] 路由器状态:", router ? "可用" : "不可用");
      console.log("🎯 [tray] 当前路由:", router?.currentRoute?.value?.path || "未知");
      
      try {
        if (router) {
          router.push('/settings');
          console.log("🎯 [tray] ✅ 成功导航到设置页面");
        } else {
          console.error("🎯 [tray] ❌ 路由器未初始化");
          alert("路由器未初始化，无法导航到设置页面");
        }
      } catch (error) {
        console.error("🎯 [tray] ❌ 导航到设置页面失败:", error);
        alert("导航失败: " + error);
      }
    });
    console.log("🔧 [main] ✅ 快速设置事件监听器已设置");
    
    // 托盘菜单事件监听 - 关于我们
    listen("show-about", () => {
      console.log("🎯 [tray] ✅ 接收到 show-about 事件！");
      console.log("🎯 [tray] 路由器状态:", router ? "可用" : "不可用");
      console.log("🎯 [tray] 当前路由:", router?.currentRoute?.value?.path || "未知");
      
      try {
        if (router) {
          router.push('/about');
          console.log("🎯 [tray] ✅ 成功导航到关于页面");
        } else {
          console.log("🎯 [tray] ⚠️ 路由器不可用，使用alert后备方案");
          alert("sys-sensor 系统监控工具\n版本: 1.0.0\n基于 Tauri + Vue 3 开发");
        }
      } catch (error) {
        console.error("🎯 [tray] ❌ 导航到关于页面失败:", error);
        alert("导航失败: " + error);
      }
    });
    console.log("🔧 [main] ✅ 关于我们事件监听器已设置");
    
    // 托盘菜单事件监听 - 显示详情（导航到主页）
    listen("navigate-to-home", () => {
      console.log("🎯 [tray] ✅ 接收到 navigate-to-home 事件！");
      console.log("🎯 [tray] 路由器状态:", router ? "可用" : "不可用");
      console.log("🎯 [tray] 当前路由:", router?.currentRoute?.value?.path || "未知");
      
      try {
        if (router) {
          router.push('/');
          console.log("🎯 [tray] ✅ 成功导航到主页");
        } else {
          console.error("🎯 [tray] ❌ 路由器未初始化");
          alert("路由器未初始化，无法导航到主页");
        }
      } catch (error) {
        console.error("🎯 [tray] ❌ 导航到主页失败:", error);
        alert("导航失败: " + error);
      }
    });
    console.log("🔧 [main] ✅ 显示详情事件监听器已设置");
    
    console.log("🔧 [main] 🎉 所有事件监听器设置完成！");
  } catch (error) {
    console.error("🔧 [main] ❌ 设置事件监听器失败:", error);
  }
}, 100);
