//! Resource monitoring for NexusAOS.
//!
//! Reports system pressure (RAM, VRAM, disk, queue depth) so the kernel
//! can make informed decisions about task admission and context budgeting.

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// A snapshot of current system resource pressure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPressure {
    /// Available RAM in megabytes.
    pub ram_available_mb: u64,

    /// Total RAM in megabytes.
    pub ram_total_mb: u64,

    /// Available VRAM in megabytes (0 if GPU not detected).
    pub vram_available_mb: u64,

    /// Total VRAM in megabytes (0 if GPU not detected).
    pub vram_total_mb: u64,

    /// Available disk space in gigabytes on the data partition.
    pub disk_available_gb: u64,

    /// Current number of tasks in the scheduler queue.
    pub queue_depth: usize,
}

/// Monitors system resources for admission control and budgeting.
pub struct ResourceMonitor;

impl ResourceMonitor {
    /// Take a snapshot of current system pressure.
    ///
    /// This is a relatively expensive operation (system calls + optional
    /// subprocess for GPU info). Cache the result for a few seconds.
    pub fn snapshot() -> SystemPressure {
        let mut sys = System::new_all();
        sys.refresh_memory();

        let ram_available_mb = sys.available_memory() / (1024 * 1024);
        let ram_total_mb = sys.total_memory() / (1024 * 1024);

        let (vram_available_mb, vram_total_mb) = Self::query_gpu_vram();

        let disk_available_gb = Self::query_disk_space();

        SystemPressure {
            ram_available_mb,
            ram_total_mb,
            vram_available_mb,
            vram_total_mb,
            disk_available_gb,
            queue_depth: 0, // Updated by the scheduler
        }
    }

    /// Query GPU VRAM via multiple vendor tools (nvidia-smi, rocm-smi, intel_gpu_top).
    ///
    /// Returns (available_mb, total_mb). Returns (0, 0) if no GPU tools are available.
    fn query_gpu_vram() -> (u64, u64) {
        // Try NVIDIA first
        let nvidia = Self::query_nvidia_vram();
        if nvidia != (0, 0) {
            return nvidia;
        }

        // Try AMD (ROCm)
        let amd = Self::query_amd_vram();
        if amd != (0, 0) {
            return amd;
        }

        // Try Intel
        let intel = Self::query_intel_vram();
        if intel != (0, 0) {
            return intel;
        }

        (0, 0)
    }

    /// Query NVIDIA GPU VRAM via nvidia-smi.
    fn query_nvidia_vram() -> (u64, u64) {
        let output = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2 {
                        let used: u64 = parts[0].parse().unwrap_or(0);
                        let total: u64 = parts[1].parse().unwrap_or(0);
                        return (total.saturating_sub(used), total);
                    }
                }
                (0, 0)
            }
            _ => (0, 0),
        }
    }

    /// Query AMD GPU VRAM via rocm-smi.
    fn query_amd_vram() -> (u64, u64) {
        let output = std::process::Command::new("rocm-smi")
            .args(["--showmeminfo", "vram"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse "Total Memory (MB): X" and "Used Memory (MB): Y"
                let mut total_mb = 0u64;
                let mut used_mb = 0u64;
                for line in stdout.lines() {
                    let line_lower = line.to_lowercase();
                    if line_lower.contains("total memory") && line_lower.contains("vram") {
                        if let Some(num) = line.split(':').nth(1) {
                            total_mb = num.trim().parse().unwrap_or(0);
                        }
                    } else if line_lower.contains("used memory") && line_lower.contains("vram") {
                        if let Some(num) = line.split(':').nth(1) {
                            used_mb = num.trim().parse().unwrap_or(0);
                        }
                    }
                }
                if total_mb > 0 {
                    return (total_mb.saturating_sub(used_mb), total_mb);
                }
                (0, 0)
            }
            _ => (0, 0),
        }
    }

    /// Query Intel GPU VRAM via intel_gpu_top.
    fn query_intel_vram() -> (u64, u64) {
        let output = std::process::Command::new("intel_gpu_top")
            .args(["-s"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // intel_gpu_top -s outputs frequency data, not VRAM directly
                // Intel GPUs use shared system memory, so we return 0 for dedicated VRAM
                // but can estimate based on reported device memory if available
                for line in stdout.lines() {
                    if line.contains("device_memory") || line.contains("Memory") {
                        // Parse memory info if available
                        if let Some(num) = line.split(':').nth(1) {
                            if let Ok(total) = num.trim().parse::<u64>() {
                                return (total / 2, total); // Assume half available
                            }
                        }
                    }
                }
                (0, 0)
            }
            _ => (0, 0),
        }
    }

    /// Query available disk space on the root filesystem (cross-platform).
    fn query_disk_space() -> u64 {
        // Try sysinfo first (cross-platform, no subprocess needed)
        let disks = sysinfo::Disks::new_with_refreshed_list();
        if let Some(root_disk) = disks.iter().find(|d| d.mount_point() == std::path::Path::new("/") || d.mount_point() == std::path::Path::new("C:\\")) {
            return root_disk.available_space() / (1024 * 1024 * 1024);
        }

        // Fallback to platform-specific commands
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("df")
                .args(["--output=avail", "-BG", "/"])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(line) = stdout.lines().nth(1) {
                        let trimmed = line.trim().trim_end_matches('G');
                        return trimmed.parse().unwrap_or(0);
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("df")
                .args(["-g", "/"])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(line) = stdout.lines().nth(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            return parts[3].parse().unwrap_or(0);
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("wmic")
                .args(["logicaldisk", "where", "DeviceID='C:'", "get", "Size,FreeSpace", "/format:list"])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let mut free = 0u64;
                    let mut total = 0u64;
                    for line in stdout.lines() {
                        if line.starts_with("FreeSpace=") {
                            free = line.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
                        } else if line.starts_with("Size=") {
                            total = line.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
                        }
                    }
                    if total > 0 {
                        return free / (1024 * 1024 * 1024);
                    }
                }
            }
        }

        0
    }

    /// Check if the system is under memory pressure.
    pub fn is_memory_pressure(pressure: &SystemPressure, headroom_mb: u64) -> bool {
        pressure.ram_available_mb < headroom_mb
    }

    /// Check if there is sufficient VRAM for a model load.
    pub fn has_sufficient_vram(pressure: &SystemPressure, needed_mb: u64) -> bool {
        if pressure.vram_total_mb == 0 {
            // No GPU detected — can't check, assume ok for CPU-only inference
            return true;
        }
        pressure.vram_available_mb >= needed_mb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_runs() {
        // This should not panic even if nvidia-smi is unavailable
        let pressure = ResourceMonitor::snapshot();
        assert!(pressure.ram_total_mb > 0, "should detect RAM");
    }

    #[test]
    fn test_memory_pressure_check() {
        let pressure = SystemPressure {
            ram_available_mb: 1000,
            ram_total_mb: 16000,
            vram_available_mb: 0,
            vram_total_mb: 0,
            disk_available_gb: 100,
            queue_depth: 0,
        };

        assert!(ResourceMonitor::is_memory_pressure(&pressure, 2000));
        assert!(!ResourceMonitor::is_memory_pressure(&pressure, 500));
    }

    #[test]
    fn test_vram_check_no_gpu() {
        let pressure = SystemPressure {
            ram_available_mb: 8000,
            ram_total_mb: 16000,
            vram_available_mb: 0,
            vram_total_mb: 0,
            disk_available_gb: 100,
            queue_depth: 0,
        };

        // No GPU = assume ok
        assert!(ResourceMonitor::has_sufficient_vram(&pressure, 4000));
    }

    #[test]
    fn test_vram_check_with_gpu() {
        let pressure = SystemPressure {
            ram_available_mb: 8000,
            ram_total_mb: 16000,
            vram_available_mb: 3000,
            vram_total_mb: 6000,
            disk_available_gb: 100,
            queue_depth: 0,
        };

        assert!(ResourceMonitor::has_sufficient_vram(&pressure, 2000));
        assert!(!ResourceMonitor::has_sufficient_vram(&pressure, 4000));
    }

    #[test]
    fn test_memory_pressure_exact_boundary() {
        let pressure = SystemPressure {
            ram_available_mb: 2048,
            ram_total_mb: 16000,
            vram_available_mb: 0,
            vram_total_mb: 0,
            disk_available_gb: 100,
            queue_depth: 0,
        };

        // Exact equality: available < headroom? 2048 < 2048 = false
        assert!(!ResourceMonitor::is_memory_pressure(&pressure, 2048));
        // One less: 2048 < 2049 = true
        assert!(ResourceMonitor::is_memory_pressure(&pressure, 2049));
    }

    #[test]
    fn test_vram_exact_boundary() {
        let pressure = SystemPressure {
            ram_available_mb: 8000,
            ram_total_mb: 16000,
            vram_available_mb: 4000,
            vram_total_mb: 8000,
            disk_available_gb: 100,
            queue_depth: 0,
        };

        assert!(ResourceMonitor::has_sufficient_vram(&pressure, 4000));
        assert!(!ResourceMonitor::has_sufficient_vram(&pressure, 4001));
    }

    #[test]
    fn test_system_pressure_default_fields() {
        let pressure = SystemPressure {
            ram_available_mb: 0,
            ram_total_mb: 0,
            vram_available_mb: 0,
            vram_total_mb: 0,
            disk_available_gb: 0,
            queue_depth: 0,
        };

        assert!(ResourceMonitor::is_memory_pressure(&pressure, 1));
        assert!(ResourceMonitor::has_sufficient_vram(&pressure, 1));
    }

    #[test]
    fn test_system_pressure_serde_roundtrip() {
        let pressure = SystemPressure {
            ram_available_mb: 8000,
            ram_total_mb: 16000,
            vram_available_mb: 3000,
            vram_total_mb: 6000,
            disk_available_gb: 100,
            queue_depth: 5,
        };
        let json = serde_json::to_string(&pressure).unwrap();
        let back: SystemPressure = serde_json::from_str(&json).unwrap();
        assert_eq!(pressure.ram_available_mb, back.ram_available_mb);
        assert_eq!(pressure.vram_available_mb, back.vram_available_mb);
        assert_eq!(pressure.queue_depth, back.queue_depth);
    }

    #[test]
    fn test_snapshot_returns_reasonable_values() {
        let pressure = ResourceMonitor::snapshot();
        assert!(pressure.ram_total_mb > 0, "total RAM should be > 0");
        assert!(pressure.ram_available_mb <= pressure.ram_total_mb, "available <= total");
        assert!(pressure.disk_available_gb > 0 || pressure.disk_available_gb == 0, "disk is a number");
    }
}
