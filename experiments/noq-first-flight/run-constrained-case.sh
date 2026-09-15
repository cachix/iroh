#!/usr/bin/env bash
set -euo pipefail
variant=$1
rate=$2
compete=$3
repeat=$4
samples=${5:-10}
queue=${6:-32}
artifact_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
bench_dir=${BENCH_RESULTS_DIR:-"$artifact_dir/reproduced-constrained"}
binary=${BENCH_BINARY:-"$artifact_dir/prototype-harness/target/release/iroh-identity-bench"}
mkdir -p "$bench_dir"
label="${rate}-bulk${compete}-repeat${repeat}-${variant}"
if [[ $queue != 32 ]]; then label="${rate}-q${queue}-bulk${compete}-repeat${repeat}-${variant}"; fi
proto_env=()
interval=1000
if [[ $rate == 1mbit && $compete == 1 ]]; then interval=2500; fi
if [[ $variant == combined ]]; then proto_env+=(BENCH_INITIAL_PAD=1 BENCH_HANDSHAKE_BURST=1); fi
if [[ $compete == 1 ]]; then proto_env+=(BENCH_COMPETE=1); fi
unshare -Urnm bash -c '
  set -euo pipefail
  ip link set lo up mtu 1500
  tc qdisc add dev lo root handle 1: htb default 10
  tc class add dev lo parent 1: classid 1:10 htb rate "$1" ceil "$1" burst 1500 cburst 1500 quantum 1500
  tc qdisc add dev lo parent 1:10 handle 10: netem delay 50ms limit "$5"
  tc -s qdisc show dev lo >&2
  samples=$2
  binary=$3
  interval=$4
  shift 5
  env -u BENCH_INITIAL_PAD -u BENCH_HANDSHAKE_BURST -u BENCH_HANDSHAKE_BURST_BYTES -u BENCH_COMPETE BENCH_VALIDATE_DATA=1 BENCH_INTERVAL_MS="$interval" "$@" "$binary" network "$samples"
  tc -s qdisc show dev lo >&2
  tc -s class show dev lo >&2
' bench "$rate" "$samples" "$binary" "$interval" "$queue" "${proto_env[@]}" > "$bench_dir/$label.csv" 2> "$bench_dir/$label.log"
echo "Completed $label"
