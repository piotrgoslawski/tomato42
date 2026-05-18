//! IPC server for the tomato42 plant simulator.
//!
//! This server allows multiple clients to connect and interact with a shared
//! tomato plant simulation instance over TCP.

use chrono::Local;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tomato42_core::{step, Action, TomatoState};
use tomato42_protocol::{
    IPCRequest, IPCResponse, SerializableTomatoEvent, SerializableTomatoState, DEFAULT_PORT,
};

const LOG_FILE: &str = "tomato42_state.log";

/// Convert a TomatoState to its serializable representation.
fn to_serializable_state(state: &TomatoState) -> SerializableTomatoState {
    SerializableTomatoState {
        time_seconds: state.time.as_secs(),
        stage: format!("{:?}", state.stage),
        soil_moisture: state.soil_moisture,
        biomass: state.biomass,
        stress: state.stress,
        health: state.health,
        temperature: state.temperature,
        light_level: state.light_level,
    }
}

/// Convert a core Event to its serializable representation.
fn to_serializable_event(event: &tomato42_core::Event) -> SerializableTomatoEvent {
    match event {
        tomato42_core::Event::StageChange { from, to } => SerializableTomatoEvent::StageChange {
            from: format!("{:?}", from),
            to: format!("{:?}", to),
        },
        tomato42_core::Event::WiltRisk => SerializableTomatoEvent::WiltRisk,
        tomato42_core::Event::Death => SerializableTomatoEvent::Death,
    }
}

/// Log a state change to the log file.
async fn log_state_change(action: &str, state: &TomatoState, events: &[SerializableTomatoEvent]) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

    let mut log_entry = format!(
        "[{}] action={} time={}s stage={:?} moisture={:.3} biomass={:.3} stress={:.3} health={:.3} temp={:.1}C light={:.2}",
        timestamp,
        action,
        state.time.as_secs(),
        state.stage,
        state.soil_moisture,
        state.biomass,
        state.stress,
        state.health,
        state.temperature,
        state.light_level,
    );

    if !events.is_empty() {
        let event_strs: Vec<String> = events
            .iter()
            .map(|e| match e {
                SerializableTomatoEvent::StageChange { from, to } => {
                    format!("StageChange({}->{}", from, to)
                }
                SerializableTomatoEvent::WiltRisk => "WiltRisk".to_string(),
                SerializableTomatoEvent::Death => "Death".to_string(),
            })
            .collect();
        log_entry.push_str(&format!(" events=[{}]", event_strs.join(", ")));
    }

    log_entry.push('\n');

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .await
    {
        let _ = file.write_all(log_entry.as_bytes()).await;
    }
}

#[tokio::main]
async fn main() {
    // Parse command line arguments for port
    let args: Vec<String> = std::env::args().collect();
    let port = if args.len() > 1 {
        args[1].parse::<u16>().unwrap_or(DEFAULT_PORT)
    } else {
        DEFAULT_PORT
    };

    println!("Starting tomato42 IPC server on port {}", port);
    println!("Logging state changes to: {}", LOG_FILE);

    // Create shared state using tokio's async-safe Mutex
    let state = Arc::new(Mutex::new(TomatoState::new()));

    // Bind to the specified port
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("Server listening on 127.0.0.1:{}", port);

    // Accept connections
    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        println!("New connection from: {}", addr);

        // Clone the state for this connection
        let state_clone = Arc::clone(&state);

        // Spawn a new task to handle this connection
        tokio::spawn(async move {
            handle_connection(socket, state_clone).await;
        });
    }
}

async fn handle_connection(socket: TcpStream, state: Arc<Mutex<TomatoState>>) {
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    // Process commands from the client
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // Connection closed
                break;
            }
            Ok(_) => {
                let response = process_command(&line, &state).await;

                // Send the response to the client
                let json = serde_json::to_string(&response).unwrap();
                if let Err(e) = write_half.write_all(format!("{}\n", json).as_bytes()).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        }
    }

    println!("Connection closed");
}

async fn process_command(command: &str, state: &Arc<Mutex<TomatoState>>) -> IPCResponse {
    // Parse the command as JSON
    let request: Result<IPCRequest, _> = serde_json::from_str(command);

    match request {
        Ok(req) => {
            let mut state_guard = state.lock().await;

            match req {
                IPCRequest::GetState => {
                    // Just return the current state, no logging needed
                    IPCResponse {
                        success: true,
                        message: "Current state".to_string(),
                        state: Some(to_serializable_state(&state_guard)),
                        events: vec![],
                    }
                }
                IPCRequest::Step { seconds } => {
                    // Step the simulation
                    let result = step(
                        state_guard.clone(),
                        Action::DoNothing,
                        Duration::from_secs(seconds),
                    );

                    // Update the state
                    *state_guard = result.state.clone();

                    // Convert events
                    let events: Vec<_> = result.events.iter().map(to_serializable_event).collect();

                    // Log the state change
                    log_state_change(&format!("Step({}s)", seconds), &state_guard, &events).await;

                    IPCResponse {
                        success: true,
                        message: format!("Stepped simulation by {} seconds", seconds),
                        state: Some(to_serializable_state(&state_guard)),
                        events,
                    }
                }
                IPCRequest::Water { amount } => {
                    if !(0.0..=1.0).contains(&amount) {
                        return IPCResponse {
                            success: false,
                            message: "Water amount must be between 0 and 1".to_string(),
                            state: Some(to_serializable_state(&state_guard)),
                            events: vec![],
                        };
                    }

                    // Apply the water action
                    let result = step(
                        state_guard.clone(),
                        Action::Water { amount },
                        Duration::from_secs(0),
                    );

                    // Update the state
                    *state_guard = result.state.clone();

                    // Convert events
                    let events: Vec<_> = result.events.iter().map(to_serializable_event).collect();

                    // Log the state change
                    log_state_change(&format!("Water({:.2})", amount), &state_guard, &events)
                        .await;

                    IPCResponse {
                        success: true,
                        message: format!("Watered plant with amount: {:.2}", amount),
                        state: Some(to_serializable_state(&state_guard)),
                        events,
                    }
                }
                IPCRequest::SetLight { level } => {
                    if !(0.0..=1.0).contains(&level) {
                        return IPCResponse {
                            success: false,
                            message: "Light level must be between 0 and 1".to_string(),
                            state: Some(to_serializable_state(&state_guard)),
                            events: vec![],
                        };
                    }

                    // Apply the set light action
                    let result = step(
                        state_guard.clone(),
                        Action::SetLight { level },
                        Duration::from_secs(0),
                    );

                    // Update the state
                    *state_guard = result.state.clone();

                    // Convert events
                    let events: Vec<_> = result.events.iter().map(to_serializable_event).collect();

                    // Log the state change
                    log_state_change(&format!("SetLight({:.2})", level), &state_guard, &events)
                        .await;

                    IPCResponse {
                        success: true,
                        message: format!("Set light level to: {:.2}", level),
                        state: Some(to_serializable_state(&state_guard)),
                        events,
                    }
                }
                IPCRequest::SetTemp { celsius } => {
                    // Apply the set temperature action
                    let result = step(
                        state_guard.clone(),
                        Action::SetTemp { celsius },
                        Duration::from_secs(0),
                    );

                    // Update the state
                    *state_guard = result.state.clone();

                    // Convert events
                    let events: Vec<_> = result.events.iter().map(to_serializable_event).collect();

                    // Log the state change
                    log_state_change(&format!("SetTemp({:.1}C)", celsius), &state_guard, &events)
                        .await;

                    IPCResponse {
                        success: true,
                        message: format!("Set temperature to: {:.2}°C", celsius),
                        state: Some(to_serializable_state(&state_guard)),
                        events,
                    }
                }
            }
        }
        Err(e) => {
            // Return an error response
            IPCResponse {
                success: false,
                message: format!("Invalid command: {}", e),
                state: None,
                events: vec![],
            }
        }
    }
}
