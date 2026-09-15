# Constrained-link and competing-flow report

### Current noq main and constrained-link follow-up

The external harness uses all three noq crates from [main at 1a26a8b](https://github.com/n0-computer/noq/commit/1a26a8b064d21e316fe6769f068617975bd8a27b). The prototype modifies noq-proto only. The control uses the same binary with both prototype flags unset. These measurements are separate from the submitted iroh code.

At 100 ms RTT without a bandwidth cap, 20 samples per algorithm and variant reproduce the earlier result: PQ median **288.43 → 152.74 ms**, Ed25519 **152.06 → 152.19 ms**.

For constrained-link tests, a shared HTB bottleneck on loopback caps aggregate traffic in both directions. Netem adds 50 ms per traversal, with a finite 32- or 128-packet queue. There is no injected random loss; queues can drop packets under load. The competitor is one established CUBIC QUIC bulk stream, with verified payloads.

Each row below pools two reverse-order repeats, for **20 measured handshakes per algorithm and variant**. Algorithms alternate within each run. Numbers are control → combined prototype.

| Rate cap | Queue (packets) | Competing bulk flow | PQ median ms | Ed25519 median ms | Bulk application Mbit/s |
|---|---:|---|---:|---:|---:|
| 1 Mbit/s | 32 | No | 299.85 → 207.58 | 152.01 → 152.05 | — |
| 10 Mbit/s | 32 | No | 288.79 → 157.32 | 152.01 → 152.03 | — |
| 1 Mbit/s | 32 | Yes | 1639.15 → 1272.22 | 573.49 → 621.25 | 0.8738 → 0.8708 |
| 10 Mbit/s | 32 | Yes | 290.24 → 163.12 | 152.43 → 152.55 | 3.2097 → 3.2888 |
| 10 Mbit/s | 128 | Yes | 488.79 → 270.21 | 242.70 → 244.95 | 9.0845 → 9.0818 |

Bulk-throughput ranges across the two repeats:

| Rate / queue | Control Mbit/s | Combined Mbit/s |
|---|---:|---:|
| 1mbit | 0.8728–0.8747 | 0.8700–0.8717 |
| 10mbit | 3.1823–3.2371 | 3.2722–3.3053 |
| 10mbit-q128 | 9.0799–9.0891 | 9.0725–9.0910 |

All **480 measured handshakes and subsequent 1,024-byte application checks passed**: 80 no-cap replication samples and 400 constrained-link samples. The bulk transfer stayed active and validated received bytes throughout each competing-flow run. Warmups and the setup pilot are excluded.

Connection attempts target one per second, or one per 2.5 seconds at 1 Mbit/s with competition; overruns lower the achieved rate. Throughput is measured over the same scheduled interval, including pauses and excluding warmups/shutdown. Five warmup rounds per identity precede each run. The bulk flow starts three seconds before setup/warmups.

**Interpretation and limits:**

- The PQ median improvement survives these bandwidth limits, but the prototype is not an unconditional latency improvement. At 10 Mbit/s with the 32-packet queue and competing traffic, observed PQ p95 worsens from **296.78 to 516.10 ms**, despite the better median. With the 128-packet queue, PQ p95 improves from **556.41 to 303.34 ms**. These are small-sample observations (20 per algorithm/variant), not reliable tail estimates.
- At 1 Mbit/s with competition, Ed25519's median gets **8.3% worse** (573.49 → 621.25 ms). There is still no case here for enabling the controls globally for small handshakes.
- The largest measured reduction in competing-flow throughput is about **0.34%**. This screening finds no large throughput penalty at the tested connection rates, but it does not establish general fairness. The achieved attempt rate at 1 Mbit/s with competition is 0.395–0.399/s, slightly below the 0.4/s target; other shaped cases are 0.997–0.999/s.
- Queue size matters: bulk-only reference runs reach **3.80 Mbit/s** with the 32-packet queue and **9.22 Mbit/s** with the 128-packet queue at the same 10 Mbit/s cap. The short-queue setup is queue-limited and does not represent a fully utilized 10 Mbit/s link. At the 1 Mbit/s cap, the bulk-only reference is 0.925 Mbit/s. These single 20-second references provide context, not matched statistical controls.
- This is one shared aggregate bottleneck, one established QUIC competitor, two rate caps and two queue limits. It excludes TCP competition, independent full-duplex links, relays, and sustained concurrent connection load.

Next: discuss the API with noq maintainers and turn the short-queue case into a deterministic regression scenario. A smaller or differently paced handshake burst is worth comparing before choosing production defaults or limits.

## Full distribution summary

| Rate | Bulk flow | Variant | Identity | N | Median ms | p95 ms | Failures |
|---|---|---|---|---:|---:|---:|---:|
| 10mbit-q128 | 1 | combined | Ed25519 | 20 | 244.95 | 285.38 | 0 |
| 10mbit-q128 | 1 | combined | ML-DSA-65 | 20 | 270.21 | 303.34 | 0 |
| 10mbit-q128 | 1 | control | Ed25519 | 20 | 242.70 | 292.17 | 0 |
| 10mbit-q128 | 1 | control | ML-DSA-65 | 20 | 488.79 | 556.41 | 0 |
| 10mbit | 0 | combined | Ed25519 | 20 | 152.03 | 152.30 | 0 |
| 10mbit | 0 | combined | ML-DSA-65 | 20 | 157.32 | 157.86 | 0 |
| 10mbit | 0 | control | Ed25519 | 20 | 152.01 | 153.05 | 0 |
| 10mbit | 0 | control | ML-DSA-65 | 20 | 288.79 | 289.88 | 0 |
| 10mbit | 1 | combined | Ed25519 | 20 | 152.55 | 155.52 | 0 |
| 10mbit | 1 | combined | ML-DSA-65 | 20 | 163.12 | 516.10 | 0 |
| 10mbit | 1 | control | Ed25519 | 20 | 152.43 | 156.27 | 0 |
| 10mbit | 1 | control | ML-DSA-65 | 20 | 290.24 | 296.78 | 0 |
| 1mbit | 0 | combined | Ed25519 | 20 | 152.05 | 152.81 | 0 |
| 1mbit | 0 | combined | ML-DSA-65 | 20 | 207.58 | 208.29 | 0 |
| 1mbit | 0 | control | Ed25519 | 20 | 152.01 | 152.95 | 0 |
| 1mbit | 0 | control | ML-DSA-65 | 20 | 299.85 | 301.11 | 0 |
| 1mbit | 1 | combined | Ed25519 | 20 | 621.25 | 698.74 | 0 |
| 1mbit | 1 | combined | ML-DSA-65 | 20 | 1272.22 | 2255.98 | 0 |
| 1mbit | 1 | control | Ed25519 | 20 | 573.49 | 637.71 | 0 |
| 1mbit | 1 | control | ML-DSA-65 | 20 | 1639.15 | 2303.36 | 0 |

| Rate | Bulk flow | Variant | Runs | Bulk Mbps (weighted) | Range Mbps | Handshakes/s range |
|---|---|---|---:|---:|---|---|
| 10mbit-q128 | 1 | combined | 2 | 9.0818 | 9.0725–9.0910 | 0.999–0.999 |
| 10mbit-q128 | 1 | control | 2 | 9.0845 | 9.0799–9.0891 | 0.997–0.999 |
| 10mbit | 0 | combined | 2 | 0.0000 | 0.0000–0.0000 | 0.999–0.999 |
| 10mbit | 0 | control | 2 | 0.0000 | 0.0000–0.0000 | 0.999–0.999 |
| 10mbit | 1 | combined | 2 | 3.2888 | 3.2722–3.3053 | 0.999–0.999 |
| 10mbit | 1 | control | 2 | 3.2097 | 3.1823–3.2371 | 0.999–0.999 |
| 1mbit | 0 | combined | 2 | 0.0000 | 0.0000–0.0000 | 0.999–0.999 |
| 1mbit | 0 | control | 2 | 0.0000 | 0.0000–0.0000 | 0.999–0.999 |
| 1mbit | 1 | combined | 2 | 0.8708 | 0.8700–0.8717 | 0.398–0.399 |
| 1mbit | 1 | control | 2 | 0.8738 | 0.8728–0.8747 | 0.395–0.399 |

See README.md for reproduction, artifacts, and execution notes.
