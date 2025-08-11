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
  net_rx_bps: number;
  net_tx_bps: number;
  disk_r_bps: number;
  disk_w_bps: number;
  // Wi‑Fi
  wifi_ssid?: string;
  wifi_signal_pct?: number;
  wifi_link_mbps?: number;
  // 网络接口/磁盘容量/SMART 健康
  net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string }[];
  logical_disks?: { drive?: string; size_bytes?: number; free_bytes?: number }[];
  smart_health?: { device?: string; predict_fail?: boolean }[];
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  // 扩展字段
  storage_temps?: { name?: string; temp_c?: number }[];
  gpus?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; fan_rpm?: number }[];
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
  timestamp_ms: number;
};

// 在非 Tauri 浏览器预览中跳过订阅，避免报错
const isTauri = typeof window !== 'undefined' && (window as any).__TAURI__ != null;
if (!isTauri) {
  console.warn('[main] Tauri API 不可用：运行于普通浏览器预览，禁用全局事件订阅');
} else {
  try {
    listen<SensorSnapshot>("sensor://snapshot", (e) => {
      console.debug("[sensor] snapshot", e.payload);
    });
  } catch (err) {
    console.warn('[main] 订阅传感器事件失败：', err);
  }
}

createApp(App).use(router).mount("#app");
