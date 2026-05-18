# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

This is a Cargo workspace. Most commands operate across all crates.

```
cargo build                                      # debug build all crates
cargo build --release                            # release build
cargo test --all                                 # run all unit + integration tests
cargo fmt --all --check                          # format check
cargo clippy --all-targets -- -D warnings        # lint (warnings treated as errors)
```

Run a single test (substring match):
```
cargo test -p tomato42-core test_value_bounds
cargo test -p tomato42-ipc --test integration -- water_command
```

Run the binaries:
```
cargo run --bin tomato42-ipc [port]    # start the IPC server (port defaults to tomato42_protocol::DEFAULT_PORT)
cargo run --bin tomato42-cli           # interactive CLI client (talks to IPC server)
cargo run --bin tomato42-tui           # TUI client with time-series graphs
```

The CLI and TUI are **clients** of the IPC server — the server must be running first, or they will fail to connect.

## Architecture

Five crates, organized as a strict dependency hierarchy. The split is load-bearing: the determinism guarantees of `tomato42-core` depend on it staying pure.

```
tomato42-core  ──┐         (pure simulation; no IO, no async, no threads)
                 ├─→ tomato42-ipc   (TCP server, tokio, the only async crate)
tomato42-protocol┘
                 └─→ tomato42-cli, tomato42-tui   (IPC clients only)
```

**tomato42-core** — Pure simulation. Exposes `step(state: TomatoState, action: Action, dt: Duration) -> StepResult`. No dependencies. **This crate must remain deterministic**: same inputs → same outputs, byte-for-byte. Do not add IO, randomness, async, threading, or wall-clock time. Bounded state values (`soil_moisture`, `stress`, `health`, `light_level` ∈ [0,1]; `biomass` ≥ 0) are enforced by `clamp_values()` at the end of every step — preserve that invariant.

**tomato42-protocol** — Shared serde DTOs (`IPCRequest`, `IPCResponse`, `SerializableTomatoState`, `SerializableTomatoEvent`) plus `DEFAULT_PORT` and `DEFAULT_HOST`. Pulled in by both server and clients so the wire format has a single source of truth. Note that the serializable types are **distinct from core types** — `Stage` and `Event` get stringified via `format!("{:?}", ...)` at the IPC boundary rather than sharing enums.

**tomato42-ipc** — The only place async/tokio lives. Single shared `Arc<Mutex<TomatoState>>`, one tokio task per connection, line-delimited JSON over TCP (`\n`-terminated). When any client mutates state, all connected clients receive an update — keep that broadcast behavior intact if you touch the connection handler. Integration tests in `tomato42-ipc/tests/integration.rs` spin up real TCP servers.

**tomato42-cli / tomato42-tui** — Pure clients. They connect via TCP and serialize `IPCRequest` / parse `IPCResponse` from `tomato42-protocol`. The TUI uses `tui` 0.19 + `crossterm` + a `ringbuffer` for time-series history. Neither crate touches `tomato42-core` directly for simulation logic — they only go through the IPC server. (The TUI lists `tomato42-core` as a dep, but the gameplay path is over the wire.)

## Project Philosophy

From the root README: "A tomato is not intelligent." This is a deterministic simulator that values correctness and clarity over cleverness. Plant death is a valid outcome — do not add code that artificially keeps plants alive or smooths over bad inputs. No magic jumps in state; all transitions go through `step()`.
