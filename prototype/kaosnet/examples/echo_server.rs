//! Echo server example.
//!
//! Run with: cargo run --example echo_server

use std::net::SocketAddr;
use std::time::Duration;

use kaos_rudp::{RudpTransport, Transport};
use kaosnet::protocol::{Message, Op};

fn main() -> std::io::Result<()> {
    let bind_addr: SocketAddr = "127.0.0.1:7350".parse().unwrap();
    let dummy_remote: SocketAddr = "127.0.0.1:0".parse().unwrap();

    println!("Starting echo server on {}", bind_addr);

    let mut transport = RudpTransport::new(bind_addr, dummy_remote, 1024)?;

    loop {
        transport.receive(|data| {
            if let Some((msg, _)) = Message::decode(data) {
                println!("Received: op={:?}, payload_len={}", msg.op, msg.payload.len());

                // Echo back
                match msg.op {
                    Op::Heartbeat => {
                        println!("  -> Heartbeat");
                    }
                    Op::RoomData => {
                        println!("  -> Room data, echoing...");
                        // Would send back here
                    }
                    _ => {}
                }
            }
        });

        std::thread::sleep(Duration::from_millis(10));
    }
}
