<template>
  <div class="debug-page">
    <h1>调试面板</h1>

    <section>
      <h2>当前配置</h2>
      <div class="actions">
        <button @click="refreshConfig" :disabled="loading">刷新</button>
      </div>
      <pre class="config-view">{{ pretty(cfg) }}</pre>
    </section>

    <section>
      <h2>热更新配置（增量补丁）</h2>
      <div class="grid">
        <label>
          interval_ms
          <input type="number" min="100" step="50" v-model.number="form.interval_ms" placeholder="如 800" />
        </label>
        <label>
          pace_rtt_multi_every
          <input type="number" min="1" step="1" v-model.number="form.pace_rtt_multi_every" placeholder="如 4" />
        </label>
        <label>
          pace_net_if_every
          <input type="number" min="1" step="1" v-model.number="form.pace_net_if_every" placeholder="如 5" />
        </label>
        <label>
          pace_logical_disk_every
          <input type="number" min="1" step="1" v-model.number="form.pace_logical_disk_every" placeholder="如 5" />
        </label>
        <label>
          pace_smart_every
          <input type="number" min="1" step="1" v-model.number="form.pace_smart_every" placeholder="如 10" />
        </label>
        <label>
          rtt_timeout_ms
          <input type="number" min="100" step="50" v-model.number="form.rtt_timeout_ms" placeholder="如 400" />
        </label>
      </div>
      <div class="actions">
        <button @click="applyPatch" :disabled="loading">应用补丁</button>
        <button @click="preset('low')" :disabled="loading">预设：低频(1500/6/6/15)</button>
        <button @click="preset('normal')" :disabled="loading">预设：常规(1000/3/5/10)</button>
        <button @click="preset('high')" :disabled="loading">预设：高频(500/2/3/6)</button>
      </div>
      <div v-if="message" class="message">{{ message }}</div>
    </section>

    <section>
      <h2>调度器状态</h2>
      <div class="actions">
        <button @click="onRefreshAll" :disabled="loadingSched">刷新</button>
        <button v-if="!autoSched" @click="startAutoSched">开始自动刷新(2s)</button>
        <button v-else @click="stopAutoSched">停止自动刷新</button>
      </div>
      <div class="tick-kpis" v-if="sched">
        <span class="kpi">tick: {{ sched?.tick ?? '—' }}</span>
        <span class="kpi">tick_cost: {{ fmtCost(sched?.tick_cost_ms) }}</span>
        <span class="badge" :class="sched?.frame_skipped ? 'warn' : 'on'">{{ sched?.frame_skipped ? '跳帧' : '对齐' }}</span>
      </div>
      <div class="task-controls" v-if="sched">
        <h3>任务控制</h3>
        <div class="task-row">
          <div class="task-name">RTT</div>
          <span class="badge" :class="sched?.rtt_is_running ? 'on' : 'off'">{{ sched?.rtt_is_running ? '运行中' : '空闲' }}</span>
          <span class="meta">last_ok: {{ fmtTs(sched?.rtt_last_ok_ms) }} (age {{ fmtAge(sched?.rtt_age_ms) }})</span>
          <label class="switch">
            <input type="checkbox" :checked="!!sched?.rtt_enabled" @change="onToggle('rtt', ($event.target as HTMLInputElement).checked)" />
            <span>启用</span>
          </label>
          <button @click="onTrigger('rtt')">一次性触发</button>
          <div class="every">
            <span>every(ticks): {{ sched?.rtt_every }}</span>
            <input type="number" min="1" step="1" v-model.number="everyForm.rtt" />
            <button @click="onSetEvery('rtt', everyForm.rtt)">保存</button>
          </div>
        </div>
        <div class="task-row">
          <div class="task-name">NetIf</div>
          <span class="badge" :class="sched?.netif_is_running ? 'on' : 'off'">{{ sched?.netif_is_running ? '运行中' : '空闲' }}</span>
          <span class="meta">last_ok: {{ fmtTs(sched?.netif_last_ok_ms) }} (age {{ fmtAge(sched?.netif_age_ms) }})</span>
          <label class="switch">
            <input type="checkbox" :checked="!!sched?.netif_enabled" @change="onToggle('netif', ($event.target as HTMLInputElement).checked)" />
            <span>启用</span>
          </label>
          <button @click="onTrigger('netif')">一次性触发</button>
          <div class="every">
            <span>every(ticks): {{ sched?.netif_every }}</span>
            <input type="number" min="1" step="1" v-model.number="everyForm.netif" />
            <button @click="onSetEvery('netif', everyForm.netif)">保存</button>
          </div>
        </div>
        <div class="task-row">
          <div class="task-name">LogicalDisk</div>
          <span class="badge" :class="sched?.ldisk_is_running ? 'on' : 'off'">{{ sched?.ldisk_is_running ? '运行中' : '空闲' }}</span>
          <span class="meta">last_ok: {{ fmtTs(sched?.ldisk_last_ok_ms) }} (age {{ fmtAge(sched?.ldisk_age_ms) }})</span>
          <label class="switch">
            <input type="checkbox" :checked="!!sched?.ldisk_enabled" @change="onToggle('ldisk', ($event.target as HTMLInputElement).checked)" />
            <span>启用</span>
          </label>
          <button @click="onTrigger('ldisk')">一次性触发</button>
          <div class="every">
            <span>every(ticks): {{ sched?.ldisk_every }}</span>
            <input type="number" min="1" step="1" v-model.number="everyForm.ldisk" />
            <button @click="onSetEvery('ldisk', everyForm.ldisk)">保存</button>
          </div>
        </div>
        <div class="task-row">
          <div class="task-name">SMART</div>
          <span class="badge" :class="sched?.smart_is_running ? 'on' : 'off'">{{ sched?.smart_is_running ? '运行中' : '空闲' }}</span>
          <span class="meta">last_ok: {{ fmtTs(sched?.smart_last_ok_ms) }} (age {{ fmtAge(sched?.smart_age_ms) }})</span>
          <label class="switch">
            <input type="checkbox" :checked="!!sched?.smart_enabled" @change="onToggle('smart', ($event.target as HTMLInputElement).checked)" />
            <span>启用</span>
          </label>
          <button @click="onTrigger('smart')">一次性触发</button>
          <div class="every">
            <span>every(ticks): {{ sched?.smart_every }}</span>
            <input type="number" min="1" step="1" v-model.number="everyForm.smart" />
            <button @click="onSetEvery('smart', everyForm.smart)">保存</button>
          </div>
        </div>
      </div>
      <pre class="config-view">{{ pretty(sched) }}</pre>

      <h3>StateStore TickTelemetry</h3>
      <div class="tick-kpis" v-if="stateTick">
        <span class="kpi">tick: {{ stateTick?.tick ?? '—' }}</span>
        <span class="kpi">tick_cost: {{ fmtCost(stateTick?.tick_cost_ms) }}</span>
        <span class="badge" :class="stateTick?.frame_skipped ? 'warn' : 'on'">{{ stateTick?.frame_skipped ? '跳帧' : '对齐' }}</span>
      </div>
      <pre class="config-view">{{ pretty(stateTick) }}</pre>

      <h3>StateStore Aggregated</h3>
      <div class="tick-kpis" v-if="stateAgg">
        <span class="kpi">cpu: {{ stateAgg?.cpu_usage?.toFixed?.(1) ?? '—' }}%</span>
        <span class="kpi">mem: {{ stateAgg?.mem_pct?.toFixed?.(1) ?? '—' }}%</span>
        <span class="kpi">net: ↓{{ fmtBps(stateAgg?.net_rx_bps) }} ↑{{ fmtBps(stateAgg?.net_tx_bps) }}</span>
        <span class="kpi">disk: R {{ fmtBps(stateAgg?.disk_r_bps) }} W {{ fmtBps(stateAgg?.disk_w_bps) }}</span>
        <span class="kpi">ping: {{ stateAgg?.ping_rtt_ms ?? '—' }}ms</span>
        <span class="kpi">battery: {{ stateAgg?.battery_percent ?? '—' }}%</span>
      </div>
      <pre class="config-view">{{ pretty(stateAgg) }}</pre>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onBeforeUnmount, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

type Cfg = Record<string, any>

const cfg = ref<Cfg | null>(null)
const loading = ref(false)
const message = ref('')
const sched = ref<Record<string, any> | null>(null)
const stateTick = ref<Record<string, any> | null>(null)
const stateAgg = ref<Record<string, any> | null>(null)
const loadingSched = ref(false)
const autoSched = ref(false)
let schedTimer: number | null = null
let unlistenAgg: null | (() => void) = null

const everyForm = ref<{ rtt?: number, netif?: number, ldisk?: number, smart?: number }>({})

const form = ref<{ [k: string]: number | undefined }>(
  {
    interval_ms: undefined,
    pace_rtt_multi_every: undefined,
    pace_net_if_every: undefined,
    pace_logical_disk_every: undefined,
    pace_smart_every: undefined,
    rtt_timeout_ms: undefined,
  }
)

function pretty(v: any) {
  try { return JSON.stringify(v, null, 2) } catch { return String(v) }
}

function fmtTs(ms?: number | null): string {
  if (!ms && ms !== 0) return '—'
  try {
    const d = new Date(ms)
    const hh = String(d.getHours()).padStart(2, '0')
    const mm = String(d.getMinutes()).padStart(2, '0')
    const ss = String(d.getSeconds()).padStart(2, '0')
    return `${hh}:${mm}:${ss}`
  } catch { return String(ms) }
}

function fmtAge(ms?: number | null): string {
  if (ms == null) return '—'
  const v = Math.max(0, ms)
  if (v < 1000) return `${v}ms`
  const s = Math.floor(v / 1000)
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  const s2 = s % 60
  return `${m}m${s2}s`
}

function fmtCost(ms?: number | null): string {
  if (ms == null) return '—'
  const v = Math.max(0, ms)
  return `${v}ms`
}

function fmtBps(v?: number | null): string {
  if (v == null) return '—'
  const bps = Math.max(0, v)
  const kb = bps / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`
  const mb = kb / 1024
  return `${mb.toFixed(1)} MB/s`
}

async function refreshConfig() {
  loading.value = true
  message.value = ''
  try {
    cfg.value = await invoke<Cfg>('get_config')
  } catch (e: any) {
    message.value = `读取配置失败: ${e?.message || e}`
  } finally {
    loading.value = false
  }
}

function buildPatch() {
  const p: Record<string, any> = {}
  for (const k of Object.keys(form.value)) {
    const val = form.value[k]
    if (val !== undefined && val !== null && !Number.isNaN(val)) p[k] = val
  }
  return p
}

async function applyPatch() {
  const patch = buildPatch()
  if (Object.keys(patch).length === 0) {
    message.value = '未填写任何字段'
    return
  }
  loading.value = true
  message.value = ''
  try {
    const merged = await invoke<Cfg>('cmd_cfg_update', { patch })
    cfg.value = merged
    message.value = '已应用并持久化'
  } catch (e: any) {
    message.value = `应用失败: ${e?.message || e}`
  } finally {
    loading.value = false
  }
}

function preset(kind: 'low'|'normal'|'high') {
  if (kind === 'low') {
    form.value.interval_ms = 1500
    form.value.pace_rtt_multi_every = 6
    form.value.pace_net_if_every = 6
    form.value.pace_logical_disk_every = 6
    form.value.pace_smart_every = 15
  } else if (kind === 'normal') {
    form.value.interval_ms = 1000
    form.value.pace_rtt_multi_every = 3
    form.value.pace_net_if_every = 5
    form.value.pace_logical_disk_every = 5
    form.value.pace_smart_every = 10
  } else {
    form.value.interval_ms = 500
    form.value.pace_rtt_multi_every = 2
    form.value.pace_net_if_every = 3
    form.value.pace_logical_disk_every = 3
    form.value.pace_smart_every = 6
  }
}

// init
refreshConfig()
onRefreshAll()

// 订阅后端按 tick 广播的聚合事件（sensor://agg）
onMounted(async () => {
  try {
    const un = await listen<Record<string, any>>('sensor://agg', (event) => {
      stateAgg.value = event.payload
    })
    unlistenAgg = un
  } catch (e) {
    // 忽略监听失败
  }
})

async function refreshScheduler() {
  loadingSched.value = true
  try {
    sched.value = await invoke<Record<string, any>>('get_scheduler_state')
  } catch (e) {
    // 忽略错误，仅在界面上保留上次值
  } finally {
    loadingSched.value = false
  }
}

async function refreshStateTick() {
  try {
    stateTick.value = await invoke<Record<string, any>>('get_state_store_tick')
  } catch (e) {
    // 忽略错误
  }
}

async function refreshStateAgg() {
  try {
    stateAgg.value = await invoke<Record<string, any>>('get_state_store_agg')
  } catch (e) {
    // 忽略错误
  }
}

async function onRefreshAll() {
  await refreshScheduler()
  await Promise.all([
    refreshStateTick(),
    refreshStateAgg(),
  ])
}

async function onToggle(kind: 'rtt'|'netif'|'ldisk'|'smart', enabled: boolean) {
  try {
    await invoke('set_task_enabled', { kind, enabled })
  } catch (e: any) {
    message.value = `设置任务(${kind})启用失败: ${e?.message || e}`
  } finally {
    await refreshScheduler()
  }
}

async function onTrigger(kind: 'rtt'|'netif'|'ldisk'|'smart') {
  try {
    await invoke('trigger_task', { kind })
  } catch (e: any) {
    message.value = `一次性触发(${kind})失败: ${e?.message || e}`
  } finally {
    await refreshScheduler()
  }
}

async function onSetEvery(kind: 'rtt'|'netif'|'ldisk'|'smart', every?: number) {
  if (!every || every < 1) {
    message.value = `无效的 every 值（必须 >= 1）`
    return
  }
  try {
    await invoke('set_task_every', { kind, every })
    message.value = `已更新 ${kind} every=${every}`
  } catch (e: any) {
    message.value = `更新频率失败(${kind}): ${e?.message || e}`
  } finally {
    await refreshScheduler()
  }
}

function startAutoSched() {
  if (schedTimer != null) return
  autoSched.value = true
  schedTimer = window.setInterval(onRefreshAll, 2000)
}

function stopAutoSched() {
  autoSched.value = false
  if (schedTimer != null) {
    window.clearInterval(schedTimer)
    schedTimer = null
  }
}

onBeforeUnmount(() => {
  stopAutoSched()
  try {
    if (unlistenAgg) unlistenAgg()
  } catch {}
})
</script>

<style scoped>
.debug-page { padding: 16px; }
section { margin-bottom: 20px; }
.actions { display: flex; gap: 10px; margin: 10px 0; flex-wrap: wrap; }
.config-view { background: #111; color: #ddd; padding: 10px; border-radius: 6px; max-height: 320px; overflow: auto; }
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; }
label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; }
input { padding: 6px 8px; border: 1px solid #555; border-radius: 4px; background: #1a1a1a; color: #eee; }
button { padding: 6px 10px; background: #2d6cdf; color: white; border: 0; border-radius: 4px; cursor: pointer; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
.message { color: #67db83; }
.task-controls { margin: 10px 0; display: flex; flex-direction: column; gap: 8px; }
.task-row { display: flex; align-items: center; gap: 12px; }
.task-name { width: 110px; font-weight: 600; }
.badge { padding: 2px 6px; border-radius: 10px; font-size: 12px; }
.badge.on { background: #2da44e; color: #fff; }
.badge.off { background: #6b7280; color: #fff; }
.badge.warn { background: #d97706; color: #fff; }
.meta { font-size: 12px; color: #9aa0a6; }
.switch { display: inline-flex; align-items: center; gap: 6px; }
.every { display: inline-flex; align-items: center; gap: 6px; margin-left: auto; }
.tick-kpis { display: flex; align-items: center; gap: 12px; margin: 6px 0 10px; }
.kpi { font-size: 13px; color: #e5e7eb; padding: 2px 6px; background: #1f2937; border-radius: 6px; }
</style>
