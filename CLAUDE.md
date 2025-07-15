# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Testing
```bash
# Run the full test suite, including doc tests and compile-tests
cargo test --features introspection,idl-parse
# For embedded
cargo test -p zlink-core --no-default-features --features embedded,introspection
```

### Code Quality
```bash
# Format code (uses nightly rustfmt)
cargo +nightly fmt --all

# Run clippy with warnings as errors
cargo clippy -- -D warnings

# Check all features compile
cargo check --features introspection,idl-parse
# For embedded
cargo check -p zlink-core --no-default-features --features embedded,introspection
```

### Git Hooks Setup
```bash
# Enable git hooks for automatic formatting and clippy checks
cp .githooks/* .git/hooks/
```

## Architecture Overview

This is a Rust workspace implementing an asynchronous no-std-compatible Varlink IPC library. The architecture is modular with clear separation of concerns:

### Core Architecture
- **zlink-core**: No-std/no-alloc foundation providing core APIs. Not used directly.
- **zlink**: Main unified API crate that re-exports appropriate subcrates based on cargo features
- **zlink-tokio**: Tokio runtime integration and transport implementations
- **zlink-usb** + **zlink-micro**: Enable USB-based IPC between Linux hosts and microcontrollers

### Key Components
- **Connection**: Low-level API for message send/receive with unique IDs for read/write halves
- **Server**: Listens for connections and handles method calls via services
- **Service**: Trait defining IPC service implementations
- **Call/Reply**: Core message types for IPC communication

### Feature System
- `std` feature: Standard library support with serde_json
- `embedded` feature: No-std support with serde-json-core and defmt logging
- I/O buffer size features: `io-buffer-2kb` (default), `io-buffer-4kb`, `io-buffer-16kb`, `io-buffer-1mb`

### Development Patterns
- Uses workspace-level package metadata (edition, rust-version, license, repository)
- Supports both std and no_std environments through feature flags
- Leverages mayheap for heap/heapless abstraction
- Uses pin-project-lite for async/await support
- Only enable needed features of dependencies

### Code Style
- Follows GNOME commit message guidelines with emoji prefixes
- Atomic commits preferred (one logical change per commit)
- Package prefixes in commit messages
- Force-push workflow for addressing review comments

## Testing Infrastructure

### Mock Socket API
Use consolidated mock socket utilities from `zlink-core/src/test_utils/mock_socket.rs`:
- `MockSocket::new(&responses)` - full socket with pre-configured responses
- `TestWriteHalf::new(expected_len)` - validates exact write lengths
- `CountingWriteHalf::new()` - counts write operations for pipelining tests
