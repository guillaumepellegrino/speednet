
# speednet: A TCP and UDP network speed measurement tool

## Summary
speednet is a speedtest tool written in Rust.
Its design is inspired from iperf3 and it tries to bring the most important features from iperf3.

CURRENTLY IN DEVELOPMENT

## Features
- Zero-conf server. Everything is configured on client side.
- Results are all reported on client side.
- NAT Traversal for UDP and TCP. Download or upload.
- Low or High througtput measurement (>10Gbps) with fearless Multi-Threading using Rust
- View (plot) results in real-time using dataviewer.
- Measure quality of service (througput, packet loss, reordered, jitter)

## Installation
```
cargo install speednet
```

## Client Usage
speednet client --help

## Server Usage
speednet server --help
