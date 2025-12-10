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
| `can_send()` | Check if window allows send | ✅ YES | `send()` |
| `on_send()` | Increment in_flight counter | ✅ YES | `send()` |
| `on_ack()` | Grow window (AIMD increase) | ✅ **FIXED** | `process_acks()` - per packet |
| `on_loss()` | Shrink window (AIMD decrease) | ✅ YES | NAK handler |
| `update_rtt()` | Update RTT estimate | ✅ **FIXED** | `process_acks()` |

### ✅ FIXED (commit 0d7a567)

- `on_ack()` now called for EACH newly acknowledged packet (not once per ACK)
- `update_rtt()` now called with measured RTT on every ACK
- Added `last_send_time` field for RTT measurement

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

### Implementation: `congestion.rs` + `lib.rs`

| Feature | Purpose | Implemented? | Status |
|---------|---------|--------------|--------|
| RTT measurement | Estimate round-trip time | ✅ **FIXED** | `last_send_time` field |
| RTT smoothing | EWMA of samples | ✅ YES | 7/8 old + 1/8 new |
| Timeout calculation | Retransmit timeout | ⚠️ PARTIAL | RTT available, timeout not used |

### ✅ FIXED (commit 0d7a567)

- Added `last_send_time` field to track send timestamps
- `update_rtt()` called on every ACK with elapsed time
- RTT smoothed via EWMA

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

| Priority | Issue | Effort | Impact | Status |
|----------|-------|--------|--------|--------|
| P0 | Call `on_ack()` | Low | Window never grows | ✅ FIXED |
| P0 | Call `update_rtt()` | Low | Timeout broken | ✅ FIXED |
| P1 | NAK backoff delay | Medium | NAK storm prevention | ❌ TODO |
| P1 | Retransmit pacing | Medium | Flood prevention | ❌ TODO |
| P2 | Status messages | High | Receiver feedback | ❌ TODO |
| P2 | Retransmit limit | Low | Overflow protection | ❌ TODO |

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

**The reliability layer is ~70% complete.** Core mechanisms connected, P1/P2 items remain.

### Honest Status

- ✅ Basic send/receive works
- ✅ NAK packets sent on gap detection
- ✅ Retransmit from send window works
- ✅ Congestion control connected (on_ack per packet)
- ✅ RTT measurement working
- ❌ No NAK storm protection (P1)
- ❌ No retransmit pacing (P1)
- ❌ No receiver feedback loop (P2)

