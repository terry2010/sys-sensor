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
  // Wi‑Fi 扩展2
  wifi_auth?: string;
  wifi_cipher?: string;
  wifi_chan_width_mhz?: number;
  // 网络接口/磁盘容量/SMART 健康
  net_ifs?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string; gateway?: string[]; dns?: string[]; dhcp_enabled?: boolean; up?: boolean }[];
  // 兼容两种磁盘容量形态：
  // - 旧版：drive/size_bytes/free_bytes（字节）
  // - 新版（Rust serde camelCase）：name/totalGb/freeGb（GB）
  // - 新版（若曾用 snake_case）：name/total_gb/free_gb（GB）
  logical_disks?: { drive?: string; size_bytes?: number; free_bytes?: number; name?: string; total_gb?: number; free_gb?: number; totalGb?: number; freeGb?: number; fs?: string }[];
  smart_health?: { device?: string; predict_fail?: boolean; temp_c?: number; power_on_hours?: number; reallocated?: number; pending?: number; uncorrectable?: number; crc_err?: number; power_cycles?: number; host_reads_bytes?: number; host_writes_bytes?: number }[];
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  mobo_voltages?: { name?: string; volts?: number }[];
  fans_extra?: { name?: string; rpm?: number; pct?: number }[];
  storage_temps?: { name?: string; temp_c?: number }[];
  gpus?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; memory_mhz?: number; fan_rpm?: number; fan_duty_pct?: number; vram_used_mb?: number; power_w?: number; power_limit_w?: number; voltage_v?: number; hotspot_temp_c?: number; vram_temp_c?: number }[];
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
  battery_ac_online?: boolean;
  battery_time_remaining_sec?: number;
  battery_time_to_full_sec?: number;
  // 公网
  public_ip?: string;
  isp?: string;
  timestamp_ms: number;
};

const snap = ref<SensorSnapshot | null>(null);
let unlisten: UnlistenFn | null = null;
const showIfs = ref(false);
const showFans = ref(false);
const showSmart = ref(false);

onMounted(async () => {
  try {
    unlisten = await listen<SensorSnapshot>("sensor://snapshot", (e) => {
      const curr = e.payload;
      // 轻量平滑：在短时间窗口内，用上一帧的有效值回填 GPU 的 fan_rpm / voltage_v，减少 UI 抖动
      const smoothed = smoothSnapshot(lastSnap, curr);
      snap.value = smoothed;
      lastSnap = smoothed;
      console.debug('[details] snapshot', smoothed);
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

function fmtVoltages(list?: { name?: string; volts?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(4, list.length);
  for (let i = 0; i < n; i++) {
    const v = list[i];
    const label = v.name ?? `V${i + 1}`;
    const val = v.volts != null && isFinite(v.volts) ? (v.volts >= 10 ? v.volts.toFixed(1) : v.volts.toFixed(3)) + " V" : "—";
    parts.push(`${label} ${val}`);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}

function fmtFansExtra(list?: { name?: string; rpm?: number; pct?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(3, list.length);
  for (let i = 0; i < n; i++) {
    const f = list[i];
    const label = f.name ?? `风扇${i + 1}`;
    const rpm = f.rpm != null ? `${f.rpm} RPM` : null;
    const pct = f.pct != null ? `${f.pct}%` : null;
    let seg = label + " ";
    if (rpm && pct) seg += `${rpm} ${pct}`;
    else if (rpm) seg += rpm;
    else if (pct) seg += pct;
    else seg += "—";
    parts.push(seg);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}

function fmtGpus(list?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; memory_mhz?: number; fan_rpm?: number; fan_duty_pct?: number; vram_used_mb?: number; power_w?: number; power_limit_w?: number; voltage_v?: number; hotspot_temp_c?: number; vram_temp_c?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(2, list.length); i++) {
    const g = list[i];
    const name = g.name ?? `GPU${i + 1}`;
    const t = g.temp_c != null ? `${g.temp_c.toFixed(1)}°C` : "—";
    const l = g.load_pct != null ? `${g.load_pct.toFixed(0)}%` : "—";
    const f = g.core_mhz != null ? `${g.core_mhz >= 1000 ? (g.core_mhz/1000).toFixed(2) + ' GHz' : g.core_mhz.toFixed(0) + ' MHz'}` : "—";
    const mem = g.memory_mhz != null ? `${g.memory_mhz >= 1000 ? (g.memory_mhz/1000).toFixed(2) + ' GHz' : g.memory_mhz.toFixed(0) + ' MHz'}` : null;
    const rpm = g.fan_rpm != null ? `${g.fan_rpm} RPM` : "—";
    const vram = g.vram_used_mb != null && isFinite(g.vram_used_mb) ? `${g.vram_used_mb.toFixed(0)} MB` : "—";
    const pw = g.power_w != null && isFinite(g.power_w) ? `${g.power_w.toFixed(1)} W` : "—";
    const pl = g.power_limit_w != null && isFinite(g.power_limit_w) ? `${g.power_limit_w.toFixed(1)} W` : null;
    const voltage = g.voltage_v != null && isFinite(g.voltage_v) ? `${g.voltage_v.toFixed(3)} V` : null;
    const hs = g.hotspot_temp_c != null ? `HS ${g.hotspot_temp_c.toFixed(1)}°C` : null;
    const vramt = g.vram_temp_c != null ? `VRAM ${g.vram_temp_c.toFixed(1)}°C` : null;
    let seg = `${name} ${t} ${l} ${f}`;
    if (mem) seg += ` Mem ${mem}`;
    seg += ` ${rpm}`;
    if (g.fan_duty_pct != null) seg += ` ${g.fan_duty_pct}%`;
    seg += ` VRAM ${vram} PWR ${pw}`;
    if (pl) seg += ` PL ${pl}`;
    if (hs) seg += ` ${hs}`;
    if (vramt) seg += ` ${vramt}`;
    if (voltage) seg += ` ${voltage}`; // 仅在有值时追加电压，且避免重复单位
    parts.push(seg);
  }
  let s = parts.join(", ");
  if (list.length > 2) s += ` +${list.length - 2}`;
  return s;
}

// —— 平滑逻辑 ——
let lastSnap: SensorSnapshot | null = null;
const SMOOTH_TTL_MS = 15000; // 在 15s 内允许用上一帧回填
function smoothSnapshot(prev: SensorSnapshot | null, curr: SensorSnapshot): SensorSnapshot {
  try {
    if (!prev) return curr;
    const tNow = curr.timestamp_ms;
    const tPrev = prev.timestamp_ms;
    if (tNow == null || tPrev == null) return curr;
    if (Math.abs(tNow - tPrev) > SMOOTH_TTL_MS) return curr;
    if (prev.gpus && prev.gpus.length > 0 && curr.gpus && curr.gpus.length > 0) {
      const map = new Map<string, { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; fan_rpm?: number; fan_duty_pct?: number; vram_used_mb?: number; power_w?: number; power_limit_w?: number; voltage_v?: number }>();
      for (const g of prev.gpus) map.set((g.name ?? '').toLowerCase(), g);
      for (const g of curr.gpus) {
        const key = (g.name ?? '').toLowerCase();
        const pg = map.get(key);
        if (!pg) continue;
        if ((g.fan_rpm == null || !isFinite(g.fan_rpm as unknown as number)) && pg.fan_rpm != null) g.fan_rpm = pg.fan_rpm;
        if ((g.fan_duty_pct == null || !isFinite(g.fan_duty_pct as unknown as number)) && pg.fan_duty_pct != null) g.fan_duty_pct = pg.fan_duty_pct;
        if ((g.voltage_v == null || !isFinite(g.voltage_v as unknown as number)) && pg.voltage_v != null) g.voltage_v = pg.voltage_v;
        if ((g.power_limit_w == null || !isFinite(g.power_limit_w as unknown as number)) && pg.power_limit_w != null) g.power_limit_w = pg.power_limit_w;
      }
    }
  } catch { /* ignore */ }
  return curr;
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

function fmtWifiSec(auth?: string, cipher?: string) {
  const parts: string[] = [];
  if (auth && auth.length > 0) parts.push(auth);
  if (cipher && cipher.length > 0) parts.push(cipher);
  return parts.length ? parts.join(" | ") : "—";
}

function fmtWifiWidth(mhz?: number) {
  if (mhz == null || !isFinite(mhz)) return "—";
  return `${mhz.toFixed(0)} MHz`;
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

function fmtBatAC(ac?: boolean) {
  if (ac == null) return "—";
  return ac ? "接通" : "电池";
}

function fmtDuration(sec?: number) {
  if (sec == null || !isFinite(sec)) return "—";
  const s = Math.max(0, Math.floor(sec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  if (h > 0) return `${h}h${m}m`;
  if (m > 0) return `${m}m${r}s`;
  return `${r}s`;
}

function fmtNetIfs(list?: { name?: string; mac?: string; ips?: string[]; link_mbps?: number; media_type?: string; gateway?: string[]; dns?: string[]; dhcp_enabled?: boolean; up?: boolean }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(2, list.length); i++) {
    const it = list[i];
    const name = it.name ?? `网卡${i+1}`;
    const ip = (it.ips && it.ips.find(x => x && x.includes('.'))) || (it.ips && it.ips[0]) || "";
    const mac = it.mac ?? "";
    const link = it.link_mbps != null && isFinite(it.link_mbps) ? `${it.link_mbps.toFixed(0)} Mbps` : "";
    const med = it.media_type ?? "";
    const up = it.up == null ? "" : (it.up ? "UP" : "DOWN");
    const dhcp = it.dhcp_enabled == null ? "" : (it.dhcp_enabled ? "DHCP" : "静态");
    const gw = (it.gateway && it.gateway.length > 0) ? `GW ${it.gateway[0]}` : "";
    const dns = (it.dns && it.dns.length > 0) ? `DNS ${it.dns[0]}` : "";
    const segs = [name, ip, mac, link || med, up, dhcp, gw, dns].filter(s => s && s.length > 0);
    parts.push(segs.join(" | "));
  }
  let s = parts.join(", ");
  if (list.length > 2) s += ` +${list.length - 2}`;
  return s || "—";
}

function toggleIfs() {
  showIfs.value = !showIfs.value;
}

function toggleFans() {
  showFans.value = !showFans.value;
}

function toggleSmart() {
  showSmart.value = !showSmart.value;
}

function fmtDisks(list?: { drive?: string; size_bytes?: number; free_bytes?: number; name?: string; total_gb?: number; free_gb?: number; totalGb?: number; freeGb?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(3, list.length); i++) {
    const d = list[i] as any;
    const name = d.drive ?? d.name ?? `盘${i+1}`;
    // 优先使用字节字段，其次使用 GB 字段
    const szGb = d.total_gb ?? d.totalGb;
    const frGb = d.free_gb ?? d.freeGb;
    const sz = (d.size_bytes != null) ? fmtBytes(d.size_bytes) : fmtGb(szGb);
    const fr = (d.free_bytes != null) ? fmtBytes(d.free_bytes) : fmtGb(frGb);
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

function fmtSmartKeys(list?: { temp_c?: number; power_on_hours?: number; reallocated?: number; pending?: number; uncorrectable?: number; crc_err?: number; power_cycles?: number }[]) {
  if (!list || list.length === 0) return '—';
  let tMin: number | null = null, tMax: number | null = null;
  let poh = 0, ralloc = 0, pend = 0, unc = 0, crc = 0, pwr = 0;
  for (const it of list) {
    if (it.temp_c != null) {
      tMin = tMin == null ? it.temp_c : Math.min(tMin, it.temp_c);
      tMax = tMax == null ? it.temp_c : Math.max(tMax, it.temp_c);
    }
    if (it.power_on_hours != null) poh = Math.max(poh, Math.max(0, Math.floor(it.power_on_hours)));
    if (it.reallocated != null) ralloc += Math.max(0, Math.floor(it.reallocated));
    if (it.pending != null) pend += Math.max(0, Math.floor(it.pending));
    if (it.uncorrectable != null) unc += Math.max(0, Math.floor(it.uncorrectable));
    if (it.crc_err != null) crc += Math.max(0, Math.floor(it.crc_err));
    if (it.power_cycles != null) pwr = Math.max(pwr, Math.max(0, Math.floor(it.power_cycles)));
  }
  const parts: string[] = [];
  if (tMin != null && tMax != null) {
    parts.push(tMin === tMax ? `温度 ${tMin.toFixed(0)}°C` : `温度 ${tMin.toFixed(0)}-${tMax.toFixed(0)}°C`);
  }
  if (poh > 0) parts.push(`通电 ${poh}h`);
  parts.push(`重映射 ${ralloc}`);
  parts.push(`待定 ${pend}`);
  parts.push(`不可恢复 ${unc}`);
  parts.push(`CRC ${crc}`);
  return parts.join(' | ');
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
      <div class="item"><span>主板电压</span><b>{{ fmtVoltages(snap?.mobo_voltages) }}</b></div>
      <div class="item"><span>更多风扇</span><b>
        {{ fmtFansExtra(snap?.fans_extra) }}
        <a v-if="snap?.fans_extra && snap.fans_extra.length" href="#" @click.prevent="toggleFans" class="link">{{ showFans ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>网络下行</span><b>{{ fmtBps(snap?.net_rx_bps) }}</b></div>
      <div class="item"><span>网络上行</span><b>{{ fmtBps(snap?.net_tx_bps) }}</b></div>
      <div class="item"><span>Wi‑Fi SSID</span><b>{{ snap?.wifi_ssid ?? '—' }}</b></div>
      <div class="item"><span>Wi‑Fi信号</span><b>{{ fmtWifiSignal(snap?.wifi_signal_pct) }}</b></div>
      <div class="item"><span>Wi‑Fi链路</span><b>{{ fmtWifiLink(snap?.wifi_link_mbps) }}</b></div>
      <div class="item"><span>Wi‑Fi BSSID</span><b>{{ snap?.wifi_bssid ?? '—' }}</b></div>
      <div class="item"><span>Wi‑Fi参数</span><b>{{ fmtWifiMeta(snap?.wifi_channel, snap?.wifi_band, snap?.wifi_radio) }}</b></div>
      <div class="item"><span>Wi‑Fi速率</span><b>{{ fmtWifiRates(snap?.wifi_rx_mbps, snap?.wifi_tx_mbps) }}</b></div>
      <div class="item"><span>Wi‑Fi RSSI</span><b>{{ fmtWifiRssi(snap?.wifi_rssi_dbm, snap?.wifi_rssi_estimated) }}</b></div>
      <div class="item"><span>Wi‑Fi安全</span><b>{{ fmtWifiSec(snap?.wifi_auth, snap?.wifi_cipher) }}</b></div>
      <div class="item"><span>Wi‑Fi信道宽度</span><b>{{ fmtWifiWidth(snap?.wifi_chan_width_mhz) }}</b></div>
      <div class="item"><span>网络接口</span><b>
        {{ fmtNetIfs(snap?.net_ifs) }}
        <a v-if="snap?.net_ifs && snap.net_ifs.length" href="#" @click.prevent="toggleIfs" class="link">{{ showIfs ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>磁盘读</span><b>{{ fmtBps(snap?.disk_r_bps) }}</b></div>
      <div class="item"><span>磁盘写</span><b>{{ fmtBps(snap?.disk_w_bps) }}</b></div>
      <div class="item"><span>磁盘容量</span><b>{{ fmtDisks(snap?.logical_disks) }}</b></div>
      <div class="item"><span>存储温度</span><b>{{ fmtStorage(snap?.storage_temps) }}</b></div>
      <div class="item"><span>SMART健康</span><b>
        {{ fmtSmart(snap?.smart_health) }}
        <a v-if="snap?.smart_health && snap.smart_health.length" href="#" @click.prevent="toggleSmart" class="link">{{ showSmart ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>SMART关键</span><b>{{ fmtSmartKeys(snap?.smart_health) }}</b></div>
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
      <div class="item"><span>AC电源</span><b>{{ fmtBatAC(snap?.battery_ac_online) }}</b></div>
      <div class="item"><span>剩余时间</span><b>{{ fmtDuration(snap?.battery_time_remaining_sec) }}</b></div>
      <div class="item"><span>充满耗时</span><b>{{ fmtDuration(snap?.battery_time_to_full_sec) }}</b></div>
    </div>
    <div v-if="showFans && snap?.fans_extra && snap.fans_extra.length" class="fans-list">
      <h3>风扇详情</h3>
      <div v-for="(f, idx) in snap.fans_extra" :key="(f.name ?? 'fan') + idx" class="fan-card">
        <div class="row"><span>名称</span><b>{{ f.name ?? `风扇${idx+1}` }}</b></div>
        <div class="row"><span>转速</span><b>{{ f.rpm != null ? `${f.rpm} RPM` : '—' }}</b></div>
        <div class="row"><span>占空比</span><b>{{ f.pct != null ? `${f.pct}%` : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showSmart && snap?.smart_health && snap.smart_health.length" class="smart-list">
      <h3>SMART 详情</h3>
      <div v-for="(d, idx) in snap.smart_health" :key="(d.device ?? 'disk') + idx" class="smart-card">
        <div class="row"><span>设备</span><b>{{ d.device ?? `磁盘${idx+1}` }}</b></div>
        <div class="row"><span>预测失败</span><b>{{ d.predict_fail == null ? '—' : (d.predict_fail ? '是' : '否') }}</b></div>
        <div class="row"><span>温度</span><b>{{ d.temp_c != null ? `${d.temp_c.toFixed(1)} °C` : '—' }}</b></div>
        <div class="row"><span>通电时长</span><b>{{ d.power_on_hours != null ? `${d.power_on_hours} h` : '—' }}</b></div>
        <div class="row"><span>重映射扇区</span><b>{{ d.reallocated ?? '—' }}</b></div>
        <div class="row"><span>待定扇区</span><b>{{ d.pending ?? '—' }}</b></div>
        <div class="row"><span>不可恢复</span><b>{{ d.uncorrectable ?? '—' }}</b></div>
        <div class="row"><span>UDMA CRC</span><b>{{ d.crc_err ?? '—' }}</b></div>
        <div class="row"><span>上电次数</span><b>{{ d.power_cycles ?? '—' }}</b></div>
        <div class="row"><span>累计读取</span><b>{{ d.host_reads_bytes != null ? fmtBytes(d.host_reads_bytes) : '—' }}</b></div>
        <div class="row"><span>累计写入</span><b>{{ d.host_writes_bytes != null ? fmtBytes(d.host_writes_bytes) : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showIfs && snap?.net_ifs && snap.net_ifs.length" class="netifs-list">
      <h3>网络接口详情</h3>
      <div v-for="(it, idx) in snap.net_ifs" :key="(it.name ?? 'if') + idx" class="netif-card">
        <div class="row"><span>名称</span><b>{{ it.name ?? `网卡${idx+1}` }}</b></div>
        <div class="row"><span>状态</span><b>{{ it.up == null ? '—' : (it.up ? 'UP' : 'DOWN') }}</b></div>
        <div class="row"><span>速率/介质</span><b>{{ it.link_mbps != null ? `${it.link_mbps.toFixed(0)} Mbps` : (it.media_type ?? '—') }}</b></div>
        <div class="row"><span>MAC</span><b>{{ it.mac ?? '—' }}</b></div>
        <div class="row"><span>IPv4/IPv6</span><b>{{ (it.ips && it.ips.length) ? it.ips.join(', ') : '—' }}</b></div>
        <div class="row"><span>DHCP</span><b>{{ it.dhcp_enabled == null ? '—' : (it.dhcp_enabled ? 'DHCP' : '静态') }}</b></div>
        <div class="row"><span>网关</span><b>{{ (it.gateway && it.gateway.length) ? it.gateway.join(', ') : '—' }}</b></div>
        <div class="row"><span>DNS</span><b>{{ (it.dns && it.dns.length) ? it.dns.join(', ') : '—' }}</b></div>
      </div>
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
.item .link { margin-left: 8px; font-weight: 500; text-decoration: underline; color: #3a7; }
.fans-list { margin-top: 14px; }
.fans-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.fan-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.fan-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.fan-card .row span { color: #666; }
.fan-card .row b { font-weight: 600; }
.netifs-list { margin-top: 14px; }
.netifs-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.netif-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.netif-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.netif-card .row span { color: #666; }
.netif-card .row b { font-weight: 600; }
.smart-list { margin-top: 14px; }
.smart-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.smart-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.smart-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.smart-card .row span { color: #666; }
.smart-card .row b { font-weight: 600; }
@media (prefers-color-scheme: dark) {
  .item { background: rgba(255,255,255,0.06); }
  .item span { color: #aaa; }
  .fan-card { background: rgba(255,255,255,0.06); }
  .fan-card .row span { color: #aaa; }
  .netif-card { background: rgba(255,255,255,0.06); }
  .netif-card .row span { color: #aaa; }
  .smart-card { background: rgba(255,255,255,0.06); }
  .smart-card .row span { color: #aaa; }
}
</style>
