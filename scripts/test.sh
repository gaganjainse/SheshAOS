#!/usr/bin/env bash
# SheshAOS test runner
set -euo pipefail

echo "=========================================="
echo "  SheshAOS Test Suite"
echo "=========================================="

echo ""
echo "==> Formatting check"
cargo fmt --check

echo ""
echo "==> Clippy lints"
cargo clippy -- -D warnings

echo ""
echo "==> Unit tests"
cargo test --lib

echo ""
echo "==> Integration tests"
cargo test --test '*'

echo ""
echo "=========================================="
echo "  All tests passed ✓"
echo "=========================================="
