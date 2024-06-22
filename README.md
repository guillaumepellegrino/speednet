
# speednet: A TCP and UDP network speed measurement tool

## Summary
speednet is a speedtest tool written in Rust.
Its design is inspired from iperf3 and it tries to bring the most important features from iperf3.

CURRENTLY IN DEVELOPMENT

## Features
- Zero-conf server. Everything is configured on client side. (SUPPORTED)
- Results are all reported on client side. (SUPPORTED)
- NAT Traversal for UDP and TCP. Download or upload. (SUPPORTED)
- Low or High througtput measurement (>10Gbps) with fearless Multi-Threading using Rust (SUPPORTED)
- View (plot) results in real-time using dataviewer. (TODO)
- Measure quality of service:
-- Througput (SUPPORTED)
-- Packet Loss and reordered (TODO)
-- Jitter (TODO)
-- Network Latency (TODO)
-- Buffer Bloat (Network Latency under load) (TODO)
- Orchestrate multiple tests (TODO)
- Propose or define tests scenarios. Scenarios should be configurable (TODO)
- Validate a test scenario is passing or not (TODO)

## Installation
```
cargo install speednet
```

## Client Usage
speednet client --help

## Server Usage
speednet server --help
