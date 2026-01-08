#!/bin/bash
set -euo pipefail

# Run quinn-perf benchmark scenario with CPU monitoring
# Usage: run_perf.sh <binary> <scenario> <output_dir>

BINARY="$1"
SCENARIO="$2"
OUTPUT_DIR="$3"

mkdir -p "$OUTPUT_DIR"

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
    echo "Available: small-single, medium-single, large-single, small-concurrent, medium-concurrent"
    exit 1
    ;;
esac

echo "Running scenario: $SCENARIO"
echo "  Upload: $UPLOAD, Download: $DOWNLOAD, Streams: $STREAMS, Duration: ${DURATION}s"

# Start server in background
"$BINARY" server &
SERVER_PID=$!

# Wait for server to be ready (up to 10 seconds)
MAX_WAIT=10
WAIT_COUNT=0
while ! nc -z localhost 4433 2>/dev/null; do
  sleep 1
  WAIT_COUNT=$((WAIT_COUNT + 1))
  if [ $WAIT_COUNT -ge $MAX_WAIT ]; then
    echo "Server failed to start within ${MAX_WAIT}s"
    kill "$SERVER_PID" 2>/dev/null || true
    exit 1
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server process died"
    exit 1
  fi
done
echo "Server ready after ${WAIT_COUNT}s"

# Capture CPU usage in background
CPU_LOG="$OUTPUT_DIR/cpu.log"
(
  while kill -0 "$SERVER_PID" 2>/dev/null; do
    # Get CPU usage for server process
    CPU=$(ps -p "$SERVER_PID" -o %cpu= 2>/dev/null || echo "0")
    TIMESTAMP=$(date +%s.%N)
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
  --json "$OUTPUT_DIR/results.json" \
  localhost:4433 || true

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

  # Add CPU stats to a metadata file
  cat > "$OUTPUT_DIR/metadata.json" << EOF
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

echo "Results written to $OUTPUT_DIR"
