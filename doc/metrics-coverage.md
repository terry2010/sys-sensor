# 指标覆盖与字段映射（v2025-08-18）

- CPU: cpu_usage, cpu_pkg_power_w, cpu_avg_freq_mhz, cpu_throttle_active, cpu_throttle_reasons, cpu_core_loads_pct[], cpu_core_clocks_mhz[], cpu_core_temps_c[]
- 内存: mem_used_gb, mem_total_gb, mem_pct, mem_avail_gb, mem_cache_gb, mem_committed_gb, mem_commit_limit_gb, mem_pool_paged_gb, mem_pool_nonpaged_gb, mem_pages_per_sec, mem_page_reads_per_sec, mem_page_writes_per_sec, mem_page_faults_per_sec
- 网络: net_rx_bps, net_tx_bps, net_rx_instant_bps, net_tx_instant_bps, net_ifs[], public_ip, isp, wifi_* 系列
- 磁盘IO: disk_r_bps, disk_w_bps, disk_r_iops, disk_w_iops
- 存储容量: logical_disks[]
- 存储温度: storage_temps[]（含 drive_letter）
- SMART 健康: smart_health[]（含 NVMe 指标与 host_reads_bytes/host_writes_bytes）
- GPU: gpus[]（含 vram_used_mb/vram_total_mb/vram_usage_pct、power_w、voltage_v、fan_rpm 等）
- 温度/风扇/电压: cpu_temp_c, mobo_temp_c, fan_rpm, mobo_voltages[], fans_extra[]
- 电池: battery_* 系列
- 运行时: hb_tick, idle_sec, exc_count, uptime_sec, since_reopen_sec
- 其他: cpu_throttle_* 系列、wifi 认证与链路信息

注：字段名以 `src-tauri/src/types.rs` 的 `SensorSnapshot` 为准；前端类型在 `src/main.ts` 与 `src/views/Details.vue` 已同步。
