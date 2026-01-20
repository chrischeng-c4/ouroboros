#!/bin/bash
set -e

MODE=$1  # subset or full

if [ -z "$MODE" ]; then
    echo "Usage: $0 <subset|full>"
    echo ""
    echo "Examples:"
    echo "  $0 subset   # Run benchmark on 30-50 module subset"
    echo "  $0 full     # Run benchmark on full project (630+ modules)"
    exit 1
fi

echo "⚡ Talos vs Vite Benchmark: $MODE mode"
echo "=========================================="
echo ""

ITERATIONS=10
BENCHMARK_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_DIR="$BENCHMARK_DIR/results"

# Create results directories
mkdir -p "$RESULTS_DIR/vite"
mkdir -p "$RESULTS_DIR/talos"

# Configuration based on mode
if [ "$MODE" = "subset" ]; then
    PROJECT_DIR="$BENCHMARK_DIR/excalidraw_subset"
    ENTRY_POINT="$PROJECT_DIR/src/index.tsx"
    OUTPUT_DIR="$PROJECT_DIR/dist"

    if [ ! -d "$PROJECT_DIR" ]; then
        echo "❌ Subset project not found at $PROJECT_DIR"
        echo "   Please create the subset first (Phase 2.1)"
        exit 1
    fi
elif [ "$MODE" = "full" ]; then
    PROJECT_DIR="/tmp/excalidraw"
    ENTRY_POINT="$PROJECT_DIR/excalidraw-app/index.tsx"
    OUTPUT_DIR="$PROJECT_DIR/excalidraw-app/build"

    if [ ! -d "$PROJECT_DIR" ]; then
        echo "❌ Excalidraw not found at $PROJECT_DIR"
        echo "   Please run ./setup.sh first"
        exit 1
    fi
else
    echo "❌ Invalid mode: $MODE"
    echo "   Use 'subset' or 'full'"
    exit 1
fi

echo "📊 Configuration:"
echo "   Mode: $MODE"
echo "   Project: $PROJECT_DIR"
echo "   Entry: $ENTRY_POINT"
echo "   Iterations: $ITERATIONS"
echo ""

# ============================================================================
# Cold Start Benchmark
# ============================================================================

echo "❄️  Cold Start Benchmark (${ITERATIONS} iterations)"
echo "-------------------------------------------"

cd "$PROJECT_DIR"

# Vite cold start
echo ""
echo "🔷 Running Vite cold start benchmarks..."
VITE_TIMES=()

for i in $(seq 1 $ITERATIONS); do
    echo "  Iteration $i/$ITERATIONS..."

    # Clear all caches
    rm -rf node_modules/.vite
    rm -rf "$OUTPUT_DIR"

    # Measure build time
    START=$(date +%s%3N)

    if [ "$MODE" = "subset" ]; then
        yarn build > "$RESULTS_DIR/vite/cold_${i}.log" 2>&1
    else
        yarn build > "$RESULTS_DIR/vite/cold_${i}.log" 2>&1
    fi

    END=$(date +%s%3N)
    BUILD_TIME=$((END - START))
    VITE_TIMES+=($BUILD_TIME)

    echo "     Time: ${BUILD_TIME}ms"
done

# Save Vite cold start results
echo "[" > "$RESULTS_DIR/vite/cold_start.json"
for i in "${!VITE_TIMES[@]}"; do
    if [ $i -lt $((${#VITE_TIMES[@]} - 1)) ]; then
        echo "  ${VITE_TIMES[$i]}," >> "$RESULTS_DIR/vite/cold_start.json"
    else
        echo "  ${VITE_TIMES[$i]}" >> "$RESULTS_DIR/vite/cold_start.json"
    fi
done
echo "]" >> "$RESULTS_DIR/vite/cold_start.json"

echo ""
echo "✅ Vite cold start complete"

# Talos cold start
echo ""
echo "🔶 Running Talos cold start benchmarks..."
TALOS_TIMES=()

for i in $(seq 1 $ITERATIONS); do
    echo "  Iteration $i/$ITERATIONS..."

    # Clear Talos output
    rm -rf "$RESULTS_DIR/talos/bundle.js"

    # Measure build time
    START=$(date +%s%3N)

    cd "$BENCHMARK_DIR/../.."
    if [ "$MODE" = "subset" ]; then
        cargo run --release --example benchmark_excalidraw_subset > "$RESULTS_DIR/talos/cold_${i}.log" 2>&1
    else
        cargo run --release --example benchmark_excalidraw_full > "$RESULTS_DIR/talos/cold_${i}.log" 2>&1
    fi

    END=$(date +%s%3N)
    BUILD_TIME=$((END - START))
    TALOS_TIMES+=($BUILD_TIME)

    echo "     Time: ${BUILD_TIME}ms"

    cd "$PROJECT_DIR"
done

# Save Talos cold start results
echo "[" > "$RESULTS_DIR/talos/cold_start.json"
for i in "${!TALOS_TIMES[@]}"; do
    if [ $i -lt $((${#TALOS_TIMES[@]} - 1)) ]; then
        echo "  ${TALOS_TIMES[$i]}," >> "$RESULTS_DIR/talos/cold_start.json"
    else
        echo "  ${TALOS_TIMES[$i]}" >> "$RESULTS_DIR/talos/cold_start.json"
    fi
done
echo "]" >> "$RESULTS_DIR/talos/cold_start.json"

echo ""
echo "✅ Talos cold start complete"

# ============================================================================
# Calculate Statistics
# ============================================================================

echo ""
echo "📈 Cold Start Statistics"
echo "------------------------"

# Calculate Vite stats
VITE_SUM=0
for time in "${VITE_TIMES[@]}"; do
    VITE_SUM=$((VITE_SUM + time))
done
VITE_MEAN=$((VITE_SUM / ${#VITE_TIMES[@]}))

# Calculate Talos stats
TALOS_SUM=0
for time in "${TALOS_TIMES[@]}"; do
    TALOS_SUM=$((TALOS_SUM + time))
done
TALOS_MEAN=$((TALOS_SUM / ${#TALOS_TIMES[@]}))

# Calculate speedup
SPEEDUP=$(echo "scale=2; $VITE_MEAN / $TALOS_MEAN" | bc)

echo ""
echo "Vite:  Mean = ${VITE_MEAN}ms"
echo "Talos: Mean = ${TALOS_MEAN}ms"
echo "Speedup: ${SPEEDUP}x"

if (( $(echo "$SPEEDUP > 1" | bc -l) )); then
    echo "✅ Talos is ${SPEEDUP}x faster"
elif (( $(echo "$SPEEDUP < 1" | bc -l) )); then
    SLOWDOWN=$(echo "scale=2; 1 / $SPEEDUP" | bc)
    echo "⚠️  Talos is ${SLOWDOWN}x slower"
else
    echo "⚖️  Performance is equivalent"
fi

# ============================================================================
# Bundle Analysis
# ============================================================================

echo ""
echo "📦 Bundle Analysis"
echo "------------------"

# Vite bundle
if [ -d "$OUTPUT_DIR" ]; then
    VITE_BUNDLE_SIZE=$(du -sb "$OUTPUT_DIR" | cut -f1)
    echo "Vite bundle size: $(numfmt --to=iec-i --suffix=B $VITE_BUNDLE_SIZE)"
fi

# Talos bundle
if [ -f "$RESULTS_DIR/talos/bundle.js" ]; then
    TALOS_BUNDLE_SIZE=$(stat -f%z "$RESULTS_DIR/talos/bundle.js")
    echo "Talos bundle size: $(numfmt --to=iec-i --suffix=B $TALOS_BUNDLE_SIZE)"
fi

echo ""
echo "✅ Benchmark complete!"
echo ""
echo "Results saved to:"
echo "  - Vite: $RESULTS_DIR/vite/"
echo "  - Talos: $RESULTS_DIR/talos/"
echo ""
echo "Next steps:"
echo "  Run statistical analysis: cargo run --bin compare_results"
echo ""
