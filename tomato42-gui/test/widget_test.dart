import 'package:flutter_test/flutter_test.dart';
import 'package:tomato42_gui/protocol.dart';

void main() {
  test('IpcRequest serializes in serde externally-tagged form', () {
    expect(const GetStateRequest().toJson(), 'GetState');
    expect(const StepRequest(3).toJson(), {
      'Step': {'seconds': 3},
    });
    expect(const WaterRequest(0.5).toJson(), {
      'Water': {'amount': 0.5},
    });
    expect(const SetLightRequest(0.7).toJson(), {
      'SetLight': {'level': 0.7},
    });
    expect(const SetTempRequest(22.5).toJson(), {
      'SetTemp': {'celsius': 22.5},
    });
  });

  test('IpcResponse parses a server reply with state and events', () {
    final response = IpcResponse.fromJson({
      'success': true,
      'message': 'Watered plant with amount: 0.50',
      'state': {
        'time_seconds': 12,
        'stage': 'Seedling',
        'soil_moisture': 0.8,
        'biomass': 2.5,
        'stress': 0.1,
        'health': 0.95,
        'temperature': 22.0,
        'light_level': 0.6,
      },
      'events': [
        {
          'StageChange': {'from': 'Seed', 'to': 'Seedling'},
        },
        'WiltRisk',
      ],
    });
    expect(response.success, isTrue);
    expect(response.state, isNotNull);
    expect(response.state!.stage, Stage.seedling);
    expect(response.state!.soilMoisture, closeTo(0.8, 1e-9));
    expect(response.events.length, 2);
    expect(response.events[0], isA<StageChangeEvent>());
    expect(response.events[1], isA<WiltRiskEvent>());
  });
}
