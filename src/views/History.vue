<script setup lang="ts">
import { ref } from 'vue';
import { historyQuery, stateLatest } from '../api/history';

type Snap = {
  timestamp_ms: number;
  cpu_usage?: number;
};

const fromStr = ref<string>(toLocalInput(new Date(Date.now() - 60 * 60 * 1000))); // 默认近1小时
const toStr = ref<string>(toLocalInput(new Date()));
const limit = ref<number>(2000);
const loading = ref<boolean>(false);
const items = ref<Snap[]>([]);
const errMsg = ref<string>("");
const canvasRef = ref<HTMLCanvasElement | null>(null);

function toLocalInput(d: Date) {
  const pad = (n: number) => n.toString().padStart(2, '0');
  const y = d.getFullYear();
  const m = pad(d.getMonth() + 1);
  const day = pad(d.getDate());
  const h = pad(d.getHours());
  const mi = pad(d.getMinutes());
  return `${y}-${m}-${day}T${h}:${mi}`; // <input type="datetime-local">
}

function parseLocalInput(s: string): number {
  // 作为本地时间解析
  const d = new Date(s);
  if (isNaN(d.getTime())) return Date.now();
  return d.getTime();
}

function setPreset(minutes: number) {
  const now = Date.now();
  fromStr.value = toLocalInput(new Date(now - minutes * 60 * 1000));
  toStr.value = toLocalInput(new Date(now));
}

async function query() {
  loading.value = true;
  errMsg.value = "";
  items.value = [];
  try {
    const from_ts = parseLocalInput(fromStr.value);
    const to_ts = parseLocalInput(toStr.value);
    const lim = limit.value && limit.value > 0 ? limit.value : 2000;
    const res = (await historyQuery({ from_ts, to_ts, limit: lim })) as Snap[];
    // 只保留有时间戳的记录
    items.value = (Array.isArray(res) ? res : []).filter(x => typeof x.timestamp_ms === 'number');
    draw();
  } catch (e: any) {
    console.error('[History] query failed', e);
    errMsg.value = String(e?.message || e);
  } finally {
    loading.value = false;
  }
}

function draw() {
  const cvs = canvasRef.value;
  if (!cvs) return;
  const ctx = cvs.getContext('2d');
  if (!ctx) return;
  const w = cvs.width;
  const h = cvs.height;
  ctx.clearRect(0, 0, w, h);
  const data = items.value;
  if (data.length === 0) {
    ctx.fillStyle = '#999';
    ctx.fillText('无数据', 10, 20);
    return;
  }
  const xs = data.map(d => d.timestamp_ms);
  const ys = data.map(d => typeof d.cpu_usage === 'number' ? Math.max(0, Math.min(100, d.cpu_usage!)) : 0);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = 0;
  const maxY = 100;
  const pad = 30;

  // 坐标轴
  ctx.strokeStyle = '#ccc';
  ctx.beginPath();
  ctx.moveTo(pad, pad);
  ctx.lineTo(pad, h - pad);
  ctx.lineTo(w - pad, h - pad);
  ctx.stroke();

  // Y 轴刻度
  ctx.fillStyle = '#666';
  for (let y = 0; y <= 100; y += 25) {
    const yy = mapY(y, minY, maxY, h, pad);
    ctx.fillText(`${y}%`, 2, yy + 3);
    ctx.strokeStyle = '#eee';
    ctx.beginPath();
    ctx.moveTo(pad, yy);
    ctx.lineTo(w - pad, yy);
    ctx.stroke();
  }

  // 折线
  ctx.strokeStyle = '#42b883';
  ctx.beginPath();
  for (let i = 0; i < data.length; i++) {
    const x = mapX(xs[i], minX, maxX, w, pad);
    const y = mapY(ys[i], minY, maxY, h, pad);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // 辅助函数
  function mapX(v: number, a: number, b: number, width: number, p: number) {
    if (a === b) return p;
    return p + (width - 2 * p) * (v - a) / (b - a);
  }
  function mapY(v: number, a: number, b: number, height: number, p: number) {
    if (a === b) return height - p;
    // Y 轴向下
    return height - p - (height - 2 * p) * (v - a) / (b - a);
  }
}

async function loadLatestWindow() {
  // 便捷：以最新快照时间为中心，查询近15分钟
  try {
    const latest = (await stateLatest()) as Snap | null;
    const t = latest?.timestamp_ms ?? Date.now();
    fromStr.value = toLocalInput(new Date(t - 15 * 60 * 1000));
    toStr.value = toLocalInput(new Date(t));
    await query();
  } catch {}
}
</script>

<template>
  <div class="history-page">
    <h2>历史图表（CPU 使用率 %）</h2>
    <div class="toolbar">
      <label>
        From:
        <input type="datetime-local" v-model="fromStr" />
      </label>
      <label>
        To:
        <input type="datetime-local" v-model="toStr" />
      </label>
      <label>
        Limit:
        <input type="number" v-model.number="limit" min="100" step="100" />
      </label>
      <button :disabled="loading" @click="query">{{ loading ? '加载中…' : '查询' }}</button>
      <button @click="() => setPreset(15)">近15分钟</button>
      <button @click="() => setPreset(60)">近1小时</button>
      <button @click="() => setPreset(360)">近6小时</button>
      <button @click="() => setPreset(1440)">近24小时</button>
      <button @click="loadLatestWindow">对齐最新</button>
      <span class="err" v-if="errMsg">{{ errMsg }}</span>
    </div>

    <div class="chart">
      <canvas ref="canvasRef" width="900" height="260"></canvas>
    </div>

    <div class="meta">
      <span>共 {{ items.length }} 条</span>
    </div>

    <table class="table">
      <thead>
        <tr>
          <th>时间</th>
          <th>CPU 使用率%</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="it in items" :key="it.timestamp_ms">
          <td>{{ new Date(it.timestamp_ms).toLocaleString() }}</td>
          <td>{{ it.cpu_usage != null ? it.cpu_usage.toFixed(1) : '—' }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.history-page { padding: 16px; }
.toolbar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 8px; }
.toolbar label { display: flex; gap: 6px; align-items: center; }
.err { color: #c00; margin-left: 8px; }
.chart { border: 1px solid #ddd; border-radius: 6px; padding: 8px; margin: 8px 0; background: #fff; }
.table { width: 100%; border-collapse: collapse; font-size: 12px; }
.table th, .table td { border: 1px solid #eee; padding: 6px 8px; text-align: left; }
.table thead { background: #fafafa; }
</style>
