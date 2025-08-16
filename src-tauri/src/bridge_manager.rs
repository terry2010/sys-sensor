// 桥接进程管理模块
// 包含sensor-bridge进程的启动、监控和数据接收功能

use crate::types::BridgeOut;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::Instant as StdInstant;
use std::path::PathBuf;

/// 启动桥接进程管理线程
pub fn start_bridge_manager(
    bridge_data: Arc<Mutex<(Option<BridgeOut>, StdInstant)>>,
    packaged_bridge_exe: Option<PathBuf>,
    shutdown_flag: Arc<AtomicBool>,
    bridge_pid: Arc<Mutex<Option<u32>>>,
) {
    std::thread::spawn(move || {
        // Resolve project root by walking up until we find `sensor-bridge/sensor-bridge.csproj`
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
        let mut cursor = exe_dir.clone();
        let mut found_root: Option<PathBuf> = None;
        for _ in 0..6 {
            if let Some(dir) = cursor {
                let probe = dir.join("sensor-bridge").join("sensor-bridge.csproj");
                if probe.exists() {
                    found_root = Some(dir.clone());
                    break;
                }
                cursor = dir.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
        let project_root = found_root
            .or_else(|| exe_dir.and_then(|p| p.parent().map(|p| p.to_path_buf())))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        eprintln!("[bridge] Using project_root: {}", project_root.display());

        // 便携版额外兜底：exe 同目录/resources/sensor-bridge/sensor-bridge.exe
        let portable_bridge_exe: Option<PathBuf> = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("resources").join("sensor-bridge").join("sensor-bridge.exe")));

        loop {
            if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) { break; }
            
            // 0) 若存在打包资源中的自包含 EXE，优先直接启动
            if let Some(ref p) = packaged_bridge_exe {
                if p.exists() {
                    if try_spawn_bridge_exe(p, &project_root, &bridge_data, &bridge_pid) {
                        continue;
                    }
                }
            }
            
            // 0b) 便携版兜底：尝试 exe 同目录下的 resources 路径
            if let Some(ref p) = portable_bridge_exe {
                if p.exists() {
                    if try_spawn_bridge_exe(p, &project_root, &bridge_data, &bridge_pid) {
                        continue;
                    }
                }
            }
            
            // 尝试开发模式路径
            if try_spawn_dev_bridge(&project_root, &bridge_data, &bridge_pid) {
                continue;
            }
            
            eprintln!("[bridge] Failed to spawn sensor-bridge process, retry in 3s.");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}

/// 尝试启动打包的桥接可执行文件
fn try_spawn_bridge_exe(
    exe_path: &PathBuf,
    project_root: &PathBuf,
    bridge_data: &Arc<Mutex<(Option<BridgeOut>, StdInstant)>>,
    bridge_pid: &Arc<Mutex<Option<u32>>>,
) -> bool {
    eprintln!("[bridge] spawning packaged exe: {}", exe_path.display());
    let mut cmd = Command::new(exe_path);
    cmd.current_dir(exe_path.parent().unwrap_or(project_root));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    if let Ok(mut child_proc) = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        if let Ok(mut g) = bridge_pid.lock() { *g = Some(child_proc.id()); }
        
        if let Some(stdout) = child_proc.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                if line.trim().is_empty() { continue; }
                if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                    if let Ok(mut guard) = bridge_data.lock() {
                        *guard = (Some(parsed), StdInstant::now());
                    }
                } else {
                    eprintln!("[bridge] Non-JSON line: {}", line);
                }
            }
        }
        
        if let Some(stderr) = child_proc.stderr.take() {
            std::thread::spawn(move || {
                let rdr = BufReader::new(stderr);
                for line in rdr.lines().flatten() {
                    if line.trim().is_empty() { continue; }
                    eprintln!("[bridge][stderr] {}", line);
                }
            });
        }
        
        let _ = child_proc.wait();
        if let Ok(mut g) = bridge_pid.lock() { *g = None; }
        eprintln!("[bridge] packaged bridge exited, respawn in 3s...");
        std::thread::sleep(std::time::Duration::from_secs(3));
        true
    } else {
        eprintln!("[bridge] Failed to spawn packaged sensor-bridge.exe, fallback to dev paths in 3s...");
        std::thread::sleep(std::time::Duration::from_secs(3));
        false
    }
}

/// 尝试启动开发模式的桥接进程
fn try_spawn_dev_bridge(
    project_root: &PathBuf,
    bridge_data: &Arc<Mutex<(Option<BridgeOut>, StdInstant)>>,
    bridge_pid: &Arc<Mutex<Option<u32>>>,
) -> bool {
    let dll_candidates = [
        project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.dll"),
        project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.dll"),
    ];
    let exe_candidates = [
        project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.exe"),
        project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.exe"),
    ];

    // 1) 优先使用 dll: dotnet <dll>
    let mut child = if let Some(dll) = dll_candidates.iter().find(|p| p.exists()) {
        eprintln!("[bridge] spawning via dotnet: {}", dll.display());
        let mut cmd = Command::new("dotnet");
        cmd.arg(dll).current_dir(project_root.clone());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()
    // 2) 其次尝试 exe 直接启动
    } else if let Some(exe) = exe_candidates.iter().find(|p| p.exists()) {
        eprintln!("[bridge] spawning exe: {}", exe.display());
        let mut cmd = Command::new(exe);
        cmd.current_dir(project_root.clone());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()
    } else {
        // 3) 最后 fallback 到 dotnet run
        eprintln!("[bridge] fallback to 'dotnet run --project sensor-bridge'");
        let mut cmd = Command::new("dotnet");
        cmd.args(["run", "--project", "sensor-bridge"]).current_dir(project_root.clone());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()
    };

    if let Some(ref mut child_proc) = child {
        if let Ok(mut g) = bridge_pid.lock() { *g = Some(child_proc.id()); }
        
        if let Some(stdout) = child_proc.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                if line.trim().is_empty() { continue; }
                if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                    if let Ok(mut guard) = bridge_data.lock() {
                        *guard = (Some(parsed), StdInstant::now());
                    }
                } else {
                    eprintln!("[bridge] Non-JSON line: {}", line);
                }
            }
        }
        
        // Drain and print stderr if available for diagnostics
        if let Some(stderr) = child_proc.stderr.take() {
            std::thread::spawn(move || {
                let rdr = BufReader::new(stderr);
                for line in rdr.lines().flatten() {
                    if line.trim().is_empty() { continue; }
                    eprintln!("[bridge][stderr] {}", line);
                }
            });
        }
        
        // Wait child and then respawn
        let _ = child_proc.wait();
        if let Ok(mut g) = bridge_pid.lock() { *g = None; }
        eprintln!("[bridge] bridge process exited, will respawn in 3s...");
        std::thread::sleep(std::time::Duration::from_secs(3));
        true
    } else {
        false
    }
}
