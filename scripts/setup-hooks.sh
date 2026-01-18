#!/bin/bash

# Set up git hooks for dioxus-virtual-scroll

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="$SCRIPT_DIR/hooks"
GIT_HOOKS_DIR="$SCRIPT_DIR/../.git/hooks"

echo "Installing git hooks..."

# Create symlink for pre-commit hook
ln -sf "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"

echo "✅ Git hooks installed"
echo ""
echo "The pre-commit hook will run:"
echo "  - cargo fmt --check"
echo "  - dx fmt --check"
echo "  - cargo clippy"
echo "  - playwright tests"
