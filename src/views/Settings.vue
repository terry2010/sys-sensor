<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 仍保留“随系统启动”占位（后续实现开机启动注册）
const startOnBoot = ref(false);

// 托盘第二行显示模式："cpu" | "mem" | "fan"
const trayBottomMode = ref<'cpu' | 'mem' | 'fan'>('cpu');

// 网卡多选（为空=聚合全部）
const nicOptions = ref<string[]>([]);
const selectedNics = ref<string[]>([]);

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
  } catch (e) {
    console.error("[settings] loadConfig", e);
  }
  try {
    nicOptions.value = (await invoke<string[]>("list_net_interfaces")) || [];
  } catch (e) {
    console.warn("[settings] list_net_interfaces", e);
  }
}

async function save() {
  try {
    const new_cfg = {
      tray_bottom_mode: trayBottomMode.value,
      // 兼容旧字段，便于老版本读取
      tray_show_mem: trayBottomMode.value === 'mem',
      net_interfaces: selectedNics.value,
    };
    await invoke("set_config", { newCfg: new_cfg });
    console.log("[settings] saved", new_cfg);
    // 可选：提示保存成功
  } catch (e) {
    console.error("[settings] save", e);
  }
}

onMounted(() => { loadConfig(); });
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
@media (prefers-color-scheme: dark) {
  button.primary { border-color: #555; }
}
</style>
