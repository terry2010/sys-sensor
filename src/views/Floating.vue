<template>
  <div class="floating">
    <div class="kpi" v-if="snap">
      <div class="row">
        <span class="lbl">CPU</span>
        <span class="val">{{ fmtPct(snap.cpu_usage) }}</span>
      </div>
      <div class="row">
        <span class="lbl">MEM</span>
        <span class="val">{{ fmtPct(snap.mem_pct) }} ({{ fmtGb(snap.mem_used_gb) }}/{{ fmtGb(snap.mem_total_gb) }})</span>
      </div>
      <div class="row">
        <span class="lbl">NET</span>
        <span class="val">{{ fmtBps(snap.net_rx_bps) }} ↓ / {{ fmtBps(snap.net_tx_bps) }} ↑</span>
      </div>
      <div class="row" v-if="snap.gpus?.length">
        <span class="lbl">GPU</span>
        <span class="val">{{ snap.gpus?.[0]?.name }} {{ fmtPct(snap.gpus?.[0]?.load_pct || 0) }} {{ fmtC(snap.gpus?.[0]?.temp_c) }}</span>
      </div>
    </div>
    <div class="empty" v-else>等待数据...</div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { } from '../main';
import { latestSnapshot } from '../main';

const snap = ref(latestSnapshot);
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  snap.value = latestSnapshot;
  unlisten = await listen<any>('sensor://snapshot', (e) => {
    snap.value = e.payload as any;
  });
});

onBeforeUnmount(async () => { if (unlisten) await unlisten(); });

function fmtPct(v?: number) { return v == null ? '—' : `${v.toFixed(0)}%`; }
function fmtGb(v?: number) { return v == null ? '—' : `${v.toFixed(1)}GB`; }
function fmtC(v?: number) { return v == null ? '—' : `${v.toFixed(0)}°C`; }
function fmtBps(v?: number) {
  if (v == null) return '—';
  if (v < 1024) return `${v.toFixed(0)} B/s`;
  const kb = v / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  return `${(kb/1024).toFixed(1)} MB/s`;
}
</script>

<style scoped>
.floating { 
  font: 13px/1.4 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial;
  color: #e8e8e8; background: rgba(20,20,20,0.85); padding: 10px 12px; 
}
.row { display: flex; justify-content: space-between; margin: 4px 0; }
.lbl { opacity: 0.7; }
.val { font-weight: 600; }
.empty { opacity: 0.6; }
</style>
