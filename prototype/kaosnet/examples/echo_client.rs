//! Echo client example.
//!
//! Run with: cargo run --example echo_client

use std::net::SocketAddr;
use std::time::Duration;

use kaos_rudp::{RudpTransport, Transport};
use kaosnet::protocol::{Message, Op};

fn main() -> std::io::Result<()> {
    let bind_addr: SocketAddr = "127.0.0.1:7351".parse().unwrap();
    let server_addr: SocketAddr = "127.0.0.1:7350".parse().unwrap();

    println!("Connecting to server at {}", server_addr);

    let mut transport = RudpTransport::new(bind_addr, server_addr, 1024)?;

    // Send a few messages
    for i in 0..5 {
        let msg = Message::new(Op::RoomData, format!("Hello {}", i).into_bytes());
        let data = msg.encode();
        transport.send(&data)?;
        println!("Sent message {}", i);
        std::thread::sleep(Duration::from_millis(100));
    }

    // Receive responses
    for _ in 0..10 {
        transport.receive(|data| {
            if let Some((msg, _)) = Message::decode(data) {
                println!("Received: op={:?}, payload={:?}",
                    msg.op,
                    String::from_utf8_lossy(&msg.payload)
                );
            }
        });
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
