// 电源状态工具模块
// 包含系统电源状态查询功能

/// 读取系统电源状态（AC 接入 / 剩余时间 / 充满耗时占位）
pub fn read_power_status() -> (Option<bool>, Option<i32>, Option<i32>) {
    #[cfg(windows)]
    {
        use ::windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
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
    #[cfg(not(windows))]
    {
        // 非Windows平台暂不支持
        (None, None, None)
    }
}
