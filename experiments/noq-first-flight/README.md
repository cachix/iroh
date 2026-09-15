# QUIC first-flight experiments

Evidence for [noq #806](https://github.com/n0-computer/noq/issues/806), using the
experimental raw-public-key identity API in [iroh #4534](https://github.com/n0-computer/iroh/pull/4534).
This separate branch preserves the logs and reproducer; it is not part of the iroh PR.

## Start with the trace

[Complete trace-level log](traces/trace-rtt100.log) contains one Ed25519 handshake
followed by one ML-DSA-65 handshake, with both endpoints logged. This uses the
unmodified crates.io noq/noq-proto 1.3.0 at 100 ms RTT, MTU 1500, without loss,
a bandwidth cap, relay, or competing flow. [Timing CSV](traces/trace-rtt100.csv).

Useful locations in the log:

- Lines 1–185: Ed25519, 152.88 ms with tracing.
- Lines 279–627: ML-DSA-65, 290.37 ms with tracing.
- Line 293: server queues 5,556 Handshake CRYPTO bytes (Ed25519: 392).
- Lines 348–375: amplification block and the returning client ACK.
- Lines 433–489: client queues/sends 5,356 Handshake CRYPTO bytes (Ed25519: 181).

The PQ server first sends at roughly 51, 79 and 106 ms after the client Initial,
then runs out of pre-validation send allowance. The returning client ACK permits
it to finish the flight around 161 ms. The final client authentication flight is
also spread across pacing intervals. Traced samples explain packet timing; they
are separate from the untraced benchmark distributions.

## Reproduce the baseline trace

Requires Rust, Linux user/network namespaces, `ip`, and `tc`. From this directory:

```sh
cargo build --release --locked --manifest-path harness/Cargo.toml
bash trace-baseline.sh
```

This writes `reproduced-trace.log` and `.csv`. On the original NixOS host, the
repository's clang/lld configuration was overridden with
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc RUSTFLAGS=''` for Cargo commands.

## Prototype and deterministic tests

The prototype adds one padded 1,200-byte client Initial and a finite Handshake
pacing credit. It retains amplification/congestion checks and explicit rate caps.
These are diagnostic environment flags, not a proposed production API.

```sh
git clone https://github.com/n0-computer/noq.git noq
git -C noq checkout 1a26a8b064d21e316fe6769f068617975bd8a27b
git -C noq apply ../patches/noq-main-burst-sweep.patch
cargo test --release --locked --manifest-path noq/Cargo.toml \
  -p noq-proto first_flight -- --nocapture
cargo build --release --locked --manifest-path prototype-harness/Cargo.toml
bash run-sweep.sh
```

The tests use virtual time and a large Ed25519 certificate to isolate the queue
mechanism. They are not an exact simulation of the real competing-flow topology.
The real-network sweep compares 2/4/8 MTU-equivalent extra pacing credits with an
unmodified control, using actual ML-DSA-65 and Ed25519 authentication. Run on a
quiet host; `monitor.pl` records CPU load alongside the sweep.

## Results and source

- [Smaller-credit sweep and deterministic regressions](results/burst-sweep/REPORT.md).
- [Earlier constrained-link comparisons](results/constrained-links/REPORT.md).
- [Current-main prototype and tests](patches/noq-main-burst-sweep.patch).
- [Original 1.3.0 prototype diff](patches/noq-proto-1.3.0.patch).
- [Benchmark source](harness/src/main.rs), with separate locked manifests for the
  baseline and prototype. Control/variant measurements within each sweep use the
  same binary; only the prototype flags change.

Experiments and documentation were prepared with AI assistance at the user's direction.

To summarize a reproduced sweep:

```sh
REQUIRE_COMPLETE=1 perl summarize.pl reproduced
```

The earlier constrained-link setup can be run with
`bash run-constrained-case.sh control 1mbit 1 1 10 32` and the same command using
`combined`. Arguments are variant, aggregate rate cap, bulk-flow flag, repeat
label, samples per identity, and queue size in packets. Historical method notes
preserve the original temporary paths; these scripts use paths relative to this
artifact directory.
