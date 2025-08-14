// ================================================================================
// 温度和风扇工具模块
// ================================================================================
// 
// 包含 CPU 温度查询和风扇转速查询的工具函数
//

use wmi::WMIConnection;

// ---- WMI helpers: temperature & fan ----
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSAcpi_ThermalZoneTemperature")]
pub struct MSAcpiThermalZoneTemperature {
    #[serde(rename = "CurrentTemperature")]
    pub current_temperature: Option<u32>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_Fan")]
pub struct Win32Fan {
    #[serde(rename = "DesiredSpeed")]
    pub desired_speed: Option<u64>,
}

/// 读取 CPU 温度（摄氏度）
/// 通过 WMI 查询 MSAcpi_ThermalZoneTemperature，将 Kelvin x10 转换为摄氏度
pub fn wmi_read_cpu_temp_c(conn: &WMIConnection) -> Option<f32> {
    let res: Result<Vec<MSAcpiThermalZoneTemperature>, _> = conn.query();
    let mut vals: Vec<f32> = Vec::new();
    if let Ok(list) = res {
        for item in list.into_iter() {
            if let Some(kx10) = item.current_temperature {
                // Kelvin x10 -> Celsius
                if kx10 > 0 {
                    let c = (kx10 as f32 / 10.0) - 273.15;
                    // 过滤异常值
                    if c > -50.0 && c < 150.0 {
                        vals.push(c);
                    }
                }
            }
        }
    }
    if vals.is_empty() { None } else { Some(vals.iter().copied().sum::<f32>() / vals.len() as f32) }
}

/// 读取风扇转速（RPM）
/// 通过 WMI 查询 Win32_Fan，读取 DesiredSpeed 作为近似值
pub fn wmi_read_fan_rpm(conn: &WMIConnection) -> Option<u32> {
    // Win32_Fan 通常不提供实时转速，这里尽力读取 DesiredSpeed 作为近似；若无则返回 None
    let res: Result<Vec<Win32Fan>, _> = conn.query();
    if let Ok(list) = res {
        for fan in list.into_iter() {
            if let Some(speed) = fan.desired_speed {
                if speed > 0 {
                    return Some(speed as u32);
                }
            }
        }
    }
    None
}
