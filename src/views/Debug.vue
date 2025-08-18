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
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

type Cfg = Record<string, any>

const cfg = ref<Cfg | null>(null)
const loading = ref(false)
const message = ref('')

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
</style>
