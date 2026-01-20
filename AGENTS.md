# PROJECT KNOWLEDGE BASE

**Generated:** 2026-01-20
**Commit:** 141b535
**Branch:** main

## OVERVIEW

壹刻 (mono) - Rust-based intelligent task scheduling engine with CLI + daemon architecture. Currently scaffold stage with comprehensive architecture plan but minimal implementation.

## STRUCTURE

```
mono/
├── Cargo.toml          # Single binary crate (Edition 2024)
├── src/
│   └── main.rs         # Placeholder entry point (24 lines)
├── docs/
│   └── PLANE.md        # Full architecture plan (406 lines)
└── target/             # Build artifacts (ignore)
```

**Planned but unimplemented** (see `docs/PLANE.md`):
- `src/cli/` - CLI client
- `src/daemon/` - Background daemon + Unix socket IPC
- `src/notification/` - Linux DBus interactive notifications
- `src/models/` - Task, Schedule, Feedback entities
- `src/storage/` - SQLite + sqlx persistence
- `src/scheduling/` - Scheduling engine with policies
- `src/learning/` - FTRL + Multi-armed Bandit per task type
- `src/protocol/` - IPC protocol
- `src/config/` - XDG-compliant configuration
- `src/platform/` - Platform abstraction layer
- `migrations/` - SQLite migrations
- `tests/` - Integration tests

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Understand full architecture | `docs/PLANE.md` | **Read first** - comprehensive design doc |
| Add CLI commands | `src/cli/` (create) | See PLANE.md "CLI 命令设计" |
| Implement daemon | `src/daemon/` (create) | Unix socket server |
| Add notifications | `src/notification/` (create) | Use `zbus` for DBus |
| Define models | `src/models/` (create) | Task, Schedule, Feedback, TimeSlot |
| Storage layer | `src/storage/` (create) | SQLite via `sqlx` |
| Scheduling logic | `src/scheduling/` (create) | Priority, deadline, adaptive policies |
| ML components | `src/learning/` (create) | FTRL, Bandit, task-type-level models |

## CONVENTIONS

### Rust Edition 2024
- Uses latest Rust edition - check feature compatibility

### Async Runtime
- Tokio full features (`tokio = { features = ["full"] }`)
- All I/O should be async

### Error Handling
- `anyhow` for application errors
- `thiserror` for library errors

### Logging
- `tracing` + `tracing-subscriber` with env-filter
- Use structured logging

### DBus (Linux)
- `zbus` for DBus interaction (NOT `notify-rust`)
- Required for interactive notification actions

### XDG Paths
- Data: `~/.local/share/mono/`
- Config: `~/.config/mono/`
- Socket: `$XDG_RUNTIME_DIR/mono.sock`
- Database: `~/.local/share/mono/mono.db`

## ANTI-PATTERNS

- **Do NOT use `notify-rust`** for notifications - limited action callback support
- **Do NOT create workspace/monorepo structure** - designed as single binary crate

## COMMANDS

```bash
# Build
cargo build

# Run (currently placeholder)
cargo run -- --name "test"

# Build release
cargo build --release

# Check
cargo check

# Test (no tests yet)
cargo test
```

## DEPENDENCIES (Notable)

| Crate | Purpose |
|-------|---------|
| `clap` | CLI parsing with derive |
| `sqlx` | Async SQLite with compile-time checks |
| `zbus` | D-Bus for Linux notifications |
| `tokio` | Async runtime |
| `ndarray` | ML computations |
| `daemonize2` | Process daemonization |
| `directories` | XDG path resolution |

## NOTES

- **Status**: Scaffold stage (~0% implementation of planned modules)
- **Name "mono"**: NOT a monorepo - refers to "壹刻" (one moment) philosophy
- **Chinese docs**: Architecture plan is in Chinese
- **MVP**: Phases 1-3 per PLANE.md = basic scheduling + notifications
- **Missing**: README.md, LICENSE, migrations/, tests/, most src/ modules
