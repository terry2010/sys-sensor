// ================================================================================
// 电池工具模块
// ================================================================================
// 
// 包含电池状态查询和状态码转换的工具函数
//

use wmi::WMIConnection;

// 电池 WMI 查询结构体（健康数据）
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_Battery")]
pub struct Win32Battery {
    #[serde(rename = "DeviceID")]
    #[allow(dead_code)]
    pub device_id: Option<String>,
    #[serde(rename = "Name")]
    #[allow(dead_code)]
    pub name: Option<String>,
    #[serde(rename = "DesignCapacity")]
    #[allow(dead_code)]
    pub design_capacity: Option<u32>,
    #[serde(rename = "FullChargeCapacity")]
    #[allow(dead_code)]
    pub full_charge_capacity: Option<u32>,
    #[serde(rename = "CycleCount")]
    #[allow(dead_code)]
    pub cycle_count: Option<u32>,
    #[serde(rename = "EstimatedChargeRemaining")]
    #[allow(dead_code)]
    pub estimated_charge_remaining: Option<u16>,
    #[serde(rename = "BatteryStatus")]
    #[allow(dead_code)]
    pub battery_status: Option<u16>,
    #[serde(rename = "EstimatedRunTime")]
    #[allow(dead_code)]
    pub estimated_run_time_min: Option<u32>,
    #[serde(rename = "TimeToFullCharge")]
    #[allow(dead_code)]
    pub time_to_full_min: Option<u32>,
    #[serde(rename = "Chemistry")]
    #[allow(dead_code)]
    pub chemistry: Option<u16>,
}

/// 将电池状态码转换为中文描述
#[allow(dead_code)]
pub fn battery_status_to_str(code: u16) -> &'static str {
    match code {
        1 => "放电中",
        2 => "接通电源、未充电",
        3 => "已充满",
        4 => "低电量",
        5 => "严重低电",
        6 => "充电中",
        7 => "充电并接通电源",
        8 => "未充电（备用）",
        9 => "未安装电池",
        10 => "未知",
        11 => "部分充电",
        _ => "未知",
    }
}

/// 读取电池电量和状态
#[allow(dead_code)]
pub fn wmi_read_battery(conn: &WMIConnection) -> (Option<i32>, Option<String>) {
    let res: Result<Vec<Win32Battery>, _> = conn.query();
    if let Ok(list) = res {
        if let Some(b) = list.into_iter().next() {
            let pct = b.estimated_charge_remaining;
            let status = b
                .battery_status
                .map(|c| battery_status_to_str(c).to_string());
            return (pct.map(|p| p as i32), status);
        }
    }
    (None, None)
}

/// 读取电池时间信息（剩余时间和充满时间）
#[allow(dead_code)]
pub fn wmi_read_battery_time(conn: &WMIConnection) -> (Option<i32>, Option<i32>) {
    let res: Result<Vec<Win32Battery>, _> = conn.query();
    if let Ok(list) = res {
        if let Some(b) = list.into_iter().next() {
            let remain_sec = b.estimated_run_time_min.and_then(|m| if m > 0 { Some((m * 60) as i32) } else { None });
            let to_full_sec = b.time_to_full_min.and_then(|m| if m > 0 { Some((m * 60) as i32) } else { None });
            return (remain_sec, to_full_sec);
        }
    }
    (None, None)
}

/// 读取电池健康信息（设计容量、满充容量、循环次数）
#[allow(dead_code)]
pub fn wmi_read_battery_health(conn: &WMIConnection) -> (Option<u32>, Option<u32>, Option<u32>) {
    let res: Result<Vec<Win32Battery>, _> = conn.query();
    if let Ok(list) = res {
        if let Some(battery) = list.first() {
            return (
                battery.design_capacity,
                battery.full_charge_capacity,
                battery.cycle_count,
            );
        }
    }
    (None, None, None)
}
