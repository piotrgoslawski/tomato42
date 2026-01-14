//! CLI driver for the tomato42 plant simulator.
//!
//! This application provides a command-line interface for manual control
//! of the tomato plant simulator, allowing step-by-step simulation.

use std::io::{self, Write};
use std::time::Duration;
use tomato42_core::{Action, Event, TomatoState, step};

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

    let mut state = TomatoState::new();
    print_status(&state);

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
                        
                        let result = step(state, Action::Water { amount }, Duration::from_secs(0));
                        state = result.state;
                        print_events(&result.events);
                        println!("Watered plant with amount: {:.2}", amount);
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
                        
                        let result = step(state, Action::SetLight { level }, Duration::from_secs(0));
                        state = result.state;
                        print_events(&result.events);
                        println!("Set light level to: {:.2}", level);
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
                        let result = step(state, Action::SetTemp { celsius }, Duration::from_secs(0));
                        state = result.state;
                        print_events(&result.events);
                        println!("Set temperature to: {:.2}°C", celsius);
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
                
                let result = step(state, Action::DoNothing, Duration::from_secs(seconds));
                state = result.state;
                print_events(&result.events);
                println!("Advanced simulation by {} seconds", seconds);
            },
            "status" => {
                print_status(&state);
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
fn print_status(state: &TomatoState) {
    println!("\nTomato Plant Status:");
    println!("-------------------");
    println!("Time:          {} seconds", state.time.as_secs());
    println!("Stage:         {:?}", state.stage);
    println!("Soil Moisture: {:.2}", state.soil_moisture);
    println!("Biomass:       {:.2}", state.biomass);
    println!("Stress:        {:.2}", state.stress);
    println!("Health:        {:.2}", state.health);
    println!("Temperature:   {:.2}°C", state.temperature);
    println!("Light Level:   {:.2}", state.light_level);
    println!();
}

/// Prints events that occurred during a simulation step.
fn print_events(events: &[Event]) {
    if events.is_empty() {
        return;
    }
    
    println!("\nEvents:");
    for event in events {
        match event {
            Event::StageChange { from, to } => {
                println!("  Plant advanced from {:?} to {:?} stage", from, to);
            },
            Event::WiltRisk => {
                println!("  WARNING: Plant is at risk of wilting due to high stress!");
            },
            Event::Death => {
                println!("  ALERT: Plant has died!");
            },
        }
    }
    println!();
}