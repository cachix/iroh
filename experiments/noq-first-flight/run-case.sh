#!/usr/bin/env bash
set -euo pipefail
variant=$1
compete=$2
repeat=$3
samples=${4:-20}
artifact_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
bench_dir=${BENCH_RESULTS_DIR:-"$artifact_dir/reproduced"}
binary=${BENCH_BINARY:-"$artifact_dir/prototype-harness/target/release/iroh-identity-bench"}
mkdir -p "$bench_dir"
proto_env=()
case "$variant" in
  control) ;;
  burst2) proto_env+=(BENCH_INITIAL_PAD=1 BENCH_HANDSHAKE_BURST=1 BENCH_HANDSHAKE_BURST_BYTES=2400) ;;
  burst4) proto_env+=(BENCH_INITIAL_PAD=1 BENCH_HANDSHAKE_BURST=1 BENCH_HANDSHAKE_BURST_BYTES=4800) ;;
  burst8) proto_env+=(BENCH_INITIAL_PAD=1 BENCH_HANDSHAKE_BURST=1 BENCH_HANDSHAKE_BURST_BYTES=9600) ;;
  *) exit 2 ;;
esac
if [[ $compete == 1 ]]; then proto_env+=(BENCH_COMPETE=1 BENCH_INTERVAL_MS=1000); fi
label="bulk${compete}-repeat${repeat}-${variant}"
unshare -Urnm bash -c '
  set -euo pipefail
  ip link set lo up mtu 1500
  if [[ $1 == 1 ]]; then
    tc qdisc add dev lo root handle 1: htb default 10
    tc class add dev lo parent 1: classid 1:10 htb rate 10mbit ceil 10mbit burst 1500 cburst 1500 quantum 1500
    tc qdisc add dev lo parent 1:10 handle 10: netem delay 50ms limit 32
  else
    tc qdisc add dev lo root netem delay 50ms
  fi
  tc -s qdisc show dev lo >&2
  samples=$2
  binary=$3
  shift 3
  env -u BENCH_INITIAL_PAD -u BENCH_HANDSHAKE_BURST -u BENCH_HANDSHAKE_BURST_BYTES -u BENCH_COMPETE -u BENCH_INTERVAL_MS BENCH_VALIDATE_DATA=1 "$@" "$binary" network "$samples"
  tc -s qdisc show dev lo >&2
' bench "$compete" "$samples" "$binary" "${proto_env[@]}" > "$bench_dir/$label.csv" 2> "$bench_dir/$label.log"
echo "Completed $label"
