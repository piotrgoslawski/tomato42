//! Core simulation logic for the tomato42 plant simulator.
//! 
//! This crate provides a deterministic tomato plant simulator with no IO, no async, and no threads.
//! It exposes a step(state, action, dt) -> StepResult API for state transitions.

use std::time::Duration;

/// Represents the growth stage of a tomato plant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Seed,
    Seedling,
    Vegetative,
    Flowering,
    Fruiting,
    Dead,
}

/// Represents an action that can be taken on the tomato plant.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Water { amount: f32 },
    SetLight { level: f32 },
    SetTemp { celsius: f32 },
    DoNothing,
}

/// Represents the state of a tomato plant.
#[derive(Debug, Clone)]
pub struct TomatoState {
    /// Time elapsed since the start of the simulation.
    pub time: Duration,
    /// Current growth stage of the plant.
    pub stage: Stage,
    /// Soil moisture level, in range [0, 1].
    pub soil_moisture: f32,
    /// Total biomass of the plant, must be >= 0.
    pub biomass: f32,
    /// Stress level of the plant, in range [0, 1].
    pub stress: f32,
    /// Health level of the plant, in range [0, 1].
    pub health: f32,
    /// Current temperature in Celsius.
    pub temperature: f32,
    /// Current light level in range [0, 1].
    pub light_level: f32,
}

/// Represents events that can occur during a simulation step.
#[derive(Debug, Clone)]
pub enum Event {
    StageChange { from: Stage, to: Stage },
    WiltRisk,
    Death,
}

/// Result of a simulation step.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// New state after the step.
    pub state: TomatoState,
    /// Events that occurred during the step.
    pub events: Vec<Event>,
}

impl TomatoState {
    /// Creates a new tomato state with default values.
    pub fn new() -> Self {
        Self {
            time: Duration::from_secs(0),
            stage: Stage::Seed,
            soil_moisture: 0.5,
            biomass: 0.0,
            stress: 0.0,
            health: 1.0,
            temperature: 20.0,
            light_level: 0.5,
        }
    }

    /// Clamps all state values to their valid ranges.
    fn clamp_values(&mut self) {
        self.soil_moisture = self.soil_moisture.clamp(0.0, 1.0);
        self.biomass = self.biomass.max(0.0);
        self.stress = self.stress.clamp(0.0, 1.0);
        self.health = self.health.clamp(0.0, 1.0);
        self.light_level = self.light_level.clamp(0.0, 1.0);
    }

    /// Checks if the plant should advance to the next stage based on biomass and time.
    fn check_stage_advancement(&self) -> Option<Stage> {
        if self.health <= 0.0 {
            return Some(Stage::Dead);
        }

        match self.stage {
            Stage::Seed if self.biomass >= 1.0 => Some(Stage::Seedling),
            Stage::Seedling if self.biomass >= 5.0 => Some(Stage::Vegetative),
            Stage::Vegetative if self.biomass >= 20.0 => Some(Stage::Flowering),
            Stage::Flowering if self.biomass >= 50.0 => Some(Stage::Fruiting),
            Stage::Dead => Some(Stage::Dead), // Dead state is absorbing
            _ => None,
        }
    }
}

/// Simulates a single step of the tomato plant's growth.
///
/// # Arguments
///
/// * `state` - The current state of the tomato plant.
/// * `action` - The action to take during this step.
/// * `dt` - The time step duration.
///
/// # Returns
///
/// A `StepResult` containing the new state and any events that occurred.
pub fn step(state: TomatoState, action: Action, dt: Duration) -> StepResult {
    let mut new_state = state.clone();
    let mut events = Vec::new();

    // Update time
    new_state.time += dt;
    let dt_seconds = dt.as_secs_f32();

    // Apply action
    match action {
        Action::Water { amount } => {
            // Increase soil moisture with saturation and drainage
            let effective_amount = amount.min(1.0 - new_state.soil_moisture) * 0.8;
            new_state.soil_moisture += effective_amount;
        }
        Action::SetLight { level } => {
            new_state.light_level = level;
        }
        Action::SetTemp { celsius } => {
            new_state.temperature = celsius;
        }
        Action::DoNothing => {}
    }

    // Skip dynamics if the plant is dead
    if new_state.stage == Stage::Dead {
        new_state.clamp_values();
        return StepResult {
            state: new_state,
            events,
        };
    }

    // Moisture decreases over time (evapotranspiration)
    let evaporation_rate = 0.05 * dt_seconds;
    let transpiration_rate = if new_state.biomass > 0.0 {
        0.02 * new_state.biomass.sqrt() * dt_seconds
    } else {
        0.0
    };
    new_state.soil_moisture -= evaporation_rate + transpiration_rate;

    // Calculate stress based on conditions
    let moisture_stress = if new_state.soil_moisture < 0.2 {
        0.5 * (0.2 - new_state.soil_moisture) / 0.2
    } else if new_state.soil_moisture > 0.8 {
        0.3 * (new_state.soil_moisture - 0.8) / 0.2
    } else {
        0.0
    };

    let temp_stress = if new_state.temperature < 10.0 {
        0.5 * (10.0 - new_state.temperature) / 10.0
    } else if new_state.temperature > 35.0 {
        0.5 * (new_state.temperature - 35.0) / 15.0
    } else {
        0.0
    };

    let light_stress = if new_state.light_level < 0.2 {
        0.3 * (0.2 - new_state.light_level) / 0.2
    } else if new_state.light_level > 0.9 {
        0.2 * (new_state.light_level - 0.9) / 0.1
    } else {
        0.0
    };

    // Update stress level
    let target_stress = (moisture_stress + temp_stress + light_stress).min(1.0);
    new_state.stress = new_state.stress * 0.9 + target_stress * 0.1;

    // Sustained stress reduces health
    if new_state.stress > 0.5 {
        new_state.health -= (new_state.stress - 0.5) * 0.1 * dt_seconds;
        
        if new_state.stress > 0.7 {
            events.push(Event::WiltRisk);
        }
    } else {
        // Slight health recovery when stress is low
        new_state.health += (1.0 - new_state.health) * 0.01 * dt_seconds;
    }

    // Growth happens when moisture & temp are in range
    let growth_conditions = 
        (new_state.soil_moisture > 0.3 && new_state.soil_moisture < 0.8) &&
        (new_state.temperature > 15.0 && new_state.temperature < 30.0) &&
        (new_state.light_level > 0.3);
    
    if growth_conditions {
        let growth_rate = match new_state.stage {
            Stage::Seed => 0.2,
            Stage::Seedling => 0.5,
            Stage::Vegetative => 1.0,
            Stage::Flowering => 0.7,
            Stage::Fruiting => 0.3,
            Stage::Dead => 0.0,
        };
        
        let stress_factor = 1.0 - new_state.stress;
        let health_factor = new_state.health;
        
        new_state.biomass += growth_rate * stress_factor * health_factor * dt_seconds;
    }

    // Check for death
    if new_state.health <= 0.0 {
        new_state.health = 0.0;
        if new_state.stage != Stage::Dead {
            events.push(Event::Death);
            events.push(Event::StageChange {
                from: new_state.stage,
                to: Stage::Dead,
            });
            new_state.stage = Stage::Dead;
        }
    } else {
        // Check for stage advancement
        if let Some(new_stage) = new_state.check_stage_advancement() {
            if new_stage != new_state.stage {
                events.push(Event::StageChange {
                    from: new_state.stage,
                    to: new_stage,
                });
                new_state.stage = new_stage;
            }
        }
    }

    // Clamp all values to valid ranges
    new_state.clamp_values();

    StepResult {
        state: new_state,
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_bounds() {
        let mut state = TomatoState::new();
        
        // Test extreme values
        state.soil_moisture = 2.0;
        state.biomass = -1.0;
        state.stress = 1.5;
        state.health = -0.5;
        
        state.clamp_values();
        
        assert!(state.soil_moisture <= 1.0);
        assert!(state.soil_moisture >= 0.0);
        assert!(state.biomass >= 0.0);
        assert!(state.stress <= 1.0);
        assert!(state.stress >= 0.0);
        assert!(state.health <= 1.0);
        assert!(state.health >= 0.0);
    }

    #[test]
    fn test_causality_watering() {
        let state = TomatoState::new();
        let action = Action::Water { amount: 0.5 };
        let dt = Duration::from_secs(1);
        
        let result = step(state, action, dt);
        
        assert!(result.state.soil_moisture > 0.5); // Initial moisture was 0.5
    }

    #[test]
    fn test_dead_state_is_absorbing() {
        let mut state = TomatoState::new();
        state.stage = Stage::Dead;
        state.health = 0.0;
        
        let action = Action::Water { amount: 1.0 };
        let dt = Duration::from_secs(1);
        
        let result = step(state, action, dt);
        
        assert_eq!(result.state.stage, Stage::Dead);
        assert_eq!(result.state.health, 0.0);
    }

    #[test]
    fn test_deterministic_regression() {
        // Fixed sequence of inputs
        let mut state = TomatoState::new();
        let dt = Duration::from_secs(1);
        
        // Step 1: Water the plant
        let result = step(state, Action::Water { amount: 0.3 }, dt);
        state = result.state;
        assert!(state.soil_moisture > 0.5);
        
        // Step 2: Set temperature
        let result = step(state, Action::SetTemp { celsius: 25.0 }, dt);
        state = result.state;
        assert_eq!(state.temperature, 25.0);
        
        // Step 3: Set light
        let result = step(state, Action::SetLight { level: 0.7 }, dt);
        state = result.state;
        assert_eq!(state.light_level, 0.7);
        
        // Step 4: Do nothing and let time pass
        let result = step(state, Action::DoNothing, dt);
        state = result.state;
        assert!(state.soil_moisture < 0.8); // Moisture should decrease
    }
}