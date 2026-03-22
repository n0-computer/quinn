# Holepunching & Network Recovery Research

## Timing Comparison Table

| Metric | iroh 0.35 (DISCO) | Tailscale | picoquic | noq/iroh (current) |
|--------|-------------------|-----------|----------|---------------------|
| **Network change detection** | <1ms (netmon) | 1s (debounce) | N/A (app-driven) | <1ms (netmon) |
| **Notify QUIC stack** | <1ms (reset states) | 1-2s (rebind+reSTUN) | Immediate (app call) | 0-5s (poll for gateway) |
| **Keepalive/ping interval** | 5s (heartbeat) | 3s (heartbeat) | N/A (QUIC keep-alive) | 5s (QUIC keep-alive) |
| **NAT mapping refresh** | 27s (STUN) | 20-26s (STUN) | N/A | 20-26s (STUN via QAD) |
| **Ping timeout** | 5s | 5s | 3x RTO (~750ms) | 3s (PathOpen timer) |
| **Trust UDP addr duration** | N/A | 6.5s | N/A | N/A |
| **Session/path idle timeout** | 45s | 45s | config (typ 30s) | 6.5s (IP), 15s (relay) |
| **PTO cap** | N/A (not QUIC) | N/A (not QUIC) | 2s | 2s |
| **Holepunch restart after change** | 0-5s (next heartbeat) | 1-2s (rebind+reSTUN) | Immediate (challenge) | 0-5s (force holepunch) |
| **Full recovery (typical)** | 5-10s | 2-5s | 250ms-1s | 1-5s (route_switch) |
| **Full recovery (interface down/up)** | 5-10s | 2-5s | N/A | FAILS (paths die) |
| **Relay reconnection** | Immediate (close+reopen) | 5s backoff | N/A | ~1.5s (TCP+WS) |
| **Path challenge retries** | N/A (DISCO pings) | N/A (DISCO pings) | 3x at 250ms, RTT, 2xRTT | 3x at PTO intervals |

## iroh 0.35 (DISCO Protocol)

### Key Differences from Current
1. **Continuous UDP pings** every 2-5s to ALL candidate addresses, not just active path
2. **No QUIC path validation** — DISCO pings are lightweight (no PATH_CHALLENGE/RESPONSE)
3. **Immediate state reset** on network change — clears best_addr, forces re-ping
4. **45s idle timeout** vs current 6.5s — much more forgiving after outages
5. **Symmetric ping-pong** — receiving a ping triggers automatic ping back (NAT puncture)

### Why It Worked for interface_down_up
- 45s idle timeout meant paths survived 5s outage easily
- State reset on network change forced immediate re-ping
- Continuous pings every 2-5s meant NAT mappings stayed fresh
- No complex path validation — just ping/pong establishes connectivity

### Key Constants
- HEARTBEAT_INTERVAL: 5s
- DISCO_PING_INTERVAL: 5s
- PING_TIMEOUT_DURATION: 5s
- SESSION_ACTIVE_TIMEOUT: 45s
- ENDPOINTS_FRESH_ENOUGH_DURATION: 27s
- STAYIN_ALIVE_MIN_ELAPSED: 2s
- UPGRADE_INTERVAL: 60s

## Tailscale

### Architecture
- WireGuard tunnel + DISCO protocol (same origin as iroh 0.35)
- DERP relays (similar to iroh relay)
- magicsock multiplexes UDP and DERP

### Network Change Handling
1. netmon debounces changes by **1 second**
2. On major change: rebind sockets, close stale DERP, reset endpoint states
3. Immediate ReSTUN for fresh address discovery
4. DERP reconnects with **5s max backoff**

### Why It Works
- **3s heartbeat** keeps NAT mappings fresher than iroh's 5s
- **45s session timeout** — very forgiving
- **Immediate state reset** on rebind — forces re-discovery
- **DERP resilient** — closes and reopens cleanly on interface change
- **1s debounce** absorbs rapid flaps without missing the real change

### Key Constants
- heartbeatInterval: 3s
- pingTimeoutDuration: 5s
- discoPingInterval: 5s
- sessionActiveTimeout: 45s
- trustUDPAddrDuration: 6.5s
- endpointsFreshEnoughDuration: 27s
- DERP backoff max: 5s
- DERP KeepAlive (server): 60s
- DERP inactive cleanup: 60s
- netmon debounce: 1s
- STUN retransmit: 100ms initial, 200ms subsequent

## picoquic

### Path Recovery
1. **Socket error → immediate challenge** on primary path
2. **Demotion** for 3x RTO (~750ms) — path can't send data
3. **Challenge retry** at 250ms, RTT, 2xRTT (3 attempts)
4. **After 3 failed challenges** → demoted for 8x RTO
5. **NAT rebinding** detected by same CID from different address

### Key Constants
- PICOQUIC_INITIAL_RETRANSMIT_TIMER: 250ms
- PICOQUIC_LARGE_RETRANSMIT_TIMER: 2s (PTO cap)
- PICOQUIC_CHALLENGE_REPEAT_MAX: 3
- Path demotion: 3x RTO
- Extended demotion (failed challenges): 8x RTO
- Initial CWIN on new path: 10x MTU (15360 bytes)

### Key Differences from noq
- **Faster path challenge** — 250ms vs PTO-based
- **Explicit demotion** — path can't send but isn't killed
- **NAT rebinding** as first-class concept (not just migration)
- **No "path idle timeout" killing paths** — uses connection idle timeout instead

## Analysis: Why interface_down_up Fails in Current iroh

### Root Cause Chain
1. **Path idle timeout too short (6.5s)** — iroh 0.35 used 45s, tailscale uses 45s
2. **Relay WebSocket breaks** — takes ~1.5s to reconnect after interface UP
3. **No symmetric ping** — DISCO auto-pinged back on receive, creating NAT mappings
4. **QUIC path validation is heavyweight** — PATH_CHALLENGE/RESPONSE vs lightweight DISCO ping
5. **No state reset** — iroh 0.35 cleared best_addr forcing re-ping; current keeps stale state

### What Would Fix It
Options (from least to most invasive):
1. **Increase IP path idle timeout to 30s** — matches picoquic, close to tailscale's 45s
   Risk: slower detection of genuinely dead paths (30s vs 6.5s)
2. **Add keepalive pings outside QUIC** — like DISCO, send lightweight pings to maintain
   NAT mappings and detect path liveness independently of QUIC timers
3. **Implement picoquic-style demotion** — don't kill the path, just stop sending data.
   Keep the path "alive" for challenges. Much more spec-compliant.
4. **Reset path state on network change** — like iroh 0.35's resetEndpointStates().
   Open new paths instead of trying to recover old ones.

### Recommended Approach
Combination of:
- **Increase default IP path idle timeout** to 15-30s (configurable)
- **Faster relay reconnection** — on network change, immediately close and reopen DERP
  (like tailscale does) instead of waiting for ping timeout
- **Immediate re-holepunch on interface UP** — already implemented with force holepunch
- **Consider demotion vs kill** for path failure (longer term, picoquic approach)
