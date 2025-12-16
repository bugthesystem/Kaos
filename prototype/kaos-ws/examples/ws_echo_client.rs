//! WebSocket echo client example.
//!
//! Run with: cargo run --example ws_echo_client

use std::time::Duration;

use kaos_ws::{Transport, WsTransport};

fn main() -> std::io::Result<()> {
    let url = "ws://127.0.0.1:9000";
    println!("Connecting to {}...", url);

    let mut client = WsTransport::connect(url)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    println!("Connected!");

    // Send a few messages
    for i in 0..5 {
        let msg = format!("Hello {}", i);
        println!("Sending: {}", msg);
        client.send(msg.as_bytes())?;
        std::thread::sleep(Duration::from_millis(100));

        // Check for response
        client.receive(|data| {
            println!("Received: {}", String::from_utf8_lossy(data));
        });
    }

    // Send binary data
    let binary: Vec<u8> = (0..32).collect();
    println!("Sending {} bytes of binary data", binary.len());
    client.send(&binary)?;

    std::thread::sleep(Duration::from_millis(100));
    client.receive(|data| {
        println!("Received {} bytes back", data.len());
    });

    client.close()?;
    println!("Connection closed");

    Ok(())
}
