# tomato42-ipc

IPC server for the tomato42 plant simulator. This component allows external applications to interact with the tomato plant simulation over a network connection.

## Overview

The tomato42-ipc server provides a TCP-based interface for controlling and monitoring the tomato plant simulator. It uses a simple JSON-based protocol for communication, making it easy to integrate with applications written in any programming language.

## Usage

### Starting the Server

```bash
cargo run --bin tomato42-ipc [port]
```

If no port is specified, the server will listen on the default port 8043.

### Protocol

The IPC protocol is based on JSON messages sent over a TCP connection. Each message is a single line of JSON text, terminated by a newline character.

#### Requests

Clients can send the following request types:

1. **GetState**: Retrieve the current state of the tomato plant.
   ```json
   {"GetState": null}
   ```

2. **Step**: Advance the simulation by a specified number of seconds.
   ```json
   {"Step": {"seconds": 10}}
   ```

3. **Water**: Water the plant with a specified amount (0.0 to 1.0).
   ```json
   {"Water": {"amount": 0.5}}
   ```

4. **SetLight**: Set the light level (0.0 to 1.0).
   ```json
   {"SetLight": {"level": 0.7}}
   ```

5. **SetTemp**: Set the temperature in Celsius.
   ```json
   {"SetTemp": {"celsius": 25.0}}
   ```

#### Responses

The server responds to each request with a JSON object containing:

- `success`: A boolean indicating whether the request was successful.
- `message`: A human-readable message describing the result.
- `state`: The current state of the tomato plant (if available).
- `events`: A list of events that occurred during the operation.

Example response:
```json
{
  "success": true,
  "message": "Watered plant with amount: 0.50",
  "state": {
    "time_seconds": 120,
    "stage": "Seedling",
    "soil_moisture": 0.8,
    "biomass": 2.5,
    "stress": 0.1,
    "health": 0.95,
    "temperature": 22.0,
    "light_level": 0.6
  },
  "events": []
}
```

### State Updates

When any client performs an action that changes the state of the tomato plant, all connected clients will receive a state update message. This allows multiple clients to stay synchronized with the current state of the simulation.

## Example Client

Here's a simple Python client that connects to the tomato42-ipc server:

```python
import socket
import json
import time

def connect_to_tomato(host='127.0.0.1', port=8042):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    return sock

def send_command(sock, command):
    sock.sendall((json.dumps(command) + '\n').encode('utf-8'))
    response = sock.recv(4096).decode('utf-8')
    return json.loads(response)

def main():
    sock = connect_to_tomato()

    # Get the current state
    response = send_command(sock, {"GetState": None})
    print("Initial state:", response)

    # Water the plant
    response = send_command(sock, {"Water": {"amount": 0.5}})
    print("After watering:", response)

    # Set the light level
    response = send_command(sock, {"SetLight": {"level": 0.7}})
    print("After setting light:", response)

    # Step the simulation
    response = send_command(sock, {"Step": {"seconds": 10}})
    print("After stepping:", response)

    sock.close()

if __name__ == "__main__":
    main()
```

## Integration with Other Languages

Since the protocol is based on simple JSON messages over TCP, it's easy to integrate with applications written in any programming language that supports TCP sockets and JSON parsing.

### Using netcat

You can interact with the server using `nc` (netcat). The server uses a persistent TCP connection with newline-delimited JSON, so use `-q` to close after receiving the response:

#### Get the current state

```bash
echo '{"GetState":null}' | nc -q 1 localhost 8043
```

#### Water the plant

```bash
echo '{"Water":{"amount":0.5}}' | nc -q 1 localhost 8043
```

#### Step the simulation

```bash
echo '{"Step":{"seconds":10}}' | nc -q 1 localhost 8043
```

#### Set the light level

```bash
echo '{"SetLight":{"level":0.7}}' | nc -q 1 localhost 8043
```

#### Set the temperature

```bash
echo '{"SetTemp":{"celsius":25.0}}' | nc -q 1 localhost 8043
```

#### Send multiple commands in one session

```bash
nc localhost 8043 <<'EOF'
{"GetState":null}
{"Water":{"amount":0.3}}
{"Step":{"seconds":10}}
EOF
```

Note: `curl` cannot be used here — this is a raw TCP protocol, not HTTP. Use `nc`, `socat`, or a custom client.

## Why TCP Instead of REST?

The tomato42-ipc component uses a TCP-based JSON protocol rather than a REST API. This design choice was made to optimize for real-time updates, bidirectional communication, and efficient handling of frequent, small messages.

For a detailed comparison of the TCP approach versus a REST API approach, see the [TCP vs REST API](TCP_vs_REST.md) document.

## Error Handling

If a request is invalid or cannot be processed, the server will respond with a JSON object where `success` is `false` and `message` contains an error description.

Example error response:
```json
{
  "success": false,
  "message": "Invalid command: expected value at line 1 column 1",
  "state": null,
  "events": []
}
```
