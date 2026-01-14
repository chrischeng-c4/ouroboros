#!/usr/bin/env bash
# Cleanup scattered Markdown files
# Move any .md file not in the whitelist to docs/archive/legacy/

set -uo pipefail  # Remove -e to handle errors manually

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ARCHIVE_DIR="docs/archive/legacy"
DRY_RUN=false
FORCE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --execute)
            DRY_RUN=false
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        *)
            echo "Usage: $0 [--dry-run|--execute] [--force]"
            exit 1
            ;;
    esac
done

# Whitelist: Legal locations for .md files (relative to project root)
# Use grep -E patterns (one per line)
WHITELIST=(
    # Root-level config files
    "^README\.md$"
    "^CLAUDE\.md$"
    "^GEMINI\.md$"

    # OpenSpec (all files allowed)
    "^openspec/"

    # Dev-docs (all files allowed)
    "^dev-docs/"

    # Docs (specific subdirectories)
    "^docs/en/"
    "^docs/zh-tw/"
    "^docs/postgres/"
    "^docs/api/"
    "^docs/index\.md$"
    "^docs/archive/"  # Already archived

    # Crate-specific docs (README only, use OpenSpec for tasks)
    "^crates/.*/README\.md$"
    "^crates/.*/benches/README\.md$"
    "^crates/.*/examples/README\.md$"
    "^crates/.*/docs/"  # Crate-specific docs subdirs

    # Benchmarks (specific structure - only docs, not test results)
    "^benchmarks/framework_comparison/"
    "^benchmarks/pyloop/"
    "^benchmarks/reports/"  # New consolidated location

    # Examples
    "^examples/.*\.md$"

    # Frontend
    "^frontend/BUILD\.md$"
    "^frontend/QUICKSTART\.md$"
    "^frontend/README\.md$"

    # Tests
    "^tests/.*/README\.md$"

    # Tools
    "^tools/README\.md$"
    "^tools/EXAMPLES\.md$"

    # Python package
    "^python/.*/README\.md$"

    # Skills
    "^\.claude/skills/.*\.md$"

    # Hidden files (e.g., .github, .venv)
    "^\."

    # Node modules and build artifacts
    "^node_modules/"
    "^target/"
    "^\.venv/"
)

# Function to check if file matches whitelist
is_whitelisted() {
    local file="$1"
    for pattern in "${WHITELIST[@]}"; do
        if echo "$file" | grep -qE "$pattern"; then
            return 0
        fi
    done
    return 1
}

# Find all .md files
echo -e "${YELLOW}Scanning for Markdown files...${NC}"
mapfile -t ALL_MD_FILES < <(find . -type f -name "*.md" 2>/dev/null | sed 's|^\./||' | sort)

echo -e "${GREEN}Found ${#ALL_MD_FILES[@]} total .md files${NC}"
echo ""

# Separate into whitelisted and non-whitelisted
WHITELISTED_FILES=()
NON_WHITELISTED_FILES=()

for file in "${ALL_MD_FILES[@]}"; do
    if is_whitelisted "$file"; then
        WHITELISTED_FILES+=("$file")
    else
        NON_WHITELISTED_FILES+=("$file")
    fi
done

# Report
echo -e "${GREEN}✅ Whitelisted (legal locations): ${#WHITELISTED_FILES[@]} files${NC}"
echo -e "${RED}❌ Non-whitelisted (will be moved): ${#NON_WHITELISTED_FILES[@]} files${NC}"
echo ""

if [[ ${#NON_WHITELISTED_FILES[@]} -eq 0 ]]; then
    echo -e "${GREEN}🎉 All Markdown files are in legal locations!${NC}"
    exit 0
fi

# Show files to be moved
echo -e "${YELLOW}Files to be moved to ${ARCHIVE_DIR}/:${NC}"
for file in "${NON_WHITELISTED_FILES[@]}"; do
    echo "  - $file"
done
echo ""

if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}🔍 DRY RUN MODE - No files will be moved${NC}"
    echo -e "${YELLOW}Run with --execute to actually move files${NC}"
    exit 0
fi

# Execute moves
echo -e "${RED}⚠️  EXECUTING MOVES (use git to revert if needed)${NC}"
if [[ "$FORCE" != true ]]; then
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
else
    echo "Force mode enabled, proceeding without confirmation..."
fi

# Create archive directory
mkdir -p "$ARCHIVE_DIR"

# Move files with error handling
MOVED_COUNT=0
FAILED_COUNT=0
FAILED_FILES=()

for file in "${NON_WHITELISTED_FILES[@]}"; do
    # Check if file still exists (might have been moved already)
    if [[ ! -f "$file" ]]; then
        echo -e "${YELLOW}⊘${NC} Skip: $file (already moved or doesn't exist)"
        continue
    fi

    # Preserve directory structure in archive
    target_dir="$ARCHIVE_DIR/$(dirname "$file")"
    target_file="$ARCHIVE_DIR/$file"

    # Create target directory
    if ! mkdir -p "$target_dir" 2>/dev/null; then
        echo -e "${RED}✗${NC} Failed to create directory: $target_dir"
        ((FAILED_COUNT++))
        FAILED_FILES+=("$file")
        continue
    fi

    # Use git mv if in git repo
    if git rev-parse --git-dir > /dev/null 2>&1; then
        if git mv "$file" "$target_file" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Moved: $file → $target_file"
            ((MOVED_COUNT++))
        else
            echo -e "${RED}✗${NC} Failed: $file"
            ((FAILED_COUNT++))
            FAILED_FILES+=("$file")
        fi
    else
        if mv "$file" "$target_file" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Moved: $file → $target_file"
            ((MOVED_COUNT++))
        else
            echo -e "${RED}✗${NC} Failed: $file"
            ((FAILED_COUNT++))
            FAILED_FILES+=("$file")
        fi
    fi
done

echo ""
echo -e "${GREEN}✅ Successfully moved ${MOVED_COUNT} files${NC}"

if [[ $FAILED_COUNT -gt 0 ]]; then
    echo -e "${RED}❌ Failed to move ${FAILED_COUNT} files:${NC}"
    for file in "${FAILED_FILES[@]}"; do
        echo "  - $file"
    done
    exit 1
fi

echo ""
echo "Next steps:"
echo "  1. Review moved files: ls -R $ARCHIVE_DIR"
echo "  2. Check for broken links: git grep -n 'docs/.*\.md' 2>/dev/null || rg -n 'docs/.*\.md'"
echo "  3. Commit changes: git add -A && git commit -m 'docs: archive scattered markdown files'"
