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
  timestamp_ms: number;
};

listen<SensorSnapshot>("sensor://snapshot", (e) => {
  console.debug("[sensor] snapshot", e.payload);
});

createApp(App).use(router).mount("#app");
