// Mirrors the wire types in `tomato42-protocol` (Rust crate).
// IPCRequest is a serde externally-tagged enum: unit variant GetState
// serializes as the JSON string "GetState"; struct variants serialize
// as {"Variant": {field: value}}.

const int defaultPort = 8043;
const String defaultHost = '127.0.0.1';

sealed class IpcRequest {
  const IpcRequest();
  Object toJson();
}

class GetStateRequest extends IpcRequest {
  const GetStateRequest();
  @override
  Object toJson() => 'GetState';
}

class StepRequest extends IpcRequest {
  final int seconds;
  const StepRequest(this.seconds);
  @override
  Object toJson() => {
        'Step': {'seconds': seconds},
      };
}

class WaterRequest extends IpcRequest {
  final double amount;
  const WaterRequest(this.amount);
  @override
  Object toJson() => {
        'Water': {'amount': amount},
      };
}

class SetLightRequest extends IpcRequest {
  final double level;
  const SetLightRequest(this.level);
  @override
  Object toJson() => {
        'SetLight': {'level': level},
      };
}

class SetTempRequest extends IpcRequest {
  final double celsius;
  const SetTempRequest(this.celsius);
  @override
  Object toJson() => {
        'SetTemp': {'celsius': celsius},
      };
}

class IpcResponse {
  final bool success;
  final String message;
  final TomatoState? state;
  final List<TomatoEvent> events;

  IpcResponse({
    required this.success,
    required this.message,
    required this.state,
    required this.events,
  });

  factory IpcResponse.fromJson(Map<String, dynamic> json) => IpcResponse(
        success: json['success'] as bool,
        message: json['message'] as String,
        state: json['state'] == null
            ? null
            : TomatoState.fromJson(json['state'] as Map<String, dynamic>),
        events: (json['events'] as List<dynamic>)
            .map((e) => TomatoEvent.fromJson(e))
            .toList(),
      );
}

enum Stage { seed, seedling, vegetative, flowering, fruiting, dead, unknown }

Stage parseStage(String s) {
  switch (s) {
    case 'Seed':
      return Stage.seed;
    case 'Seedling':
      return Stage.seedling;
    case 'Vegetative':
      return Stage.vegetative;
    case 'Flowering':
      return Stage.flowering;
    case 'Fruiting':
      return Stage.fruiting;
    case 'Dead':
      return Stage.dead;
    default:
      return Stage.unknown;
  }
}

String stageLabel(Stage s) {
  switch (s) {
    case Stage.seed:
      return 'Seed';
    case Stage.seedling:
      return 'Seedling';
    case Stage.vegetative:
      return 'Vegetative';
    case Stage.flowering:
      return 'Flowering';
    case Stage.fruiting:
      return 'Fruiting';
    case Stage.dead:
      return 'Dead';
    case Stage.unknown:
      return '?';
  }
}

class TomatoState {
  final int timeSeconds;
  final Stage stage;
  final double soilMoisture;
  final double biomass;
  final double stress;
  final double health;
  final double temperature;
  final double lightLevel;

  TomatoState({
    required this.timeSeconds,
    required this.stage,
    required this.soilMoisture,
    required this.biomass,
    required this.stress,
    required this.health,
    required this.temperature,
    required this.lightLevel,
  });

  factory TomatoState.fromJson(Map<String, dynamic> json) => TomatoState(
        timeSeconds: (json['time_seconds'] as num).toInt(),
        stage: parseStage(json['stage'] as String),
        soilMoisture: (json['soil_moisture'] as num).toDouble(),
        biomass: (json['biomass'] as num).toDouble(),
        stress: (json['stress'] as num).toDouble(),
        health: (json['health'] as num).toDouble(),
        temperature: (json['temperature'] as num).toDouble(),
        lightLevel: (json['light_level'] as num).toDouble(),
      );
}

sealed class TomatoEvent {
  const TomatoEvent();

  factory TomatoEvent.fromJson(dynamic json) {
    if (json is String) {
      switch (json) {
        case 'WiltRisk':
          return const WiltRiskEvent();
        case 'Death':
          return const DeathEvent();
      }
    }
    if (json is Map<String, dynamic>) {
      if (json.containsKey('StageChange')) {
        final inner = json['StageChange'] as Map<String, dynamic>;
        return StageChangeEvent(
          from: parseStage(inner['from'] as String),
          to: parseStage(inner['to'] as String),
        );
      }
      if (json.containsKey('WiltRisk')) return const WiltRiskEvent();
      if (json.containsKey('Death')) return const DeathEvent();
    }
    return const UnknownEvent();
  }
}

class StageChangeEvent extends TomatoEvent {
  final Stage from;
  final Stage to;
  const StageChangeEvent({required this.from, required this.to});
}

class WiltRiskEvent extends TomatoEvent {
  const WiltRiskEvent();
}

class DeathEvent extends TomatoEvent {
  const DeathEvent();
}

class UnknownEvent extends TomatoEvent {
  const UnknownEvent();
}
