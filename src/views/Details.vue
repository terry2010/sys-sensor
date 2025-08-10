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
  storage_temps?: { name?: string; temp_c?: number }[];
  hb_tick?: number;
  idle_sec?: number;
  exc_count?: number;
  uptime_sec?: number;
  // 第二梯队
  cpu_pkg_power_w?: number;
  cpu_avg_freq_mhz?: number;
  cpu_throttle_active?: boolean;
  cpu_throttle_reasons?: string[];
  since_reopen_sec?: number;
  // 第二梯队：磁盘/网络/延迟
  disk_r_iops?: number;
  disk_w_iops?: number;
  disk_queue_len?: number;
  net_rx_err_ps?: number;
  net_tx_err_ps?: number;
  ping_rtt_ms?: number;
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

function fmtUptime(sec?: number) {
  if (sec == null) return undefined;
  const s = Math.max(0, Math.floor(sec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  if (h > 0) return `${h}h${m}m`;
  if (m > 0) return `${m}m${r}s`;
  return `${r}s`;
}

function fmtStorage(list?: { name?: string; temp_c?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(3, list.length); i++) {
    const st = list[i];
    const label = st.name ?? `驱动${i + 1}`;
    const val = st.temp_c != null ? `${st.temp_c.toFixed(1)} °C` : "—";
    parts.push(`${label} ${val}`);
  }
  let s = parts.join(", ");
  if (list.length > 3) s += ` +${list.length - 3}`;
  return s;
}

function fmtBridge(s: SensorSnapshot | null) {
  if (!s) return "—";
  const parts: string[] = [];
  if (s.hb_tick != null) parts.push(`hb ${s.hb_tick}`);
  if (s.idle_sec != null) parts.push(`idle ${s.idle_sec}s`);
  if (s.exc_count != null) parts.push(`exc ${s.exc_count}`);
  const up = fmtUptime(s.uptime_sec);
  if (up) parts.push(`up ${up}`);
  if (s.since_reopen_sec != null) parts.push(`reopen ${s.since_reopen_sec}s`);
  return parts.length ? parts.join(" ") : "—";
}

function fmtPowerW(w?: number) {
  if (w == null) return "—";
  if (!isFinite(w)) return "—";
  return `${w.toFixed(1)} W`;
}

function fmtFreq(mhz?: number) {
  if (mhz == null) return "—";
  if (!isFinite(mhz)) return "—";
  if (mhz >= 1000) return `${(mhz / 1000).toFixed(2)} GHz`;
  return `${mhz.toFixed(0)} MHz`;
}

function fmtThrottle(s: SensorSnapshot | null) {
  if (!s) return "—";
  if (s.cpu_throttle_active == null && (!s.cpu_throttle_reasons || s.cpu_throttle_reasons.length === 0)) return "—";
  const on = s.cpu_throttle_active === true;
  const reasons = (s.cpu_throttle_reasons && s.cpu_throttle_reasons.length > 0) ? ` (${s.cpu_throttle_reasons.join(", ")})` : "";
  return on ? `是${reasons}` : (s.cpu_throttle_active === false ? "否" : `—${reasons}`);
}

function fmtIOPS(v?: number) {
  if (v == null || !isFinite(v)) return "—";
  return `${v.toFixed(0)} IOPS`;
}

function fmtQueue(v?: number) {
  if (v == null || !isFinite(v)) return "—";
  if (v < 10) return v.toFixed(2);
  return v.toFixed(1);
}

function fmtPktErr(v?: number) {
  if (v == null || !isFinite(v)) return "—";
  return `${v.toFixed(0)}/s`;
}

function fmtRtt(ms?: number) {
  if (ms == null || !isFinite(ms)) return "—";
  if (ms < 1) return `${(ms*1000).toFixed(0)} µs`;
  if (ms < 100) return `${ms.toFixed(1)} ms`;
  return `${ms.toFixed(0)} ms`;
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
      <div class="item"><span>存储温度</span><b>{{ fmtStorage(snap?.storage_temps) }}</b></div>
      <div class="item"><span>桥接健康</span><b>{{ fmtBridge(snap) }}</b></div>
      <div class="item"><span>CPU包功耗</span><b>{{ fmtPowerW(snap?.cpu_pkg_power_w) }}</b></div>
      <div class="item"><span>CPU平均频率</span><b>{{ fmtFreq(snap?.cpu_avg_freq_mhz) }}</b></div>
      <div class="item"><span>CPU限频</span><b>{{ fmtThrottle(snap) }}</b></div>
      <div class="item"><span>磁盘读IOPS</span><b>{{ fmtIOPS(snap?.disk_r_iops) }}</b></div>
      <div class="item"><span>磁盘写IOPS</span><b>{{ fmtIOPS(snap?.disk_w_iops) }}</b></div>
      <div class="item"><span>磁盘队列</span><b>{{ fmtQueue(snap?.disk_queue_len) }}</b></div>
      <div class="item"><span>网络错误(RX)</span><b>{{ fmtPktErr(snap?.net_rx_err_ps) }}</b></div>
      <div class="item"><span>网络错误(TX)</span><b>{{ fmtPktErr(snap?.net_tx_err_ps) }}</b></div>
      <div class="item"><span>网络延迟</span><b>{{ fmtRtt(snap?.ping_rtt_ms) }}</b></div>
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
