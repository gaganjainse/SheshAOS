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

    /// Query GPU VRAM via nvidia-smi.
    ///
    /// Returns (available_mb, total_mb). Returns (0, 0) if nvidia-smi
    /// is not available or fails.
    fn query_gpu_vram() -> (u64, u64) {
        let output = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse "used, total" from the first line
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

    /// Query available disk space on the root filesystem.
    fn query_disk_space() -> u64 {
        let output = std::process::Command::new("df").args(["--output=avail", "-BG", "/"]).output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Skip header line, parse the number
                if let Some(line) = stdout.lines().nth(1) {
                    let trimmed = line.trim().trim_end_matches('G');
                    return trimmed.parse().unwrap_or(0);
                }
                0
            }
            _ => 0,
        }
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
}
