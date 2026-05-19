import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';

import 'ipc_client.dart';
import 'protocol.dart';

const int historyCapacity = 128;

class HistoryPoint {
  final double time;
  final double value;
  const HistoryPoint(this.time, this.value);
}

class AppState extends ChangeNotifier {
  final IpcClient client;

  TomatoState? _state;
  String? _connectionError;
  String? _lastMessage;
  bool _busy = false;
  bool _autoStep = false;
  Timer? _autoStepTimer;

  final Queue<HistoryPoint> _soilMoisture = Queue();
  final Queue<HistoryPoint> _stress = Queue();
  final Queue<HistoryPoint> _health = Queue();
  final Queue<HistoryPoint> _biomass = Queue();
  final List<TomatoEvent> _events = [];

  double waterAmount = 0.5;
  double lightLevel = 0.5;
  double temperature = 20.0;

  AppState({required this.client});

  TomatoState? get state => _state;
  String? get connectionError => _connectionError;
  String? get lastMessage => _lastMessage;
  bool get busy => _busy;
  bool get autoStep => _autoStep;
  bool get isConnected => client.isConnected && _connectionError == null;

  List<HistoryPoint> get soilMoistureHistory => _soilMoisture.toList();
  List<HistoryPoint> get stressHistory => _stress.toList();
  List<HistoryPoint> get healthHistory => _health.toList();
  List<HistoryPoint> get biomassHistory => _biomass.toList();
  List<TomatoEvent> get events => List.unmodifiable(_events);

  Future<void> connectAndFetch() async {
    _connectionError = null;
    notifyListeners();
    try {
      await client.connect();
      await _send(const GetStateRequest());
    } on Exception catch (e) {
      _connectionError = e.toString();
      notifyListeners();
    }
  }

  Future<void> step() => _send(const StepRequest(1));
  Future<void> water() => _send(WaterRequest(waterAmount));
  Future<void> setLight() => _send(SetLightRequest(lightLevel));
  Future<void> setTemp() => _send(SetTempRequest(temperature));

  void toggleAutoStep() {
    _autoStep = !_autoStep;
    _autoStepTimer?.cancel();
    if (_autoStep) {
      _autoStepTimer = Timer.periodic(const Duration(milliseconds: 500), (_) {
        if (!_busy && isConnected) step();
      });
    } else {
      _autoStepTimer = null;
    }
    notifyListeners();
  }

  void adjustWater(double delta) {
    waterAmount = (waterAmount + delta).clamp(0.0, 1.0);
    notifyListeners();
  }

  void adjustLight(double delta) {
    lightLevel = (lightLevel + delta).clamp(0.0, 1.0);
    notifyListeners();
  }

  void adjustTemp(double delta) {
    temperature += delta;
    notifyListeners();
  }

  Future<void> _send(IpcRequest request) async {
    if (_busy) return;
    _busy = true;
    notifyListeners();
    try {
      final response = await client.send(request);
      _lastMessage = response.message;
      if (response.state != null) {
        _state = response.state;
        _appendHistory(response.state!);
      }
      _events
        ..clear()
        ..addAll(response.events);
    } on Exception catch (e) {
      _connectionError = e.toString();
      _autoStep = false;
      _autoStepTimer?.cancel();
      _autoStepTimer = null;
      await client.close();
    } finally {
      _busy = false;
      notifyListeners();
    }
  }

  void _appendHistory(TomatoState s) {
    final t = s.timeSeconds.toDouble();
    _pushBounded(_soilMoisture, HistoryPoint(t, s.soilMoisture));
    _pushBounded(_stress, HistoryPoint(t, s.stress));
    _pushBounded(_health, HistoryPoint(t, s.health));
    _pushBounded(_biomass, HistoryPoint(t, s.biomass));
  }

  void _pushBounded(Queue<HistoryPoint> q, HistoryPoint p) {
    q.add(p);
    while (q.length > historyCapacity) {
      q.removeFirst();
    }
  }

  @override
  void dispose() {
    _autoStepTimer?.cancel();
    client.close();
    super.dispose();
  }
}
