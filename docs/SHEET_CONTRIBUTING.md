# Contributing to RuSheet

Thank you for your interest in contributing to RuSheet! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Code Style](#code-style)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Issue Guidelines](#issue-guidelines)

## Code of Conduct

Please be respectful and considerate in all interactions. We're building something together, and a positive environment helps everyone contribute their best work.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rusheet.git
   cd rusheet
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/chrischeng-c4/rusheet.git
   ```

## Development Setup

### Prerequisites

- **Rust** 1.70+ (with `wasm32-unknown-unknown` target)
- **Node.js** 18+
- **pnpm** (package manager)
- **wasm-pack** (for building WASM)
- **just** (command runner)
- **Docker** (optional, for collaboration server)

### Installation

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Install just
cargo install just

# Install Node.js dependencies
pnpm install

# Build WASM module
just build-wasm

# Verify setup
just check
```

### Running the Development Server

```bash
# Start Vite dev server (frontend only)
just dev

# With collaboration server (requires Docker)
just db-up      # Start PostgreSQL
just server     # Start collaboration server
just dev        # Start frontend (in another terminal)
```

## Project Structure

```
rusheet/
├── crates/                 # Rust crates
│   ├── rusheet-core/       # Core data structures
│   ├── rusheet-formula/    # Formula parser & evaluator
│   ├── rusheet-history/    # Undo/redo system
│   ├── rusheet-wasm/       # WebAssembly bindings
│   └── rusheet-server/     # Collaboration server
├── src/                    # TypeScript frontend
│   ├── core/               # API layer (RusheetAPI, WasmBridge)
│   ├── canvas/             # Canvas rendering
│   ├── ui/                 # UI components
│   ├── collab/             # Collaboration client
│   └── worker/             # Web Worker for rendering
├── docs/                   # VitePress documentation
├── migrations/             # Database migrations
└── pkg/                    # Built WASM package (generated)
```

## Making Changes

### Branching Strategy

- `main` - Stable release branch
- `feature/*` - New features
- `fix/*` - Bug fixes
- `docs/*` - Documentation updates

### Workflow

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git checkout main
   git merge upstream/main
   ```

2. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes** with clear, atomic commits

4. **Test your changes**:
   ```bash
   just test-rust       # Rust tests
   just test-unit       # TypeScript unit tests
   just check           # Type checking
   ```

5. **Push and create a PR**:
   ```bash
   git push origin feature/your-feature-name
   ```

## Code Style

### Rust

- Follow standard Rust conventions
- Use `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Document public APIs with doc comments

```bash
# Format Rust code
cargo fmt --all

# Run linter
cargo clippy --workspace
```

### TypeScript

- Use TypeScript strict mode
- Prefer `const` over `let`
- Use meaningful variable names
- Add JSDoc comments for public functions

```bash
# Type check
npx tsc --noEmit
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation
- `style` - Formatting (no code change)
- `refactor` - Code restructuring
- `test` - Adding tests
- `chore` - Maintenance tasks

**Examples:**
```
feat(formula): add VLOOKUP function
fix(core): prevent panic on empty cell reference
docs(api): add examples for event handlers
test(history): add undo/redo edge cases
```

## Testing

### Rust Tests

```bash
# Run all Rust tests
cargo test --workspace

# Run specific crate tests
cargo test -p rusheet-core
cargo test -p rusheet-formula

# Run with output
cargo test --workspace -- --nocapture
```

### TypeScript Tests

```bash
# Unit tests (fast, no WASM)
pnpm test:unit

# Integration tests (browser, real WASM)
pnpm test:integration

# All tests with coverage
pnpm test:coverage

# E2E tests (Playwright)
pnpm test:e2e
```

### Writing Tests

**Rust:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_value() {
        let mut cell = Cell::new();
        cell.set_value(CellValue::Number(42.0));
        assert_eq!(cell.value(), &CellValue::Number(42.0));
    }
}
```

**TypeScript:**
```typescript
import { describe, it, expect } from 'vitest';

describe('RusheetAPI', () => {
  it('should set and get cell value', async () => {
    await rusheet.init();
    rusheet.setCellValue(0, 0, 'Hello');
    const cell = rusheet.getCellData(0, 0);
    expect(cell?.value).toBe('Hello');
  });
});
```

## Submitting Changes

### Pull Request Process

1. **Ensure all tests pass** locally
2. **Update documentation** if needed
3. **Create a Pull Request** with:
   - Clear title following commit message conventions
   - Description of what changed and why
   - Link to related issues (e.g., "Fixes #123")
   - Screenshots for UI changes

### PR Template

```markdown
## Summary
Brief description of changes

## Changes
- Change 1
- Change 2

## Testing
- [ ] Rust tests pass
- [ ] TypeScript tests pass
- [ ] Manual testing done

## Related Issues
Fixes #123
```

### Review Process

- PRs require at least one approval
- Address review feedback promptly
- Keep PRs focused and reasonably sized
- Large changes should be discussed in an issue first

## Issue Guidelines

### Bug Reports

Include:
- **Description** - What happened?
- **Expected behavior** - What should happen?
- **Steps to reproduce** - How can we see the bug?
- **Environment** - OS, browser, Rust version, Node version
- **Screenshots/logs** - If applicable

### Feature Requests

Include:
- **Problem statement** - What problem does this solve?
- **Proposed solution** - How should it work?
- **Alternatives considered** - Other approaches?
- **Additional context** - Examples, mockups, etc.

## Areas to Contribute

Looking for something to work on? Check these areas:

### Good First Issues
- Documentation improvements
- Adding test cases
- Small bug fixes
- Typo corrections

### Medium Complexity
- New formula functions
- UI improvements
- Performance optimizations
- Error message improvements

### Advanced
- XLSX import/export
- Cross-sheet references
- React/Vue component wrappers
- Accessibility improvements

See [TODOS.md](TODOS.md) for the complete roadmap.

## Questions?

- Open a [GitHub Discussion](https://github.com/chrischeng-c4/rusheet/discussions)
- Check existing [Issues](https://github.com/chrischeng-c4/rusheet/issues)

Thank you for contributing!
