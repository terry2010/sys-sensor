<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

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

const snap = ref<SensorSnapshot | null>(null);
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  unlisten = await listen<SensorSnapshot>("sensor://snapshot", (e) => {
    snap.value = e.payload;
  });
});

onBeforeUnmount(() => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
});

function fmtBps(bps: number | undefined) {
  if (bps == null) return "-";
  const kb = bps / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  return `${(kb / 1024).toFixed(1)} MB/s`;
}
</script>

<template>
  <div class="details-wrap">
    <h2>系统详情</h2>
    <div class="grid">
      <div class="item"><span>CPU</span><b>{{ snap ? snap.cpu_usage.toFixed(0) + '%' : '—' }}</b></div>
      <div class="item"><span>内存</span><b>{{ snap ? `${snap.mem_used_gb.toFixed(1)}/${snap.mem_total_gb.toFixed(1)} GB (${snap.mem_pct.toFixed(0)}%)` : '—' }}</b></div>
      <div class="item"><span>CPU温度</span><b>{{ snap?.cpu_temp_c != null ? `${snap.cpu_temp_c.toFixed(1)} °C` : '—' }}</b></div>
      <div class="item"><span>主板温度</span><b>{{ snap?.mobo_temp_c != null ? `${snap.mobo_temp_c.toFixed(1)} °C` : '—' }}</b></div>
      <div class="item"><span>风扇</span><b>{{ snap?.fan_rpm != null ? `${snap.fan_rpm} RPM` : '—' }}</b></div>
      <div class="item"><span>网络下行</span><b>{{ fmtBps(snap?.net_rx_bps) }}</b></div>
      <div class="item"><span>网络上行</span><b>{{ fmtBps(snap?.net_tx_bps) }}</b></div>
      <div class="item"><span>磁盘读</span><b>{{ fmtBps(snap?.disk_r_bps) }}</b></div>
      <div class="item"><span>磁盘写</span><b>{{ fmtBps(snap?.disk_w_bps) }}</b></div>
    </div>
  </div>
</template>

<style scoped>
.details-wrap { padding: 16px; }
.grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.item {
  display: flex;
  justify-content: space-between;
  background: var(--card-bg, rgba(0,0,0,0.04));
  padding: 10px 12px;
  border-radius: 8px;
}
.item span { color: #666; }
.item b { font-weight: 600; }
@media (prefers-color-scheme: dark) {
  .item { background: rgba(255,255,255,0.06); }
  .item span { color: #aaa; }
}
</style>
