//! kaos-driver library
//!
//! Provides high-performance I/O backends for kaos.

pub mod xdp;

#[cfg(all(target_os = "linux", feature = "uring"))]
pub mod uring;
