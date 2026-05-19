import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'protocol.dart';

/// Single-shot request/response client for the tomato42-ipc server.
///
/// The server is strictly synchronous: one line-delimited JSON request in,
/// one line-delimited JSON response out. Requests must therefore be serialized.
class IpcClient {
  final String host;
  final int port;

  Socket? _socket;
  StreamQueue<String>? _lines;
  final _sendLock = _Mutex();

  IpcClient({this.host = defaultHost, this.port = defaultPort});

  bool get isConnected => _socket != null;

  Future<void> connect({Duration timeout = const Duration(seconds: 2)}) async {
    if (_socket != null) return;
    final socket = await Socket.connect(host, port, timeout: timeout);
    socket.setOption(SocketOption.tcpNoDelay, true);
    final lines = StreamQueue<String>(
      socket
          .cast<List<int>>()
          .transform(utf8.decoder)
          .transform(const LineSplitter()),
    );
    _socket = socket;
    _lines = lines;
  }

  Future<void> close() async {
    final socket = _socket;
    _socket = null;
    final lines = _lines;
    _lines = null;
    if (socket != null) {
      try {
        await socket.close();
      } catch (_) {}
      socket.destroy();
    }
    if (lines != null) {
      await lines.cancel(immediate: true);
    }
  }

  Future<IpcResponse> send(IpcRequest request) async {
    return _sendLock.run(() async {
      final socket = _socket;
      final lines = _lines;
      if (socket == null || lines == null) {
        throw const SocketException('Not connected');
      }
      socket.write('${jsonEncode(request.toJson())}\n');
      await socket.flush();
      final String line;
      try {
        line = await lines.next.timeout(const Duration(seconds: 5));
      } on TimeoutException {
        await close();
        throw const SocketException('IPC server did not respond');
      } on StateError {
        await close();
        throw const SocketException('IPC connection closed');
      }
      final json = jsonDecode(line) as Map<String, dynamic>;
      return IpcResponse.fromJson(json);
    });
  }
}

/// Minimal async mutex so concurrent send() callers serialize cleanly.
class _Mutex {
  Future<void> _last = Future.value();

  Future<T> run<T>(Future<T> Function() body) {
    final completer = Completer<void>();
    final prev = _last;
    _last = completer.future;
    return prev.then((_) async {
      try {
        return await body();
      } finally {
        completer.complete();
      }
    });
  }
}

/// Pulled out so the IpcClient stays free of `package:async`.
class StreamQueue<T> {
  final StreamSubscription<T> _sub;
  final _pending = <Completer<T>>[];
  final _buffered = <T>[];
  bool _done = false;
  Object? _error;

  StreamQueue(Stream<T> stream)
      : _sub = stream.listen(null, cancelOnError: true) {
    _sub
      ..onData(_onData)
      ..onError(_onError)
      ..onDone(_onDone);
  }

  void _onData(T value) {
    if (_pending.isNotEmpty) {
      _pending.removeAt(0).complete(value);
    } else {
      _buffered.add(value);
    }
  }

  void _onError(Object error, StackTrace st) {
    _error = error;
    _done = true;
    while (_pending.isNotEmpty) {
      _pending.removeAt(0).completeError(error, st);
    }
  }

  void _onDone() {
    _done = true;
    while (_pending.isNotEmpty) {
      _pending.removeAt(0).completeError(StateError('Stream closed'));
    }
  }

  Future<T> get next {
    if (_buffered.isNotEmpty) {
      return Future.value(_buffered.removeAt(0));
    }
    if (_done) {
      return Future.error(_error ?? StateError('Stream closed'));
    }
    final c = Completer<T>();
    _pending.add(c);
    return c.future;
  }

  Future<void> cancel({bool immediate = false}) => _sub.cancel();
}
