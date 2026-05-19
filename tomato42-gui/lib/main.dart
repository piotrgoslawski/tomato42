import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'app_state.dart';
import 'ipc_client.dart';
import 'protocol.dart';

void main() {
  runApp(const TomatoApp());
}

class TomatoApp extends StatefulWidget {
  const TomatoApp({super.key});

  @override
  State<TomatoApp> createState() => _TomatoAppState();
}

class _TomatoAppState extends State<TomatoApp> {
  late final AppState _appState;

  @override
  void initState() {
    super.initState();
    _appState = AppState(client: IpcClient());
    _appState.connectAndFetch();
  }

  @override
  void dispose() {
    _appState.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'tomato42 GUI',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.green,
          brightness: Brightness.dark,
        ),
      ),
      home: HomePage(appState: _appState),
    );
  }
}

class HomePage extends StatelessWidget {
  final AppState appState;
  const HomePage({super.key, required this.appState});

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: appState,
      builder: (context, _) {
        return Scaffold(
          appBar: AppBar(
            title: const Text('tomato42 — Deterministic Tomato Plant Simulator'),
            actions: [_ConnectionChip(appState: appState)],
          ),
          body: appState.connectionError != null && appState.state == null
              ? _ErrorView(appState: appState)
              : _MainView(appState: appState),
        );
      },
    );
  }
}

class _ConnectionChip extends StatelessWidget {
  final AppState appState;
  const _ConnectionChip({required this.appState});

  @override
  Widget build(BuildContext context) {
    final connected = appState.isConnected;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Chip(
        avatar: Icon(
          connected ? Icons.check_circle : Icons.error,
          color: connected ? Colors.greenAccent : Colors.redAccent,
          size: 18,
        ),
        label: Text(connected ? 'Connected' : 'Disconnected'),
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  final AppState appState;
  const _ErrorView({required this.appState});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.cloud_off, size: 64, color: Colors.redAccent),
              const SizedBox(height: 16),
              Text(
                'Cannot reach the tomato42-ipc server',
                style: Theme.of(context).textTheme.titleLarge,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(
                'Start it with `cargo run --bin tomato42-ipc` and retry.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
              const SizedBox(height: 16),
              SelectableText(
                appState.connectionError ?? '',
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                  fontFamily: 'monospace',
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              FilledButton.icon(
                onPressed: appState.busy ? null : appState.connectAndFetch,
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MainView extends StatefulWidget {
  final AppState appState;
  const _MainView({required this.appState});

  @override
  State<_MainView> createState() => _MainViewState();
}

class _MainViewState extends State<_MainView> {
  final _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
  }

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final s = widget.appState;
    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.space) {
      s.step();
    } else if (key == LogicalKeyboardKey.keyA) {
      s.toggleAutoStep();
    } else if (key == LogicalKeyboardKey.keyW) {
      s.water();
    } else if (key == LogicalKeyboardKey.keyL) {
      s.setLight();
    } else if (key == LogicalKeyboardKey.keyT) {
      s.setTemp();
    } else if (key == LogicalKeyboardKey.arrowUp) {
      s.adjustWater(0.1);
    } else if (key == LogicalKeyboardKey.arrowDown) {
      s.adjustWater(-0.1);
    } else if (key == LogicalKeyboardKey.arrowRight) {
      s.adjustLight(0.1);
    } else if (key == LogicalKeyboardKey.arrowLeft) {
      s.adjustLight(-0.1);
    } else if (key == LogicalKeyboardKey.equal || key == LogicalKeyboardKey.add) {
      s.adjustTemp(1.0);
    } else if (key == LogicalKeyboardKey.minus || key == LogicalKeyboardKey.numpadSubtract) {
      s.adjustTemp(-1.0);
    } else {
      return KeyEventResult.ignored;
    }
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.appState;
    return Focus(
      focusNode: _focus,
      autofocus: true,
      onKeyEvent: _onKey,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _StatusCard(state: s),
            const SizedBox(height: 12),
            Expanded(child: _ChartsGrid(state: s)),
            const SizedBox(height: 12),
            _ControlsCard(state: s),
            const SizedBox(height: 12),
            _EventsCard(state: s),
          ],
        ),
      ),
    );
  }
}

class _StatusCard extends StatelessWidget {
  final AppState state;
  const _StatusCard({required this.state});

  @override
  Widget build(BuildContext context) {
    final st = state.state;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            _statusItem(
              context,
              'Stage',
              st == null ? '—' : stageLabel(st.stage),
              color: st == null ? null : _stageColor(st.stage),
            ),
            _statusItem(context, 'Time', st == null ? '—' : '${st.timeSeconds}s'),
            _statusItem(
              context,
              'Temp',
              st == null ? '—' : '${st.temperature.toStringAsFixed(1)}°C',
            ),
            _statusItem(
              context,
              'Light',
              st == null ? '—' : st.lightLevel.toStringAsFixed(2),
            ),
            _statusItem(
              context,
              'Auto',
              state.autoStep ? 'ON' : 'OFF',
              color: state.autoStep ? Colors.greenAccent : Colors.redAccent,
            ),
            if (state.busy)
              const Padding(
                padding: EdgeInsets.only(left: 12),
                child: SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _statusItem(BuildContext context, String label, String value, {Color? color}) {
    return Expanded(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: Theme.of(context).textTheme.labelSmall),
          Text(
            value,
            style: Theme.of(context).textTheme.titleMedium?.copyWith(color: color),
          ),
        ],
      ),
    );
  }
}

class _ChartsGrid extends StatelessWidget {
  final AppState state;
  const _ChartsGrid({required this.state});

  @override
  Widget build(BuildContext context) {
    return GridView.count(
      crossAxisCount: 2,
      mainAxisSpacing: 12,
      crossAxisSpacing: 12,
      childAspectRatio: 1.8,
      children: [
        _MetricChart(
          title: 'Soil Moisture',
          color: Colors.lightBlueAccent,
          history: state.soilMoistureHistory,
          minY: 0,
          maxY: 1,
        ),
        _MetricChart(
          title: 'Stress',
          color: Colors.redAccent,
          history: state.stressHistory,
          minY: 0,
          maxY: 1,
        ),
        _MetricChart(
          title: 'Health',
          color: Colors.greenAccent,
          history: state.healthHistory,
          minY: 0,
          maxY: 1,
        ),
        _MetricChart(
          title: 'Biomass',
          color: Colors.amberAccent,
          history: state.biomassHistory,
        ),
      ],
    );
  }
}

class _MetricChart extends StatelessWidget {
  final String title;
  final Color color;
  final List<HistoryPoint> history;
  final double? minY;
  final double? maxY;

  const _MetricChart({
    required this.title,
    required this.color,
    required this.history,
    this.minY,
    this.maxY,
  });

  @override
  Widget build(BuildContext context) {
    final spots = [
      for (final p in history) FlSpot(p.time, p.value),
    ];
    final hasData = spots.isNotEmpty;
    final autoMaxY = hasData
        ? spots.map((s) => s.y).reduce((a, b) => a > b ? a : b)
        : 1.0;
    final autoMinY = hasData
        ? spots.map((s) => s.y).reduce((a, b) => a < b ? a : b)
        : 0.0;
    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Expanded(
              child: hasData
                  ? LineChart(
                      LineChartData(
                        minY: minY ?? (autoMinY < 0 ? autoMinY : 0),
                        maxY: maxY ?? (autoMaxY < 1 ? 1 : autoMaxY),
                        titlesData: const FlTitlesData(
                          leftTitles: AxisTitles(
                            sideTitles: SideTitles(
                              showTitles: true,
                              reservedSize: 32,
                            ),
                          ),
                          bottomTitles: AxisTitles(
                            sideTitles: SideTitles(
                              showTitles: true,
                              reservedSize: 22,
                            ),
                          ),
                          rightTitles: AxisTitles(sideTitles: SideTitles(showTitles: false)),
                          topTitles: AxisTitles(sideTitles: SideTitles(showTitles: false)),
                        ),
                        gridData: const FlGridData(show: true),
                        borderData: FlBorderData(show: false),
                        lineBarsData: [
                          LineChartBarData(
                            spots: spots,
                            isCurved: false,
                            color: color,
                            barWidth: 2,
                            dotData: const FlDotData(show: false),
                          ),
                        ],
                      ),
                    )
                  : const Center(child: Text('No data yet')),
            ),
          ],
        ),
      ),
    );
  }
}

class _ControlsCard extends StatelessWidget {
  final AppState state;
  const _ControlsCard({required this.state});

  @override
  Widget build(BuildContext context) {
    final disabled = state.busy || !state.isConnected;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          children: [
            Row(
              children: [
                FilledButton.icon(
                  onPressed: disabled ? null : state.step,
                  icon: const Icon(Icons.skip_next),
                  label: const Text('Step (Space)'),
                ),
                const SizedBox(width: 12),
                FilledButton.tonalIcon(
                  onPressed: disabled ? null : state.toggleAutoStep,
                  icon: Icon(state.autoStep ? Icons.pause : Icons.play_arrow),
                  label: Text(state.autoStep ? 'Stop Auto (a)' : 'Auto-step (a)'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            _Adjuster(
              label: 'Water (w)',
              value: state.waterAmount.toStringAsFixed(1),
              unit: 'amount',
              onMinus: () => state.adjustWater(-0.1),
              onPlus: () => state.adjustWater(0.1),
              onApply: disabled ? null : state.water,
            ),
            const SizedBox(height: 8),
            _Adjuster(
              label: 'Light (l)',
              value: state.lightLevel.toStringAsFixed(1),
              unit: 'level',
              onMinus: () => state.adjustLight(-0.1),
              onPlus: () => state.adjustLight(0.1),
              onApply: disabled ? null : state.setLight,
            ),
            const SizedBox(height: 8),
            _Adjuster(
              label: 'Temp (t)',
              value: '${state.temperature.toStringAsFixed(1)}°C',
              unit: 'celsius',
              onMinus: () => state.adjustTemp(-1.0),
              onPlus: () => state.adjustTemp(1.0),
              onApply: disabled ? null : state.setTemp,
            ),
          ],
        ),
      ),
    );
  }
}

class _Adjuster extends StatelessWidget {
  final String label;
  final String value;
  final String unit;
  final VoidCallback onMinus;
  final VoidCallback onPlus;
  final VoidCallback? onApply;

  const _Adjuster({
    required this.label,
    required this.value,
    required this.unit,
    required this.onMinus,
    required this.onPlus,
    required this.onApply,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(width: 100, child: Text(label)),
        IconButton(onPressed: onMinus, icon: const Icon(Icons.remove)),
        SizedBox(width: 64, child: Text(value, textAlign: TextAlign.center)),
        IconButton(onPressed: onPlus, icon: const Icon(Icons.add)),
        const SizedBox(width: 12),
        FilledButton(onPressed: onApply, child: const Text('Apply')),
        const SizedBox(width: 8),
        Text(unit, style: Theme.of(context).textTheme.bodySmall),
      ],
    );
  }
}

class _EventsCard extends StatelessWidget {
  final AppState state;
  const _EventsCard({required this.state});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: SizedBox(
          height: 64,
          child: state.events.isEmpty
              ? Row(
                  children: [
                    const Icon(Icons.info_outline, size: 16),
                    const SizedBox(width: 8),
                    Text(state.lastMessage ?? 'No recent events'),
                  ],
                )
              : ListView(
                  children: state.events.map(_renderEvent).toList(),
                ),
        ),
      ),
    );
  }

  Widget _renderEvent(TomatoEvent e) {
    switch (e) {
      case StageChangeEvent(:final from, :final to):
        return Row(
          children: [
            const Icon(Icons.eco, color: Colors.greenAccent, size: 18),
            const SizedBox(width: 8),
            Text('Stage: ${stageLabel(from)} → ${stageLabel(to)}'),
          ],
        );
      case WiltRiskEvent():
        return const Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.amberAccent, size: 18),
            SizedBox(width: 8),
            Text('Wilt risk — plant is under high stress'),
          ],
        );
      case DeathEvent():
        return const Row(
          children: [
            Icon(Icons.dangerous, color: Colors.redAccent, size: 18),
            SizedBox(width: 8),
            Text('The plant has died'),
          ],
        );
      case UnknownEvent():
        return const SizedBox.shrink();
    }
  }
}

Color _stageColor(Stage s) {
  switch (s) {
    case Stage.seed:
      return Colors.white70;
    case Stage.seedling:
      return Colors.lightGreenAccent;
    case Stage.vegetative:
      return Colors.greenAccent;
    case Stage.flowering:
      return Colors.purpleAccent;
    case Stage.fruiting:
      return Colors.redAccent;
    case Stage.dead:
      return Colors.blueGrey;
    case Stage.unknown:
      return Colors.white54;
  }
}
