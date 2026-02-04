//! CLI driver for the tomato42 plant simulator.
//!
//! This application provides a command-line interface for manual control
//! of the tomato plant simulator, allowing step-by-step simulation.
//!
//! Instead of directly calling core functions, this CLI communicates with
//! the tomato42-ipc server over a TCP connection. This allows for a clean
//! separation between the CLI and the core simulation logic, and enables
//! multiple clients to interact with the same simulation instance.
//!
//! Before running this CLI, make sure the tomato42-ipc server is running:
//! ```
//! cargo run --bin tomato42-ipc
//! ```

use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use tomato42_protocol::{
    DEFAULT_HOST, DEFAULT_PORT, IPCRequest, IPCResponse,
    SerializableTomatoState, SerializableTomatoEvent,
};

/// Connects to the IPC server and returns a TCP stream
fn connect_to_server() -> Result<TcpStream, io::Error> {
    let server_addr = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT);
    println!("Connecting to IPC server at {}...", server_addr);

    match TcpStream::connect(&server_addr) {
        Ok(stream) => {
            println!("Connected to IPC server");
            Ok(stream)
        },
        Err(e) => {
            eprintln!("Failed to connect to IPC server: {}", e);
            eprintln!("Make sure the tomato42-ipc server is running with:");
            eprintln!("  cargo run --bin tomato42-ipc");
            Err(e)
        }
    }
}

/// Sends a command to the IPC server and returns the response
fn send_command(stream: &mut TcpStream, request: IPCRequest) -> Result<IPCResponse, io::Error> {
    // Serialize the request to JSON
    let json = serde_json::to_string(&request)?;

    // Send the request to the server
    stream.write_all(format!("{}\n", json).as_bytes())?;
    stream.flush()?;

    // Read the response
    let mut response_str = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut response_str)?;

    // Deserialize the response
    match serde_json::from_str(&response_str) {
        Ok(response) => Ok(response),
        Err(e) => {
            eprintln!("Failed to parse server response: {}", e);
            eprintln!("Response was: {}", response_str);
            Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid server response"))
        }
    }
}

fn main() {
    println!("Tomato42 CLI - Deterministic Tomato Plant Simulator");
    println!("---------------------------------------------------");
    println!("Enter commands to control the simulation:");
    println!("  water <amount>     - Water the plant (amount between 0 and 1)");
    println!("  light <level>      - Set light level (between 0 and 1)");
    println!("  temp <celsius>     - Set temperature in Celsius");
    println!("  step [seconds]     - Advance simulation by specified seconds (default: 1)");
    println!("  status             - Show current plant status");
    println!("  help               - Show this help message");
    println!("  exit               - Exit the simulator");
    println!();

    // Connect to the IPC server
    let mut stream = match connect_to_server() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Exiting due to connection failure");
            return;
        }
    };

    // Get initial state
    match send_command(&mut stream, IPCRequest::GetState) {
        Ok(response) => {
            if let Some(state) = response.state {
                print_status(&state);
            } else {
                eprintln!("Failed to get initial state");
            }
        },
        Err(e) => {
            eprintln!("Error getting initial state: {}", e);
            return;
        }
    }

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0].to_lowercase();

        match command.as_str() {
            "water" => {
                if parts.len() < 2 {
                    println!("Error: Missing amount parameter");
                    continue;
                }

                match parts[1].parse::<f32>() {
                    Ok(amount) => {
                        if amount < 0.0 || amount > 1.0 {
                            println!("Error: Amount must be between 0 and 1");
                            continue;
                        }

                        match send_command(&mut stream, IPCRequest::Water { amount }) {
                            Ok(response) => {
                                if response.success {
                                    print_events(&response.events);
                                    println!("{}", response.message);
                                } else {
                                    println!("Error: {}", response.message);
                                }
                            },
                            Err(e) => println!("Error communicating with server: {}", e),
                        }
                    },
                    Err(_) => println!("Error: Invalid amount value"),
                }
            },
            "light" => {
                if parts.len() < 2 {
                    println!("Error: Missing level parameter");
                    continue;
                }

                match parts[1].parse::<f32>() {
                    Ok(level) => {
                        if level < 0.0 || level > 1.0 {
                            println!("Error: Level must be between 0 and 1");
                            continue;
                        }

                        match send_command(&mut stream, IPCRequest::SetLight { level }) {
                            Ok(response) => {
                                if response.success {
                                    print_events(&response.events);
                                    println!("{}", response.message);
                                } else {
                                    println!("Error: {}", response.message);
                                }
                            },
                            Err(e) => println!("Error communicating with server: {}", e),
                        }
                    },
                    Err(_) => println!("Error: Invalid level value"),
                }
            },
            "temp" => {
                if parts.len() < 2 {
                    println!("Error: Missing temperature parameter");
                    continue;
                }

                match parts[1].parse::<f32>() {
                    Ok(celsius) => {
                        match send_command(&mut stream, IPCRequest::SetTemp { celsius }) {
                            Ok(response) => {
                                if response.success {
                                    print_events(&response.events);
                                    println!("{}", response.message);
                                } else {
                                    println!("Error: {}", response.message);
                                }
                            },
                            Err(e) => println!("Error communicating with server: {}", e),
                        }
                    },
                    Err(_) => println!("Error: Invalid temperature value"),
                }
            },
            "step" => {
                let seconds = if parts.len() > 1 {
                    match parts[1].parse::<u64>() {
                        Ok(s) => s,
                        Err(_) => {
                            println!("Error: Invalid seconds value, using default of 1");
                            1
                        }
                    }
                } else {
                    1
                };

                match send_command(&mut stream, IPCRequest::Step { seconds }) {
                    Ok(response) => {
                        if response.success {
                            print_events(&response.events);
                            println!("{}", response.message);
                        } else {
                            println!("Error: {}", response.message);
                        }
                    },
                    Err(e) => println!("Error communicating with server: {}", e),
                }
            },
            "status" => {
                match send_command(&mut stream, IPCRequest::GetState) {
                    Ok(response) => {
                        if response.success {
                            if let Some(state) = response.state {
                                print_status(&state);
                            }
                        } else {
                            println!("Error: {}", response.message);
                        }
                    },
                    Err(e) => println!("Error communicating with server: {}", e),
                }
            },
            "help" => {
                println!("Commands:");
                println!("  water <amount>     - Water the plant (amount between 0 and 1)");
                println!("  light <level>      - Set light level (between 0 and 1)");
                println!("  temp <celsius>     - Set temperature in Celsius");
                println!("  step [seconds]     - Advance simulation by specified seconds (default: 1)");
                println!("  status             - Show current plant status");
                println!("  help               - Show this help message");
                println!("  exit               - Exit the simulator");
            },
            "exit" => {
                println!("Exiting tomato42 simulator. Goodbye!");
                break;
            },
            _ => {
                println!("Unknown command: {}", command);
                println!("Type 'help' for a list of commands");
            }
        }
    }
}

/// Prints the current status of the tomato plant.
fn print_status(state: &SerializableTomatoState) {
    println!("\nTomato Plant Status:");
    println!("-------------------");
    println!("Time:          {} seconds", state.time_seconds);
    println!("Stage:         {}", state.stage);
    println!("Soil Moisture: {:.2}", state.soil_moisture);
    println!("Biomass:       {:.2}", state.biomass);
    println!("Stress:        {:.2}", state.stress);
    println!("Health:        {:.2}", state.health);
    println!("Temperature:   {:.2}°C", state.temperature);
    println!("Light Level:   {:.2}", state.light_level);
    println!();
}

/// Prints events that occurred during a simulation step.
fn print_events(events: &[SerializableTomatoEvent]) {
    if events.is_empty() {
        return;
    }

    println!("\nEvents:");
    for event in events {
        match event {
            SerializableTomatoEvent::StageChange { from, to } => {
                println!("  Plant advanced from {} to {} stage", from, to);
            },
            SerializableTomatoEvent::WiltRisk => {
                println!("  WARNING: Plant is at risk of wilting due to high stress!");
            },
            SerializableTomatoEvent::Death => {
                println!("  ALERT: Plant has died!");
            },
        }
    }
    println!();
}
