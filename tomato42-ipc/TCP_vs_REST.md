# TCP vs REST API for tomato42 IPC

## Introduction

This document explains the rationale behind choosing a TCP-based JSON protocol over a REST API for the tomato42 IPC component. It compares the two approaches, discusses their pros and cons, and provides recommendations for when each approach might be more appropriate.

## Current Implementation: TCP-based JSON Protocol

The current tomato42-ipc implementation uses a custom TCP-based JSON protocol with the following characteristics:

- Direct TCP socket communication
- JSON message format for requests and responses
- Long-lived connections
- Real-time state updates via broadcast to all connected clients
- Simple request/response model with specific commands
- Stateful server that maintains the simulation state

## Alternative: REST API

A REST API implementation would have the following characteristics:

- HTTP-based communication
- JSON (or other formats) for request and response bodies
- Stateless request/response model
- Resource-oriented design with URLs representing resources
- Standard HTTP methods (GET, POST, PUT, DELETE) for operations
- Typically short-lived connections

## Comparison

### Advantages of the Current TCP Approach

1. **Real-time Updates**: The TCP implementation allows for real-time push notifications to clients when the state changes, without requiring polling.

2. **Lower Overhead**: TCP has less protocol overhead compared to HTTP, making it more efficient for frequent, small messages.

3. **Connection Persistence**: Long-lived TCP connections reduce the overhead of connection establishment for frequent interactions.

4. **Bidirectional Communication**: TCP allows for easy bidirectional communication, enabling the server to push updates to clients.

5. **Simplicity for This Use Case**: For a simple simulation with a small set of commands, a custom protocol can be more straightforward than mapping to REST resources.

### Advantages of a REST API Approach

1. **Standardization**: REST APIs follow well-established patterns and conventions, making them easier for developers to understand and use.

2. **Web Integration**: REST APIs can be easily consumed by web applications directly from browsers.

3. **Tooling**: Extensive tooling exists for developing, testing, and documenting REST APIs (Swagger, Postman, etc.).

4. **Caching**: HTTP includes built-in caching mechanisms that can improve performance for read-heavy applications.

5. **Scalability**: REST's stateless nature makes horizontal scaling easier in some scenarios.

6. **Firewall Friendliness**: HTTP traffic is commonly allowed through firewalls, while custom TCP ports might be blocked.

## Why TCP Was Chosen for tomato42

The TCP-based approach was chosen for tomato42 for the following reasons:

1. **Real-time State Synchronization**: The tomato42 simulation requires all clients to stay synchronized with the current state. The broadcast mechanism in the TCP implementation efficiently pushes updates to all connected clients without polling.

2. **Simplicity**: The interaction model is simple with a small set of commands, making a custom protocol straightforward to implement and use.

3. **Efficiency**: For frequent interactions with the simulation, the lower overhead of TCP compared to HTTP reduces latency and bandwidth usage.

4. **Long-lived Connections**: Clients are expected to maintain long-lived connections to receive state updates, which aligns well with TCP's connection-oriented nature.

5. **Language Agnostic**: The simple JSON-over-TCP protocol is easy to implement in any programming language that supports TCP sockets and JSON parsing.

## When REST Might Be More Appropriate

A REST API might be more appropriate in the following scenarios:

1. **Web-based Clients**: If the primary clients are web browsers, a REST API would be more directly consumable.

2. **Public API**: For a public-facing API that many developers will use, the standardization and documentation benefits of REST are valuable.

3. **Infrequent Interactions**: If clients interact with the server infrequently, the connection establishment overhead of HTTP is less significant.

4. **Complex Resource Modeling**: If the domain model involves complex resources with many relationships, REST's resource-oriented design can be beneficial.

5. **Integration with Web Ecosystems**: If integration with web frameworks, API gateways, or other web infrastructure is important.

## Conclusion

Both TCP-based protocols and REST APIs have their place in modern application architecture. The choice depends on the specific requirements of the application.

For tomato42, the TCP-based approach provides efficient real-time updates and simple bidirectional communication, which are well-suited to the simulation's needs. However, if the project's requirements evolve to need more web integration or public API access, adding a REST API alongside the existing TCP interface could provide the best of both worlds.

## Potential Future Work

If there's interest in adding a REST API to tomato42, it could be implemented as:

1. A separate crate (e.g., tomato42-rest) that provides HTTP endpoints for the same functionality
2. An extension to the existing tomato42-ipc crate that offers both TCP and HTTP interfaces
3. A proxy layer that translates between HTTP requests and the existing TCP protocol

This would allow clients to choose the interface that best suits their needs while maintaining the benefits of the current implementation.