#!/bin/bash

# Memory profiler for build processes
# Usage: ./memory_profiler.sh <command> <log_file>

COMMAND=$1
LOG_FILE=$2

if [ -z "$COMMAND" ] || [ -z "$LOG_FILE" ]; then
    echo "Usage: $0 <command> <log_file>"
    echo ""
    echo "Example:"
    echo "  $0 \"yarn build\" vite_memory.log"
    exit 1
fi

echo "📊 Memory Profiler"
echo "=================="
echo "Command: $COMMAND"
echo "Log file: $LOG_FILE"
echo ""

# Use /usr/bin/time with -l flag for detailed memory stats (macOS/BSD)
if [[ "$OSTYPE" == "darwin"* ]]; then
    /usr/bin/time -l sh -c "$COMMAND" 2>&1 | tee "$LOG_FILE"

    # Extract key metrics
    echo ""
    echo "Memory Statistics:"
    grep -E "maximum resident|peak memory" "$LOG_FILE" || true

elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux uses different format
    /usr/bin/time -v sh -c "$COMMAND" 2>&1 | tee "$LOG_FILE"

    echo ""
    echo "Memory Statistics:"
    grep -E "Maximum resident set size" "$LOG_FILE" || true
else
    echo "⚠️  Unsupported OS: $OSTYPE"
    echo "Running command without memory profiling..."
    eval "$COMMAND"
fi

echo ""
echo "✅ Complete. Full log saved to: $LOG_FILE"
