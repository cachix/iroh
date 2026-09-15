# First-flight prototype: constrained links and a competing QUIC flow

External experiment; no changes to the iroh checkout. All results here use noq,
noq-proto and noq-udp from main at 1a26a8b064d21e316fe6769f068617975bd8a27b,
with the diagnostic first-flight diff applied to noq-proto. The control uses the
same binary with both prototype flags unset. This is not a production patch.

## Reproduction

- `Cargo.toml`, `Cargo.lock`, `src/main.rs`: complete benchmark source and locked
  dependencies, with local paths to the iroh checkout and experimental noq copy.
- `bench`: release binary used for every run in this directory.
- `run-case.sh`: runs one shaped-network comparison case in an isolated user,
  network and mount namespace. It configures loopback MTU 1500, a shared HTB
  bottleneck with 1500-byte burst/cburst and quantum, then a netem queue with
  50 ms delay and 32-packet limit.
- `run-all.sh`: no-cap current-main replication (20 samples per identity and
  variant), then 1/10 Mbit/s with/without a single established QUIC bulk flow.
  Each shaped case has two repeats of 10 samples per identity and variant;
  variant order reverses for the second repeat. Algorithms alternate per sample.
  Finally, a 20-second bulk-only reference runs at each shaped rate.
- `summarize.pl`: summarizes numbered repeats only; pilot and bulk-only reference
  runs are excluded. Run `perl summarize.pl > tables.md` after completion.

Build from the iroh checkout using:

```
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc RUSTFLAGS='' \
  CARGO_TARGET_DIR=/tmp/iroh-identity-bench/target \
  cargo build --release --locked --manifest-path /tmp/iroh-identity-bench/fairness/Cargo.toml
cp /tmp/iroh-identity-bench/target/release/iroh-identity-bench /tmp/iroh-identity-bench/fairness/bench
bash /tmp/iroh-identity-bench/fairness/run-all.sh
```

## Timing and correctness

Every timed connection authenticates both peers and is followed by a verified
1024-byte application transfer, outside the handshake timer. Identity keys and
endpoints are reused; connections are fresh, without resumption. Five warmup
rounds per identity precede every measured case. There is a minimum 120 ms pause
following each connection's data check and close.

Shaped cases target one handshake attempt per second, or one per 2.5 seconds for
1 Mbit/s with competing traffic. An overloaded pilot exceeded a one-second
interval, motivating the longer interval for that case. Attempts are sequential;
if a handshake/data check overruns its slot, the achieved rate will be lower and
must be reported. The interval includes handshake time, verification transfer,
and the close pause. No-cap replication retains the original 120 ms pause only.

The competitor is one established Ed25519 QUIC connection with a unidirectional
bulk stream and the default CUBIC controller. It starts three seconds before
endpoint setup/warmups and continues throughout the run. All received payload
bytes are checked. Its throughput counts application bytes over the measured
handshake interval, including scheduled pauses, excluding handshake warmups and
shutdown. Sender and receiver tasks must remain active through measurement.

`BULK_RESULT` records duration, received application bytes, throughput and achieved
handshake rate. Qdisc/class statistics record actual queue drops and transmitted
bytes. No random loss is injected; the finite queue can drop packets under load.

## Scope

The shaper is shared by both directions on loopback, so its configured rate is
an aggregate bottleneck, not two independent full-duplex links. It is a simple
screening topology. There is one established QUIC competitor, no relay, no TCP
comparison, no sustained concurrent handshake flood, and no range of queue sizes.
Two repeats and 20 samples per algorithm/variant are not enough for reliable
loss-tail estimates. Report medians and repeat-level bulk throughput variability;
do not infer general fairness or deployability from this experiment.

## Follow-up queue check

The initial 10 Mbit/s competing-flow run showed intermittent slow prototype
handshakes. `queue-sensitivity.sh` repeats that condition with a 128-packet queue,
again in both variant orders, plus a bulk-only reference. It uses the same binary
and rate; only the netem queue limit changes. The `q128` filenames identify it.

Execution note: adding the optional queue parameter to the runner while the
first sweep was active caused its shell wrapper to stop after the completed
`1mbit-bulk0-repeat2-combined` measurement. Its CSV, BULK_RESULT, and final qdisc
statistics were complete and checked. `resume.sh` ran the remaining cases and
then the queue-sensitivity cases. No partial measurement was included or silently
replaced. `run-all.sh` followed by `queue-sensitivity.sh` reproduces the full
intended matrix from scratch without that interruption.

## Reports

`REPORT.md` contains the final interpretation and full distribution tables.
`report.pl` audits all 22 measured run files (480 successful samples), requires
completion markers and correct counts, then generates the numerical section.
`interpretation.md` adds the limitations and observed regressions. The public
updates preserve original baseline/prototype numbers and add this distinct
current-main follow-up to noq issue #806 and iroh PR #4534.

```
perl report.pl > results-section.md
cat interpretation.md >> results-section.md
perl summarize.pl > tables.md
perl prepare-updates.pl
```

`SHA256SUMS` identifies the unchanged binary and Cargo.lock used throughout.
`noq-main-first-flight.patch` is the exact experimental diff against the pinned
main commit. The earlier 1.3.0 diff applies to this main version with line offsets.
