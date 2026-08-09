# SheshaAOS Test Suite

This directory contains comprehensive tests for SheshaAOS organized by category:

## Test Categories

### `integration/` - Full CLI → Kernel → Tools Flow Tests
- `cli_kernel_tools_flow.rs` - End-to-end CLI commands, task submission, event persistence, state transitions

### `visual/` - Terminal Rendering Visual Regression Tests
- `terminal_rendering.rs` - ANSI parsing, cursor positioning, scrollback, SGR attributes, true color, visual regression with reference images

### `pty/` - PTY/SSH Integration Tests
- `pty_integration.rs` - PTY shell spawning, I/O, resize, Zig VT100 parser, special keys, large output, SSH connection

### `benchmarks/` - Performance Benchmarks
- `performance.rs` - Terminal parsing, kernel task submission, event store, terminal rendering (span batching)

### `ai_integration/` - AI Provider Integration Tests
- `ai_provider.rs` - Streaming, mock HTTP server, error handling, concurrent requests, model selection, history management

## Running Tests

```bash
# Run all integration tests
cargo test -p sheshaaos-tests --test integration

# Run visual regression tests
cargo test -p sheshaaos-tests --test visual

# Run PTY/SSH tests
cargo test -p sheshaaos-tests --test pty

# Run benchmarks
cargo bench -p sheshaaos-tests

# Run AI integration tests
cargo test -p sheshaaos-tests --test ai_integration

# Run all tests
cargo test -p sheshaaos-tests
```

## Visual Regression Testing

Reference images are stored in `tests/visual/references/`. To regenerate:

```bash
cargo test -p sheshaaos-tests --test visual generate_reference_images -- --ignored
```

To run visual regression tests:

```bash
cargo test -p sheshaaos-tests --test visual
```

## Benchmarks

Run with criterion:

```bash
cargo bench -p sheshaaos-tests
```

Results are saved to `target/criterion/`.