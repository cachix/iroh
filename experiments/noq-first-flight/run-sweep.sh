#!/usr/bin/env bash
set -euo pipefail
artifact_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
export BENCH_RESULTS_DIR=${BENCH_RESULTS_DIR:-"$artifact_dir/reproduced"}
mkdir -p "$BENCH_RESULTS_DIR"
perl "$artifact_dir/monitor.pl" > "$BENCH_RESULTS_DIR/host-load.csv" &
monitor_pid=$!
trap 'kill "$monitor_pid" 2>/dev/null || true' EXIT
for variant in control burst2 burst4 burst8; do
  bash "$artifact_dir/run-case.sh" "$variant" 0 1 20
done
for repeat in 1 2; do
  if [[ $repeat == 1 ]]; then variants=(control burst2 burst4 burst8); else variants=(burst8 burst4 burst2 control); fi
  for variant in "${variants[@]}"; do
    bash "$artifact_dir/run-case.sh" "$variant" 1 "$repeat" 20
  done
done
