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
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  // 扩展字段
  storage_temps?: { name?: string; temp_c?: number }[];
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
  timestamp_ms: number;
};

listen<SensorSnapshot>("sensor://snapshot", (e) => {
  console.debug("[sensor] snapshot", e.payload);
});

createApp(App).use(router).mount("#app");
