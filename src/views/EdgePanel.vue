<template>
  <div class="edge-panel" :class="{ collapsed }">
    <div class="header" @click="collapsed = !collapsed">
      <span>边缘面板</span>
      <span class="toggle">{{ collapsed ? '展开' : '收起' }}</span>
    </div>
    <div class="content" v-if="!collapsed">
      <div class="row">
        <span class="lbl">CPU</span>
        <span class="val">{{ fmtPct(snap?.cpu_usage) }}</span>
      </div>
      <div class="row">
        <span class="lbl">内存</span>
        <span class="val">{{ fmtPct(snap?.mem_pct) }} · {{ fmtGb(snap?.mem_used_gb) }}/{{ fmtGb(snap?.mem_total_gb) }}</span>
      </div>
      <div class="row">
        <span class="lbl">磁盘</span>
        <span class="val">{{ fmtBps(snap?.disk_r_bps) }} R / {{ fmtBps(snap?.disk_w_bps) }} W</span>
      </div>
      <div class="row">
        <span class="lbl">网络</span>
        <span class="val">{{ fmtBps(snap?.net_rx_bps) }} ↓ / {{ fmtBps(snap?.net_tx_bps) }} ↑</span>
      </div>
      <div class="row" v-if="snap?.gpus?.length">
        <span class="lbl">GPU</span>
        <span class="val">{{ snap?.gpus?.[0]?.name }} · {{ fmtPct(snap?.gpus?.[0]?.load_pct || 0) }} · {{ fmtC(snap?.gpus?.[0]?.temp_c) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { latestSnapshot } from '../main';

const snap = ref(latestSnapshot as any);
const collapsed = ref(false);
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  snap.value = latestSnapshot as any;
  unlisten = await listen<any>('sensor://snapshot', (e) => {
    snap.value = e.payload as any;
  });
});

onBeforeUnmount(async () => { if (unlisten) await unlisten(); });

function fmtPct(v?: number | null) { return v == null ? '—' : `${v.toFixed(0)}%`; }
function fmtGb(v?: number | null) { return v == null ? '—' : `${v.toFixed(1)}GB`; }
function fmtC(v?: number | null) { return v == null ? '—' : `${v.toFixed(0)}°C`; }
function fmtBps(v?: number | null) {
  if (v == null) return '—';
  if (v < 1024) return `${v.toFixed(0)} B/s`;
  const kb = v / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
  return `${(kb/1024).toFixed(1)} MB/s`;
}
</script>

<style scoped>
.edge-panel { 
  width: 260px; background: rgba(18,18,18,0.92); color: #f0f0f0; 
  border-left: 1px solid rgba(255,255,255,0.06);
}
.header { display: flex; justify-content: space-between; align-items: center; padding: 8px 10px; cursor: pointer; background: rgba(255,255,255,0.04); }
.toggle { opacity: 0.7; }
.content { padding: 8px 10px; }
.row { display: flex; justify-content: space-between; margin: 6px 0; }
.lbl { opacity: 0.7; }
.val { font-weight: 600; }
.collapsed .content { display: none; }
</style>
