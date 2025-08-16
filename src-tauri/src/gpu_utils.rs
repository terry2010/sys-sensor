// ================================================================================
// GPU查询工具模块
// ================================================================================


// ---- GPU相关结构体 ----

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeGpu {
    pub name: Option<String>,
    pub temp_c: Option<f32>,
    pub load_pct: Option<f32>,
    pub core_mhz: Option<f64>,
    pub memory_mhz: Option<f64>,
    pub fan_rpm: Option<i32>,
    pub fan_duty_pct: Option<i32>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
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


// ---- GPU查询函数 ----




