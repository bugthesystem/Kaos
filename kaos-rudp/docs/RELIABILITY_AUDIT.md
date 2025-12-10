# RUDP Reliability Audit

**Date:** 2024-12-10  
**Status:** INCOMPLETE - Multiple systems not connected

---

## Executive Summary

The kaos-rudp crate claims to be "reliable UDP" but several critical systems are **not connected**. This document tracks each component's actual status.

---

## 1. Congestion Control

### Implementation: `congestion.rs`

| Method | Purpose | Called? | Where? |
|--------|---------|---------|--------|
| `can_send()` | Check if window allows send | ✅ YES | `send()` line 339 |
| `on_send()` | Increment in_flight counter | ✅ YES | `send()` line 394 |
| `on_ack()` | Grow window (AIMD increase) | ❌ **NO** | **NEVER CALLED** |
| `on_loss()` | Shrink window (AIMD decrease) | ✅ YES | NAK handler line 592 |
| `update_rtt()` | Update RTT estimate | ❌ **NO** | **NEVER CALLED** |

### Problem

The congestion window **never grows** because `on_ack()` is never called. After initial slow start, the window stays at `ssthresh` forever.

### Fix Required

```rust
// In receive handling, when we get confirmation of delivery:
fn on_status_message(&mut self, acked_seq: u64) {
    let newly_acked = acked_seq.saturating_sub(self.acked_seq);
    for _ in 0..newly_acked {
        self.congestion.on_ack();
    }
    self.acked_seq = acked_seq;
}
```

---

## 2. NAK (Negative Acknowledgment)

### Implementation: `lib.rs` + `window.rs`

| Feature | Purpose | Implemented? | Issue |
|---------|---------|--------------|-------|
| Gap detection | Find missing sequences | ✅ YES | Works |
| NAK packet send | Request retransmit | ✅ YES | Works |
| NAK delay/backoff | Prevent NAK storm | ❌ **NO** | Immediate send |
| NAK deduplication | Don't send same NAK twice | ⚠️ PARTIAL | Basic |

### Problem: NAK Storm Risk

When packets are lost, ALL receivers send NAKs **immediately** and **simultaneously**. This causes:
1. Network congestion from NAK flood
2. Sender overwhelmed with duplicate NAKs
3. Retransmit flood makes congestion worse

### Aeron's Solution

```java
// FeedbackDelayGenerator.java
public long generateDelay() {
    return baseDelay + random.nextLong(maxDelay - baseDelay);
}
```

### Fix Required

```rust
// Add to BitmapWindow or create NakController
pub struct NakController {
    last_nak_time: Instant,
    backoff_ns: u64,
    rng: SmallRng,
}

impl NakController {
    pub fn should_send_nak(&mut self) -> bool {
        if self.last_nak_time.elapsed().as_nanos() < self.backoff_ns as u128 {
            return false;
        }
        self.last_nak_time = Instant::now();
        // Random backoff: 0-10ms
        self.backoff_ns = self.rng.gen_range(0..10_000_000);
        true
    }
}
```

---

## 3. Retransmission

### Implementation: `lib.rs`

| Feature | Purpose | Implemented? | Issue |
|---------|---------|--------------|-------|
| Retransmit on NAK | Resend lost packet | ✅ YES | Works |
| Retransmit pacing | Limit retransmit rate | ❌ **NO** | Unlimited |
| Retransmit limit | Max retransmits per NAK | ❌ **NO** | Unlimited |
| Retransmit from window | Access old packets | ✅ YES | Works |

### Problem: Retransmit Flood

On receiving a batch NAK for 100 packets, we immediately retransmit all 100. This:
1. Floods the network
2. Ignores congestion window
3. Makes congestion worse

### Aeron's Solution

```java
// RetransmitHandler.java
private final int maxRetransmits; // Default: 16
private int activeRetransmitCount = 0;

public void onNak(...) {
    if (activeRetransmitCount >= maxRetransmits) {
        retransmitOverflowCounter.increment();
        return; // Ignore NAK
    }
    // Queue retransmit with delay
}
```

### Fix Required

```rust
pub struct RetransmitController {
    max_pending: usize,
    pending: VecDeque<(u64, Instant)>, // (seq, scheduled_time)
}

impl RetransmitController {
    pub fn queue_retransmit(&mut self, seq: u64) -> bool {
        if self.pending.len() >= self.max_pending {
            return false; // Drop NAK
        }
        let delay = Duration::from_micros(self.rng.gen_range(0..1000));
        self.pending.push_back((seq, Instant::now() + delay));
        true
    }
    
    pub fn process_pending(&mut self) -> Vec<u64> {
        let now = Instant::now();
        let mut ready = Vec::new();
        while let Some(&(seq, time)) = self.pending.front() {
            if time <= now {
                ready.push(seq);
                self.pending.pop_front();
            } else {
                break;
            }
        }
        ready
    }
}
```

---

## 4. Flow Control

### Implementation: Implicit via congestion window

| Feature | Purpose | Implemented? | Issue |
|---------|---------|--------------|-------|
| Sender window limit | Don't overflow receiver | ✅ YES | `can_send()` |
| Receiver feedback | Tell sender our capacity | ❌ **NO** | No status messages |
| Back-pressure | Block when full | ⚠️ PARTIAL | Returns error |

### Problem: No Receiver Feedback

Sender only knows its own congestion window. It doesn't know:
1. Receiver's buffer space
2. Receiver's processing rate
3. Network path capacity

### Aeron's Solution

Receivers send **Status Messages** containing:
- `receiverWindowLength` - how much they can accept
- `receiverId` - for multi-receiver tracking

### Fix Required (Simplified)

```rust
// Status message: receiver → sender
pub struct StatusMessage {
    receiver_window: u32,
    highest_received: u64,
}

// Sender tracks receiver capacity
fn on_status_message(&mut self, status: StatusMessage) {
    self.receiver_window = status.receiver_window;
    // Limit send rate to min(congestion_window, receiver_window)
}
```

---

## 5. RTT Estimation

### Implementation: `congestion.rs` has `update_rtt()` but...

| Feature | Purpose | Implemented? | Issue |
|---------|---------|--------------|-------|
| RTT measurement | Estimate round-trip time | ❌ **NO** | Never measured |
| RTT smoothing | EWMA of samples | ✅ YES | Code exists |
| Timeout calculation | Retransmit timeout | ❌ **NO** | Not used |

### Problem

RTT is hardcoded to 1ms initial value and **never updated**.

### Fix Required

```rust
// On send, record timestamp
fn send(&mut self, data: &[u8]) -> io::Result<u64> {
    let seq = self.next_send_seq;
    self.send_times.insert(seq, Instant::now());
    // ... send packet
}

// On ACK/Status, measure RTT
fn on_ack(&mut self, seq: u64) {
    if let Some(send_time) = self.send_times.remove(&seq) {
        let rtt_us = send_time.elapsed().as_micros() as u64;
        self.congestion.update_rtt(rtt_us);
    }
}
```

---

## 6. Transport Trait Completeness

### Current Trait

```rust
pub trait Reliable: Transport {
    fn retransmit_pending(&mut self) -> io::Result<usize>;
    fn acked_sequence(&self) -> u64;
}
```

### What's Missing

```rust
pub trait Reliable: Transport {
    // Existing
    fn retransmit_pending(&mut self) -> io::Result<usize>;
    fn acked_sequence(&self) -> u64;
    
    // Should add
    fn congestion_window(&self) -> u32;
    fn in_flight(&self) -> u32;
    fn rtt_us(&self) -> u64;
    fn process_naks(&mut self) -> usize;
    fn send_status(&mut self) -> io::Result<()>; // Receiver feedback
}
```

---

## Priority Fix Order

| Priority | Issue | Effort | Impact |
|----------|-------|--------|--------|
| P0 | Call `on_ack()` | Low | Window never grows |
| P0 | Call `update_rtt()` | Low | Timeout broken |
| P1 | NAK backoff delay | Medium | NAK storm prevention |
| P1 | Retransmit pacing | Medium | Flood prevention |
| P2 | Status messages | High | Receiver feedback |
| P2 | Retransmit limit | Low | Overflow protection |

---

## Testing Gaps

| Test | Exists? | Needed? |
|------|---------|---------|
| Congestion window grows on ACK | ❌ | ✅ |
| Window shrinks on loss | ✅ | - |
| NAK not sent too fast | ❌ | ✅ |
| Retransmit rate limited | ❌ | ✅ |
| RTT measurement accuracy | ❌ | ✅ |
| Multi-receiver NAK handling | ❌ | ✅ |

---

## Conclusion

**The reliability layer is ~50% complete.** Core mechanisms exist but are not fully connected. Before claiming "reliable UDP", these issues must be fixed.

### Honest Status

- ✅ Basic send/receive works
- ✅ NAK packets sent on gap detection
- ✅ Retransmit from send window works
- ❌ Congestion control incomplete (no ACK handling)
- ❌ No NAK storm protection
- ❌ No retransmit pacing
- ❌ No RTT measurement
- ❌ No receiver feedback loop

