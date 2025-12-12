//! io_uring async I/O driver (Linux 5.6+)
#![cfg(all(target_os = "linux", feature = "uring"))]

use io_uring::{opcode, types, IoUring};
use std::io;
use std::net::UdpSocket;
use std::os::fd::AsRawFd;

/// io_uring submission queue depth (256 = good balance of latency/throughput)
const QUEUE_DEPTH: u32 = 256;

/// Number of receive buffers to keep queued
const RECV_BUFS: usize = 64;

/// User data offset to distinguish recv completions from sends
const RECV_USER_DATA_BASE: u64 = 0x1000;

pub struct UringDriver {
    ring: IoUring,
    fd: i32,
    bufs: Vec<[u8; 8]>,
    pending: usize,
}

impl UringDriver {
    pub fn new(socket: &UdpSocket) -> io::Result<Self> {
        Ok(Self {
            ring: IoUring::builder().setup_sqpoll(1000).build(QUEUE_DEPTH)?,
            fd: socket.as_raw_fd(),
            bufs: vec![[0u8; 8]; RECV_BUFS],
            pending: 0,
        })
    }

    pub fn submit_sends(&mut self, data: &[[u8; 8]]) -> io::Result<usize> {
        let mut sq = self.ring.submission();
        let mut n = 0;
        for (i, buf) in data.iter().enumerate() {
            let e = opcode::Send::new(types::Fd(self.fd), buf.as_ptr(), buf.len() as u32)
                .build()
                .user_data(i as u64);
            // SAFETY: Entry is valid for the lifetime of self.fd
            if unsafe { sq.push(&e) }.is_err() {
                break;
            }
            n += 1;
        }
        drop(sq);
        self.ring.submit()?;
        Ok(n)
    }

    pub fn queue_recvs(&mut self) -> io::Result<usize> {
        let mut sq = self.ring.submission();
        let mut n = 0;
        while self.pending < RECV_BUFS {
            let e = opcode::Recv::new(types::Fd(self.fd), self.bufs[self.pending].as_mut_ptr(), 8)
                .build()
                .user_data(RECV_USER_DATA_BASE + (self.pending as u64));
            // SAFETY: Entry is valid for the lifetime of self.bufs
            if unsafe { sq.push(&e) }.is_err() {
                break;
            }
            self.pending += 1;
            n += 1;
        }
        drop(sq);
        if n > 0 {
            self.ring.submit()?;
        }
        Ok(n)
    }

    pub fn poll_completions<F: FnMut(u64)>(&mut self, mut on_recv: F) -> usize {
        let mut count = 0;
        for cqe in &mut self.ring.completion() {
            let ud = cqe.user_data();
            if ud >= RECV_USER_DATA_BASE {
                if cqe.result() >= 8 {
                    on_recv(u64::from_le_bytes(
                        self.bufs[(ud - RECV_USER_DATA_BASE) as usize],
                    ));
                }
                self.pending = self.pending.saturating_sub(1);
            }
            if cqe.result() > 0 {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uring_driver_create() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let driver = UringDriver::new(&socket);
        
        // io_uring may fail if kernel doesn't support SQPOLL (needs root)
        // or if kernel version < 5.6
        match driver {
            Ok(d) => {
                println!("io_uring: driver created, fd={}", d.fd);
                assert!(d.pending == 0);
            }
            Err(e) => {
                // Expected on kernels without SQPOLL support or in containers
                println!("io_uring: not available - {:?}", e);
            }
        }
    }

    #[test]
    fn test_uring_send() {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();
        
        sender.connect(recv_addr).unwrap();
        receiver.set_nonblocking(true).unwrap();

        let mut driver = match UringDriver::new(&sender) {
            Ok(d) => d,
            Err(_) => {
                println!("io_uring: skipping test (not available)");
                return;
            }
        };

        // Submit sends
        let data: Vec<[u8; 8]> = (0..10).map(|i| [i as u8; 8]).collect();
        let submitted = driver.submit_sends(&data).unwrap();
        
        assert!(submitted > 0, "Should submit at least one send");
        println!("io_uring: submitted {} sends", submitted);

        // Wait for completions
        std::thread::sleep(std::time::Duration::from_millis(50));
        let completed = driver.poll_completions(|_| {});
        println!("io_uring: {} completions", completed);
    }
}
