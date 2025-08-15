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
  // - 新版（Rust serde camelCase）：name/totalGb/freeGb（GB）
  // - 新版（若曾用 snake_case）：name/total_gb/free_gb（GB）
  logical_disks?: { drive?: string; size_bytes?: number; free_bytes?: number; name?: string; total_gb?: number; free_gb?: number; totalGb?: number; freeGb?: number; fs?: string }[];
  smart_health?: { device?: string; predict_fail?: boolean; temp_c?: number; power_on_hours?: number; reallocated?: number; pending?: number; uncorrectable?: number; crc_err?: number; power_cycles?: number; host_reads_bytes?: number; host_writes_bytes?: number; life_percentage_used_pct?: number; nvme_percentage_used_pct?: number; nvme_available_spare_pct?: number; nvme_available_spare_threshold_pct?: number; nvme_media_errors?: number }[];
  cpu_temp_c?: number;
  mobo_temp_c?: number;
  fan_rpm?: number;
  mobo_voltages?: { name?: string; volts?: number }[];
  fans_extra?: { name?: string; rpm?: number; pct?: number }[];
  storage_temps?: { name?: string; tempC?: number }[];
  gpus?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; memory_mhz?: number; fan_rpm?: number; fan_duty_pct?: number; vram_used_mb?: number; vram_total_mb?: number; vram_usage_pct?: number; power_w?: number; power_limit_w?: number; voltage_v?: number; hotspot_temp_c?: number; vram_temp_c?: number }[];
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
  // 多目标 RTT & Top 进程
  rtt_multi?: { target: string; rtt_ms?: number }[];
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

const snap = ref<SensorSnapshot | null>(null);
let unlisten: UnlistenFn | null = null;
const showIfs = ref(false);
const showFans = ref(false);
const showSmart = ref(false);
const showSmartKeysList = ref(false);
const showRtt = ref(false);
const showTopCpu = ref(false);
const showTopMem = ref(false);
const showDisks = ref(false);
const showStorageTemps = ref(false);

onMounted(async () => {
  try {
    unlisten = await listen<SensorSnapshot>("sensor://snapshot", (e) => {
      const curr = e.payload;
      // 轻量平滑：在短时间窗口内，用上一帧的有效值回填 GPU 的 fan_rpm / voltage_v，减少 UI 抖动
      const smoothed = smoothSnapshot(lastSnap, curr);
      snap.value = smoothed;
      lastSnap = smoothed;
      console.debug('[details] snapshot', smoothed);
      // 调试SMART盘符数据
      if (smoothed.storage_temps && smoothed.storage_temps.length > 0) {
        console.log('[SMART_DEBUG] 接收到storage_temps数据:', smoothed.storage_temps);
        smoothed.storage_temps.forEach((item: any, index: number) => {
          console.log(`[SMART_DEBUG] 设备${index}: name=${item.name} tempC=${item.tempC} driveLetter=${item.driveLetter}`);
        });
      } else {
        console.log('[SMART_DEBUG] 未接收到storage_temps数据');
      }
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

function fmtStorage(list?: { name?: string; tempC?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(3, list.length); i++) {
    const st = list[i];
    const label = st.name ?? `驱动${i + 1}`;
    const val = st.tempC != null && isFinite(st.tempC) ? `${st.tempC.toFixed(1)} °C` : "—";
    parts.push(`${label} ${val}`);
  }
  let s = parts.join(", ");
  if (list.length > 3) s += ` +${list.length - 3}`;
  return s;
}

// 获取磁盘设备名（不包含盘符）
function getDiskLabel(d: any, idx: number): string {
  // 只返回设备名，盘符单独显示
  return d.device || `设备${idx+1}`;
}

// 获取磁盘盘符
function getDriveLetter(d: any): string {
  // 优先使用 driveLetter 字段（Rust camelCase 序列化）
  if (d.driveLetter && typeof d.driveLetter === 'string' && d.driveLetter.length > 0) {
    return d.driveLetter;
  }
  // 兼容 drive_letter 字段（snake_case）
  if (d.drive_letter && typeof d.drive_letter === 'string' && d.drive_letter.length > 0) {
    return d.drive_letter;
  }
  // 兼容旧版本的 drive 字段
  if (d.drive && typeof d.drive === 'string' && d.drive.length > 0) {
    return d.drive;
  }
  return '—';
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

function fmtGpus(list?: { name?: string; temp_c?: number; load_pct?: number; core_mhz?: number; memory_mhz?: number; fan_rpm?: number; fan_duty_pct?: number; vram_used_mb?: number; vram_total_mb?: number; vram_usage_pct?: number; power_w?: number; power_limit_w?: number; voltage_v?: number; hotspot_temp_c?: number; vram_temp_c?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(list.length, 2); i++) {
    const g = list[i] as any;
    // 兼容 snake_case 与 camelCase
    const nameVal: string | undefined = g.name;
    const tempC: number | undefined = g.temp_c ?? g.tempC;
    const loadPct: number | undefined = g.load_pct ?? g.loadPct;
    const coreMhz: number | undefined = g.core_mhz ?? g.coreMhz;
    const memoryMhz: number | undefined = g.memory_mhz ?? g.memoryMhz;
    const fanRpm: number | undefined = g.fan_rpm ?? g.fanRpm;
    const fanDutyPct: number | undefined = g.fan_duty_pct ?? g.fanDutyPct;
    const vramUsedMb: number | undefined = g.vram_used_mb ?? g.vramUsedMb;
    const vramTotalMb: number | undefined = g.vram_total_mb ?? g.vramTotalMb;
    const powerW: number | undefined = g.power_w ?? g.powerW;
    const powerLimitW: number | undefined = g.power_limit_w ?? g.powerLimitW;
    const voltageV: number | undefined = g.voltage_v ?? g.voltageV;
    const hotspotTempC: number | undefined = g.hotspot_temp_c ?? g.hotspotTempC;
    const vramTempC: number | undefined = g.vram_temp_c ?? g.vramTempC;

    console.log(`[FRONTEND_GPU_DEBUG] GPU ${i}:`, {
      name: nameVal,
      vramUsedMb,
      vramTotalMb,
      powerW,
    });
    const name = nameVal ?? `GPU${i + 1}`;
    const t = tempC != null && isFinite(tempC) ? `${tempC.toFixed(1)}°C` : "—";
    const l = loadPct != null && isFinite(loadPct) ? `${loadPct.toFixed(0)}%` : "—";
    const f = coreMhz != null && isFinite(coreMhz) ? `${coreMhz >= 1000 ? (coreMhz/1000).toFixed(2) + ' GHz' : coreMhz.toFixed(0) + ' MHz'}` : "—";
    const mem = memoryMhz != null && isFinite(memoryMhz) ? `${memoryMhz >= 1000 ? (memoryMhz/1000).toFixed(2) + ' GHz' : memoryMhz.toFixed(0) + ' MHz'}` : null;
    const rpm = fanRpm != null && isFinite(fanRpm) ? `${fanRpm} RPM` : "—";
    const vram = vramUsedMb != null && isFinite(vramUsedMb) && vramTotalMb != null && isFinite(vramTotalMb)
      ? `${(vramUsedMb/1024).toFixed(1)}/${(vramTotalMb/1024).toFixed(1)}GB`
      : (vramUsedMb != null && isFinite(vramUsedMb) ? `${vramUsedMb.toFixed(0)} MB` : "—");
    const pw = powerW != null && isFinite(powerW) ? `${powerW.toFixed(1)} W` : "—";
    const pl = powerLimitW != null && isFinite(powerLimitW) ? `${powerLimitW.toFixed(1)} W` : null;
    const voltage = voltageV != null && isFinite(voltageV) ? `${voltageV.toFixed(3)} V` : null;
    const hs = hotspotTempC != null && isFinite(hotspotTempC) ? `HS ${hotspotTempC.toFixed(1)}°C` : null;
    const vramt = vramTempC != null && isFinite(vramTempC) ? `VRAM ${vramTempC.toFixed(1)}°C` : null;
    let seg = `${name} ${t} ${l} ${f}`;
    if (mem) seg += ` Mem ${mem}`;
    seg += ` ${rpm}`;
    if (fanDutyPct != null && isFinite(fanDutyPct)) seg += ` ${fanDutyPct}%`;
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
      // 建立 name 映射（兼容 camelCase/snake_case）
      const map = new Map<string, any>();
      for (const g of prev.gpus as any[]) {
        const key = (g.name ?? '').toLowerCase();
        map.set(key, g);
      }
      for (const g of curr.gpus as any[]) {
        const key = (g.name ?? '').toLowerCase();
        const pg = map.get(key);
        if (!pg) continue;
        const curFanRpm = g.fan_rpm ?? g.fanRpm;
        const prevFanRpm = pg.fan_rpm ?? pg.fanRpm;
        if ((curFanRpm == null || !isFinite(curFanRpm)) && prevFanRpm != null && isFinite(prevFanRpm)) {
          g.fan_rpm = prevFanRpm; g.fanRpm = prevFanRpm;
        }
        const curFanPct = g.fan_duty_pct ?? g.fanDutyPct;
        const prevFanPct = pg.fan_duty_pct ?? pg.fanDutyPct;
        if ((curFanPct == null || !isFinite(curFanPct)) && prevFanPct != null && isFinite(prevFanPct)) {
          g.fan_duty_pct = prevFanPct; g.fanDutyPct = prevFanPct;
        }
        const curVolt = g.voltage_v ?? g.voltageV;
        const prevVolt = pg.voltage_v ?? pg.voltageV;
        if ((curVolt == null || !isFinite(curVolt)) && prevVolt != null && isFinite(prevVolt)) {
          g.voltage_v = prevVolt; g.voltageV = prevVolt;
        }
        const curPl = g.power_limit_w ?? g.powerLimitW;
        const prevPl = pg.power_limit_w ?? pg.powerLimitW;
        if ((curPl == null || !isFinite(curPl)) && prevPl != null && isFinite(prevPl)) {
          g.power_limit_w = prevPl; g.powerLimitW = prevPl;
        }
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

function fmtNetIfs(list?: { name?: string; mac?: string; ips?: string[]; ipv4?: string; ipv6?: string; link_mbps?: number; linkMbps?: number; speed_mbps?: number; speedMbps?: number; media_type?: string; mediaType?: string; media?: string; gateway?: string[]; dns?: string[]; dhcp_enabled?: boolean; dhcpEnabled?: boolean; up?: boolean }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  for (let i = 0; i < Math.min(2, list.length); i++) {
    const it = list[i];
    const name = it.name ?? `网卡${i+1}`;
    // IP 优先取 ips 数组，其次回退独立的 ipv4/ipv6
    const ipFromList = (it.ips && it.ips.find(x => x && x.includes('.'))) || (it.ips && it.ips[0]) || "";
    const ip = ipFromList || it.ipv4 || it.ipv6 || "";
    const mac = it.mac ?? "";
    const linkNum = (it.link_mbps ?? it.linkMbps ?? it.speed_mbps ?? it.speedMbps) as number | undefined;
    const link = linkNum != null && isFinite(linkNum) ? `${Number(linkNum).toFixed(0)} Mbps` : "";
    const med = it.media_type ?? (it as any).mediaType ?? it.media ?? "";
    const up = it.up == null ? "" : (it.up ? "UP" : "DOWN");
    const dhcpVal = (it as any).dhcp_enabled ?? (it as any).dhcpEnabled;
    const dhcp = dhcpVal == null ? "" : (dhcpVal ? "DHCP" : "静态");
    const gwArr: string[] | undefined = (it as any).gateway;
    const dnsArr: string[] | undefined = (it as any).dns;
    const gw = (gwArr && gwArr.length > 0) ? `GW ${gwArr[0]}` : "";
    const dns = (dnsArr && dnsArr.length > 0) ? `DNS ${dnsArr[0]}` : "";
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

function toggleSmartKeys() {
  showSmartKeysList.value = !showSmartKeysList.value;
}
function toggleDisks() {
  showDisks.value = !showDisks.value;
}
function toggleStorageTemps() {
  showStorageTemps.value = !showStorageTemps.value;
}

function toggleRtt() {
  showRtt.value = !showRtt.value;
}

function toggleTopCpu() {
  showTopCpu.value = !showTopCpu.value;
}

function toggleTopMem() {
  showTopMem.value = !showTopMem.value;
}

function fmtDisks(list?: { drive?: string; size_bytes?: number; free_bytes?: number; name?: string; total_gb?: number; free_gb?: number; totalGb?: number; freeGb?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(3, list.length);
  for (let i = 0; i < n; i++) {
    const d = list[i] as any;
    const label = d.name ?? d.drive ?? `盘${i+1}`;
    const totalGb = (d.total_gb ?? d.totalGb) as number | undefined;
    const freeGb = (d.free_gb ?? d.freeGb) as number | undefined;
    let val = "—";
    if (totalGb != null && freeGb != null && isFinite(totalGb) && isFinite(freeGb)) {
      val = `${(totalGb - freeGb).toFixed(1)}/${totalGb.toFixed(1)} GB`;
    } else if (d.size_bytes != null && d.free_bytes != null && isFinite(d.size_bytes) && isFinite(d.free_bytes)) {
      const used = (d.size_bytes - d.free_bytes) / 1073741824;
      const tot = d.size_bytes / 1073741824;
      val = `${used.toFixed(1)}/${tot.toFixed(1)} GB`;
    }
    parts.push(`${label} ${val}`);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}

function fmtSmart(list?: { device?: string; predict_fail?: boolean }[]) {
  if (!list || list.length === 0) return "—";
  // 兼容 snake_case 与 camelCase（predict_fail / predictFail）
  const warn = list.filter((x: any) => (x.predict_fail ?? x.predictFail) === true).length;
  if (warn > 0) return `预警 ${warn}`;
  return `OK (${list.length})`;
}

function fmtSmartKeys(list?: { temp_c?: number; power_on_hours?: number; reallocated?: number; pending?: number; uncorrectable?: number; crc_err?: number; power_cycles?: number; life_percentage_used_pct?: number; nvme_percentage_used_pct?: number; nvme_available_spare_pct?: number; nvme_available_spare_threshold_pct?: number; nvme_media_errors?: number }[]) {
  if (!list || list.length === 0) return '—';
  let tMin: number | null = null, tMax: number | null = null;
  let poh = 0, ralloc = 0, pend = 0, unc = 0, crc = 0, pwr = 0;
  // NVMe 汇总：最大已用寿命、最小可用备用、介质错误合计
  let nvmeUsedMax: number | null = null;
  let nvmeSpareMin: number | null = null;
  let nvmeMediaErr = 0;
  for (const it0 of list as any[]) {
    const it: any = it0 || {};
    const temp = it.temp_c ?? it.tempC;
    if (temp != null && isFinite(temp)) {
      tMin = tMin == null ? temp : Math.min(tMin, temp);
      tMax = tMax == null ? temp : Math.max(tMax, temp);
    }
    const poh1 = it.power_on_hours ?? it.powerOnHours;
    if (poh1 != null && isFinite(poh1)) poh = Math.max(poh, Math.max(0, Math.floor(poh1)));
    const r1 = it.reallocated;
    if (r1 != null && isFinite(r1)) ralloc += Math.max(0, Math.floor(r1));
    const p1 = it.pending;
    if (p1 != null && isFinite(p1)) pend += Math.max(0, Math.floor(p1));
    const u1 = it.uncorrectable;
    if (u1 != null && isFinite(u1)) unc += Math.max(0, Math.floor(u1));
    const c1 = it.crc_err ?? it.crcErr;
    if (c1 != null && isFinite(c1)) crc += Math.max(0, Math.floor(c1));
    const pc1 = it.power_cycles ?? it.powerCycles;
    if (pc1 != null && isFinite(pc1)) pwr = Math.max(pwr, Math.max(0, Math.floor(pc1)));

    // —— 已用寿命（统一：SATA+NVMe）——
    const usedPct = it.life_percentage_used_pct ?? it.lifePercentageUsedPct ?? it.nvme_percentage_used_pct ?? it.nvmePercentageUsedPct;
    if (usedPct != null && isFinite(usedPct)) nvmeUsedMax = nvmeUsedMax == null ? usedPct : Math.max(nvmeUsedMax, usedPct);
    const sparePct = it.nvme_available_spare_pct ?? it.nvmeAvailableSparePct;
    if (sparePct != null && isFinite(sparePct)) nvmeSpareMin = nvmeSpareMin == null ? sparePct : Math.min(nvmeSpareMin, sparePct);
    const mediaErr = it.nvme_media_errors ?? it.nvmeMediaErrors;
    if (mediaErr != null && isFinite(mediaErr)) nvmeMediaErr += Math.max(0, Math.floor(mediaErr));
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
  if (nvmeUsedMax != null) parts.push(`已用 ${nvmeUsedMax.toFixed(0)}%`);
  if (nvmeSpareMin != null) parts.push(`备用 ${nvmeSpareMin.toFixed(0)}%`);
  // 与 SATA 指标保持一致风格，显示总数
  if (nvmeUsedMax != null || nvmeSpareMin != null || nvmeMediaErr > 0) {
    parts.push(`介质 ${nvmeMediaErr}`);
  }
  return parts.join(' | ');
}

// 统一获取 SMART 列表（兼容 smart_health 与 smartHealth）
function getSmartList(snap: any): any[] | undefined {
  if (!snap) return undefined;
  return (snap.smart_health ?? snap.smartHealth) as any[] | undefined;
}

function fmtRttMulti(list?: { target: string; rtt_ms?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(3, list.length);
  for (let i = 0; i < n; i++) {
    const it = list[i];
    const t = it.target || `t${i+1}`;
    const v = it.rtt_ms != null && isFinite(it.rtt_ms) ? `${it.rtt_ms.toFixed(1)} ms` : "—";
    parts.push(`${t} ${v}`);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}

function fmtMBFromBytes(n?: number) {
  if (n == null || !isFinite(n)) return "—";
  const mb = n / (1024*1024);
  return `${Math.max(0, mb).toFixed(0)} MB`;
}

function fmtTopCpuProcs(list?: { name?: string; cpu_pct?: number; mem_bytes?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(3, list.length);
  for (let i = 0; i < n; i++) {
    const p = list[i];
    const name = p.name ?? `P${i+1}`;
    const cpu = p.cpu_pct != null && isFinite(p.cpu_pct) ? `${p.cpu_pct.toFixed(0)}%` : "—";
    const mem = p.mem_bytes != null ? fmtMBFromBytes(p.mem_bytes) : "—";
    parts.push(`${name} ${cpu} ${mem}`);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}

function fmtTopMemProcs(list?: { name?: string; cpu_pct?: number; mem_bytes?: number }[]) {
  if (!list || list.length === 0) return "—";
  const parts: string[] = [];
  const n = Math.min(3, list.length);
  for (let i = 0; i < n; i++) {
    const p = list[i];
    const name = p.name ?? `P${i+1}`;
    const mem = p.mem_bytes != null ? fmtMBFromBytes(p.mem_bytes) : "—";
    const cpu = p.cpu_pct != null && isFinite(p.cpu_pct) ? `${p.cpu_pct.toFixed(0)}%` : "—";
    parts.push(`${name} ${mem} ${cpu}`);
  }
  let s = parts.join(", ");
  if (list.length > n) s += ` +${list.length - n}`;
  return s;
}



function fmtBatteryHealth(designCap?: number, fullCap?: number, cycleCount?: number) {
  const parts: string[] = [];
  if (designCap != null) parts.push(`设计容量 ${designCap}mWh`);
  if (fullCap != null) parts.push(`满充容量 ${fullCap}mWh`);
  if (cycleCount != null) parts.push(`循环次数 ${cycleCount}`);
  if (designCap != null && fullCap != null) {
    const health = ((fullCap / designCap) * 100).toFixed(1);
    parts.push(`健康度 ${health}%`);
  }
  return parts.length > 0 ? parts.join(" | ") : "—";
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
      <div class="item"><span>内存缓存</span><b>{{ fmtGb(snap?.mem_cache_gb) }}</b></div>
      <div class="item"><span>内存提交</span><b>{{ snap?.mem_committed_gb != null && snap?.mem_commit_limit_gb != null ? `${snap.mem_committed_gb.toFixed(1)}/${snap.mem_commit_limit_gb.toFixed(1)} GB` : fmtGb(snap?.mem_committed_gb) }}</b></div>
      <div class="item"><span>分页池</span><b>{{ fmtGb(snap?.mem_pool_paged_gb) }}</b></div>
      <div class="item"><span>非分页池</span><b>{{ fmtGb(snap?.mem_pool_nonpaged_gb) }}</b></div>
      <div class="item"><span>分页速率</span><b>{{ snap?.mem_pages_per_sec != null ? `${snap.mem_pages_per_sec.toFixed(1)}/s` : '—' }}</b></div>
      <div class="item"><span>页面读取</span><b>{{ snap?.mem_page_reads_per_sec != null ? `${snap.mem_page_reads_per_sec.toFixed(1)}/s` : '—' }}</b></div>
      <div class="item"><span>页面写入</span><b>{{ snap?.mem_page_writes_per_sec != null ? `${snap.mem_page_writes_per_sec.toFixed(1)}/s` : '—' }}</b></div>
      <div class="item"><span>页面错误</span><b>{{ snap?.mem_page_faults_per_sec != null ? `${snap.mem_page_faults_per_sec.toFixed(1)}/s` : '—' }}</b></div>
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
      <div class="item"><span>磁盘容量</span><b>
        {{ fmtDisks(snap?.logical_disks) }}
        <a v-if="snap?.logical_disks && snap.logical_disks.length" href="#" @click.prevent="toggleDisks" class="link">{{ showDisks ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>存储温度</span><b>
        {{ fmtStorage(snap?.storage_temps) }}
        <a v-if="snap?.storage_temps && snap.storage_temps.length" href="#" @click.prevent="toggleStorageTemps" class="link">{{ showStorageTemps ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>SMART健康</span><b>
        {{ fmtSmart(getSmartList(snap)) }}
        <a v-if="getSmartList(snap) && getSmartList(snap)!.length" href="#" @click.prevent="toggleSmart" class="link">{{ showSmart ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>SMART关键</span><b>{{ fmtSmartKeys(getSmartList(snap)) }}
        <a v-if="getSmartList(snap) && getSmartList(snap)!.length" href="#" @click.prevent="toggleSmartKeys" class="link">{{ showSmartKeysList ? '收起' : '展开' }}</a>
      </b></div>
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
      <div class="item"><span>多目标延迟</span><b>
        {{ fmtRttMulti(snap?.rtt_multi) }}
        <a v-if="snap?.rtt_multi && snap.rtt_multi.length" href="#" @click.prevent="toggleRtt" class="link">{{ showRtt ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>高CPU进程</span><b>
        {{ fmtTopCpuProcs(snap?.top_cpu_procs) }}
        <a v-if="snap?.top_cpu_procs && snap.top_cpu_procs.length" href="#" @click.prevent="toggleTopCpu" class="link">{{ showTopCpu ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>高内存进程</span><b>
        {{ fmtTopMemProcs(snap?.top_mem_procs) }}
        <a v-if="snap?.top_mem_procs && snap.top_mem_procs.length" href="#" @click.prevent="toggleTopMem" class="link">{{ showTopMem ? '收起' : '展开' }}</a>
      </b></div>
      <div class="item"><span>公网IP</span><b>{{ snap?.public_ip ?? '—' }}</b></div>
      <div class="item"><span>运营商</span><b>{{ snap?.isp ?? '—' }}</b></div>
      <div class="item"><span>电池电量</span><b>{{ fmtBatPct(snap?.battery_percent) }}</b></div>
      <div class="item"><span>电池状态</span><b>{{ fmtBatStatus(snap?.battery_status) }}</b></div>
      <div class="item"><span>电池健康</span><b>{{ fmtBatteryHealth(snap?.battery_design_capacity, snap?.battery_full_charge_capacity, snap?.battery_cycle_count) }}</b></div>
      <div class="item"><span>AC电源</span><b>{{ fmtBatAC(snap?.battery_ac_online) }}</b></div>
      <div class="item"><span>剩余时间</span><b>{{ fmtDuration(snap?.battery_time_remaining_sec) }}</b></div>
      <div class="item"><span>充满时间</span><b>{{ fmtDuration(snap?.battery_time_to_full_sec) }}</b></div>
      <div class="item"><span>GPU汇总</span><b>{{ fmtGpus(snap?.gpus) }}</b></div>
    </div>
    <div v-if="showFans && snap?.fans_extra && snap.fans_extra.length" class="fans-list">
      <h3>风扇详情</h3>
      <div v-for="(f, idx) in snap.fans_extra" :key="(f.name ?? 'fan') + idx" class="fan-card">
        <div class="row"><span>名称</span><b>{{ f.name ?? `风扇${idx+1}` }}</b></div>
        <div class="row"><span>转速</span><b>{{ f.rpm != null ? `${f.rpm} RPM` : '—' }}</b></div>
        <div class="row"><span>占空比</span><b>{{ f.pct != null ? `${f.pct}%` : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showRtt && snap?.rtt_multi && snap.rtt_multi.length" class="rtt-list">
      <h3>多目标延迟详情</h3>
      <div v-for="(it, idx) in snap.rtt_multi" :key="(it.target ?? 't') + idx" class="rtt-card">
        <div class="row"><span>目标</span><b>{{ it.target ?? `t${idx+1}` }}</b></div>
        <div class="row"><span>RTT</span><b>{{ fmtRtt(it.rtt_ms) }}</b></div>
      </div>
    </div>

    <div v-if="showTopCpu && snap?.top_cpu_procs && snap.top_cpu_procs.length" class="procs-list">
      <h3>高CPU进程详情</h3>
      <div v-for="(p, idx) in snap.top_cpu_procs" :key="(p.name ?? 'cpu') + idx" class="proc-card">
        <div class="row"><span>进程</span><b>{{ p.name ?? `P${idx+1}` }}</b></div>
        <div class="row"><span>CPU</span><b>{{ p.cpu_pct != null && isFinite(p.cpu_pct) ? `${p.cpu_pct.toFixed(0)}%` : '—' }}</b></div>
        <div class="row"><span>内存</span><b>{{ p.mem_bytes != null ? fmtMBFromBytes(p.mem_bytes) : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showTopMem && snap?.top_mem_procs && snap.top_mem_procs.length" class="procs-list">
      <h3>高内存进程详情</h3>
      <div v-for="(p, idx) in snap.top_mem_procs" :key="(p.name ?? 'mem') + idx" class="proc-card">
        <div class="row"><span>进程</span><b>{{ p.name ?? `P${idx+1}` }}</b></div>
        <div class="row"><span>内存</span><b>{{ p.mem_bytes != null ? fmtMBFromBytes(p.mem_bytes) : '—' }}</b></div>
        <div class="row"><span>CPU</span><b>{{ p.cpu_pct != null && isFinite(p.cpu_pct) ? `${p.cpu_pct.toFixed(0)}%` : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showDisks && snap?.logical_disks && snap.logical_disks.length" class="disks-list">
      <h3>磁盘容量详情</h3>
      <div v-for="(d, idx) in snap.logical_disks" :key="(d.name ?? d.drive ?? 'disk') + idx" class="disk-card">
        <div class="row"><span>卷</span><b>{{ d.name ?? d.drive ?? `盘${idx+1}` }}</b></div>
        <div class="row"><span>文件系统</span><b>{{ d.fs ?? '—' }}</b></div>
        <div class="row"><span>总容量</span><b>{{ (d.total_gb ?? d.totalGb) != null ? `${(d.total_gb ?? d.totalGb)!.toFixed(1)} GB` : (d.size_bytes != null ? `${(d.size_bytes/1073741824).toFixed(1)} GB` : '—') }}</b></div>
        <div class="row"><span>可用</span><b>{{ (d.free_gb ?? d.freeGb) != null ? `${(d.free_gb ?? d.freeGb)!.toFixed(1)} GB` : (d.free_bytes != null ? `${(d.free_bytes/1073741824).toFixed(1)} GB` : '—') }}</b></div>
      </div>
    </div>

    <div v-if="showStorageTemps && snap?.storage_temps && snap.storage_temps.length" class="storaget-list">
      <h3>存储温度详情</h3>
      <div v-for="(st, idx) in snap.storage_temps" :key="(st.name ?? 'st') + idx" class="st-card">
        <div class="row"><span>设备</span><b>{{ st.name ?? `驱动${idx+1}` }}</b></div>
        <div class="row"><span>温度</span><b>{{ st.tempC != null ? `${st.tempC.toFixed(1)} °C` : '—' }}</b></div>
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

    <div v-if="showStorageTemps && snap?.storage_temps && snap.storage_temps.length" class="storage-temps-list">
      <h3>存储温度详情</h3>
      <div v-for="(st, idx) in snap.storage_temps" :key="(st.name ?? 'storage') + idx" class="storage-temp-card">
        <div class="row"><span>设备</span><b>{{ st.name ?? `存储${idx+1}` }}</b></div>
        <div class="row"><span>温度</span><b>{{ st.tempC != null && isFinite(st.tempC) ? `${st.tempC.toFixed(1)} °C` : '—' }}</b></div>
      </div>
    </div>

    <div v-if="showSmartKeysList && getSmartList(snap) && getSmartList(snap)!.length" class="smart-keys-list">
      <h3>SMART 关键指标详情</h3>
      <div v-for="(d0, idx) in getSmartList(snap)" :key="((d0 as any).device ?? 'disk') + idx" class="smart-key-card">
        <div class="row"><span>设备</span><b>{{ getDiskLabel(d0 as any, idx) }}</b></div>
        <div class="row"><span>指标</span><b>{{ fmtSmartKeys([d0 as any]) }}</b></div>
      </div>
    </div>

    <div v-if="showSmart && getSmartList(snap) && getSmartList(snap)!.length" class="smart-list">
      <h3>SMART 详情</h3>
      <div v-for="(d0, idx) in getSmartList(snap)" :key="((d0 as any).device ?? 'disk') + idx" class="smart-card">
        <template v-for="d in [d0 as any]">
          <div class="row"><span>设备</span><b>{{ getDiskLabel(d, idx) }}</b></div>
          <div class="row"><span>盘符</span><b>{{ getDriveLetter(d) }}</b></div>
          <div class="row"><span>预测失败</span><b>{{ (d.predict_fail ?? d.predictFail) == null ? '—' : ((d.predict_fail ?? d.predictFail) ? '是' : '否') }}</b></div>
          <div class="row"><span>温度</span><b>{{ (d.temp_c ?? d.tempC) != null ? `${(d.temp_c ?? d.tempC).toFixed(1)} °C` : '—' }}</b></div>
          <div class="row"><span>通电时长</span><b>{{ (d.power_on_hours ?? d.powerOnHours) != null ? `${(d.power_on_hours ?? d.powerOnHours)} h` : '—' }}</b></div>
          <div class="row"><span>重映射扇区</span><b>{{ d.reallocated ?? '—' }}</b></div>
          <div class="row"><span>待定扇区</span><b>{{ d.pending ?? '—' }}</b></div>
          <div class="row"><span>不可恢复</span><b>{{ d.uncorrectable ?? '—' }}</b></div>
          <div class="row"><span>UDMA CRC</span><b>{{ (d.crc_err ?? d.crcErr) ?? '—' }}</b></div>
          <div class="row"><span>上电次数</span><b>{{ (d.power_cycles ?? d.powerCycles) ?? '—' }}</b></div>
          <div class="row"><span>累计读取</span><b>{{ (d.host_reads_bytes ?? d.hostReadsBytes) != null ? fmtBytes((d.host_reads_bytes ?? d.hostReadsBytes)) : '—' }}</b></div>
          <div class="row"><span>累计写入</span><b>{{ (d.host_writes_bytes ?? d.hostWritesBytes) != null ? fmtBytes((d.host_writes_bytes ?? d.hostWritesBytes)) : '—' }}</b></div>
          <div class="row"><span>已用寿命</span><b>{{ (d.life_percentage_used_pct ?? (d as any).lifePercentageUsedPct ?? d.nvme_percentage_used_pct ?? (d as any).nvmePercentageUsedPct) != null && isFinite(d.life_percentage_used_pct ?? (d as any).lifePercentageUsedPct ?? d.nvme_percentage_used_pct ?? (d as any).nvmePercentageUsedPct) ? `${(d.life_percentage_used_pct ?? (d as any).lifePercentageUsedPct ?? d.nvme_percentage_used_pct ?? (d as any).nvmePercentageUsedPct).toFixed(0)}%` : '—' }}</b></div>
          <div class="row"><span>可用备用</span><b>{{ (d.nvme_available_spare_pct ?? (d as any).nvmeAvailableSparePct) != null && isFinite(d.nvme_available_spare_pct ?? (d as any).nvmeAvailableSparePct) ? `${(d.nvme_available_spare_pct ?? (d as any).nvmeAvailableSparePct).toFixed(0)}%` : '—' }}</b></div>
          <div class="row"><span>备用阈值</span><b>{{ (d.nvme_available_spare_threshold_pct ?? (d as any).nvmeAvailableSpareThresholdPct) != null && isFinite(d.nvme_available_spare_threshold_pct ?? (d as any).nvmeAvailableSpareThresholdPct) ? `${(d.nvme_available_spare_threshold_pct ?? (d as any).nvmeAvailableSpareThresholdPct).toFixed(0)}%` : '—' }}</b></div>
          <div class="row"><span>介质错误</span><b>{{ (d.nvme_media_errors ?? (d as any).nvmeMediaErrors) != null && isFinite(d.nvme_media_errors ?? (d as any).nvmeMediaErrors) ? `${(d.nvme_media_errors ?? (d as any).nvmeMediaErrors).toFixed(0)}` : '—' }}</b></div>
        </template>
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
.storage-temps-list { margin-top: 14px; }
.storage-temps-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.storage-temp-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.storage-temp-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.storage-temp-card .row span { color: #666; }
.storage-temp-card .row b { font-weight: 600; }
.smart-list { margin-top: 14px; }
.smart-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.smart-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.smart-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.smart-card .row span { color: #666; }
.smart-card .row b { font-weight: 600; }
.smart-keys-list { margin-top: 14px; }
.smart-keys-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.smart-key-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.smart-key-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.smart-key-card .row span { color: #666; }
.smart-key-card .row b { font-weight: 600; }
.rtt-list, .procs-list { margin-top: 14px; }
.rtt-list h3, .procs-list h3 { margin: 6px 0 10px; font-size: 14px; color: #666; }
.rtt-card, .proc-card { padding: 10px 12px; border-radius: 8px; background: var(--card-bg, rgba(0,0,0,0.04)); margin-bottom: 8px; }
.rtt-card .row, .proc-card .row { display: flex; justify-content: space-between; padding: 4px 0; }
.rtt-card .row span, .proc-card .row span { color: #666; }
.rtt-card .row b, .proc-card .row b { font-weight: 600; }
@media (prefers-color-scheme: dark) {
  .item { background: rgba(255,255,255,0.06); }
  .item span { color: #aaa; }
  .fan-card { background: rgba(255,255,255,0.06); }
  .fan-card .row span { color: #aaa; }
  .netif-card { background: rgba(255,255,255,0.06); }
  .netif-card .row span { color: #aaa; }
  .storage-temp-card { background: rgba(255,255,255,0.06); }
  .storage-temp-card .row span { color: #aaa; }
  .smart-card { background: rgba(255,255,255,0.06); }
  .smart-card .row span { color: #aaa; }
  .smart-key-card { background: rgba(255,255,255,0.06); }
  .smart-key-card .row span { color: #aaa; }
  .rtt-card { background: rgba(255,255,255,0.06); }
  .rtt-card .row span { color: #aaa; }
  .proc-card { background: rgba(255,255,255,0.06); }
  .proc-card .row span { color: #aaa; }
}
</style>
