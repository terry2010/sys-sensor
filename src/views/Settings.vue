<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 仍保留“随系统启动”占位（后续实现开机启动注册）
const startOnBoot = ref(false);

// 托盘第二行显示模式："cpu" | "mem" | "fan"
const trayBottomMode = ref<'cpu' | 'mem' | 'fan'>('cpu');
// 启用 SMART 采集（默认 true）
const smartEnabled = ref(true);
let unlistenSmartStatus: null | (() => void) = null;

// 统一采样节拍（毫秒），默认 1000，最小 100
const intervalMs = ref<number>(1000);

// 网卡多选（为空=聚合全部）
const nicOptions = ref<string[]>([]);
const selectedNics = ref<string[]>([]);
// 监听句柄：配置变更后自动刷新本页配置
let unlistenCfg: null | (() => void) = null;

async function loadConfig() {
  try {
    const cfg: any = await invoke("get_config");
    // 兼容：优先使用 tray_bottom_mode；若无则根据 tray_show_mem 推断
    const mode = (cfg?.tray_bottom_mode ?? '').toString();
    if (mode === 'cpu' || mode === 'mem' || mode === 'fan') {
      trayBottomMode.value = mode as any;
    } else {
      trayBottomMode.value = cfg?.tray_show_mem ? 'mem' : 'cpu';
    }
    selectedNics.value = Array.isArray(cfg?.net_interfaces) ? cfg.net_interfaces : [];
    smartEnabled.value = (cfg?.smart_enabled ?? true) === true;
    intervalMs.value = Math.max(100, Number(cfg?.interval_ms ?? 1000));
  } catch (e) {
    console.error("[settings] loadConfig", e);
  }
  try {
    nicOptions.value = (await invoke<string[]>("list_net_interfaces")) || [];
  } catch (e) {
    console.warn("[settings] list_net_interfaces", e);
  }
}

async function toggleSmart() {
  try {
    const ok = await invoke<boolean>("smart_enable", { enabled: smartEnabled.value });
    console.log("[settings] smart_enable =>", ok);
  } catch (e) {
    console.error("[settings] smart_enable failed", e);
  }
}

async function save() {
  try {
    const new_cfg = {
      tray_bottom_mode: trayBottomMode.value,
      // 兼容旧字段，便于老版本读取
      tray_show_mem: trayBottomMode.value === 'mem',
      net_interfaces: selectedNics.value,
      smart_enabled: smartEnabled.value,
    };
    await invoke("set_config", { newCfg: new_cfg });
    console.log("[settings] saved", new_cfg);
    // 可选：提示保存成功
  } catch (e) {
    console.error("[settings] save", e);
  }
}

// 热更新统一节拍（立即生效并持久化）
async function applyInterval() {
  try {
    const v = Math.max(100, Math.floor(Number(intervalMs.value) || 1000));
    intervalMs.value = v;
    const patch = { interval_ms: v } as any;
    const merged = await invoke<any>("cmd_cfg_update", { patch });
    console.log("[settings] cmd_cfg_update interval_ms =>", merged?.interval_ms);
  } catch (e) {
    console.error("[settings] applyInterval", e);
  }
}

onMounted(async () => {
  await loadConfig();
  try {
    unlistenCfg = await listen("config://changed", async () => {
      console.debug("[settings] config changed -> reload");
      await loadConfig();
    });
  } catch (e) {
    // 非 Tauri 环境或事件不可用时静默降级
    console.warn("[settings] listen config://changed failed", e);
  }
  try {
    unlistenSmartStatus = await listen("sensor://smart_status", (ev: any) => {
      const en = !!ev?.payload?.enabled;
      smartEnabled.value = en;
    });
  } catch (e) {
    // 忽略
  }
});

onUnmounted(() => {
  try { if (typeof unlistenCfg === 'function') { unlistenCfg(); } } catch {}
  unlistenCfg = null;
  try { if (typeof unlistenSmartStatus === 'function') { unlistenSmartStatus(); } } catch {}
  unlistenSmartStatus = null;
});
</script>

<template>
  <div class="settings-wrap">
    <h2>快速设置</h2>
    <div class="group">
      <label>
        <input type="checkbox" v-model="startOnBoot" /> 随系统启动
      </label>
    </div>
    <div class="group">
      <label>
        <input type="checkbox" v-model="smartEnabled" @change="toggleSmart" /> 启用 SMART 采集（即时生效）
      </label>
    </div>
    <div class="group">
      <label>统一采样节拍（ms）：</label>
      <input type="number" v-model.number="intervalMs" min="100" step="100" style="width:120px; margin-left:6px;" />
      <button class="secondary" style="margin-left:8px;" @click="applyInterval">应用（热更新）</button>
      <div style="margin-top:6px; color:#888;">提示：最小 100ms；修改后立即生效并写入配置。</div>
    </div>
    <div class="group">
      <label>托盘第二行显示：</label>
      <select v-model="trayBottomMode">
        <option value="cpu">CPU%</option>
        <option value="mem">内存%</option>
        <option value="fan">风扇RPM</option>
      </select>
    </div>
    <div class="group">
      <div>网络速率来源（可多选；不选=聚合全部）：</div>
      <select multiple v-model="selectedNics" size="6" style="min-width: 220px;">
        <option v-for="nic in nicOptions" :key="nic" :value="nic">{{ nic }}</option>
      </select>
      <div style="margin-top:6px; color:#888;">提示：清空选择即统计所有网卡；保存后生效。</div>
    </div>
    <button class="primary" @click="save">保存</button>
  </div>
</template>

<style scoped>
.settings-wrap { padding: 16px; }
.group { margin: 12px 0; }
button.primary {
  padding: 8px 14px;
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #1677ff;
  color: #fff;
}
button.secondary {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #fff;
  color: #333;
}
@media (prefers-color-scheme: dark) {
  button.primary { border-color: #555; }
}
</style>
