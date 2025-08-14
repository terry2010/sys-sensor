// ================================================================================
// PowerShell 查询工具模块
// ================================================================================
// 
// 本模块包含使用 PowerShell 查询系统信息的功能：
// 1. NVMe Storage 可靠性计数器查询
// 2. PowerShell 脚本执行和结果解析
// 3. 控制台输出字节解码
//
// ================================================================================

use crate::types::SmartHealthPayload;
use crate::wmi_utils::decode_console_bytes;

// 使用 PowerShell 查询 NVMe 的 Storage 可靠性计数器作为回退（适用于多数 NVMe 不支持 MSStorageDriver_* 的情况）
// 仅填充可获取到的字段：温度/通电/上电次数/累计读写字节数。其余保持 None。
#[cfg(windows)]
pub fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> {
    // 组合对象：把 PhysicalDisk 的标识（FriendlyName/UniqueId/SerialNumber）与计数器合并输出为 JSON
    let ps_script: &str = r#"
        $ErrorActionPreference='SilentlyContinue';
        $items = Get-PhysicalDisk | ForEach-Object {
          $pd = $_; $c = $_ | Get-StorageReliabilityCounter;
          if ($c) {
            [PSCustomObject]@{
              FriendlyName = $pd.FriendlyName;
              UniqueId = $pd.UniqueId;
              SerialNumber = $pd.SerialNumber;
              Temperature = $c.Temperature;
              PowerOnHours = $c.PowerOnHours;
              PowerCycleCount = $c.PowerCycleCount;
              ReadBytes = $c.ReadBytes;
              WriteBytes = $c.WriteBytes;
            }
          }
        };
        $items | ConvertTo-Json -Depth 3
    "#;

    let output = (|| {
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.output().ok()
    })()?;

    if !output.status.success() { return None; }
    let text = decode_console_bytes(&output.stdout);
    let s = text.trim();
    if s.is_empty() { return None; }

    #[derive(serde::Deserialize, Debug)]
    struct PsReliability {
        #[serde(rename = "FriendlyName")] friendly_name: Option<String>,
        #[serde(rename = "UniqueId")] unique_id: Option<String>,
        #[serde(rename = "SerialNumber")] serial_number: Option<String>,
        #[serde(rename = "Temperature")] temperature: Option<i32>,
        #[serde(rename = "PowerOnHours")] power_on_hours: Option<u64>,
        #[serde(rename = "PowerCycleCount")] power_cycle_count: Option<u64>,
        #[serde(rename = "ReadBytes")] read_bytes: Option<u64>,
        #[serde(rename = "WriteBytes")] write_bytes: Option<u64>,
    }

    // 处理单对象/数组两种 JSON 形态
    let mut rows: Vec<PsReliability> = match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => arr.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect(),
        Ok(v) => serde_json::from_value(v).ok().map(|one| vec![one]).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    if rows.is_empty() { return None; }

    let mut out: Vec<SmartHealthPayload> = Vec::new();
    for r in rows.drain(..) {
        let device = r
            .friendly_name
            .or(r.unique_id)
            .or(r.serial_number)
            .or_else(|| Some("NVMe".to_string()));
        out.push(SmartHealthPayload {
            device,
            predict_fail: None,
            temp_c: r.temperature.map(|t| t as f32),
            power_on_hours: r.power_on_hours.and_then(|v| i32::try_from(v).ok()),
            reallocated: None,
            pending: None,
            uncorrectable: None,
            crc_err: None,
            power_cycles: r.power_cycle_count.and_then(|v| i32::try_from(v).ok()),
            host_reads_bytes: r.read_bytes.and_then(|v| i64::try_from(v).ok()),
            host_writes_bytes: r.write_bytes.and_then(|v| i64::try_from(v).ok()),
            nvme_percentage_used_pct: None,
            nvme_available_spare_pct: None,
            nvme_available_spare_threshold_pct: None,
            nvme_media_errors: None,
        });
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(not(windows))]
pub fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> { None }
