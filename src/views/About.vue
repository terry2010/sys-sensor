<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getVersion } from "@tauri-apps/api/app";

const version = ref<string>("");
onMounted(async () => {
  try { version.value = await getVersion(); } catch { version.value = ""; }
});
</script>

<template>
  <div class="about-wrap">
    <h2>关于 sys-sensor</h2>
    <p>一个基于 Tauri 的 Windows 系统托盘硬件监控工具。</p>
    <p>版本：{{ version || '0.1.0' }}</p>
    <p>
      项目主页：
      <a href="https://github.com/" target="_blank">GitHub</a>
    </p>
  </div>
</template>

<style scoped>
.about-wrap { padding: 16px; }
</style>
