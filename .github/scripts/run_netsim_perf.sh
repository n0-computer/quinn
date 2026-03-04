#!/bin/bash
set -euo pipefail

# Run netsim-based performance tests for noq-perf
# Usage: run_netsim_perf.sh <impl> <netsim_dir>
#
# impl: noq or upstream-quinn
# netsim_dir: path to chuck/netsim directory

IMPL="$1"
NETSIM_DIR="$2"

echo "Running netsim perf tests: impl=$IMPL"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GITHUB_DIR="$(dirname "$SCRIPT_DIR")"

# Copy the sim config to netsim
mkdir -p "$NETSIM_DIR/sims/perf"
cp "$GITHUB_DIR/sims/perf.json" "$NETSIM_DIR/sims/perf/perf.json"

cd "$NETSIM_DIR"

# Run all netsim tests sequentially
echo "Running all test cases sequentially..."
sudo python3 main.py --max-workers=1 sims/perf

echo "Netsim tests complete: $IMPL"
