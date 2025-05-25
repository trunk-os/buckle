#![allow(dead_code)]
use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Info {
    uptime: u64,            // in seconds
    available_memory: u64,  // bytes
    total_memory: u64,      // bytes
    cpus: usize,            // count of cpus
    cpu_usage: f32,         // percentage
    host_name: String,      // short name
    kernel_version: String, // only the version string
    load_average: [f64; 3], // 1, 5, 15 min
    processes: usize,       // just the count
    total_disk: usize,      // bytes
    available_disk: usize,  // bytes
}

impl Default for Info {
    fn default() -> Self {
        let s = System::new_all();
        let la = sysinfo::System::load_average();
        let la = [la.one, la.five, la.fifteen];

        Self {
            uptime: sysinfo::System::uptime(),
            available_memory: s.available_memory(),
            total_memory: s.total_memory(),
            cpus: s.cpus().len(),
            cpu_usage: s.global_cpu_usage(),
            host_name: sysinfo::System::host_name().unwrap_or("trunk".into()),
            kernel_version: sysinfo::System::kernel_version().unwrap_or("unknown".into()),
            load_average: la,
            processes: s.processes().len(),
            total_disk: 0,
            available_disk: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn info_defaults() {
        let info = Info::default();
        assert_ne!(info.uptime, 0);
        assert_ne!(info.available_memory, 0);
        assert_ne!(info.total_memory, 0);
        assert_ne!(info.cpus, 0);
        assert_ne!(info.cpu_usage, 0.0);
        assert!(!info.host_name.is_empty());
        assert!(!info.kernel_version.is_empty());
        assert_ne!(info.load_average, [0.0, 0.0, 0.0]);
        assert_ne!(info.processes, 0);
        assert_eq!(info.total_disk, 0);
        assert_eq!(info.available_disk, 0);
    }
}
