#!/bin/bash
set -euo pipefail

# Run all noq-perf benchmark scenarios with CPU monitoring
# Usage: run_perf.sh <binary> <output_dir>

BINARY="$1"
OUTPUT_DIR="$2"

# All scenarios to run
SCENARIOS="small-single medium-single large-single small-concurrent medium-concurrent"

run_scenario() {
  local SCENARIO="$1"
  local SCENARIO_DIR="$OUTPUT_DIR/$SCENARIO"

  mkdir -p "$SCENARIO_DIR"

  # Scenario configurations
  case "$SCENARIO" in
    small-single)
      UPLOAD="10M"
      DOWNLOAD="10M"
      STREAMS=1
      DURATION=30
      ;;
    medium-single)
      UPLOAD="1000M"
      DOWNLOAD="1000M"
      STREAMS=1
      DURATION=30
      ;;
    large-single)
      UPLOAD="10000M"
      DOWNLOAD="10000M"
      STREAMS=1
      DURATION=60
      ;;
    small-concurrent)
      UPLOAD="10M"
      DOWNLOAD="10M"
      STREAMS=10
      DURATION=30
      ;;
    medium-concurrent)
      UPLOAD="1000M"
      DOWNLOAD="1000M"
      STREAMS=10
      DURATION=60
      ;;
    *)
      echo "Unknown scenario: $SCENARIO"
      return 1
      ;;
  esac

  echo "=== Running scenario: $SCENARIO ==="
  echo "  Upload: $UPLOAD, Download: $DOWNLOAD, Streams: $STREAMS, Duration: ${DURATION}s"

  # Start server in background
  "$BINARY" server --listen 127.0.0.1:4433 &
  SERVER_PID=$!

  # Wait for server to be ready (QUIC uses UDP)
  MAX_WAIT=10
  WAIT_COUNT=0
  while ! ss -uln 2>/dev/null | grep -q ':4433 ' && ! lsof -i UDP:4433 >/dev/null 2>&1; do
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
    if [ $WAIT_COUNT -ge $MAX_WAIT ]; then
      echo "Server failed to start within ${MAX_WAIT}s"
      kill "$SERVER_PID" 2>/dev/null || true
      return 1
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
      echo "Server process died"
      return 1
    fi
  done
  echo "Server ready after ${WAIT_COUNT}s"

  # Capture CPU usage in background
  CPU_LOG="$SCENARIO_DIR/cpu.log"
  (
    while kill -0 "$SERVER_PID" 2>/dev/null; do
      CPU=$(ps -p "$SERVER_PID" -o %cpu= 2>/dev/null || echo "0")
      TIMESTAMP=$(date +%s)
      echo "$TIMESTAMP $CPU" >> "$CPU_LOG"
      sleep 0.5
    done
  ) &
  CPU_MONITOR_PID=$!

  # Run client
  echo "Starting client..."
  "$BINARY" client \
    --upload-size "$UPLOAD" \
    --download-size "$DOWNLOAD" \
    --bi-requests "$STREAMS" \
    --duration "$DURATION" \
    --json "$SCENARIO_DIR/results.json" \
    127.0.0.1:4433 || true

  # Stop server
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true

  # Stop CPU monitor
  kill "$CPU_MONITOR_PID" 2>/dev/null || true
  wait "$CPU_MONITOR_PID" 2>/dev/null || true

  # Calculate CPU stats
  if [[ -f "$CPU_LOG" ]]; then
    AVG_CPU=$(awk '{ sum += $2; count++ } END { if (count > 0) print sum/count; else print 0 }' "$CPU_LOG")
    MAX_CPU=$(awk 'BEGIN { max = 0 } { if ($2 > max) max = $2 } END { print max }' "$CPU_LOG")

    cat > "$SCENARIO_DIR/metadata.json" << EOF
{
  "scenario": "$SCENARIO",
  "upload_size": "$UPLOAD",
  "download_size": "$DOWNLOAD",
  "streams": $STREAMS,
  "duration": $DURATION,
  "cpu_avg": $AVG_CPU,
  "cpu_max": $MAX_CPU
}
EOF
    echo "CPU stats: avg=$AVG_CPU%, max=$MAX_CPU%"
  fi

  echo "Scenario $SCENARIO complete"
  echo ""
}

# Run all scenarios sequentially
echo "Running all perf scenarios..."
for scenario in $SCENARIOS; do
  run_scenario "$scenario" || echo "Warning: $scenario failed"
done

echo "All scenarios complete. Results in $OUTPUT_DIR"
