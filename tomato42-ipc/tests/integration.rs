use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tomato42_protocol::{IPCRequest, IPCResponse, DEFAULT_HOST};

/// Helper: start the IPC server on a given port and return the child process.
fn start_server(port: u16) -> std::process::Child {
    std::process::Command::new(env!("CARGO_BIN_EXE_tomato42-ipc"))
        .arg(port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to start server")
}

/// Helper: send a request and read the response.
async fn send_request(stream: &mut TcpStream, request: &IPCRequest) -> IPCResponse {
    let json = serde_json::to_string(request).unwrap();
    let (read, mut write) = stream.split();
    write
        .write_all(format!("{}\n", json).as_bytes())
        .await
        .unwrap();
    let mut reader = BufReader::new(read);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    serde_json::from_str(&line).unwrap()
}

/// Find a free port by binding to port 0.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_get_state_returns_initial_state() {
    let port = free_port();
    let mut server = start_server(port);

    // Give server time to bind
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    let resp = send_request(&mut stream, &IPCRequest::GetState).await;

    assert!(resp.success);
    let state = resp.state.unwrap();
    assert_eq!(state.stage, "Seed");
    assert_eq!(state.time_seconds, 0);
    assert!((state.health - 1.0).abs() < f32::EPSILON);

    server.kill().ok();
}

#[tokio::test]
async fn test_water_increases_moisture() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    // Get initial state
    let initial = send_request(&mut stream, &IPCRequest::GetState).await;
    let initial_moisture = initial.state.unwrap().soil_moisture;

    // Water the plant
    let resp = send_request(&mut stream, &IPCRequest::Water { amount: 0.5 }).await;

    assert!(resp.success);
    let state = resp.state.unwrap();
    assert!(state.soil_moisture >= initial_moisture);

    server.kill().ok();
}

#[tokio::test]
async fn test_step_advances_time() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    let resp = send_request(&mut stream, &IPCRequest::Step { seconds: 10 }).await;

    assert!(resp.success);
    let state = resp.state.unwrap();
    assert_eq!(state.time_seconds, 10);

    server.kill().ok();
}

#[tokio::test]
async fn test_set_light_and_temp() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    // Set light
    let resp = send_request(&mut stream, &IPCRequest::SetLight { level: 0.8 }).await;
    assert!(resp.success);
    assert!((resp.state.unwrap().light_level - 0.8).abs() < f32::EPSILON);

    // Set temp
    let resp = send_request(&mut stream, &IPCRequest::SetTemp { celsius: 28.0 }).await;
    assert!(resp.success);
    assert!((resp.state.unwrap().temperature - 28.0).abs() < f32::EPSILON);

    server.kill().ok();
}

#[tokio::test]
async fn test_invalid_water_amount_rejected() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    let resp = send_request(&mut stream, &IPCRequest::Water { amount: 1.5 }).await;
    assert!(!resp.success);

    server.kill().ok();
}

#[tokio::test]
async fn test_invalid_json_returns_error() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    // Send raw invalid JSON
    let (read, mut write) = stream.split();
    write.write_all(b"not valid json\n").await.unwrap();

    let mut reader = BufReader::new(read);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    let resp: IPCResponse = serde_json::from_str(&line).unwrap();
    assert!(!resp.success);
    assert!(resp.message.contains("Invalid command"));

    server.kill().ok();
}

#[tokio::test]
async fn test_multiple_steps_accumulate_time() {
    let port = free_port();
    let mut server = start_server(port);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stream = TcpStream::connect(format!("{}:{}", DEFAULT_HOST, port))
        .await
        .unwrap();

    send_request(&mut stream, &IPCRequest::Step { seconds: 5 }).await;
    send_request(&mut stream, &IPCRequest::Step { seconds: 10 }).await;
    let resp = send_request(&mut stream, &IPCRequest::GetState).await;

    assert_eq!(resp.state.unwrap().time_seconds, 15);

    server.kill().ok();
}
