use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tomato42_core::{Action, TomatoState, step};
use serde::{Serialize, Deserialize};

// Default port for the IPC server
const DEFAULT_PORT: u16 = 8043;

// Message types for IPC communication
#[derive(Debug, Serialize, Deserialize)]
enum IPCRequest {
    GetState,
    Step { seconds: u64 },
    Water { amount: f32 },
    SetLight { level: f32 },
    SetTemp { celsius: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IPCResponse {
    success: bool,
    message: String,
    state: Option<SerializableTomatoState>,
    events: Vec<SerializableTomatoEvent>,
}

// Serializable versions of the core types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableTomatoState {
    time_seconds: u64,
    stage: String,
    soil_moisture: f32,
    biomass: f32,
    stress: f32,
    health: f32,
    temperature: f32,
    light_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializableTomatoEvent {
    StageChange { from: String, to: String },
    WiltRisk,
    Death,
}

// Convert between core and serializable types
impl From<&TomatoState> for SerializableTomatoState {
    fn from(state: &TomatoState) -> Self {
        Self {
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
}

impl From<&tomato42_core::Event> for SerializableTomatoEvent {
    fn from(event: &tomato42_core::Event) -> Self {
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

    // Create shared state
    let state = Arc::new(StdMutex::new(TomatoState::new()));

    // Create a broadcast channel for state updates
    let (tx, _) = broadcast::channel(16);

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

        // Clone the state and sender for this connection
        let state_clone = Arc::clone(&state);
        let tx_clone = tx.clone();

        // Spawn a new task to handle this connection
        tokio::spawn(async move {
            handle_connection(socket, state_clone, tx_clone).await;
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    state: Arc<StdMutex<TomatoState>>,
    tx: broadcast::Sender<IPCResponse>,
) {
    // Split the socket into read and write halves
    let (mut read_half, write_half) = socket.into_split();
    let mut reader = BufReader::new(&mut read_half);
    let mut line = String::new();

    // Subscribe to state updates
    let mut rx = tx.subscribe();

    // Wrap the write half in an Arc<Mutex> to share between tasks
    let write_half = Arc::new(Mutex::new(write_half));
    let write_half_clone = Arc::clone(&write_half);

    // Spawn a task to listen for state updates and send them to the client
    let update_task = tokio::spawn(async move {
        while let Ok(response) = rx.recv().await {
            let json = serde_json::to_string(&response).unwrap();
            let mut writer = write_half_clone.lock().await;
            if let Err(e) = writer.write_all(format!("{}\n", json).as_bytes()).await {
                eprintln!("Failed to send update: {}", e);
                break;
            }
        }
    });

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
                let mut writer = write_half.lock().await;
                if let Err(e) = writer.write_all(format!("{}\n", json).as_bytes()).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
                drop(writer); // Release the lock before broadcasting

                // Broadcast the state update to all clients
                let _ = tx.send(response);
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        }
    }

    // Cancel the update task when the connection is closed
    update_task.abort();
    println!("Connection closed");
}

async fn process_command(
    command: &str,
    state: &Arc<StdMutex<TomatoState>>,
) -> IPCResponse {
    // Parse the command as JSON
    let request: Result<IPCRequest, _> = serde_json::from_str(command);

    match request {
        Ok(req) => {
            let mut state_guard = state.lock().unwrap();

            match req {
                IPCRequest::GetState => {
                    // Just return the current state
                    IPCResponse {
                        success: true,
                        message: "Current state".to_string(),
                        state: Some(SerializableTomatoState::from(&*state_guard)),
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
                        .map(|e| SerializableTomatoEvent::from(e))
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Stepped simulation by {} seconds", seconds),
                        state: Some(SerializableTomatoState::from(&*state_guard)),
                        events,
                    }
                }
                IPCRequest::Water { amount } => {
                    if amount < 0.0 || amount > 1.0 {
                        return IPCResponse {
                            success: false,
                            message: "Water amount must be between 0 and 1".to_string(),
                            state: Some(SerializableTomatoState::from(&*state_guard)),
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
                        .map(|e| SerializableTomatoEvent::from(e))
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Watered plant with amount: {:.2}", amount),
                        state: Some(SerializableTomatoState::from(&*state_guard)),
                        events,
                    }
                }
                IPCRequest::SetLight { level } => {
                    if level < 0.0 || level > 1.0 {
                        return IPCResponse {
                            success: false,
                            message: "Light level must be between 0 and 1".to_string(),
                            state: Some(SerializableTomatoState::from(&*state_guard)),
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
                        .map(|e| SerializableTomatoEvent::from(e))
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Set light level to: {:.2}", level),
                        state: Some(SerializableTomatoState::from(&*state_guard)),
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
                        .map(|e| SerializableTomatoEvent::from(e))
                        .collect();

                    IPCResponse {
                        success: true,
                        message: format!("Set temperature to: {:.2}°C", celsius),
                        state: Some(SerializableTomatoState::from(&*state_guard)),
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
