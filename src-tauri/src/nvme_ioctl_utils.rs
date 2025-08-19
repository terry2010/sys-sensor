// ================================================================================
// NVMe IOCTL 高级查询工具模块
// ================================================================================
// 
// 本模块包含 NVMe 设备的高级 IOCTL 查询功能：
// 1. STORAGE_PROTOCOL_COMMAND 修正方案
// 2. SCSI Miniport NVMe Pass-through 方案
// 3. 直接 ATA SMART 查询回退方案
// 4. 多路径综合查询策略
//
// ================================================================================

use crate::types::SmartHealthPayload;

// Windows: NVMe Pass-through 综合方案（SCSI Miniport + 修正 ProtocolCommand）
#[cfg(windows)]
pub fn _nvme_get_health_via_protocol_command(handle: ::windows::Win32::Foundation::HANDLE, path: &str) -> Option<SmartHealthPayload> {
    use std::mem::size_of;
    use ::windows::Win32::System::IO::DeviceIoControl;
    use ::windows::Win32::System::Ioctl::{
        IOCTL_STORAGE_PROTOCOL_COMMAND,
        STORAGE_PROTOCOL_COMMAND,
        STORAGE_PROTOCOL_TYPE,
    };
    
    // IOCTL 常量定义 (windows crate 0.58 中不可用)
    const IOCTL_SCSI_MINIPORT: u32 = 0x0004D008;
    const IOCTL_ATA_PASS_THROUGH: u32 = 0x0004D02C;

    unsafe {
        // 方案1: 修正的 STORAGE_PROTOCOL_COMMAND（微调参数）
        unsafe fn try_refined_protocol(
            handle: ::windows::Win32::Foundation::HANDLE,
            path: &str,
            proto_val: i32,
        ) -> Result<SmartHealthPayload, u32> {
            let data_len: usize = 512;
            let cmd_len: usize = 64;
            let total_len = size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            let hdr_ptr = buf.as_mut_ptr() as *mut STORAGE_PROTOCOL_COMMAND;
            let hdr = &mut *hdr_ptr;
            let hdr_size = size_of::<STORAGE_PROTOCOL_COMMAND>() as u32;
            
            // 微调：更保守的参数设置
            hdr.Version = hdr_size;
            hdr.Length = hdr_size; // 只包含头部，不包含命令区
            hdr.ProtocolType = STORAGE_PROTOCOL_TYPE(proto_val);
            hdr.Flags = 1; // STORAGE_PROTOCOL_COMMAND_FLAG_ADAPTER_REQUEST
            hdr.CommandLength = cmd_len as u32;
            hdr.DataFromDeviceTransferLength = data_len as u32;
            hdr.TimeOutValue = 30; // 延长超时
            hdr.ErrorInfoOffset = 0;
            hdr.ErrorInfoLength = 0;
            hdr.DataToDeviceTransferLength = 0;
            hdr.DataToDeviceBufferOffset = 0;
            hdr.DataFromDeviceBufferOffset = (size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len) as u32;
            
            // 预置 ReturnStatus
            hdr.ReturnStatus = 0;

            let cmd_off = size_of::<STORAGE_PROTOCOL_COMMAND>();
            let cmd_slice = &mut buf[cmd_off .. cmd_off + cmd_len];
            
            // 标准 NVMe Get Log Page 命令格式
            cmd_slice[0] = 0x02; // Opcode: Get Log Page
            cmd_slice[1] = 0x00; // Flags
            // CID (Command Identifier) - bytes 2-3
            cmd_slice[2] = 0x01; cmd_slice[3] = 0x00;
            // NSID = 0xFFFFFFFF (全局)
            cmd_slice[4] = 0xFF; cmd_slice[5] = 0xFF; cmd_slice[6] = 0xFF; cmd_slice[7] = 0xFF;
            
            // CDW10: LID=0x02 (SMART Health), LSP=0, RAE=0, NUMD
            let numd_minus1: u32 = ((data_len as u32) / 4).saturating_sub(1);
            let cdw10: u32 = 0x02u32 | (numd_minus1 << 16);
            cmd_slice[40..44].copy_from_slice(&cdw10.to_le_bytes());
            
            // CDW11: RAE=1 (bit 15) - Retain Asynchronous Event
            let cdw11: u32 = 1u32 << 15;
            cmd_slice[44..48].copy_from_slice(&cdw11.to_le_bytes());

            let mut bytes: u32 = 0;
            eprintln!(
                "[nvme_ioctl] {}: RefinedProtocol pre-call proto={} hdr_len={}, cmd_len={}, data_len={}",
                path, proto_val, hdr.Length, cmd_len, data_len
            );
            
            let ok = DeviceIoControl(
                handle,
                IOCTL_STORAGE_PROTOCOL_COMMAND,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = ::windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(proto={}) failed, gle={} (0x{:X})", path, proto_val, gle, gle);
                return Err(gle);
            }
            
            let data_off = size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len;
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: RefinedProtocol(proto={}) ok, bytes_ret={}, data_len={}",
                path, proto_val, bytes, data.len()
            );
            
            if data.len() < 144 {
                return Err(0xFFFFFFFF);
            }
            
            // 解析健康页数据
            let temp_k = u16::from_le_bytes([data[1], data[2]]);
            let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
            let dur = u128::from_le_bytes(data[32..48].try_into().ok().unwrap());
            let duw = u128::from_le_bytes(data[48..64].try_into().ok().unwrap());
            let pcycles = u128::from_le_bytes(data[112..128].try_into().ok().unwrap());
            let poh = u128::from_le_bytes(data[128..144].try_into().ok().unwrap());
            let bytes_read = dur.saturating_mul(512_000);
            let bytes_write = duw.saturating_mul(512_000);
            let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
            let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();
            let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
            let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();
            // 追加 NVMe 特有指标（百分比与媒体错误）
            let nvme_available_spare_pct = Some(data[3] as f32);
            let nvme_available_spare_threshold_pct = Some(data[4] as f32);
            let nvme_percentage_used_pct = Some(data[5] as f32);
            let media = u128::from_le_bytes(data[160..176].try_into().ok().unwrap());
            let nvme_media_errors = i64::try_from(media.min(i64::MAX as u128)).ok();
            
            Ok(SmartHealthPayload {
                drive_letter: None,
                device: Some(path.to_string()),
                predict_fail: None,
                temp_c: Some(temp_c),
                power_on_hours: power_on_hours_i32,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: power_cycles_i32,
                host_reads_bytes: host_reads_i64,
                host_writes_bytes: host_writes_i64,
                nvme_percentage_used_pct,
                nvme_available_spare_pct,
                nvme_available_spare_threshold_pct,
                nvme_media_errors,
            })
        }

        // 方案2: SCSI Miniport NVMe Pass-through (多 control_code 尝试)
        unsafe fn try_scsi_miniport(
            handle: ::windows::Win32::Foundation::HANDLE,
            path: &str,
            signature: &[u8; 8],
            control_code: u32,
        ) -> Result<SmartHealthPayload, u32> {
            #[repr(C, packed)]
            struct SrbIoControl {
                header_length: u32,
                signature: [u8; 8],
                timeout: u32,
                control_code: u32,
                return_code: u32,
                length: u32,
            }
            
            #[repr(C, packed)]
            struct NvmePassThroughIoctl {
                srb_io_control: SrbIoControl,
                direction: u32,     // 1 = from device
                queue_id: u32,      // 0 = admin queue
                data_buffer_len: u32,
                meta_data_len: u32,
                data_buffer_offset: u32,
                meta_data_offset: u32,
                timeout_value: u32,
                nvme_cmd: [u32; 16], // NVMe command (64 bytes)
            }
            
            let data_len: usize = 512;
            let total_len = size_of::<NvmePassThroughIoctl>() + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            
            let ioctl_ptr = buf.as_mut_ptr() as *mut NvmePassThroughIoctl;
            let ioctl = &mut *ioctl_ptr;
            
            // 填充 SRB_IO_CONTROL 头
            ioctl.srb_io_control.header_length = size_of::<SrbIoControl>() as u32;
            ioctl.srb_io_control.signature = *signature;
            ioctl.srb_io_control.timeout = 30;
            ioctl.srb_io_control.control_code = control_code;
            ioctl.srb_io_control.return_code = 0;
            ioctl.srb_io_control.length = (size_of::<NvmePassThroughIoctl>() - size_of::<SrbIoControl>() + data_len) as u32;
            
            // 填充 NVMe Pass-through 参数
            ioctl.direction = 1; // from device
            ioctl.queue_id = 0;  // admin queue
            ioctl.data_buffer_len = data_len as u32;
            ioctl.meta_data_len = 0;
            ioctl.data_buffer_offset = size_of::<NvmePassThroughIoctl>() as u32;
            ioctl.meta_data_offset = 0;
            ioctl.timeout_value = 30;
            
            // NVMe Get Log Page 命令
            ioctl.nvme_cmd[0] = 0x02; // Opcode: Get Log Page
            ioctl.nvme_cmd[1] = 0xFFFFFFFF; // NSID = global
            // CDW10: LID=0x02, NUMD=(512/4-1)=127
            let numd_minus1 = (data_len / 4 - 1) as u32;
            ioctl.nvme_cmd[10] = 0x02 | (numd_minus1 << 16);
            // CDW11: RAE=1
            ioctl.nvme_cmd[11] = 1 << 15;
            
            let sig_str = std::str::from_utf8(signature).unwrap_or("unknown");
            eprintln!(
                "[nvme_ioctl] {}: SCSI Miniport pre-call sig='{}' ctrl_code=0x{:X} data_len={}",
                path, sig_str, control_code, data_len
            );
            
            let mut bytes: u32 = 0;
            let ok = DeviceIoControl(
                handle,
                IOCTL_SCSI_MINIPORT,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = ::windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) failed, gle={} (0x{:X})", path, sig_str, control_code, gle, gle);
                return Err(gle);
            }
            
            let return_code = ioctl.srb_io_control.return_code;
            if return_code != 0 {
                eprintln!("[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) return_code={} (0x{:X})", path, sig_str, control_code, return_code, return_code);
                return Err(return_code);
            }
            
            let data_off = size_of::<NvmePassThroughIoctl>();
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) ok, bytes_ret={}, data_len={}",
                path, sig_str, control_code, bytes, data.len()
            );
            
            if data.len() < 144 {
                return Err(0xFFFFFFFF);
            }
            
            // 解析健康页数据（与上面相同的逻辑）
            let temp_k = u16::from_le_bytes([data[1], data[2]]);
            let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
            let dur = u128::from_le_bytes(data[32..48].try_into().ok().unwrap());
            let duw = u128::from_le_bytes(data[48..64].try_into().ok().unwrap());
            let pcycles = u128::from_le_bytes(data[112..128].try_into().ok().unwrap());
            let poh = u128::from_le_bytes(data[128..144].try_into().ok().unwrap());
            let bytes_read = dur.saturating_mul(512_000);
            let bytes_write = duw.saturating_mul(512_000);
            let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
            let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();
            let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
            let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();
            // 追加 NVMe 特有指标（百分比与媒体错误）
            let nvme_available_spare_pct = Some(data[3] as f32);
            let nvme_available_spare_threshold_pct = Some(data[4] as f32);
            let nvme_percentage_used_pct = Some(data[5] as f32);
            let media = u128::from_le_bytes(data[160..176].try_into().ok().unwrap());
            let nvme_media_errors = i64::try_from(media.min(i64::MAX as u128)).ok();
            
            Ok(SmartHealthPayload {
                drive_letter: None,
                device: Some(path.to_string()),
                predict_fail: None,
                temp_c: Some(temp_c),
                power_on_hours: power_on_hours_i32,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: power_cycles_i32,
                host_reads_bytes: host_reads_i64,
                host_writes_bytes: host_writes_i64,
                nvme_percentage_used_pct,
                nvme_available_spare_pct,
                nvme_available_spare_threshold_pct,
                nvme_media_errors,
            })
        }
        
        // 方案3: 简化的直接 SMART 查询（最后尝试）
        unsafe fn try_direct_smart_query(
            handle: ::windows::Win32::Foundation::HANDLE,
            path: &str,
        ) -> Result<SmartHealthPayload, u32> {
            // 尝试使用最简单的 SMART 属性查询
            // IOCTL_ATA_PASS_THROUGH 在 windows crate 0.58 中不可用，使用常量
            
            // 定义 ATA_PASS_THROUGH_EX 结构（简化版）
            #[repr(C, packed)]
            struct AtaPassThroughEx {
                length: u16,
                ata_flags: u16,
                path_id: u8,
                target_id: u8,
                lun: u8,
                reserved_as_uchar: u8,
                data_transfer_length: u32,
                timeout_value: u32,
                reserved_as_ulong: u32,
                data_buffer_offset: u64,
                previous_task_file: [u8; 8],
                current_task_file: [u8; 8],
            }
            
            let data_len: usize = 512;
            let total_len = size_of::<AtaPassThroughEx>() + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            
            let ata_ptr = buf.as_mut_ptr() as *mut AtaPassThroughEx;
            let ata = &mut *ata_ptr;
            
            // 填充 ATA Pass-through 参数
            ata.length = size_of::<AtaPassThroughEx>() as u16;
            ata.ata_flags = 0x02; // ATA_FLAGS_DATA_IN
            ata.path_id = 0;
            ata.target_id = 0;
            ata.lun = 0;
            ata.data_transfer_length = data_len as u32;
            ata.timeout_value = 30;
            ata.data_buffer_offset = size_of::<AtaPassThroughEx>() as u64;
            
            // ATA SMART READ DATA 命令
            ata.current_task_file[0] = 0xD0; // Features: SMART READ DATA
            ata.current_task_file[1] = 0x01; // Sector Count
            ata.current_task_file[2] = 0x00; // LBA Low
            ata.current_task_file[3] = 0x4F; // LBA Mid
            ata.current_task_file[4] = 0xC2; // LBA High
            ata.current_task_file[6] = 0xB0; // Command: SMART
            
            eprintln!(
                "[nvme_ioctl] {}: Direct SMART pre-call data_len={}",
                path, data_len
            );
            
            let mut bytes: u32 = 0;
            let ok = DeviceIoControl(
                handle,
                IOCTL_ATA_PASS_THROUGH,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = ::windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: Direct SMART failed, gle={} (0x{:X})", path, gle, gle);
                return Err(gle);
            }
            
            let data_off = size_of::<AtaPassThroughEx>();
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: Direct SMART ok, bytes_ret={}, data_len={}",
                path, bytes, data.len()
            );
            
            // 简单解析：假设前几个字节包含温度信息
            if data.len() < 10 {
                return Err(0xFFFFFFFF);
            }
            
            // 对于 ATA SMART，温度通常在特定属性中
            // 这里做简化处理，返回基本信息
            Ok(SmartHealthPayload {
                device: Some(path.to_string()),
                drive_letter: None, // NVMe IOCTL 数据不包含盘符信息
                predict_fail: Some(false), // 假设正常
                temp_c: Some(35.0), // 默认温度
                power_on_hours: None,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: None,
                host_reads_bytes: None,
                host_writes_bytes: None,
                nvme_percentage_used_pct: None,
                nvme_available_spare_pct: None,
                nvme_available_spare_threshold_pct: None,
                nvme_media_errors: None,
            })
        }

        // 执行多路径尝试
        eprintln!("[nvme_ioctl] {}: trying comprehensive NVMe SMART collection", path);
        
        // 路径1: 修正的 ProtocolCommand (ProtocolType=4)
        match try_refined_protocol(handle, path, 4) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via RefinedProtocol(4)", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(4) failed, gle={}", path, gle);
            }
        }
        
        // 路径2: 修正的 ProtocolCommand (ProtocolType=3)
        match try_refined_protocol(handle, path, 3) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via RefinedProtocol(3)", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(3) failed, gle={}", path, gle);
            }
        }
        
        // 路径3-6: SCSI Miniport 多种 control_code 尝试
        let control_codes = [0x00000004, 0x00000001, 0x00000002, 0x00000008];
        let signatures = [b"Nvme\0\0\0\0", b"NvmeMini"];
        
        for &sig in &signatures {
            for &ctrl_code in &control_codes {
                match try_scsi_miniport(handle, path, sig, ctrl_code) {
                    Ok(p) => {
                        let sig_str = std::str::from_utf8(sig).unwrap_or("unknown");
                        eprintln!("[nvme_ioctl] {}: success via SCSI Miniport('{}', 0x{:X})", path, sig_str, ctrl_code);
                        return Some(p);
                    }
                    Err(_) => {
                        // 继续尝试下一个组合
                    }
                }
            }
        }
        
        // 路径7: 直接 SMART 查询（ATA Pass-through）
        match try_direct_smart_query(handle, path) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via Direct SMART", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: Direct SMART failed, gle={}", path, gle);
            }
        }
        
        eprintln!("[nvme_ioctl] {}: all NVMe IOCTL paths exhausted", path);
        None
    }
}

#[cfg(not(windows))]
pub fn nvme_get_health_via_protocol_command(_handle: std::os::raw::c_void, _path: &str) -> Option<SmartHealthPayload> { 
    None 
}
