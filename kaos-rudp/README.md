# kaos-rudp

Reliable UDP transport using kaos ring buffers.

## Usage

```rust
use kaos_rudp::RudpTransport;

let mut transport = RudpTransport::new(
    "127.0.0.1:9000".parse()?,
    "127.0.0.1:9001".parse()?,
    65536
)?;

// Send
transport.send(b"hello")?;

// Receive
transport.receive_batch_with(64, |msg| {
    println!("{} bytes", msg.len());
});

// Process ACKs/NAKs
transport.process_acks();
```

## Protocol

NAK-based reliable delivery with AIMD congestion control.

| Feature | Status |
|---------|--------|
| Sequence numbers | ✅ |
| NAK retransmission | ✅ |
| NAK backoff (per RTT) | ✅ |
| Retransmit pacing | ✅ |
| Sliding window | ✅ |
| Congestion control (AIMD) | ✅ |
| RTT measurement | ✅ |

## Performance

| Benchmark | Kaos RUDP | Aeron UDP |
|-----------|-----------|-----------|
| Throughput | **3.7 M/s** | 2.6 M/s |
| Delivery | 100% | 100% |

Same conditions: 500K messages, 8 bytes, localhost, M1 Pro.

```bash
cargo run -p kaos-rudp --release --example rudp_bench
```

## License

MIT OR Apache-2.0
