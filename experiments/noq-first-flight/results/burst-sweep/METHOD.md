# Smaller first-flight credits and deterministic queue regressions

All work is external to the iroh checkout and submitted iroh PR code. The noq
checkout is `/tmp/noq-burst-sweep`, based on main commit
`1a26a8b064d21e316fe6769f068617975bd8a27b`. It contains the previous prototype,
an optional diagnostic byte budget, and test-only connection configuration.

## Diagnostic controls

- Control: all BENCH_INITIAL_PAD / BENCH_HANDSHAKE_BURST variables unset.
- burst2: one extra padded Initial, plus 2400 bytes of Handshake pacing credit.
- burst4: same Initial, plus 4800 bytes of credit.
- burst8: same Initial, plus 9600 bytes, matching the earlier prototype.

These are 2/4/8 initial-MTU-equivalent *extra scheduling credits*, not strict
caps on the total instantaneous burst. The ordinary pacer can allow additional
packets. Amplification, congestion, explicit outgoing-rate checks and the old
non-replenishing credit accounting are retained. Production API design is still
open; environment-variable configuration remains an external diagnostic only.

## Deterministic mechanism test

`noq-proto/src/tests/first_flight.rs` uses noq's existing bandwidth-limited
routing simulator and caller-driven virtual time. The TLS server presents a
trusted Ed25519 certificate containing a 5000-byte noncritical extension. Its
private key is a fixed public test fixture, certificate serial is fixed, and
transport RNG seeds and packet numbering are fixed. Real TLS is used, with
constant-size Ed25519 signatures; TLS random contents are not recorded or relied
upon for the deterministic packet schedule.

This isolates the large-server-flight transport mechanism. It is **not** a PQ
mutual-authentication test or an exact model of the real HTB/competing-CUBIC
experiment. It has one connection, independent 10 Mbit/s links in each direction,
50 ms one-way delay, and nominal queue thresholds of 1200/2400/3600/9600 bytes.
The existing limiter tests its threshold before enqueueing the next packet, so
it permits a packet beyond the nominal byte threshold; these values are not
strict queue-capacity byte limits. It also marks ECN above half its threshold.

Tests:

1. A 20-case sweep repeats each scenario and requires identical handshake time,
   packet-loss count and CRYPTO frame count.
2. Four seeds reproduce the 2400-byte-threshold regression: 9600 credits lose
   packets and take longer than control, while 2400 credits avoid loss and win.
3. At a 1200-byte threshold, even 2400 credits cause loss and take longer than
   control. Smaller credits are not universally safe.
4. At 9600 bytes, larger credits win without loss, documenting the tradeoff.

Every simulated connection also completes a verified application-data transfer.
All four tests were run twice; the 20 printed sweep rows matched exactly across
separate invocations. Five existing tests selected by `handshake` passed too.

```
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc RUSTFLAGS='' \
  CARGO_TARGET_DIR=/tmp/iroh-identity-bench/target \
  cargo test --release --locked --manifest-path /tmp/noq-burst-sweep/Cargo.toml \
  -p noq-proto first_flight -- --nocapture
```

## Real-network sweep

The unchanged external fairness harness is built against the modified noq copy;
`bench` and `SHA256SUMS` preserve the binary and lockfile used. Four variants each
run 20 handshakes per identity without a rate cap. Then two reversed-order
repeats run 20 handshakes per identity and variant with a competing QUIC bulk
stream: 480 measured handshakes in total, excluding warmups.

All network cases have 100 ms base RTT and MTU 1500. The competing-flow setup uses
the previous shared 10 Mbit/s HTB cap and a 32-packet netem queue. No random loss
is injected, but the queue drops under load. The bulk stream uses default CUBIC,
checks every payload byte, starts three seconds before setup/warmups, and remains
active throughout the measured interval. Authentication completes on both peers
before the handshake timer stops. Each connection then exchanges a verified
1024-byte application payload, outside timing.

Five warmup rounds per identity are excluded. Algorithms alternate within each
round. Competing-flow cases target one connection attempt per second, including
all pauses/data validation; overruns reduce the achieved rate. No-cap cases use
the original 120 ms untimed pause. Bulk throughput covers the measured interval,
including pauses, excluding warmups and shutdown. The second repeat reverses the
four variants. This agent ran no additional benchmark or compilation during measurement; other host activity is described below.

```
bash run-all.sh
REQUIRE_COMPLETE=1 perl summarize.pl > tables.md
```

This is a screening experiment, not a production-default decision. A p95 based
on 40 samples per algorithm/variant remains noisy, especially with queue drops.

## Host-load qualification and interruptions

An earlier sweep was discarded in full when the user reported CPU saturation
(`cpu-contended/`, not included in the results). This replacement sweep records
host CPU utilization and pressure in `host-load.csv` and
`host-load-resumed.csv`. Other work again saturated the host during the first
competing-flow round. The runner was stopped, and the partial second-round
burst8 case was excluded (`interrupted/`). The user then explicitly requested
continuing regardless of CPU load. All complete replacement-sweep cases are
retained, and the second round restarted at burst8.

Consequently the real-network timings are exploratory and cannot establish a
reliable ranking between burst budgets. The deterministic virtual-time queue
regressions do not depend on host scheduling. No timing numbers from the first,
fully discarded sweep are included. There is no automatic CPU gating in the
published reproduction scripts.
