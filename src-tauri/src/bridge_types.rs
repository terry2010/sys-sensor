// ================================================================================
// 桥接数据结构模块
// ================================================================================
// 
// 本模块包含与桥接进程通信的数据结构定义：
// 1. 桥接风扇数据结构
// 2. 桥接电压数据结构
// 3. 系统传感器快照结构
// 4. 电源状态读取函数
//
// ================================================================================

use crate::types::{
    NetIfPayload, VoltagePayload, FanPayload, StorageTempPayload, 
    LogicalDiskPayload, SmartHealthPayload, GpuPayload
};
use crate::process_utils::{RttResultPayload, TopProcessPayload};

// 读取系统电源状态（AC 接入 / 剩余时间 / 充满耗时占位）
pub fn _read_power_status() -> (Option<bool>, Option<i32>, Option<i32>) {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    unsafe {
        let mut sps = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut sps).is_ok() {
            let ac = match sps.ACLineStatus {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            };
            let remain = if sps.BatteryLifeTime == u32::MAX { None } else { Some(sps.BatteryLifeTime as i32) };
            // WinAPI 未直接提供"充满耗时"，后续可尝试 WMI Win32_Battery.TimeToFullCharge（分钟）
            let to_full: Option<i32> = None;
            (ac, remain, to_full)
        } else {
            (None, None, None)
        }
    }
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeFan {
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub rpm: Option<i32>,
    #[allow(dead_code)]
    pub pct: Option<i32>,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeVoltage {
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub volts: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
#[allow(dead_code)]
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
    pub rtt_multi: Option<Vec<RttResultPayload>>,
    pub top_cpu_procs: Option<Vec<TopProcessPayload>>,
    pub top_mem_procs: Option<Vec<TopProcessPayload>>,
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
