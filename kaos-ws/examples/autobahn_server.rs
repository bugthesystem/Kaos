//! Autobahn testsuite server.
//!
//! This server is designed to pass the Autobahn WebSocket testsuite.
//!
//! Run with: cargo run --example autobahn_server --release
//!
//! Then run Autobahn:
//! ```bash
//! docker run -it --rm \
//!   -v "${PWD}/autobahn:/config" \
//!   -v "${PWD}/autobahn/reports:/reports" \
//!   --network host \
//!   crossbario/autobahn-testsuite \
//!   wstest -m fuzzingclient -s /config/fuzzingclient.json
//! ```

use std::net::TcpListener;

use tungstenite::accept;
use tungstenite::Message;

fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:9001";
    println!("Autobahn test server listening on ws://{}", addr);
    println!();
    println!("Run Autobahn testsuite with:");
    println!("  docker run -it --rm \\");
    println!("    -v \"${{PWD}}/autobahn:/config\" \\");
    println!("    -v \"${{PWD}}/autobahn/reports:/reports\" \\");
    println!("    --network host \\");
    println!("    crossbario/autobahn-testsuite \\");
    println!("    wstest -m fuzzingclient -s /config/fuzzingclient.json");
    println!();

    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(stream: std::net::TcpStream) {
    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Handshake error: {}", e);
            return;
        }
    };

    loop {
        match ws.read() {
            Ok(msg) => {
                match msg {
                    Message::Text(text) => {
                        if ws.send(Message::Text(text)).is_err() {
                            break;
                        }
                    }
                    Message::Binary(data) => {
                        if ws.send(Message::Binary(data)).is_err() {
                            break;
                        }
                    }
                    Message::Ping(data) => {
                        if ws.send(Message::Pong(data)).is_err() {
                            break;
                        }
                    }
                    Message::Pong(_) => {}
                    Message::Close(frame) => {
                        let _ = ws.close(frame);
                        break;
                    }
                    Message::Frame(_) => {}
                }
            }
            Err(tungstenite::Error::ConnectionClosed) => {
                break;
            }
            Err(tungstenite::Error::AlreadyClosed) => {
                break;
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}
