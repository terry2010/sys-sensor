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
  net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string; gateway?: string[]; dns?: string[]; dhcp_enabled?: boolean; up?: boolean }[];
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
  storage_temps?: { name?: string; temp_c?: number }[];
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
