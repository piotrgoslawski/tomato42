# tomato42 - Deterministic Tomato Plant Simulator

A boring, deterministic tomato plant simulator that favors correctness, testability, and clarity over cleverness.

## Core Philosophy

- **A tomato is not intelligent.** This simulator models a tomato plant as a simple, deterministic system.
- **Deterministic and testable.** Given the same inputs, the simulator will always produce the same outputs.
- **"Boring stability" is success.** The goal is a reliable, predictable simulation, not flashy features.
- **If the tomato dies, that is a valid outcome.** The simulator doesn't artificially keep plants alive.
- **No hype, no mythology.** Just simple, explicit math and readable code.

## Architecture

The project is split into five crates:

1. **tomato42-core**: Pure simulation logic with no IO, no async, and no threads. Provides a deterministic `step(state, action, dt) -> StepResult` API.

2. **tomato42-cli**: Command-line interface for manual control and step-by-step simulation.

3. **tomato42-tui**: Text-based user interface with time-series graphs showing the internal state of the tomato plant.

4. **tomato42-ipc**: IPC server that allows external applications to interact with the tomato plant simulator over a network connection using a JSON-based protocol.

5. **tomato42-protocol**: Shared serialization types (DTOs) used for communication between the IPC server and its clients.

## Tomato Model

The simulator models a tomato plant with the following state variables:

- **Time**: Elapsed time since the start of the simulation
- **Stage**: Growth stage (Seed, Seedling, Vegetative, Flowering, Fruiting, Dead)
- **Soil Moisture**: Water content in the soil (range: 0-1)
- **Biomass**: Total plant mass (≥ 0)
- **Stress**: Plant stress level (range: 0-1)
- **Health**: Plant health level (range: 0-1)
- **Temperature**: Current temperature in Celsius
- **Light Level**: Current light intensity (range: 0-1)

The simulator supports the following actions:

- **Water**: Add water to the soil
- **SetLight**: Change the light level
- **SetTemp**: Change the temperature
- **DoNothing**: Let time pass without taking any action

## Dynamics

The simulator implements the following dynamics:

- Water increases soil moisture with saturation and drainage
- Moisture decreases over time (evapotranspiration)
- Growth happens when moisture, temperature, and light are in optimal ranges
- Stress increases when conditions are bad
- Sustained stress reduces health
- Death occurs when health reaches 0
- Stage advances based on biomass and time

All values are clamped to their valid ranges, and there are no magic jumps in state.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021 or later)

## Building

Build all crates:

```
cargo build
```

Build in release mode:

```
cargo build --release
```

Release binaries will be in `target/release/`.

## Running

### CLI

```
cargo run --bin tomato42-cli
```

Commands:
- `water <amount>` - Water the plant (amount between 0 and 1)
- `light <level>` - Set light level (between 0 and 1)
- `temp <celsius>` - Set temperature in Celsius
- `step [seconds]` - Advance simulation by specified seconds (default: 1)
- `status` - Show current plant status
- `help` - Show help message
- `exit` - Exit the simulator

### TUI (Text-based User Interface)

The TUI (Text-based User Interface) component provides a rich terminal-based graphical interface for visualizing the tomato plant simulation. Unlike the CLI which uses simple text commands and output, the TUI offers:

- Real-time visualization with four time-series graphs showing:
  - Soil Moisture levels over time
  - Stress levels over time
  - Health levels over time
  - Biomass growth over time
- Color-coded status information
- Interactive keyboard controls
- Visual event notifications
- Auto-stepping capability for continuous simulation

The TUI uses a ring buffer to store historical data points, allowing you to see trends and patterns in the plant's development over time.

Run the TUI with:

```
cargo run --bin tomato42-tui
```

Controls:
- `q` - Quit
- `Space` - Step simulation
- `a` - Toggle auto-step (automatically advances simulation at regular intervals)
- `w` - Water the plant
- `l` - Set light level
- `t` - Set temperature
- `↑/↓` - Adjust water amount
- `←/→` - Adjust light level
- `+/-` - Adjust temperature

### IPC Server

The IPC (Inter-Process Communication) server allows external applications to interact with the tomato plant simulator over a network connection. This enables integration with applications written in any programming language that supports TCP sockets and JSON parsing.

Run the IPC server with:

```
cargo run --bin tomato42-ipc [port]
```

If no port is specified, the server will listen on the default port 8042.

The server uses a simple JSON-based protocol for communication:

- Clients send JSON commands to perform actions (GetState, Step, Water, SetLight, SetTemp)
- The server responds with JSON objects containing the current state and any events
- All connected clients receive updates when the state changes

You can interact with the server using various tools:
- Custom clients in any programming language
- Command-line tools like curl or netcat
- The provided CLI and TUI applications

For detailed documentation on the protocol, curl examples (including how to add water using curl), and example clients, see the [tomato42-ipc README](tomato42-ipc/README.md).

## Testing

Run all tests (unit + integration):

```
cargo test --all
```

### Core unit tests (22 tests)

- **Invariants**: value bounds preserved across extreme inputs, determinism across identical runs
- **Evapotranspiration**: moisture decreases over time
- **Stress mechanics**: low moisture and extreme temperature cause stress, sustained stress kills the plant
- **Growth**: biomass increases under optimal conditions, no growth without light
- **Lifecycle**: full Seed → Fruiting stage progression, stage order is monotonic
- **Dead state**: no growth possible, health stays at zero
- **Actions**: Water, SetLight, SetTemp take effect correctly
- **Events**: Death, StageChange, and WiltRisk events emitted at correct thresholds

### IPC integration tests (7 tests)

- Server returns correct initial state
- Water, Step, SetLight, SetTemp commands work end-to-end over TCP
- Invalid inputs (out-of-range values, malformed JSON) are rejected
- Multiple steps accumulate time correctly

## CI

GitHub Actions runs on every push and PR to `main`:

- `cargo fmt --all --check` — formatting
- `cargo clippy --all-targets -- -D warnings` — linting
- `cargo test --all` — all tests
