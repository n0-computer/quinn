# Holepunching / Network Change Review

Date: 2026-03-21
Scope: noq PRs #522-#526, iroh PRs #4038-#4041, chuck PR #97

## Findings (ordered by severity)

### 1) High: stale-probe cleanup can trigger on ignored REACH_OUT (noq #524)

- Location: `noq-proto/src/connection/mod.rs` (`REACH_OUT` handling)
- Problem: `is_new_round` is computed before `handle_reach_out`, and stale probe cleanup/CID rotation can run even when `handle_reach_out` ignores the frame (for example unsupported IP family).
- Risk: valid ongoing traversal state can be cleared prematurely, hurting NAT traversal reliability.

### 2) High: NoViablePath grace window can be too short (noq #522)

- Location: `noq-proto/src/connection/mod.rs` (`close_path_inner`)
- Problem: grace timer is set to approximately `1 PTO` with no minimum bound.
- Risk: on low RTT paths, peers/app may get too little time to open a replacement path before forced close.

### 3) Medium: default-route polling can still notify too early (iroh #4039)

- Location: `iroh/src/socket.rs`
- Problem: usable-network check now allows `have_v4 || have_v6` even when no default route exists.
- Risk: QUIC network-change handling may still run before routing is actually restored.

### 4) Medium: relay health-check timeout can exceed default timeout (iroh #4041)

- Location: `iroh-relay/src/ping_tracker.rs`
- Problem: `health_check_timeout()` uses `max(3*rtt, 500ms)` without an upper cap to default timeout.
- Risk: in high-RTT environments this “faster check” can become slower than normal ping timeout.

### 5) Medium: route restoration is not interface-specific (chuck #97)

- Location: `netsim/main.py` (`link_up` action)
- Problem: `ip route replace default via <gw>` is applied without binding to specific interface/device.
- Risk: multi-interface scenarios can get non-deterministic or incorrect default-route behavior.

### 6) Medium: disabling path idle timeout may leave old timer armed (noq #525)

- Location: `noq-proto/src/connection/mod.rs` (`set_path_max_idle_timeout`, `reset_idle_timeout`)
- Problem: re-arm behavior was improved, but setting timeout to `None` does not explicitly clear previously armed `PathIdle` timer.
- Risk: path may still close due to stale timer after timeout is “disabled”.

### 7) Low: stale API semantics around LastOpenPath (noq #522)

- Location: `noq-proto/src/connection/mod.rs` (`ClosePathError`)
- Problem: `LastOpenPath` variant/docs remain though code path is effectively obsolete under new behavior.
- Risk: confusion for API users and future maintainers.

### 8) Low: test coverage gaps in some critical changes

- `noq #525`: limited direct regression tests for changed recovery semantics.
- `noq #524`: needs stronger end-to-end retry + stale CID retirement scenarios.
- `iroh #4038-#4041`: mostly validated via broad netsim runs; limited focused unit/integration guards for specific edge cases above.

## Known Remaining Issues (from fix summary)

- DNS resolver cache is reset on network change; relay hostname resolution may fail during transition.
- No path demotion model yet (dead-path probing vs hard close), unlike picoquic-style approach.
