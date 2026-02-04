//! IPC server for the tomato42 plant simulator.
//!
//! This server allows multiple clients to connect and interact with a shared
//! tomato plant simulation instance over TCP.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tomato42_core::{Action, TomatoState, step};
use tomato42_protocol::{
    DEFAULT_PORT, IPCRequest, IPCResponse, SerializableTomatoState, SerializableTomatoEvent,
};

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
        tomato42_core::Event::StageChange { from, to } => {
            SerializableTomatoEvent::StageChange {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            }
        }
        tomato42_core::Event::WiltRisk => SerializableTomatoEvent::WiltRisk,
        tomato42_core::Event::Death => SerializableTomatoEvent::Death,
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

    // Create shared state using tokio's async-safe Mutex
    let state = Arc::new(Mutex::new(TomatoState::new()));

    // Bind to the specified port
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();
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

async fn handle_connection(
    socket: TcpStream,
    state: Arc<Mutex<TomatoState>>,
) {
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

async fn process_command(
    command: &str,
    state: &Arc<Mutex<TomatoState>>,
) -> IPCResponse {
    // Parse the command as JSON
    let request: Result<IPCRequest, _> = serde_json::from_str(command);

    match request {
        Ok(req) => {
            let mut state_guard = state.lock().await;

            match req {
                IPCRequest::GetState => {
                    // Just return the current state
                    IPCResponse {
                        success: true,
                        message: "Current state".to_string(),
                        state: Some(to_serializable_state(&*state_guard)),
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
                    let events = result.events.iter()
                        .map(to_serializable_event)
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Stepped simulation by {} seconds", seconds),
                        state: Some(to_serializable_state(&*state_guard)),
                        events,
                    }
                }
                IPCRequest::Water { amount } => {
                    if amount < 0.0 || amount > 1.0 {
                        return IPCResponse {
                            success: false,
                            message: "Water amount must be between 0 and 1".to_string(),
                            state: Some(to_serializable_state(&*state_guard)),
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
                    let events = result.events.iter()
                        .map(to_serializable_event)
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Watered plant with amount: {:.2}", amount),
                        state: Some(to_serializable_state(&*state_guard)),
                        events,
                    }
                }
                IPCRequest::SetLight { level } => {
                    if level < 0.0 || level > 1.0 {
                        return IPCResponse {
                            success: false,
                            message: "Light level must be between 0 and 1".to_string(),
                            state: Some(to_serializable_state(&*state_guard)),
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
                    let events = result.events.iter()
                        .map(to_serializable_event)
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Set light level to: {:.2}", level),
                        state: Some(to_serializable_state(&*state_guard)),
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
                    let events = result.events.iter()
                        .map(to_serializable_event)
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Set temperature to: {:.2}°C", celsius),
                        state: Some(to_serializable_state(&*state_guard)),
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
