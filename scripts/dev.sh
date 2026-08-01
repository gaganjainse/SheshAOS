#!/usr/bin/env bash
# NexusAOS development helper script
set -euo pipefail

case "${1:-help}" in
    check)
        echo "==> cargo check"
        cargo check
        ;;
    test)
        echo "==> cargo test"
        cargo test
        ;;
    lint)
        echo "==> cargo fmt --check"
        cargo fmt --check
        echo "==> cargo clippy"
        cargo clippy -- -D warnings
        ;;
    fmt)
        echo "==> cargo fmt"
        cargo fmt
        ;;
    all)
        echo "==> Full verification"
        cargo fmt --check
        cargo clippy -- -D warnings
        cargo test
        echo "==> All checks passed"
        ;;
    run)
        shift
        cargo run -- "$@"
        ;;
    help|*)
        echo "Usage: $0 {check|test|lint|fmt|all|run}"
        echo ""
        echo "  check  - Run cargo check"
        echo "  test   - Run all tests"
        echo "  lint   - Run fmt check and clippy"
        echo "  fmt    - Format code"
        echo "  all    - Run fmt check, clippy, and tests"
        echo "  run    - Run nexusaos with arguments"
        ;;
esac
