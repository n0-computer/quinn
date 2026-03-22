# Fix Plan: Remaining Network Recovery Issues

## Fix 1: Increase IP Path Idle Timeout

### Current State
- `PATH_MAX_IDLE_TIMEOUT`: 6.5s (HEARTBEAT_INTERVAL 5s + 1.5s)
- `RELAY_PATH_MAX_IDLE_TIMEOUT`: 30s
- `QuicTransportConfigBuilder::default_path_max_idle_timeout()` REJECTS values > 6.5s
- iroh 0.35 used 45s, tailscale uses 45s

### Problem
5s outage leaves only 1.5s for recovery. Real-world outages (WiFi reconnect,
cellular handoff) commonly last 2-10s.

### Proposed Change
Increase `PATH_MAX_IDLE_TIMEOUT` to **15s** in iroh/src/socket.rs:
- 15s gives 10s of margin after a 5s outage
- Still detects dead paths 3x faster than iroh 0.35/tailscale (15s vs 45s)
- HEARTBEAT_INTERVAL stays at 5s — 3 heartbeats fit in 15s, giving multiple retry chances
- Remove the hard cap in `QuicTransportConfigBuilder` or raise it to 15s

### Files to Change
1. `iroh/src/socket.rs:104` — change constant to `Duration::from_secs(15)`
2. `iroh/src/socket.rs:102-103` — update comment
3. `iroh/src/endpoint/quic.rs:487-492` — raise or remove the cap
4. Update any tests that depend on the 6.5s timeout

### Risk
- Dead direct paths take up to 15s to detect instead of 6.5s
- Slightly slower failover from broken direct path to relay
- Mitigated by: handle_network_change already closes non-recoverable paths immediately

---

## Fix 2: Faster Relay Reconnection on Network Change

### Current State
- Relay ping timeout: 5s (PING_TIMEOUT in iroh-relay/src/ping_tracker.rs)
- Relay reconnect backoff: 10ms min, 16s max (ExponentialBuilder in actor.rs:340)
- On ping timeout: connection considered lost, reconnect with backoff
- On established connection loss: backoff resets, immediate reconnect
- On dial failure: exponential backoff (10ms, ~20ms, ~40ms, ~80ms, ...)

### Problem
After interface DOWN at T+5:
- Relay ping sent at T+5 (just before down) gets no pong
- Ping timeout fires at T+10 (5s later) — just as interface comes back UP
- Reconnection starts with backoff: 10ms, 33ms, 55ms, 142ms, 231ms, 334ms, 715ms...
- All fail with "Network is unreachable" because routing isn't ready yet
- Relay finally connects at ~T+11.5 (1.5s after interface UP)
- But by then, direct path already timed out at T+11.3

### Proposed Changes

**A. Immediate health check + backoff reset on network change**
On major network change, send `CheckConnection` to the relay actor AND reset its
reconnect backoff. If the relay is healthy, ping succeeds and nothing changes.
If broken, detected immediately via RTT-based timeout (2x relay_rtt) instead of
waiting for the 5s ping timeout. Then reconnect with fresh backoff.

This is safe in all cases — never breaks a working connection.

Files:
- `iroh/src/socket/transports/relay/actor.rs` — reset backoff on CheckConnection
  when triggered by network change; use RTT-based timeout for health check
- `iroh/src/socket.rs` — send CheckConnection in `notify_quic_network_change`
- `iroh-relay/src/ping_tracker.rs` — expose last RTT for timeout calculation

### Risk
- Slightly more aggressive reconnection attempts
- Mitigated by: only triggered on explicit network change events

---

## Future: Picoquic-Style Path Demotion (TODO)

Instead of killing paths on idle timeout, demote them:
- Demoted paths can't send data but stay in the path table
- PATH_CHALLENGE continues on demoted paths
- If challenge succeeds, path is promoted back
- If challenge fails 3x, path is finally killed

This is a significant noq-proto architecture change. Defer to later.

---

## Implementation Order

1. **Fix 1** first — single constant change, biggest impact, lowest risk
2. **Fix 2B** next — force relay reconnect on network change
3. **Fix 2A** if needed — reset backoff
4. Test with full netsim suite
5. Future: path demotion
