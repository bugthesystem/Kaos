//! Composable transport trait
//!
//! Enables layered transports: `Archived<Reliable<Udp>>`, `Driver<Reliable<Udp>>`, etc.

use std::io;

/// Core transport trait - all transports implement this
pub trait Transport {
    /// Send data, returns sequence number
    fn send(&mut self, data: &[u8]) -> io::Result<u64>;

    /// Receive into callback, returns count received
    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize;

    /// Flush pending operations
    fn flush(&mut self) {}
}

/// Batch sending (optional optimization)
pub trait BatchTransport: Transport {
    fn send_batch(&mut self, data: &[&[u8]]) -> io::Result<usize>;
}

/// Reliability layer (NAK/ACK retransmission)
pub trait Reliable: Transport {
    fn retransmit_pending(&mut self) -> io::Result<usize>;
    fn acked_sequence(&self) -> u64;
}

/// Archiving layer (disk persistence)
pub trait Archived: Transport {
    fn archived_count(&self) -> u64;
    fn wait_for_archive(&self);
}

