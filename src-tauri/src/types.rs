// 数据结构定义模块
// 包含所有前端数据结构和WMI查询结构体定义

use serde::{Deserialize, Serialize};

// ================================================================================
// 前端数据结构定义 (PAYLOAD 结构体)
// ================================================================================

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltagePayload {
    pub name: Option<String>,
    pub volts: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FanPayload {
    pub name: Option<String>,
    pub rpm: Option<i32>,
    pub pct: Option<i32>,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StorageTempPayload {
    pub name: Option<String>,
    pub temp_c: Option<f32>,
    pub drive_letter: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuPayload {
    pub name: Option<String>,
    pub temp_c: Option<f32>,
    pub load_pct: Option<f32>,
    pub core_mhz: Option<f64>,
    pub memory_mhz: Option<f64>,
    pub fan_rpm: Option<i32>,
    pub fan_duty_pct: Option<i32>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
    pub vram_usage_pct: Option<f64>,
    pub power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub voltage_v: Option<f64>,
    pub hotspot_temp_c: Option<f32>,
    pub vram_temp_c: Option<f32>,
    // GPU深度监控新增字段
    pub encode_util_pct: Option<f32>,    // 编码单元使用率
    pub decode_util_pct: Option<f32>,    // 解码单元使用率
    pub vram_bandwidth_pct: Option<f32>, // 显存带宽使用率
    pub p_state: Option<String>,         // P-State功耗状态
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartHealthPayload {
    pub device: Option<String>,
    pub drive_letter: Option<String>, // 盘符信息，如 "C:", "D:" 等
    pub predict_fail: Option<bool>,
    pub temp_c: Option<f32>,
    pub power_on_hours: Option<i32>,
    pub reallocated: Option<i64>,
    pub pending: Option<i64>,
    pub uncorrectable: Option<i64>,
    pub crc_err: Option<i64>,
    pub power_cycles: Option<i32>,
    pub host_reads_bytes: Option<i64>,
    pub host_writes_bytes: Option<i64>,
    // NVMe 特有指标
    pub nvme_percentage_used_pct: Option<f32>,
    pub nvme_available_spare_pct: Option<f32>,
    pub nvme_available_spare_threshold_pct: Option<f32>,
    pub nvme_media_errors: Option<i64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetIfPayload {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub ip: Option<String>,
    pub ips: Option<Vec<String>>,
    pub mac: Option<String>,
    pub speed_mbps: Option<u64>,
    pub media_type: Option<String>,
    pub dhcp_enabled: Option<bool>,
    pub gateway: Option<Vec<String>>,
    pub dns_servers: Option<Vec<String>>,
    pub dns: Option<Vec<String>>, // 前端兼容性别名
    pub status: Option<String>,
    pub up: Option<bool>, // 网络接口状态
    pub bytes_recv: Option<u64>,
    pub bytes_sent: Option<u64>,
    pub packets_recv: Option<u64>,
    pub packets_sent: Option<u64>,
    pub errors_recv: Option<u64>,
    pub errors_sent: Option<u64>,
    // 新增网络质量监控指标
    pub packet_loss_pct: Option<f64>,     // 丢包率百分比
    pub active_connections: Option<u32>,   // 活动连接数
    pub discarded_recv: Option<u64>,       // 接收丢弃包数
    pub discarded_sent: Option<u64>,       // 发送丢弃包数
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalDiskPayload {
    pub drive_letter: Option<String>,
    pub total_gb: Option<f64>,
    pub free_gb: Option<f64>,
    pub usage_pct: Option<f64>,
    pub fs: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CpuPayload {
    pub name: Option<String>,
    pub cores: Option<u32>,
    pub threads: Option<u32>,
    pub base_mhz: Option<f64>,
    pub max_mhz: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MemoryPayload {
    pub total_gb: Option<f64>,
    pub available_gb: Option<f64>,
    pub usage_pct: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DiskPayload {
    pub name: Option<String>,
    pub model: Option<String>,
    pub size_gb: Option<f64>,
    pub interface_type: Option<String>,
    pub media_type: Option<String>,
    #[allow(dead_code)]
    pub health: Option<String>,
}

// ================================================================================
// WMI 查询结构体定义
// ================================================================================

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PerfDiskPhysical {
    pub name: Option<String>,
    #[serde(rename = "DiskReadBytesPerSec")]
    #[allow(dead_code)]
    pub disk_read_bytes_per_sec: Option<f64>,
    #[serde(rename = "DiskWriteBytesPerSec")]
    #[allow(dead_code)]
    pub disk_write_bytes_per_sec: Option<f64>,
    #[serde(rename = "DiskReadsPerSec")]
    pub disk_reads_per_sec: Option<f64>,
    #[serde(rename = "DiskWritesPerSec")]
    pub disk_writes_per_sec: Option<f64>,
    #[serde(rename = "CurrentDiskQueueLength")]
    pub current_disk_queue_length: Option<f64>,
    #[serde(rename = "PercentDiskTime")]
    #[allow(dead_code)]
    pub percent_disk_time: Option<f64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PerfTcpipNic {
    pub name: Option<String>,
    #[serde(rename = "BytesReceivedPerSec")]
    pub bytes_received_per_sec: Option<f64>,
    #[serde(rename = "BytesSentPerSec")]
    pub bytes_sent_per_sec: Option<f64>,
    #[serde(rename = "BytesTotalPerSec")]
    #[allow(dead_code)]
    pub bytes_total_per_sec: Option<f64>,
    #[serde(rename = "CurrentBandwidth")]
    #[allow(dead_code)]
    pub current_bandwidth: Option<f64>,
    #[serde(rename = "OutputQueueLength")]
    #[allow(dead_code)]
    pub output_queue_length: Option<f64>,
    #[serde(rename = "PacketsOutboundDiscarded")]
    pub packets_outbound_discarded: Option<u64>,
    #[serde(rename = "PacketsOutboundErrors")]
    pub packets_outbound_errors: Option<u64>,
    #[serde(rename = "PacketsReceivedDiscarded")]
    pub packets_received_discarded: Option<u64>,
    #[serde(rename = "PacketsReceivedErrors")]
    pub packets_received_errors: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PerfOsMemory {
    #[serde(rename = "AvailableBytes")]
    #[allow(dead_code)]
    pub available_bytes: Option<u64>,
    #[serde(rename = "CacheBytes")]
    pub cache_bytes: Option<u64>,
    #[serde(rename = "CommittedBytes")]
    pub committed_bytes: Option<u64>,
    #[serde(rename = "CommitLimit")]
    pub commit_limit: Option<u64>,
    #[serde(rename = "PoolPagedBytes")]
    pub pool_paged_bytes: Option<u64>,
    #[serde(rename = "PoolNonpagedBytes")]
    pub pool_nonpaged_bytes: Option<u64>,
    #[serde(rename = "PagesPerSec")]
    pub pages_per_sec: Option<f64>,
    #[serde(rename = "PageReadsPerSec")]
    pub page_reads_per_sec: Option<f64>,
    #[serde(rename = "PageWritesPerSec")]
    pub page_writes_per_sec: Option<f64>,
    #[serde(rename = "PageFaultsPerSec")]
    pub page_faults_per_sec: Option<f64>,
}

// ================================================================================
// 桥接进程相关结构体
// ================================================================================

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeFan {
    pub name: Option<String>,
    pub rpm: Option<i32>,
    pub pct: Option<i32>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeVoltage {
    pub name: Option<String>,
    pub volts: Option<f64>,
}

#[derive(Clone, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BridgeOut {
    pub cpu_temp_c: Option<f32>,
    pub mobo_temp_c: Option<f32>,
    pub fans: Option<Vec<BridgeFan>>,
    // 透传：多风扇与主板电压
    pub fans_extra: Option<Vec<BridgeFan>>,
    pub mobo_voltages: Option<Vec<BridgeVoltage>>,
    pub storage_temps: Option<Vec<BridgeStorageTemp>>,
    pub gpus: Option<Vec<crate::gpu_utils::BridgeGpu>>,
    #[allow(dead_code)]
    pub is_admin: Option<bool>,
    #[allow(dead_code)]
    pub has_temp: Option<bool>,
    #[allow(dead_code)]
    pub has_temp_value: Option<bool>,
    #[allow(dead_code)]
    pub has_fan: Option<bool>,
    #[allow(dead_code)]
    pub has_fan_value: Option<bool>,
    // 第二梯队：CPU 指标
    pub cpu_pkg_power_w: Option<f64>,
    pub cpu_avg_freq_mhz: Option<f64>,
    pub cpu_throttle_active: Option<bool>,
    pub cpu_throttle_reasons: Option<Vec<String>>,
    pub since_reopen_sec: Option<i32>,
    // 每核心：负载/频率/温度（桥接输出：cpuCoreLoadsPct/cpuCoreClocksMhz/cpuCoreTempsC）
    pub cpu_core_loads_pct: Option<Vec<Option<f32>>>,
    pub cpu_core_clocks_mhz: Option<Vec<Option<f64>>>,
    pub cpu_core_temps_c: Option<Vec<Option<f32>>>,
    // 健康指标
    pub hb_tick: Option<i64>,
    pub idle_sec: Option<i32>,
    pub exc_count: Option<i32>,
    pub uptime_sec: Option<i32>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStorageTemp {
    pub name: Option<String>,
    #[serde(rename = "tempC")]
    pub temp_c: Option<f32>,
    #[allow(dead_code)]
    pub health: Option<String>,
}

// ================================================================================
// 实时快照数据结构
// ================================================================================

#[derive(Clone, Serialize)]
pub struct SensorSnapshot {
    pub cpu_usage: f32,
    pub mem_used_gb: f32,
    pub mem_total_gb: f32,
    pub mem_pct: f32,
    // 内存细分（可用）与交换区
    pub mem_avail_gb: Option<f32>,
    pub swap_used_gb: Option<f32>,
    pub swap_total_gb: Option<f32>,
    // 内存细分扩展：缓存/提交/分页相关
    pub mem_cache_gb: Option<f32>,
    pub mem_committed_gb: Option<f32>,
    pub mem_commit_limit_gb: Option<f32>,
    pub mem_pool_paged_gb: Option<f32>,
    pub mem_pool_nonpaged_gb: Option<f32>,
    pub mem_pages_per_sec: Option<f64>,
    pub mem_page_reads_per_sec: Option<f64>,
    pub mem_page_writes_per_sec: Option<f64>,
    pub mem_page_faults_per_sec: Option<f64>,
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
    // 新增：瞬时网速（未经EMA平滑）
    pub net_rx_instant_bps: f64,
    pub net_tx_instant_bps: f64,
    // 新增：公网 IP 与 ISP
    pub public_ip: Option<String>,
    pub isp: Option<String>,
    // 新增：Wi‑Fi 指标（若无连接则为 None）
    pub wifi_ssid: Option<String>,
    pub wifi_signal_pct: Option<i32>,
    pub wifi_link_mbps: Option<i32>,
    // Wi‑Fi 扩展
    pub wifi_bssid: Option<String>,
    pub wifi_channel: Option<i32>,
    pub wifi_radio: Option<String>,
    pub wifi_band: Option<String>,
    pub wifi_rx_mbps: Option<i32>,
    pub wifi_tx_mbps: Option<i32>,
    pub wifi_rssi_dbm: Option<i32>,
    pub wifi_rssi_estimated: Option<bool>,
    // Wi‑Fi 扩展2：安全/加密/信道宽度
    pub wifi_auth: Option<String>,
    pub wifi_cipher: Option<String>,
    pub wifi_chan_width_mhz: Option<i32>,
    // 新增：网络接口（IP/MAC/速率/介质）
    pub net_ifs: Option<Vec<NetIfPayload>>,
    pub disk_r_bps: f64,
    pub disk_w_bps: f64,
    // 新增：温度（摄氏度）与风扇转速（RPM），可能不可用
    pub cpu_temp_c: Option<f32>,
    pub mobo_temp_c: Option<f32>,
    pub fan_rpm: Option<i32>,
    // 新增：主板电压与多风扇详细（从桥接透传）
    pub mobo_voltages: Option<Vec<VoltagePayload>>, // [{ name, volts }]
    pub fans_extra: Option<Vec<FanPayload>>,         // [{ name, rpm, pct }]
    // 新增：存储温度（NVMe/SSD），与桥接字段 storageTemps 对应
    pub storage_temps: Option<Vec<StorageTempPayload>>,
    // 新增：逻辑磁盘容量（每盘总容量/可用空间）
    pub logical_disks: Option<Vec<LogicalDiskPayload>>,
    // 新增：SMART 健康（是否预测失败）
    pub smart_health: Option<Vec<SmartHealthPayload>>,
    // 新增：桥接健康指标
    pub hb_tick: Option<i64>,
    pub idle_sec: Option<i32>,
    pub exc_count: Option<i32>,
    pub uptime_sec: Option<i32>,
    // 第二梯队：CPU 扩展与桥接重建秒数
    pub cpu_pkg_power_w: Option<f64>,
    pub cpu_avg_freq_mhz: Option<f64>,
    pub cpu_throttle_active: Option<bool>,
    pub cpu_throttle_reasons: Option<Vec<String>>,
    pub since_reopen_sec: Option<i32>,
    // 每核心：负载/频率/温度（与桥接输出对应）。数组元素可为 null。
    pub cpu_core_loads_pct: Option<Vec<Option<f32>>>,
    pub cpu_core_clocks_mhz: Option<Vec<Option<f64>>>,
    pub cpu_core_temps_c: Option<Vec<Option<f32>>>,
    // 第二梯队：磁盘 IOPS/队列长度
    pub disk_r_iops: Option<f64>,
    pub disk_w_iops: Option<f64>,
    pub disk_queue_len: Option<f64>,
    // 第二梯队：网络错误率（每秒）与近似延迟（ms）
    pub net_rx_err_ps: Option<f64>,
    pub net_tx_err_ps: Option<f64>,
    pub ping_rtt_ms: Option<f64>,
    // 新增：网络丢包率与活动连接数
    pub packet_loss_pct: Option<f64>,
    pub active_connections: Option<u32>,
    // 新增：多目标 RTT 结果与 Top 进程
    pub rtt_multi: Option<Vec<crate::process_utils::RttResultPayload>>,
    pub top_cpu_procs: Option<Vec<crate::process_utils::TopProcessPayload>>,
    pub top_mem_procs: Option<Vec<crate::process_utils::TopProcessPayload>>,
    // 新增：GPU 列表
    pub gpus: Option<Vec<GpuPayload>>,
    // 新增：电池
    pub battery_percent: Option<i32>,
    pub battery_status: Option<String>,
    pub battery_design_capacity: Option<u32>,
    pub battery_full_charge_capacity: Option<u32>,
    pub battery_cycle_count: Option<u32>,
    pub battery_ac_online: Option<bool>,
    pub battery_time_remaining_sec: Option<i32>,
    pub battery_time_to_full_sec: Option<i32>,
    pub timestamp_ms: i64,
}
