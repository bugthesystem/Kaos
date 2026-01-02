//! Thread and NUMA affinity for Linux.
//!
//! ```rust,ignore
//! use kaos::affinity::pin_to_core;
//! pin_to_core(0).unwrap(); // Pin to core 0
//! ```

use std::io;

/// Pin current thread to a specific CPU core.
#[cfg(target_os = "linux")]
pub fn pin_to_core(core_id: usize) -> io::Result<()> {
    use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};

    let mut set: cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe {
        CPU_ZERO(&mut set);
        CPU_SET(core_id, &mut set);

        if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Pin current thread to all cores in a NUMA node.
#[cfg(target_os = "linux")]
pub fn pin_to_numa_node(node: usize) -> io::Result<()> {
    // Read cpus for this node from sysfs
    let path = format!("/sys/devices/system/node/node{}/cpulist", node);
    let cpulist = std::fs::read_to_string(&path)
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "NUMA node not found"))?;

    let cores = parse_cpulist(&cpulist)?;
    if cores.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No CPUs in node",
        ));
    }

    use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};

    let mut set: cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe {
        CPU_ZERO(&mut set);
        for core in cores {
            CPU_SET(core, &mut set);
        }

        if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Get the NUMA node for current thread.
#[cfg(target_os = "linux")]
pub fn current_numa_node() -> io::Result<usize> {
    // Read from /proc/self/numa_maps or use sched_getcpu + lookup
    // Simpler: read current CPU and map to node
    let cpu = unsafe {
        let ret = libc::sched_getcpu();
        if ret < 0 {
            return Ok(0); // Fallback to node 0
        }
        ret as usize
    };

    // Map CPU to NUMA node via sysfs
    for node in 0..16 {
        let path = format!("/sys/devices/system/node/node{}/cpulist", node);
        if let Ok(cpulist) = std::fs::read_to_string(&path) {
            if let Ok(cpus) = parse_cpulist(&cpulist) {
                if cpus.contains(&cpu) {
                    return Ok(node);
                }
            }
        }
    }
    Ok(0) // Default to node 0
}

/// Get number of NUMA nodes on the system.
#[cfg(target_os = "linux")]
pub fn numa_node_count() -> usize {
    let mut count = 0;
    while std::path::Path::new(&format!("/sys/devices/system/node/node{}", count)).exists() {
        count += 1;
    }
    count.max(1)
}

/// Check if NUMA is available.
#[cfg(target_os = "linux")]
pub fn numa_available() -> bool {
    numa_node_count() > 1
}

// Parse "0-3,8-11" format
#[cfg(target_os = "linux")]
fn parse_cpulist(s: &str) -> io::Result<Vec<usize>> {
    let mut result = Vec::new();
    for part in s.trim().split(',') {
        if part.contains('-') {
            let mut iter = part.split('-');
            let start: usize = iter
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad range"))?
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad number"))?;
            let end: usize = iter
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad range"))?
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad number"))?;
            result.extend(start..=end);
        } else if !part.is_empty() {
            let cpu: usize = part
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad number"))?;
            result.push(cpu);
        }
    }
    Ok(result)
}

// Stubs for non-Linux
#[cfg(not(target_os = "linux"))]
pub fn pin_to_core(_core_id: usize) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "Linux only"))
}

#[cfg(not(target_os = "linux"))]
pub fn pin_to_numa_node(_node: usize) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "Linux only"))
}

#[cfg(not(target_os = "linux"))]
pub fn current_numa_node() -> io::Result<usize> {
    Ok(0)
}

#[cfg(not(target_os = "linux"))]
pub fn numa_node_count() -> usize {
    1
}

#[cfg(not(target_os = "linux"))]
pub fn numa_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_cpulist() {
        assert_eq!(parse_cpulist("0-3").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(parse_cpulist("0,2,4").unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_cpulist("0-2,8-10").unwrap(), vec![0, 1, 2, 8, 9, 10]);
    }

    #[test]
    fn test_numa_available() {
        // Just check it doesn't panic
        let _ = numa_available();
        let _ = numa_node_count();
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_current_node() {
        // Should return a valid node
        let node = current_numa_node().unwrap();
        assert!(node < 256); // Sanity check
    }
}
