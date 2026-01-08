#!/bin/bash
set -euo pipefail

# Run netsim-based performance tests for quinn-perf
# Usage: run_netsim_perf.sh <impl> <netsim_dir>
#
# impl: iroh-quinn or upstream-quinn
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

# Generate reports (log errors but don't fail)
if ! python3 reports_csv.py > "report/${IMPL}_results.json" 2>&1; then
  echo "Warning: reports_csv.py failed or not found"
fi

echo "Netsim tests complete: $IMPL"
