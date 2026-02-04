# Tomato42 Core - Business Logic

A deterministic tomato plant growth simulator. This document describes the simulation model and its parameters.

## Plant State

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| `time` | Duration | >= 0 | Elapsed simulation time |
| `stage` | Stage | enum | Current growth stage |
| `soil_moisture` | f32 | [0, 1] | Water content in soil |
| `biomass` | f32 | >= 0 | Total plant mass (arbitrary units) |
| `stress` | f32 | [0, 1] | Current stress level |
| `health` | f32 | [0, 1] | Plant vitality (0 = dead) |
| `temperature` | f32 | any | Ambient temperature in Celsius |
| `light_level` | f32 | [0, 1] | Light intensity |

## Growth Stages

```
Seed -> Seedling -> Vegetative -> Flowering -> Fruiting
                         |
                         v
                       Dead (absorbing state)
```

| Stage | Biomass Threshold | Growth Rate |
|-------|-------------------|-------------|
| Seed | 0 | 0.2 |
| Seedling | >= 1.0 | 0.5 |
| Vegetative | >= 5.0 | 1.0 |
| Flowering | >= 20.0 | 0.7 |
| Fruiting | >= 50.0 | 0.3 |
| Dead | health <= 0 | 0.0 |

## Actions

| Action | Effect |
|--------|--------|
| `Water { amount }` | Increases soil_moisture by `amount * 0.8` (capped at saturation) |
| `SetLight { level }` | Sets light_level directly |
| `SetTemp { celsius }` | Sets temperature directly |
| `DoNothing` | No immediate effect |

## Environmental Dynamics

### Moisture Loss (per second)
```
evaporation = 0.05
transpiration = 0.02 * sqrt(biomass)
total_loss = evaporation + transpiration
```

### Stress Calculation

Stress is computed from three sources and smoothed over time:

**Moisture Stress:**
| Condition | Stress |
|-----------|--------|
| moisture < 0.2 | 0.5 * (0.2 - moisture) / 0.2 |
| moisture > 0.8 | 0.3 * (moisture - 0.8) / 0.2 |
| otherwise | 0.0 |

**Temperature Stress:**
| Condition | Stress |
|-----------|--------|
| temp < 10C | 0.5 * (10 - temp) / 10 |
| temp > 35C | 0.5 * (temp - 35) / 15 |
| otherwise | 0.0 |

**Light Stress:**
| Condition | Stress |
|-----------|--------|
| light < 0.2 | 0.3 * (0.2 - light) / 0.2 |
| light > 0.9 | 0.2 * (light - 0.9) / 0.1 |
| otherwise | 0.0 |

**Stress Update:**
```
target_stress = min(moisture_stress + temp_stress + light_stress, 1.0)
new_stress = old_stress * 0.9 + target_stress * 0.1
```

### Health Dynamics

| Condition | Effect (per second) |
|-----------|---------------------|
| stress > 0.5 | health -= (stress - 0.5) * 0.1 |
| stress <= 0.5 | health += (1.0 - health) * 0.01 |

**WiltRisk event** is emitted when stress > 0.7

### Growth Conditions

Growth occurs only when ALL conditions are met:
- 0.3 < soil_moisture < 0.8
- 15C < temperature < 30C
- light_level > 0.3

**Growth Formula:**
```
biomass += growth_rate * (1 - stress) * health * dt_seconds
```

## Events

| Event | Trigger |
|-------|---------|
| `StageChange { from, to }` | Biomass threshold reached or death |
| `WiltRisk` | stress > 0.7 |
| `Death` | health drops to 0 |

## Optimal Growing Conditions

For maximum growth rate:
- **Moisture:** 0.3 - 0.8 (target ~0.5)
- **Temperature:** 15C - 30C (target ~22C)
- **Light:** 0.3 - 0.9 (target ~0.6)

## Determinism

The simulation is fully deterministic: given the same initial state, action sequence, and time steps, the output will always be identical. This enables:
- Reproducible experiments
- Regression testing
- State synchronization across clients
