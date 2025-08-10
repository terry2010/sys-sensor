<script setup lang="ts">
import { ref } from "vue";

const startOnBoot = ref(false);
const showMemOnTray = ref(true);

function save() {
  // TODO: 持久化到配置文件（后续实现）
  console.log("[settings]", { startOnBoot: startOnBoot.value, showMemOnTray: showMemOnTray.value });
}
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
        <input type="checkbox" v-model="showMemOnTray" /> 托盘第二行显示内存占用
      </label>
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
