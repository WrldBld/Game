# Pre-commit Hooks

This document describes the pre-commit hooks configured for WrldBldr development.

## Overview

Pre-commit hooks run automatically before each commit to catch common issues early. They run fast checks to prevent broken code from being committed.

## Installed Hooks

### 1. Architecture Validation (`arch-check`)

**Purpose:** Enforce architectural rules and prevent layer violations.

**Run Command:** `cargo xtask arch-check`

**What it checks:**
- Internal crate dependencies (forbidden imports between crates)
- Handler complexity (files over 400 lines need refactoring)
- Use case layer violations (no protocol types in use cases)
- Engine protocol isolation (no `wrldbldr_shared::messages` in engine internals)
- Protocol vs contract module classification (per ADR-011 addendum)
- Engine/player app protocol isolation
- And more (see `crates/xtask/src/main.rs`)

**On Failure:** Commit is blocked. Fix violations before committing.

**Common Issues:**
- Importing `ServerMessage` or `ClientMessage` in use cases
- Using `wrldbldr_shared::messages` in engine internal code
- Handler files growing too large (extract to use cases)
- Cross-crate dependency violations

**See Also:**
- [ADR-011 Addendum: Protocol vs Contracts](../architecture/ADR-011-protocol-contracts-distinction.md)
- [Tier-Level Classification](../architecture/tier-levels.md)
- [AGENTS.md](../../AGENTS.md) - Agent guidelines

### 2. Rust Linting (`clippy`)

**Purpose:** Catch common Rust mistakes and enforce style.

**Run Command:** `cargo clippy --workspace -- -D warnings`

**What it checks:**
- Unused variables and dead code
- Inefficient code patterns
- Potential panics or UB
- Style inconsistencies

**On Failure:** Commit is blocked. Fix clippy warnings before committing.

**Common Issues:**
- `clippy::unwrap_used`: Use `?` or proper error handling instead
- `clippy::todo!`: Replace TODO with actual implementation
- `clippy::inefficient_to_string`: Avoid unnecessary allocations

### 3. Formatting (`fmt`)

**Purpose:** Ensure consistent code formatting.

**Run Command:** `cargo fmt -- --check`

**What it checks:**
- Code matches `rustfmt` style

**On Failure:** Commit is blocked. Run `cargo fmt` to fix.

### 4. Cargo Check

**Purpose:** Quick syntax and dependency check without full compilation.

**Run Command:** `cargo check --workspace`

**What it checks:**
- Syntax errors
- Missing dependencies
- Type mismatches
- Dead code warnings

**On Failure:** Commit is blocked. Fix compilation errors before committing.

## Installation

```bash
# Install pre-commit framework (if not already installed)
brew install pre-commit  # macOS
# Or: pip install pre-commit

# Install the hooks
pre-commit install
```

## Usage

### Running Manually

Run all hooks manually without committing:

```bash
pre-commit run --all-files
```

Run specific hook:

```bash
pre-commit run arch-check --all-files
```

### Skipping Hooks

**WARNING:** Only skip hooks when absolutely necessary (e.g., temporary debugging).

Skip a single hook:

```bash
SKIP=clippy git commit -m "WIP: debug print"
```

Skip all hooks:

```bash
SKIP=hook-name,hook-name git commit -m "message"
```

## Hook Configuration

Hooks are configured in `.pre-commit-config.yaml` at repository root.

## Adding New Hooks

1. Add hook configuration to `.pre-commit-config.yaml`
2. Ensure hook is idempotent (can run multiple times safely)
3. Update this documentation
4. Run `pre-commit run --all-files` to test

## Troubleshooting

### Pre-commit: command not found

Install pre-commit framework:

```bash
brew install pre-commit  # macOS
# or
pip install pre-commit
```

### Hook fails but code looks correct

Run hook with verbose output:

```bash
pre-commit run hook-name --verbose
```

### Slow hooks on large changes

Pre-commit only checks changed files by default (for performance). If you need to check all files:

```bash
pre-commit run --all-files
```

## Continuous Integration

CI also runs these checks (without pre-commit overhead) on all pull requests. All pre-commit checks must pass before PR can be merged.

## References

- [Pre-commit Documentation](https://pre-commit.com/)
- [AGENTS.md](../../AGENTS.md) - Development guidelines
- [Rustic DDD Refactor Plan](../plans/RUSTIC_DDD_REFACTOR.md)
