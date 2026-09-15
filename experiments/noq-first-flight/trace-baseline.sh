#!/usr/bin/env bash
set -euo pipefail
artifact_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
binary="$artifact_dir/harness/target/release/iroh-identity-bench"
unshare -Urnm bash -c '
  set -euo pipefail
  ip link set lo up mtu 1500
  tc qdisc add dev lo root netem delay 50ms
  env -u BENCH_INITIAL_PAD -u BENCH_HANDSHAKE_BURST -u BENCH_HANDSHAKE_BURST_BYTES -u BENCH_COMPETE -u BENCH_INTERVAL_MS TRACE_BENCH=1 RUST_LOG=noq_proto=trace "$1" network 1
' trace "$binary" > "$artifact_dir/reproduced-trace.csv" 2> "$artifact_dir/reproduced-trace.log"
