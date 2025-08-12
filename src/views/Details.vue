<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

type SensorSnapshot = {
  cpu_usage: number;
  mem_used_gb: number;
  mem_total_gb: number;
  mem_pct: number;
  // 内存细分
  mem_avail_gb?: number;
  swap_used_gb?: number;
  swap_total_gb?: number;
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
  // 网络接口/磁盘容量/SMART 健康
  net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string }[];
  logical_disks?: { drive?: string; size_bytes?: number; free_bytes?: number }[];
  smart_health?: { device?: string; predict_fail?: boolean }[];
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  storage_temps?: { name?: string; temp_c?: number }[];
  gpus?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; fan_rpm?: number; vram_used_mb?: number; power_w?: number }[];
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
  // 电池
  battery_percent?: number;
  battery_status?: string;
  // 公网
  public_ip?: string;
  isp?: string;
  timestamp_ms: number;
};

const snap = ref<SensorSnapshot | null>(null);
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  try {
    unlisten = await listen<SensorSnapshot>("sensor://snapshot", (e) => {
      snap.value = e.payload;
      console.debug('[details] snapshot', e.payload);
    });
  } catch (err) {
    console.warn('[Details] 订阅传感器事件失败：', err);
  }
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

function fmtGpus(list?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; fan_rpm?: number; vram_used_mb?: number; power_w?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(2, list.length); i++) {
    const g = list[i];
    const name = g.name ?? `GPU${i + 1}`;
    const t = g.temp_c != null ? `${g.temp_c.toFixed(1)}°C` : "—";
    const l = g.load_pct != null ? `${g.load_pct.toFixed(0)}%` : "—";
    const f = g.core_mhz != null ? `${g.core_mhz >= 1000 ? (g.core_mhz/1000).toFixed(2) + ' GHz' : g.core_mhz.toFixed(0) + ' MHz'}` : "—";
    const rpm = g.fan_rpm != null ? `${g.fan_rpm} RPM` : "—";
    const vram = g.vram_used_mb != null && isFinite(g.vram_used_mb) ? `${g.vram_used_mb.toFixed(0)} MB` : "—";
    const pw = g.power_w != null && isFinite(g.power_w) ? `${g.power_w.toFixed(1)} W` : "—";
    parts.push(`${name} ${t} ${l} ${f} ${rpm} VRAM ${vram} PWR ${pw}`);
  }
  let s = parts.join(", ");
  if (list.length > 2) s += ` +${list.length - 2}`;
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

function fmtWifiSignal(p?: number) {
  if (p == null || !isFinite(p)) return "—";
  return `${p.toFixed(0)}%`;
}

function fmtWifiLink(mbps?: number) {
  if (mbps == null || !isFinite(mbps)) return "—";
  return `${mbps.toFixed(0)} Mbps`;
}

function fmtWifiMeta(ch?: number, band?: string, radio?: string) {
  const parts: string[] = [];
  if (ch != null && isFinite(ch)) parts.push(`CH ${ch}`);
  if (band && band.length > 0) parts.push(band);
  if (radio && radio.length > 0) parts.push(radio);
  return parts.length ? parts.join(" | ") : "—";
}

function fmtWifiRates(rx?: number, tx?: number) {
  const parts: string[] = [];
  if (rx != null && isFinite(rx)) parts.push(`RX ${rx.toFixed(0)} Mbps`);
  if (tx != null && isFinite(tx)) parts.push(`TX ${tx.toFixed(0)} Mbps`);
  return parts.length ? parts.join(" / ") : "—";
}

function fmtWifiRssi(dbm?: number, estimated?: boolean) {
  if (dbm == null || !isFinite(dbm)) return "—";
  return estimated ? `${dbm} dBm (估算)` : `${dbm} dBm`;
}

function fmtCoreLoads(arr?: (number | null)[]) {
  if (!arr || arr.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(8, arr.length);
  for (let i = 0; i < n; i++) {
    const v = arr[i];
    parts.push(v != null && isFinite(v) ? `${v.toFixed(0)}%` : "—");
  }
  let s = parts.join(", ");
  if (arr.length > n) s += ` +${arr.length - n}`;
  return s;
}

function fmtCoreClocks(arr?: (number | null)[]) {
  if (!arr || arr.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(8, arr.length);
  for (let i = 0; i < n; i++) {
    const v = arr[i];
    if (v == null || !isFinite(v)) { parts.push("—"); continue; }
    parts.push(v >= 1000 ? `${(v/1000).toFixed(2)} GHz` : `${v.toFixed(0)} MHz`);
  }
  let s = parts.join(", ");
  if (arr.length > n) s += ` +${arr.length - n}`;
  return s;
}

function fmtCoreTemps(arr?: (number | null)[]) {
  if (!arr || arr.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(8, arr.length);
  for (let i = 0; i < n; i++) {
    const v = arr[i];
    parts.push(v != null && isFinite(v) ? `${v.toFixed(1)}°C` : "—");
  }
  let s = parts.join(", ");
  if (arr.length > n) s += ` +${arr.length - n}`;
  return s;
}

function fmtBytes(n?: number) {
  if (n == null || !isFinite(n)) return "—";
  const gb = n / (1024*1024*1024);
  if (gb < 10) return `${gb.toFixed(2)} GB`;
  return `${gb.toFixed(1)} GB`;
}

function fmtGb(n?: number) {
  if (n == null || !isFinite(n)) return "—";
  if (n < 10) return `${n.toFixed(2)} GB`;
  return `${n.toFixed(1)} GB`;
}

function fmtSwap(u?: number, t?: number) {
  if (t == null || !isFinite(t) || t <= 0) return "—";
  const used = (u != null && isFinite(u)) ? u : 0;
  const pct = t > 0 ? (used / t * 100) : 0;
  return `${used.toFixed(1)}/${t.toFixed(1)} GB (${pct.toFixed(0)}%)`;
}

function fmtBatPct(p?: number) {
  if (p == null || !isFinite(p)) return "—";
  return `${p.toFixed(0)}%`;
}

function fmtBatStatus(s?: string) {
  return s && s.length > 0 ? s : "—";
}

function fmtNetIfs(list?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(2, list.length); i++) {
    const it = list[i];
    const name = it.name ?? `网卡${i+1}`;
    const ip = (it.ips && it.ips.find(x => x && x.includes('.'))) || (it.ips && it.ips[0]) || "";
    const mac = it.mac ?? "";
    const link = it.link_mbps != null && isFinite(it.link_mbps) ? `${it.link_mbps.toFixed(0)} Mbps` : "";
    const med = it.media_type ?? "";
    const segs = [name, ip, mac, link || med].filter(s => s && s.length > 0);
    parts.push(segs.join(" | "));
  }
  let s = parts.join(", ");
  if (list.length > 2) s += ` +${list.length - 2}`;
  return s || "—";
}

function fmtDisks(list?: { drive?: string; size_bytes?: number; free_bytes?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(3, list.length); i++) {
    const d = list[i];
    const name = d.drive ?? `盘${i+1}`;
    const sz = fmtBytes(d.size_bytes);
    const fr = fmtBytes(d.free_bytes);
    parts.push(`${name} ${sz} / 可用 ${fr}`);
  }
  let s = parts.join(", ");
  if (list.length > 3) s += ` +${list.length - 3}`;
  return s;
}

function fmtSmart(list?: { device?: string; predict_fail?: boolean }[]) {
  if (!list || list.length === 0) return "—";
  const warn = list.filter(x => x.predict_fail === true).length;
  if (warn > 0) return `预警 ${warn}`;
  return `OK (${list.length})`;
}
</script>

<template>
  <div class="details-wrap">
    <h2>系统详情</h2>
    <div class="grid">
      <div class="item"><span>CPU</span><b>{{ snap ? snap.cpu_usage.toFixed(0) + '%' : '—' }}</b></div>
      <div class="item"><span>内存</span><b>{{ snap ? `${snap.mem_used_gb.toFixed(1)}/${snap.mem_total_gb.toFixed(1)} GB (${snap.mem_pct.toFixed(0)}%)` : '—' }}</b></div>
      <div class="item"><span>内存可用</span><b>{{ fmtGb(snap?.mem_avail_gb) }}</b></div>
      <div class="item"><span>交换区</span><b>{{ fmtSwap(snap?.swap_used_gb, snap?.swap_total_gb) }}</b></div>
      <div class="item"><span>CPU温度</span><b>{{ snap?.cpu_temp_c != null ? `${snap.cpu_temp_c.toFixed(1)} °C` : '—' }}</b></div>
      <div class="item"><span>主板温度</span><b>{{ snap?.mobo_temp_c != null ? `${snap.mobo_temp_c.toFixed(1)} °C` : '—' }}</b></div>
      <div class="item"><span>风扇</span><b>{{ snap?.fan_rpm != null ? `${snap.fan_rpm} RPM` : '—' }}</b></div>
      <div class="item"><span>网络下行</span><b>{{ fmtBps(snap?.net_rx_bps) }}</b></div>
      <div class="item"><span>网络上行</span><b>{{ fmtBps(snap?.net_tx_bps) }}</b></div>
      <div class="item"><span>Wi‑Fi SSID</span><b>{{ snap?.wifi_ssid ?? '—' }}</b></div>
      <div class="item"><span>Wi‑Fi信号</span><b>{{ fmtWifiSignal(snap?.wifi_signal_pct) }}</b></div>
      <div class="item"><span>Wi‑Fi链路</span><b>{{ fmtWifiLink(snap?.wifi_link_mbps) }}</b></div>
      <div class="item"><span>Wi‑Fi BSSID</span><b>{{ snap?.wifi_bssid ?? '—' }}</b></div>
      <div class="item"><span>Wi‑Fi参数</span><b>{{ fmtWifiMeta(snap?.wifi_channel, snap?.wifi_band, snap?.wifi_radio) }}</b></div>
      <div class="item"><span>Wi‑Fi速率</span><b>{{ fmtWifiRates(snap?.wifi_rx_mbps, snap?.wifi_tx_mbps) }}</b></div>
      <div class="item"><span>Wi‑Fi RSSI</span><b>{{ fmtWifiRssi(snap?.wifi_rssi_dbm, snap?.wifi_rssi_estimated) }}</b></div>
      <div class="item"><span>网络接口</span><b>{{ fmtNetIfs(snap?.net_ifs) }}</b></div>
      <div class="item"><span>磁盘读</span><b>{{ fmtBps(snap?.disk_r_bps) }}</b></div>
      <div class="item"><span>磁盘写</span><b>{{ fmtBps(snap?.disk_w_bps) }}</b></div>
      <div class="item"><span>磁盘容量</span><b>{{ fmtDisks(snap?.logical_disks) }}</b></div>
      <div class="item"><span>存储温度</span><b>{{ fmtStorage(snap?.storage_temps) }}</b></div>
      <div class="item"><span>SMART健康</span><b>{{ fmtSmart(snap?.smart_health) }}</b></div>
      <div class="item"><span>GPU</span><b>{{ fmtGpus(snap?.gpus) }}</b></div>
      <div class="item"><span>CPU每核负载</span><b>{{ fmtCoreLoads(snap?.cpu_core_loads_pct) }}</b></div>
      <div class="item"><span>CPU每核频率</span><b>{{ fmtCoreClocks(snap?.cpu_core_clocks_mhz) }}</b></div>
      <div class="item"><span>CPU每核温度</span><b>{{ fmtCoreTemps(snap?.cpu_core_temps_c) }}</b></div>
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
      <div class="item"><span>公网IP</span><b>{{ snap?.public_ip ?? '—' }}</b></div>
      <div class="item"><span>运营商</span><b>{{ snap?.isp ?? '—' }}</b></div>
      <div class="item"><span>电池电量</span><b>{{ fmtBatPct(snap?.battery_percent) }}</b></div>
      <div class="item"><span>电池状态</span><b>{{ fmtBatStatus(snap?.battery_status) }}</b></div>
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
