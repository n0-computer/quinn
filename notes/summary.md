## Holepunch After Network Change — Fix Summary

**Issue:** [iroh#3931](https://github.com/n0-computer/iroh/issues/3931) / [noq#376](https://github.com/n0-computer/noq/issues/376)

**Dev branches:** noq [`fix-all`](https://github.com/n0-computer/noq/tree/fix-all), iroh [`dig/network-change`](https://github.com/n0-computer/iroh/tree/dig/network-change), chuck [`dig/easy-nat`](https://github.com/n0-computer/chuck/tree/dig/easy-nat)

### noq PRs (all independent, based on `main`)

| PR | Title | Issues |
|---|---|---|
| [#522](https://github.com/n0-computer/noq/pull/522) | accept remote PATH_ABANDON for last path | [#397](https://github.com/n0-computer/noq/issues/397), [#398](https://github.com/n0-computer/noq/issues/398), [#400](https://github.com/n0-computer/noq/issues/400) |
| [#523](https://github.com/n0-computer/noq/pull/523) | cap PTO backoff at 2s post-handshake | [#376](https://github.com/n0-computer/noq/issues/376) |
| [#524](https://github.com/n0-computer/noq/pull/524) | retry off-path probes + retire stale CIDs | [#410](https://github.com/n0-computer/noq/issues/410) |
| [#525](https://github.com/n0-computer/noq/pull/525) | improve path recovery after network change | [#376](https://github.com/n0-computer/noq/issues/376) |
| [#526](https://github.com/n0-computer/noq/pull/526) | prioritize ADD_ADDRESS before NEW_CONNECTION_ID | [#376](https://github.com/n0-computer/noq/issues/376) |

### iroh PRs (all independent, based on [`#3928`](https://github.com/n0-computer/iroh/pull/3928))

| PR | Title |
|---|---|
| [#4038](https://github.com/n0-computer/iroh/pull/4038) | increase path idle timeouts and configure relay paths |
| [#4039](https://github.com/n0-computer/iroh/pull/4039) | poll for default route with exponential backoff after network change |
| [#4040](https://github.com/n0-computer/iroh/pull/4040) | force holepunching on major network change |
| [#4041](https://github.com/n0-computer/iroh/pull/4041) | faster relay health check after network change |

Pending: move netsim test configs from `paused/` to `integration/` after all fixes land.

### chuck/netsim PR

| PR | Title |
|---|---|
| [#97](https://github.com/n0-computer/chuck/pull/97) | full-cone NAT + link_up route restoration for holepunch testing |

### Results

Full netsim suite: [run #23377675607](https://github.com/n0-computer/iroh/actions/runs/23377675607) — **ALL 48 TESTS PASS**

| Test Suite | Cases | Status |
|-----------|-------|--------|
| interface_switch_iroh | 3 | ✅ (0.97–1.75 gbps) |
| adverse_iroh | 6 | ✅ (0.67–2.01 gbps) |
| relay_nat | 3 | ✅ conn_upgrade=1, final_direct=1 |
| iroh throughput | 8 | ✅ (2.82–24.93 gbps) |
| iroh_latency_20ms | 8 | ✅ (3.05–25.86 gbps) |
| iroh_latency_200ms | 8 | ✅ (2.78–25.52 gbps) |
| iroh_cust_10gb | 8 | ✅ (2.28–16.64 gbps) |
| iroh_relay_only | 8 | ✅ (0.54–1.80 gbps) |
| relay | 1 | ✅ |

### Key Findings

1. Post-switch holepunch failed because `trigger_holepunching()` skipped — `net_report` hadn't completed yet when network change fired, so candidates appeared unchanged
2. IP path idle timeout (6.5s) was too short — iroh 0.35 used 10s, tailscale uses 45s at WireGuard level; increased to 15s
3. Netsim `link_up` didn't restore the default route that Linux removes on interface down
4. `handle_network_change` cleared `local_ip` before the hint callback could check it

### Known remaining issues

- DNS resolver cache lost on `reset()` after network change — relay can't resolve hostname during transition
- Picoquic-style path demotion (probe dead paths instead of killing them) — deferred as architectural change
