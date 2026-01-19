#!/bin/bash

# Set up git hooks for dioxus-virtual-scroll

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$SCRIPT_DIR/.."
HOOKS_DIR="$SCRIPT_DIR/hooks"

# Handle both standalone repo and submodule cases
if [ -f "$REPO_DIR/.git" ]; then
    # Submodule: .git is a file pointing to the actual git dir
    GIT_DIR=$(cat "$REPO_DIR/.git" | sed 's/gitdir: //')
    # Resolve relative path
    GIT_HOOKS_DIR="$REPO_DIR/$GIT_DIR/hooks"
else
    # Standalone repo
    GIT_HOOKS_DIR="$REPO_DIR/.git/hooks"
fi

echo "Installing git hooks..."
echo "Hooks directory: $GIT_HOOKS_DIR"

# Create symlink for pre-commit hook
ln -sf "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"

echo "✅ Git hooks installed"
echo ""
echo "The pre-commit hook will run:"
echo "  - cargo fmt --check"
echo "  - dx fmt --check"
echo "  - cargo clippy"
echo "  - playwright tests"
