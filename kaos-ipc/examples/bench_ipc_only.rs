//! Pure IPC benchmark - no UDP, just measure IPC round-trip
use kaos_ipc::{Publisher, Subscriber};
use std::fs;
use std::thread;
use std::time::Instant;

fn main() {
    let send_path = "/tmp/kaos-ipc-bench-send";
    let recv_path = "/tmp/kaos-ipc-bench-recv";
    let _ = fs::remove_file(send_path);
    let _ = fs::remove_file(recv_path);
    
    const N: u64 = 500_000;
    
    // Simulate driver: read from send, write to recv (no UDP)
    let driver = thread::spawn(move || {
        let mut from_app = loop {
            if let Ok(s) = Subscriber::open(send_path) { break s; }
            thread::yield_now();
        };
        let mut to_app = Publisher::create(recv_path, 64 * 1024).unwrap();
        
        let mut count = 0u64;
        while count < N {
            while let Some(v) = from_app.try_receive() {
                let _ = to_app.send(v);
                count += 1;
            }
        }
        count
    });
    
    thread::sleep(std::time::Duration::from_millis(100));
    
    let mut to_driver = Publisher::create(send_path, 64 * 1024).unwrap();
    let mut from_driver = loop {
        if let Ok(s) = Subscriber::open(recv_path) { break s; }
        thread::yield_now();
    };
    
    let start = Instant::now();
    let mut sent = 0u64;
    let mut received = 0u64;
    
    while received < N {
        while sent < N {
            if to_driver.send(sent).is_ok() {
                sent += 1;
            } else {
                break;
            }
        }
        while let Some(_) = from_driver.try_receive() {
            received += 1;
        }
    }
    
    let elapsed = start.elapsed();
    driver.join().unwrap();
    
    println!("IPC-only round-trip (no UDP):");
    println!("  Messages: {}", N);
    println!("  Time: {:?}", elapsed);
    println!("  Throughput: {:.2} M/s", N as f64 / elapsed.as_secs_f64() / 1_000_000.0);
    
    let _ = fs::remove_file(send_path);
    let _ = fs::remove_file(recv_path);
}
