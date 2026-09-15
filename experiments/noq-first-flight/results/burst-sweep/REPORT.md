# Smaller Handshake pacing credits

The deterministic tests reproduce the queue regression and show why simply
choosing a smaller fixed allowance is not a general fix. The real-network sweep
is supplemental: other host work overlapped, and the user explicitly requested
continuing regardless of CPU load. Use those timings as exploratory observations,
not a reliable ranking of the budgets.

## Deterministic result

Four new tests pass, as do five existing noq tests selected by `handshake`.
A 20-case sweep repeats each scenario internally; its timings, loss counts and
CRYPTO frame counts also match exactly across two separate test invocations.
All simulated handshakes complete a verified application-data transfer.

The test uses real TLS with a fixed Ed25519 certificate containing a 5,000-byte
extension, virtual time, fixed transport seeds, 50 ms one-way delay, and noq's
existing bandwidth-limited routing helper at 10 Mbit/s per direction. There is
one connection and no simulated competing flow. This isolates the large-flight
mechanism; it does not recreate the real shared-HTB/CUBIC topology exactly.

All durations below are milliseconds. The variants add one padded client Initial
and 2/4/8 MTU-equivalent extra pacing credits (2,400/4,800/9,600 bytes). These are
extra credits, not strict limits on the total instantaneous burst: the normal
pacer can allow further packets.

| Nominal queue threshold | Control | 2 credits | 4 credits | 8 credits |
|---|---:|---:|---:|---:|
| 1,200 bytes | 265.402 | 279.652 | 367.679 | 367.679 |
| 2,400 bytes | 261.301 | 229.977 | 321.221 | 321.221 |
| 3,600 bytes | 261.301 | 229.977 | 155.806 | 155.806 |
| 9,600 bytes | 261.301 | 229.977 | 155.806 | 155.806 |

At the 2,400-byte threshold, the control and two-credit variant lose no packets;
the larger variants record five lost packets and additional CRYPTO transmissions.
Four transport seeds reproduce that regression. At 1,200 bytes, even two credits
cause loss and take longer than control. With more queue room, larger credits
win without loss.

The existing routing helper checks the threshold before enqueueing the next
packet, so it permits a packet beyond the nominal byte threshold. It also marks
ECN above half that threshold. The table does not claim strict queue byte limits.
See [test output](deterministic.log), [repeat output](deterministic-repeat.log),
and the [complete prototype/test diff](../../patches/noq-main-burst-sweep.patch).

## Real-network sweep

The same binary compares control, two-, four-, and eight-credit variants. All
use noq main at `1a26a8b064d21e316fe6769f068617975bd8a27b`. Without competition,
there are 20 samples per algorithm/variant at 100 ms RTT with no rate cap. With
competition, there are two reverse-order repeats of 20 samples per algorithm and
variant: shared 10 Mbit/s HTB cap, 32-packet netem queue, 100 ms base RTT, and one
established CUBIC bulk stream. Algorithms alternate within each round.

CPU utilization and pressure are recorded in `host-load.csv` and
`host-load-resumed.csv`; load varied substantially during the competing-flow
measurements. The first abandoned sweep and an interrupted second-round case
are excluded. The completed replacement-sweep cases are retained per the user's
instruction to continue despite CPU activity. [Full method](METHOD.md).

All 480 measured handshakes and their subsequent 1,024-byte application checks
passed. The established bulk stream also validated all received payload bytes.
Warmups are excluded. The tables below are calculated directly from the CSVs;
p95 uses the nearest-rank order statistic and remains noisy at these sample sizes.

| Competition | Variant | Identity | N | Median ms | p95 ms | >300 ms |
|---|---|---|---:|---:|---:|---:|
| 0 | burst2 | Ed25519 | 20 | 151.78 | 152.54 | 0 |
| 0 | burst2 | ML-DSA-65 | 20 | 233.98 | 235.53 | 0 |
| 0 | burst4 | Ed25519 | 20 | 151.80 | 152.19 | 0 |
| 0 | burst4 | ML-DSA-65 | 20 | 152.06 | 152.71 | 0 |
| 0 | burst8 | Ed25519 | 20 | 151.77 | 152.40 | 0 |
| 0 | burst8 | ML-DSA-65 | 20 | 152.09 | 152.60 | 0 |
| 0 | control | Ed25519 | 20 | 151.73 | 152.16 | 0 |
| 0 | control | ML-DSA-65 | 20 | 288.13 | 288.64 | 0 |
| 1 | burst2 | Ed25519 | 40 | 152.36 | 157.05 | 0 |
| 1 | burst2 | ML-DSA-65 | 40 | 242.19 | 336.53 | 3 |
| 1 | burst4 | Ed25519 | 40 | 152.58 | 158.24 | 0 |
| 1 | burst4 | ML-DSA-65 | 40 | 183.78 | 378.69 | 7 |
| 1 | burst8 | Ed25519 | 40 | 151.82 | 153.23 | 0 |
| 1 | burst8 | ML-DSA-65 | 40 | 158.55 | 507.66 | 14 |
| 1 | control | Ed25519 | 40 | 152.24 | 158.71 | 0 |
| 1 | control | ML-DSA-65 | 40 | 288.63 | 403.42 | 7 |

| Competition | Variant | Runs | Bulk Mbit/s | Range | Attempts/s range |
|---|---|---:|---:|---|---|
| 0 | burst2 | 1 | 0.0000 | 0.0000–0.0000 | 2.7436–2.7436 |
| 0 | burst4 | 1 | 0.0000 | 0.0000–0.0000 | 3.0902–3.0902 |
| 0 | burst8 | 1 | 0.0000 | 0.0000–0.0000 | 3.0929–3.0929 |
| 0 | control | 1 | 0.0000 | 0.0000–0.0000 | 2.5547–2.5547 |
| 1 | burst2 | 2 | 3.1570 | 3.1429–3.1710 | 0.9982–0.9989 |
| 1 | burst4 | 2 | 3.1769 | 3.1600–3.1937 | 0.9986–0.9986 |
| 1 | burst8 | 2 | 3.0648 | 3.0003–3.1293 | 0.9989–0.9990 |
| 1 | control | 2 | 3.2203 | 3.1845–3.2561 | 0.9983–0.9989 |

Completed sample rows: 480 across 12 files.

## Interpretation

The no-competition PQ median is 288 ms for control, 234 ms for two credits, and
152 ms for four or eight. Two credits give up part of the clean-link gain.

Under competition, the observed PQ medians/p95s are 289/403 ms for control,
242/337 ms for two credits, 184/379 ms for four, and 159/508 ms for eight. Samples
above 300 ms number 7, 3, 7 and 14 respectively, out of 40 each. The eight-credit
variant also records lower bulk throughput (3.065 versus 3.220 Mbit/s control),
but host-load variation prevents attributing that difference to the credits.

These observations are consistent with a latency/queue-pressure tradeoff, but
only the deterministic tests establish the mechanism in a controlled setting.
They do not identify a universally safe fixed allowance: two credits also regress
with a sufficiently small queue, while larger credits win when there is room.

Next: review the actual trace and deterministic reproducer with noq maintainers
before choosing an API or default. A paced or feedback-aware allowance is another
option to compare; replacing eight credits with two everywhere is not justified.
