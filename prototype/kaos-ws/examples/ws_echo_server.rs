//! WebSocket echo server example.
//!
//! Run with: cargo run --example ws_echo_server
//! Test with: websocat ws://127.0.0.1:9000

use std::time::Duration;

use kaos_ws::{Transport, WsServer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:9000";
    println!("WebSocket echo server listening on ws://{}", addr);

    let mut server = WsServer::bind(addr)?;
    let mut clients = Vec::new();

    loop {
        // Accept new connections
        match server.accept() {
            Ok(Some(client)) => {
                println!("Client connected");
                clients.push(client);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }

        // Process existing clients
        clients.retain_mut(|client| {
            if !client.is_open() {
                println!("Client disconnected");
                return false;
            }

            let mut messages = Vec::new();
            client.receive(|data| {
                messages.push(data.to_vec());
            });

            for msg in messages {
                println!("Received {} bytes, echoing...", msg.len());
                if client.send(&msg).is_err() {
                    return false;
                }
            }

            true
        });

        std::thread::sleep(Duration::from_millis(10));
    }
}
