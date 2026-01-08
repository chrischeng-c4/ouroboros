#!/bin/bash
# Generate OpenSpec proposal using Gemini CLI with direct file creation
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Usage information
usage() {
    cat << EOF
Usage: $0 <change-id> <user-request>

Arguments:
  change-id      Verb-led kebab-case identifier (e.g., add-feature, update-module)
  user-request   Description of the change to propose

Example:
  $0 add-docker-compose "Add Docker Compose support to project management"

Environment:
  PROJECT_ROOT   Project root directory (default: git root or current directory)
EOF
    exit 1
}

# Validate arguments
if [[ $# -lt 2 ]]; then
    echo -e "${RED}Error: Missing required arguments${NC}" >&2
    usage
fi

CHANGE_ID="$1"
USER_REQUEST="$2"

# Validate change-id format (verb-led kebab-case)
if ! [[ "$CHANGE_ID" =~ ^[a-z]+-[a-z0-9-]+$ ]]; then
    echo -e "${RED}Error: Invalid change-id format${NC}" >&2
    echo "Must be verb-led kebab-case (e.g., add-feature, update-module, refactor-core)" >&2
    exit 1
fi

# Find project root
PROJECT_ROOT="${PROJECT_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$PROJECT_ROOT"

echo -e "${GREEN}📝 Generating OpenSpec proposal: ${CHANGE_ID}${NC}"
echo ""

# Check if change-id already exists
CHANGE_DIR="openspec/changes/${CHANGE_ID}"
if [[ -d "$CHANGE_DIR" ]]; then
    echo -e "${RED}Error: Change '${CHANGE_ID}' already exists${NC}" >&2
    echo "Directory: ${CHANGE_DIR}" >&2
    echo "" >&2
    echo "Options:" >&2
    echo "  1. Choose a different change-id" >&2
    echo "  2. Delete existing directory: rm -rf ${CHANGE_DIR}" >&2
    exit 1
fi

# Check if Gemini CLI is available
if ! command -v gemini &> /dev/null; then
    echo -e "${RED}Error: Gemini CLI not found${NC}" >&2
    echo "Install from: https://geminicli.com" >&2
    exit 1
fi

# Gather context for Gemini
echo -e "${YELLOW}Gathering project context...${NC}"
EXISTING_SPECS=$(openspec list --specs 2>/dev/null || echo "No specs found")
ACTIVE_CHANGES=$(openspec list 2>/dev/null || echo "No active changes")

# Build comprehensive prompt
GEMINI_PROMPT=$(cat << EOF
## User Request
${USER_REQUEST}

## Change ID
${CHANGE_ID}

## Existing Specs
${EXISTING_SPECS}

## Active Changes
${ACTIVE_CHANGES}

## Instructions
Read openspec/project.md and openspec/AGENTS.md for conventions.
Explore the codebase to understand patterns.
Use write_file tool to create all proposal files directly in openspec/changes/${CHANGE_ID}/.
EOF
)

# Log file for debugging
LOG_FILE="/tmp/gemini-proposal-${CHANGE_ID}.jsonl"

# Call Gemini CLI with streaming JSON output
echo -e "${YELLOW}Calling Gemini CLI to generate proposal...${NC}"
echo "This will:"
echo "  - Explore the codebase"
echo "  - Generate proposal files in ${CHANGE_DIR}"
echo "  - Stream progress to console"
echo "  - Log to: ${LOG_FILE}"
echo ""

if echo "$GEMINI_PROMPT" | gemini /openspec:proposal --output-format stream-json 2>&1 | tee "$LOG_FILE"; then
    echo ""
    echo -e "${GREEN}✅ Proposal generation completed${NC}"
    echo ""

    # Validate the generated proposal
    echo -e "${YELLOW}Running validation...${NC}"
    if openspec validate "${CHANGE_ID}" --strict; then
        echo -e "${GREEN}✅ Validation passed${NC}"
    else
        echo -e "${RED}⚠️  Validation failed - manual fixes may be needed${NC}"
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Review files: ls -la ${CHANGE_DIR}"
    echo "  2. Read proposal: cat ${CHANGE_DIR}/proposal.md"
    echo "  3. Check tasks: cat ${CHANGE_DIR}/tasks.md"
    echo "  4. Debug log: cat ${LOG_FILE}"
    echo ""
    echo "When ready to implement, use: /openspec:apply"
else
    echo ""
    echo -e "${RED}❌ Proposal generation failed${NC}" >&2
    echo ""
    echo "Troubleshooting:" >&2
    echo "  - Check log: cat ${LOG_FILE}" >&2
    echo "  - Verify GEMINI.md has OpenSpec Instructions" >&2
    echo "  - Ensure .gemini/commands/openspec/proposal.toml exists" >&2
    exit 1
fi
