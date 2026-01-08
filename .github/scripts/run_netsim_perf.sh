#!/bin/bash
set -euo pipefail

# Run netsim-based performance tests for quinn-perf
# Usage: run_netsim_perf.sh <impl> <condition> <netsim_dir>
#
# impl: iroh-quinn or upstream-quinn
# condition: ideal, lan, wan, lossy
# netsim_dir: path to chuck/netsim directory

IMPL="$1"
CONDITION="$2"
NETSIM_DIR="$3"

echo "Running netsim perf test: impl=$IMPL, condition=$CONDITION"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GITHUB_DIR="$(dirname "$SCRIPT_DIR")"

# Copy the sim config to netsim
mkdir -p "$NETSIM_DIR/sims/perf"
cp "$GITHUB_DIR/sims/perf.json" "$NETSIM_DIR/sims/perf/perf.json"

# Determine which test case to run based on condition
case "$CONDITION" in
  ideal)
    TEST_CASE="ideal_1_to_1"
    ;;
  lan)
    TEST_CASE="lan_1_to_1"
    ;;
  wan)
    TEST_CASE="wan_1_to_1"
    ;;
  lossy)
    TEST_CASE="lossy_1_to_1"
    ;;
  *)
    echo "Unknown condition: $CONDITION"
    echo "Available: ideal, lan, wan, lossy"
    exit 1
    ;;
esac

cd "$NETSIM_DIR"

# Run the netsim test
# Using --skip to only run the specific test case for this condition
SKIP_CASES=""
for case in ideal_1_to_1 lan_1_to_1 wan_1_to_1 lossy_1_to_1; do
  if [[ "$case" != "$TEST_CASE" ]]; then
    if [[ -n "$SKIP_CASES" ]]; then
      SKIP_CASES="$SKIP_CASES,$case"
    else
      SKIP_CASES="$case"
    fi
  fi
done

echo "Running test case: $TEST_CASE"
echo "Skipping: $SKIP_CASES"

# Run netsim
if [[ -n "$SKIP_CASES" ]]; then
  sudo python3 main.py --max-workers=1 --skip="$SKIP_CASES" sims/perf
else
  sudo python3 main.py --max-workers=1 sims/perf
fi

# Generate reports (log errors but don't fail)
if ! python3 reports_csv.py > "report/${IMPL}_${CONDITION}.json" 2>&1; then
  echo "Warning: reports_csv.py failed or not found"
fi

echo "Netsim test complete: $IMPL $CONDITION"
