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

impl Default for TomatoState {
    fn default() -> Self {
        Self::new()
    }
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
    let growth_conditions = (new_state.soil_moisture > 0.3 && new_state.soil_moisture < 0.8)
        && (new_state.temperature > 15.0 && new_state.temperature < 30.0)
        && (new_state.light_level > 0.3);

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

    // --- Invariant tests ---

    #[test]
    fn test_bounds_preserved_after_step() {
        // Run many steps with various actions and verify bounds hold every time
        let actions = [
            Action::Water { amount: 1.0 },
            Action::Water { amount: 0.0 },
            Action::SetLight { level: 0.0 },
            Action::SetLight { level: 1.0 },
            Action::SetTemp { celsius: -20.0 },
            Action::SetTemp { celsius: 60.0 },
            Action::DoNothing,
        ];
        let mut state = TomatoState::new();
        let dt = Duration::from_secs(5);

        for action in actions.iter().cycle().take(50) {
            let result = step(state, *action, dt);
            state = result.state;

            assert!(state.soil_moisture >= 0.0 && state.soil_moisture <= 1.0,
                "soil_moisture out of bounds: {}", state.soil_moisture);
            assert!(state.biomass >= 0.0,
                "biomass negative: {}", state.biomass);
            assert!(state.stress >= 0.0 && state.stress <= 1.0,
                "stress out of bounds: {}", state.stress);
            assert!(state.health >= 0.0 && state.health <= 1.0,
                "health out of bounds: {}", state.health);
            assert!(state.light_level >= 0.0 && state.light_level <= 1.0,
                "light_level out of bounds: {}", state.light_level);
        }
    }

    #[test]
    fn test_determinism_identical_runs() {
        // Two identical input sequences must produce identical output
        let actions = vec![
            Action::Water { amount: 0.3 },
            Action::SetTemp { celsius: 25.0 },
            Action::SetLight { level: 0.6 },
            Action::DoNothing,
            Action::Water { amount: 0.1 },
            Action::DoNothing,
        ];
        let dt = Duration::from_secs(2);

        let run = |actions: &[Action]| -> TomatoState {
            let mut state = TomatoState::new();
            for &action in actions {
                state = step(state, action, dt).state;
            }
            state
        };

        let s1 = run(&actions);
        let s2 = run(&actions);

        assert_eq!(s1.time, s2.time);
        assert_eq!(s1.soil_moisture, s2.soil_moisture);
        assert_eq!(s1.biomass, s2.biomass);
        assert_eq!(s1.stress, s2.stress);
        assert_eq!(s1.health, s2.health);
        assert_eq!(s1.temperature, s2.temperature);
        assert_eq!(s1.light_level, s2.light_level);
        assert_eq!(s1.stage, s2.stage);
    }

    // --- Evapotranspiration ---

    #[test]
    fn test_moisture_decreases_over_time() {
        let state = TomatoState::new(); // moisture = 0.5
        let result = step(state, Action::DoNothing, Duration::from_secs(10));
        assert!(result.state.soil_moisture < 0.5);
    }

    // --- Stress mechanics ---

    #[test]
    fn test_low_moisture_causes_stress() {
        let mut state = TomatoState::new();
        state.soil_moisture = 0.05;
        // Run several steps to let stress accumulate
        for _ in 0..20 {
            state = step(state, Action::DoNothing, Duration::from_secs(1)).state;
        }
        assert!(state.stress > 0.0, "stress should be positive under dry conditions");
    }

    #[test]
    fn test_extreme_temp_causes_stress() {
        let mut state = TomatoState::new();
        state.temperature = 50.0;
        for _ in 0..20 {
            state = step(state, Action::DoNothing, Duration::from_secs(1)).state;
        }
        assert!(state.stress > 0.0, "stress should be positive under extreme heat");
    }

    #[test]
    fn test_sustained_high_stress_kills_plant() {
        let mut state = TomatoState::new();
        state.soil_moisture = 0.0;
        state.temperature = 50.0;
        state.light_level = 0.0;
        // Run many steps under terrible conditions
        for _ in 0..500 {
            state = step(state, Action::DoNothing, Duration::from_secs(1)).state;
            if state.stage == Stage::Dead {
                break;
            }
        }
        assert_eq!(state.stage, Stage::Dead, "plant should die under sustained extreme stress");
        assert_eq!(state.health, 0.0);
    }

    // --- Growth and stage progression ---

    #[test]
    fn test_growth_under_optimal_conditions() {
        let mut state = TomatoState::new();
        state.soil_moisture = 0.5;
        state.temperature = 22.0;
        state.light_level = 0.6;

        let initial_biomass = state.biomass;
        for _ in 0..10 {
            state = step(state, Action::DoNothing, Duration::from_secs(1)).state;
        }
        assert!(state.biomass > initial_biomass, "biomass should increase under optimal conditions");
    }

    #[test]
    fn test_no_growth_without_light() {
        let mut state = TomatoState::new();
        state.light_level = 0.0;
        state.soil_moisture = 0.5;
        state.temperature = 22.0;

        let initial_biomass = state.biomass;
        for _ in 0..10 {
            // Keep resetting light each step since it persists
            state = step(state, Action::SetLight { level: 0.0 }, Duration::from_secs(1)).state;
        }
        assert_eq!(state.biomass, initial_biomass, "biomass should not increase without light");
    }

    #[test]
    fn test_full_lifecycle_seed_to_fruiting() {
        let mut state = TomatoState::new();
        let dt = Duration::from_secs(1);

        // Keep conditions optimal and water regularly
        for _ in 0..2000 {
            // Maintain optimal conditions each step
            state.temperature = 22.0;
            state.light_level = 0.6;

            // Water whenever moisture drops below 0.4 to stay in growth range
            let action = if state.soil_moisture < 0.4 {
                Action::Water { amount: 0.5 }
            } else {
                Action::DoNothing
            };
            let result = step(state, action, dt);
            state = result.state;

            if state.stage == Stage::Fruiting {
                break;
            }
        }

        assert_eq!(state.stage, Stage::Fruiting,
            "plant should reach Fruiting stage under sustained optimal conditions (biomass={})", state.biomass);
    }

    #[test]
    fn test_stage_order_is_monotonic() {
        // Stages only advance forward (never regress), except to Dead
        let mut state = TomatoState::new();
        let dt = Duration::from_secs(1);
        let mut seen_stages = vec![state.stage];

        for _ in 0..2000 {
            state.temperature = 22.0;
            state.light_level = 0.6;
            let action = if state.soil_moisture < 0.4 {
                Action::Water { amount: 0.5 }
            } else {
                Action::DoNothing
            };
            let result = step(state, action, dt);
            state = result.state;

            if state.stage != *seen_stages.last().unwrap() {
                seen_stages.push(state.stage);
            }
            if state.stage == Stage::Fruiting || state.stage == Stage::Dead {
                break;
            }
        }

        let order = [Stage::Seed, Stage::Seedling, Stage::Vegetative, Stage::Flowering, Stage::Fruiting];
        // Check seen stages follow the order (each must appear after its predecessor)
        for window in seen_stages.windows(2) {
            let prev_idx = order.iter().position(|s| *s == window[0]);
            let next_idx = order.iter().position(|s| *s == window[1]);
            match (prev_idx, next_idx) {
                (Some(p), Some(n)) => assert!(n > p, "stage went backwards: {:?} -> {:?}", window[0], window[1]),
                (_, None) => assert_eq!(window[1], Stage::Dead, "unexpected stage: {:?}", window[1]),
                _ => {}
            }
        }
    }

    // --- Dead state invariants ---

    #[test]
    fn test_dead_plant_cannot_grow() {
        let mut state = TomatoState::new();
        state.stage = Stage::Dead;
        state.health = 0.0;
        let initial_biomass = state.biomass;

        // Try optimal conditions
        state.soil_moisture = 0.5;
        state.temperature = 22.0;
        state.light_level = 0.6;

        for _ in 0..20 {
            state = step(state, Action::Water { amount: 0.5 }, Duration::from_secs(1)).state;
        }

        assert_eq!(state.stage, Stage::Dead);
        assert_eq!(state.biomass, initial_biomass);
    }

    #[test]
    fn test_dead_plant_health_stays_zero() {
        let mut state = TomatoState::new();
        state.stage = Stage::Dead;
        state.health = 0.0;

        for _ in 0..20 {
            state = step(state, Action::DoNothing, Duration::from_secs(1)).state;
        }

        assert_eq!(state.health, 0.0);
    }

    // --- Action tests ---

    #[test]
    fn test_set_light_takes_effect() {
        let state = TomatoState::new();
        let result = step(state, Action::SetLight { level: 0.9 }, Duration::from_secs(0));
        assert!((result.state.light_level - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_temp_takes_effect() {
        let state = TomatoState::new();
        let result = step(state, Action::SetTemp { celsius: 30.0 }, Duration::from_secs(0));
        assert!((result.state.temperature - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_water_at_max_moisture_is_noop() {
        let mut state = TomatoState::new();
        state.soil_moisture = 1.0;
        let result = step(state, Action::Water { amount: 0.5 }, Duration::from_secs(0));
        // effective_amount = min(0.5, 1.0 - 1.0) * 0.8 = 0
        assert!((result.state.soil_moisture - 1.0).abs() < f32::EPSILON);
    }

    // --- Event tests ---

    #[test]
    fn test_death_event_emitted() {
        let mut state = TomatoState::new();
        state.health = 0.01;
        state.stress = 1.0;
        // One step with max stress should push health to 0 and emit Death
        let result = step(state, Action::DoNothing, Duration::from_secs(10));
        let has_death = result.events.iter().any(|e| matches!(e, Event::Death));
        assert!(has_death, "Death event should be emitted when health reaches 0");
        assert_eq!(result.state.stage, Stage::Dead);
    }

    #[test]
    fn test_stage_change_event_emitted() {
        let mut state = TomatoState::new();
        state.biomass = 0.99;
        state.soil_moisture = 0.5;
        state.temperature = 22.0;
        state.light_level = 0.6;

        // Step enough to push biomass past 1.0 (Seed -> Seedling threshold)
        let result = step(state, Action::DoNothing, Duration::from_secs(1));
        let has_stage_change = result.events.iter().any(|e| matches!(e, Event::StageChange { .. }));
        assert!(has_stage_change, "StageChange event should be emitted at threshold");
        assert_eq!(result.state.stage, Stage::Seedling);
    }

    #[test]
    fn test_wilt_risk_event_under_high_stress() {
        let mut state = TomatoState::new();
        state.stress = 0.8; // Already high stress
        state.soil_moisture = 0.0;
        state.temperature = 50.0;
        state.light_level = 0.0;

        let result = step(state, Action::DoNothing, Duration::from_secs(1));
        let has_wilt = result.events.iter().any(|e| matches!(e, Event::WiltRisk));
        assert!(has_wilt, "WiltRisk event should be emitted when stress > 0.7");
    }
}
