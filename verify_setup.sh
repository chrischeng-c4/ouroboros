#!/bin/bash
# Quick verification script for ouroboros-agent setup
# Run this before the full integration test to verify everything is ready

echo "🔍 Verifying ouroboros-agent Setup"
echo "===================================="
echo ""

# Check 1: OpenAI API Key
echo "✓ Checking OpenAI API Key..."
if [ -z "$OPENAI_API_KEY" ]; then
    echo "  ✗ OPENAI_API_KEY not set!"
    echo "  → Run: export OPENAI_API_KEY='sk-...'"
    echo ""
    exit 1
else
    # Show partial key for verification
    KEY_START="${OPENAI_API_KEY:0:10}"
    KEY_END="${OPENAI_API_KEY: -4}"
    echo "  ✓ API Key found: ${KEY_START}...${KEY_END}"
fi
echo ""

# Check 2: Python installation
echo "✓ Checking Python..."
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version)
    echo "  ✓ $PYTHON_VERSION"
else
    echo "  ✗ Python 3 not found!"
    exit 1
fi
echo ""

# Check 3: uv installation
echo "✓ Checking uv..."
if command -v uv &> /dev/null; then
    UV_VERSION=$(uv --version 2>&1 | head -1)
    echo "  ✓ $UV_VERSION"
else
    echo "  ✗ uv not found!"
    echo "  → Install: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi
echo ""

# Check 4: ouroboros package installation
echo "✓ Checking ouroboros package..."
if uv run python -c "import ouroboros; print(ouroboros.__version__)" 2>/dev/null; then
    VERSION=$(uv run python -c "import ouroboros; print(ouroboros.__version__)" 2>/dev/null)
    echo "  ✓ ouroboros version: $VERSION"
else
    echo "  ✗ ouroboros package not installed!"
    echo "  → Run: uv run --with maturin maturin develop"
    exit 1
fi
echo ""

# Check 5: ouroboros.agent module
echo "✓ Checking ouroboros.agent module..."
if uv run python -c "from ouroboros import agent; print('✓ agent module available')" 2>/dev/null; then
    echo "  ✓ agent module found"
else
    echo "  ✗ agent module not found!"
    echo "  → Rebuild with: uv run --with maturin maturin develop"
    exit 1
fi
echo ""

# Check 6: Agent classes
echo "✓ Checking agent classes..."
AGENT_CHECK=$(uv run python -c "
from ouroboros.agent import Agent, OpenAI, Tool, ToolRegistry, get_global_registry
print('Agent:', Agent.__name__)
print('OpenAI:', OpenAI.__name__)
print('Tool:', Tool.__name__)
print('ToolRegistry:', ToolRegistry.__name__)
" 2>&1)

if [ $? -eq 0 ]; then
    echo "  ✓ All agent classes imported successfully"
    echo "$AGENT_CHECK" | sed 's/^/    /'
else
    echo "  ✗ Failed to import agent classes!"
    echo "$AGENT_CHECK"
    exit 1
fi
echo ""

# Summary
echo "===================================="
echo "✅ All checks passed!"
echo ""
echo "Ready to run integration test:"
echo "  uv run --env-file=.env python python/examples/agent/integration_test.py"
echo ""
echo "Or run individual examples:"
echo "  uv run --env-file=.env python python/examples/agent/simple_agent.py"
echo "  uv run --env-file=.env python python/examples/agent/tool_agent.py"
echo ""
echo "Or run unit tests (no API key needed):"
echo "  uv run pytest python/tests/agent/ -v"
echo ""
