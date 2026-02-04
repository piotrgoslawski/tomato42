//! Shared IPC protocol types for the tomato42 plant simulator.
//!
//! This crate provides the common data types used for communication between
//! the tomato42-ipc server and its clients (CLI, TUI).

use serde::{Deserialize, Serialize};

/// Default port for the IPC server.
pub const DEFAULT_PORT: u16 = 8043;

/// Default host for the IPC server.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Request messages sent from clients to the IPC server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPCRequest {
    /// Get the current state of the tomato plant.
    GetState,
    /// Advance the simulation by the specified number of seconds.
    Step { seconds: u64 },
    /// Water the plant with the specified amount (0.0 to 1.0).
    Water { amount: f32 },
    /// Set the light level (0.0 to 1.0).
    SetLight { level: f32 },
    /// Set the temperature in Celsius.
    SetTemp { celsius: f32 },
}

/// Response messages sent from the IPC server to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// A human-readable message describing the result.
    pub message: String,
    /// The current state of the tomato plant (if available).
    pub state: Option<SerializableTomatoState>,
    /// Events that occurred as a result of the request.
    pub events: Vec<SerializableTomatoEvent>,
}

/// Serializable representation of the tomato plant state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTomatoState {
    /// Time elapsed in seconds since the simulation started.
    pub time_seconds: u64,
    /// Current growth stage of the plant.
    pub stage: String,
    /// Soil moisture level (0.0 to 1.0).
    pub soil_moisture: f32,
    /// Plant biomass.
    pub biomass: f32,
    /// Plant stress level (0.0 to 1.0).
    pub stress: f32,
    /// Plant health (0.0 to 1.0).
    pub health: f32,
    /// Current temperature in Celsius.
    pub temperature: f32,
    /// Current light level (0.0 to 1.0).
    pub light_level: f32,
}

/// Serializable representation of tomato plant events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableTomatoEvent {
    /// The plant changed growth stages.
    StageChange { from: String, to: String },
    /// The plant is at risk of wilting due to high stress.
    WiltRisk,
    /// The plant has died.
    Death,
}
