#!/usr/bin/env bash
#
# Frontend Migration Verification Script
# Verifies that the frontend configuration has been properly updated
#

set -e

echo "======================================================================"
echo "Frontend Migration Verification"
echo "======================================================================"
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0

# Helper functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check 1: Package name
echo "Checking package.json..."
if grep -q '"name": "@data-bridge/sheet"' frontend/package.json; then
    check_pass "Package name is @data-bridge/sheet"
else
    check_fail "Package name not updated"
fi

# Check 2: Build script
if grep -q 'data-bridge-sheet-wasm' frontend/package.json; then
    check_pass "Build script references data-bridge-sheet-wasm"
else
    check_fail "Build script not updated"
fi

# Check 3: Output filenames
if grep -q 'data-bridge-sheet.umd.js' frontend/package.json; then
    check_pass "Output filenames updated"
else
    check_fail "Output filenames not updated"
fi

echo ""

# Check 4: Vite config
echo "Checking vite.config.ts..."
if grep -q "exclude: \['data-bridge-sheet-wasm'\]" frontend/vite.config.ts; then
    check_pass "Vite config excludes data-bridge-sheet-wasm"
else
    check_fail "Vite config not updated"
fi

if grep -q "inline: \['data-bridge-sheet-wasm'\]" frontend/vite.config.ts; then
    check_pass "Vite config inlines data-bridge-sheet-wasm"
else
    check_fail "Vite config inline not updated"
fi

echo ""

# Check 5: Browser config
echo "Checking vite.config.browser.ts..."
if grep -q "exclude: \['data-bridge-sheet-wasm'\]" frontend/vite.config.browser.ts; then
    check_pass "Browser config excludes data-bridge-sheet-wasm"
else
    check_fail "Browser config not updated"
fi

echo ""

# Check 6: Lib config
echo "Checking vite.config.lib.ts..."
if grep -q 'name: .DataBridgeSheet.' frontend/vite.config.lib.ts; then
    check_pass "Library name is DataBridgeSheet"
else
    check_fail "Library name not updated"
fi

if grep -q 'data-bridge-sheet\.\${format}\.js' frontend/vite.config.lib.ts; then
    check_pass "Library filename pattern updated"
else
    check_fail "Library filename not updated"
fi

echo ""

# Check 7: WasmBridge
echo "Checking WasmBridge.ts..."
if grep -q "import('../../pkg/data_bridge_sheet_wasm')" frontend/src/core/WasmBridge.ts; then
    check_pass "WasmBridge imports data_bridge_sheet_wasm"
else
    check_fail "WasmBridge import not updated"
fi

echo ""

# Check 8: Test setup
echo "Checking test setup..."
if grep -q "data_bridge_sheet_wasm_bg" frontend/src/__tests__/setup.ts; then
    check_pass "Test setup references data_bridge_sheet_wasm_bg"
else
    check_fail "Test setup not updated"
fi

echo ""

# Check 9: No old references
echo "Checking for old references..."
OLD_REFS=$(grep -r "rusheet-wasm\|rusheet_wasm" frontend/src frontend/*.json frontend/*.ts 2>/dev/null | grep -v node_modules | grep -v dist | grep -v pkg | wc -l | tr -d ' ')
if [ "$OLD_REFS" -eq 0 ]; then
    check_pass "No old references to rusheet-wasm found"
else
    check_fail "Found $OLD_REFS old references to rusheet-wasm"
    echo "    Run: grep -r 'rusheet-wasm\|rusheet_wasm' frontend/ --include='*.ts' --include='*.json'"
fi

echo ""

# Check 10: WASM crate exists
echo "Checking WASM crate..."
if [ -f "crates/data-bridge-sheet-wasm/Cargo.toml" ]; then
    check_pass "WASM crate exists at crates/data-bridge-sheet-wasm"
else
    check_fail "WASM crate not found"
fi

echo ""

# Check 11: Justfile commands
echo "Checking justfile..."
if grep -q "build-wasm:" justfile; then
    check_pass "justfile has build-wasm command"
else
    check_fail "justfile missing build-wasm command"
fi

if grep -q "build-frontend:" justfile; then
    check_pass "justfile has build-frontend command"
else
    check_fail "justfile missing build-frontend command"
fi

if grep -q "test-frontend:" justfile; then
    check_pass "justfile has test-frontend command"
else
    check_fail "justfile missing test-frontend command"
fi

echo ""

# Check 12: Documentation exists
echo "Checking documentation..."
if [ -f "frontend/BUILD.md" ]; then
    check_pass "BUILD.md exists"
else
    check_warn "BUILD.md not found"
fi

if [ -f "frontend/QUICKSTART.md" ]; then
    check_pass "QUICKSTART.md exists"
else
    check_warn "QUICKSTART.md not found"
fi

if [ -f "PHASE5_SUMMARY.md" ]; then
    check_pass "PHASE5_SUMMARY.md exists"
else
    check_warn "PHASE5_SUMMARY.md not found"
fi

echo ""

# Summary
echo "======================================================================"
echo "Summary"
echo "======================================================================"
echo -e "${GREEN}Passed:${NC} $PASSED"
echo -e "${RED}Failed:${NC} $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Build WASM: just build-wasm"
    echo "  2. Test frontend: just test-frontend-unit"
    echo "  3. Start dev server: just dev-frontend"
    exit 0
else
    echo -e "${RED}✗ Some checks failed!${NC}"
    echo ""
    echo "Please review the failed checks above."
    exit 1
fi
