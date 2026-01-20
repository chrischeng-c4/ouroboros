#!/bin/bash
set -e

echo "🔧 Talos vs Vite Benchmark: Setup Script"
echo "=========================================="
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install Node.js 18+"
    exit 1
fi

if ! command -v yarn &> /dev/null; then
    echo "❌ Yarn not found. Please install Yarn"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust"
    exit 1
fi

echo "✅ All prerequisites met"
echo ""

# Clone Excalidraw if not exists
if [ ! -d "/tmp/excalidraw" ]; then
    echo "📦 Cloning Excalidraw repository..."
    cd /tmp
    git clone https://github.com/excalidraw/excalidraw.git
    echo "✅ Excalidraw cloned"
else
    echo "✅ Excalidraw already exists at /tmp/excalidraw"
fi

# Install dependencies
echo ""
echo "📦 Installing Excalidraw dependencies..."
cd /tmp/excalidraw
yarn install
echo "✅ Dependencies installed"

# Analyze project structure
echo ""
echo "📊 Analyzing project structure..."
MODULE_COUNT=$(find excalidraw-app packages -type f \( -name "*.ts" -o -name "*.tsx" -o -name "*.scss" -o -name "*.css" \) | grep -v node_modules | wc -l | xargs)
echo "   Total modules: $MODULE_COUNT"
echo "   Entry point: excalidraw-app/index.tsx"
echo ""

# Run baseline Vite build
echo "⚡ Running baseline Vite build..."
echo "   (This may take a few minutes)"

# Ensure clean build
rm -rf excalidraw-app/build

# Build and capture metrics
BUILD_START=$(date +%s%3N)
yarn build 2>&1 | tee /tmp/excalidraw_baseline_build.log
BUILD_END=$(date +%s%3N)
BUILD_TIME=$((BUILD_END - BUILD_START))

echo ""
echo "✅ Baseline build complete in ${BUILD_TIME}ms"

# Analyze bundle
if [ -d "excalidraw-app/build" ]; then
    BUNDLE_SIZE=$(du -sh excalidraw-app/build | cut -f1)
    echo "   Bundle size: $BUNDLE_SIZE"
else
    echo "⚠️  Build directory not found at expected location"
fi

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Review the baseline build log: /tmp/excalidraw_baseline_build.log"
echo "  2. Run subset benchmark: ./run_benchmark.sh subset"
echo "  3. Run full benchmark: ./run_benchmark.sh full"
echo ""
