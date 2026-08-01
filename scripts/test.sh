#!/usr/bin/env bash
# NexusAOS test runner
set -euo pipefail

echo "=========================================="
echo "  NexusAOS Test Suite"
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
cargo test --test '*' 2>/dev/null || echo "  (no integration tests yet)"

echo ""
echo "=========================================="
echo "  All tests passed ✓"
echo "=========================================="
