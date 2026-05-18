//! TUI overview with graphs for the tomato42 plant simulator.
//!
//! This application provides a text-based user interface with time-series graphs
//! showing the internal state of the tomato plant.
//!
//! This TUI communicates with the tomato42-ipc server over a TCP connection,
//! allowing it to run in parallel with other clients like the CLI application.
//! Multiple clients can interact with the same simulation instance simultaneously.
//!
//! Before running this TUI, make sure the tomato42-ipc server is running:
//! ```
//! cargo run --bin tomato42-ipc
//! ```
//!
//! If the IPC server is not available, the TUI will automatically fall back to
//! standalone mode, using the local simulation engine. This ensures the application
//! remains functional even without the IPC server, but it won't be synchronized
//! with other clients.
//!
//! The connection status is displayed in the status bar:
//! - "Connected" - Successfully connected to the IPC server
//! - "Standalone" - Running in standalone mode (local simulation)

use ringbuffer::RingBufferExt;
use ringbuffer::RingBufferWrite;
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ringbuffer::AllocRingBuffer;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap},
    Frame, Terminal,
};

use tomato42_core::{step, Action, Event as TomatoEvent, Stage, TomatoState};
use tomato42_protocol::{
    IPCRequest, IPCResponse, SerializableTomatoEvent, SerializableTomatoState, DEFAULT_HOST,
    DEFAULT_PORT,
};

const BUFFER_SIZE: usize = 128; // Must be power of 2

/// Helper to convert string stage to Stage enum
fn string_to_stage(stage_str: &str) -> Stage {
    match stage_str {
        "Seed" => Stage::Seed,
        "Seedling" => Stage::Seedling,
        "Vegetative" => Stage::Vegetative,
        "Flowering" => Stage::Flowering,
        "Fruiting" => Stage::Fruiting,
        "Dead" => Stage::Dead,
        _ => Stage::Seed, // Default
    }
}

/// Convert SerializableTomatoState to TomatoState
fn to_tomato_state(serializable: &SerializableTomatoState) -> TomatoState {
    let mut state = TomatoState::new();
    state.time = Duration::from_secs(serializable.time_seconds);
    state.stage = string_to_stage(&serializable.stage);
    state.soil_moisture = serializable.soil_moisture;
    state.biomass = serializable.biomass;
    state.stress = serializable.stress;
    state.health = serializable.health;
    state.temperature = serializable.temperature;
    state.light_level = serializable.light_level;
    state
}

/// Convert SerializableTomatoEvent to TomatoEvent
fn to_tomato_event(serializable: &SerializableTomatoEvent) -> TomatoEvent {
    match serializable {
        SerializableTomatoEvent::StageChange { from, to } => TomatoEvent::StageChange {
            from: string_to_stage(from),
            to: string_to_stage(to),
        },
        SerializableTomatoEvent::WiltRisk => TomatoEvent::WiltRisk,
        SerializableTomatoEvent::Death => TomatoEvent::Death,
    }
}

/// IPC client to communicate with the server
struct IPCClient {
    stream: TcpStream,
}

impl IPCClient {
    /// Connect to the IPC server
    fn connect() -> Result<Self, io::Error> {
        let server_addr = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT);
        println!("Connecting to IPC server at {}...", server_addr);

        match TcpStream::connect(&server_addr) {
            Ok(stream) => {
                println!("Connected to IPC server");
                Ok(Self { stream })
            }
            Err(e) => {
                eprintln!("Failed to connect to IPC server: {}", e);
                eprintln!("Make sure the tomato42-ipc server is running with:");
                eprintln!("  cargo run --bin tomato42-ipc");
                Err(e)
            }
        }
    }

    /// Send a command to the server and get the response
    fn send_command(&mut self, request: IPCRequest) -> Result<IPCResponse, io::Error> {
        // Serialize the request to JSON
        let json = serde_json::to_string(&request)?;

        // Send the request to the server
        self.stream.write_all(format!("{}\n", json).as_bytes())?;
        self.stream.flush()?;

        // Read the response
        let mut response_str = String::new();
        let mut reader = BufReader::new(self.stream.try_clone()?);
        reader.read_line(&mut response_str)?;

        // Deserialize the response
        match serde_json::from_str(&response_str) {
            Ok(response) => Ok(response),
            Err(e) => {
                eprintln!("Failed to parse server response: {}", e);
                eprintln!("Response was: {}", response_str);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid server response",
                ))
            }
        }
    }
}

/// Stores historical data for graphing
struct HistoricalData {
    time_points: AllocRingBuffer<f64>,
    soil_moisture: AllocRingBuffer<f64>,
    stress: AllocRingBuffer<f64>,
    health: AllocRingBuffer<f64>,
    biomass: AllocRingBuffer<f64>,
}

impl HistoricalData {
    fn new(capacity: usize) -> Self {
        Self {
            time_points: AllocRingBuffer::with_capacity(capacity),
            soil_moisture: AllocRingBuffer::with_capacity(capacity),
            stress: AllocRingBuffer::with_capacity(capacity),
            health: AllocRingBuffer::with_capacity(capacity),
            biomass: AllocRingBuffer::with_capacity(capacity),
        }
    }

    fn add_data_point(&mut self, state: &TomatoState) {
        let time = state.time.as_secs_f64();
        self.time_points.push(time);
        self.soil_moisture.push(state.soil_moisture as f64);
        self.stress.push(state.stress as f64);
        self.health.push(state.health as f64);
        self.biomass.push(state.biomass as f64);
    }

    fn get_data_points(&self, buffer: &AllocRingBuffer<f64>) -> Vec<(f64, f64)> {
        self.time_points
            .iter()
            .zip(buffer.iter())
            .map(|(&x, &y)| (x, y))
            .collect()
    }
}

/// Application state
struct App {
    state: TomatoState,
    historical_data: HistoricalData,
    last_events: Vec<TomatoEvent>,
    last_update: Instant,
    auto_step: bool,
    auto_step_interval: Duration,
    selected_action: Action,
    water_amount: f32,
    light_level: f32,
    temperature: f32,
    ipc_client: Option<IPCClient>,
    connection_error: Option<String>,
}

impl App {
    fn new() -> Self {
        // Try to connect to the IPC server
        let (mut ipc_client, connection_error) = match IPCClient::connect() {
            Ok(client) => (Some(client), None),
            Err(e) => (
                None,
                Some(format!("Failed to connect to IPC server: {}", e)),
            ),
        };

        // Initialize with local state if we couldn't connect
        let state = TomatoState::new();
        let mut historical_data = HistoricalData::new(BUFFER_SIZE);
        historical_data.add_data_point(&state);

        // If we have a client, try to get the initial state from the server
        let (state, last_events) = if let Some(client) = &mut ipc_client {
            match client.send_command(IPCRequest::GetState) {
                Ok(response) => {
                    if let Some(server_state) = response.state {
                        let tomato_state = to_tomato_state(&server_state);
                        historical_data = HistoricalData::new(BUFFER_SIZE);
                        historical_data.add_data_point(&tomato_state);

                        // Convert events
                        let events = response.events.iter().map(to_tomato_event).collect();

                        (tomato_state, events)
                    } else {
                        (state, Vec::new())
                    }
                }
                Err(e) => {
                    eprintln!("Error getting initial state from server: {}", e);
                    (state, Vec::new())
                }
            }
        } else {
            (state, Vec::new())
        };

        Self {
            state,
            historical_data,
            last_events,
            last_update: Instant::now(),
            auto_step: false,
            auto_step_interval: Duration::from_millis(500),
            selected_action: Action::DoNothing,
            water_amount: 0.5,
            light_level: 0.5,
            temperature: 20.0,
            ipc_client,
            connection_error,
        }
    }

    fn step(&mut self) {
        // If we have an IPC client, use it
        if let Some(client) = &mut self.ipc_client {
            // Convert the selected action to an IPC request
            let request = match self.selected_action {
                Action::DoNothing => IPCRequest::Step { seconds: 1 },
                Action::Water { amount } => IPCRequest::Water { amount },
                Action::SetLight { level } => IPCRequest::SetLight { level },
                Action::SetTemp { celsius } => IPCRequest::SetTemp { celsius },
            };

            // Send the command to the server
            match client.send_command(request) {
                Ok(response) => {
                    if response.success {
                        if let Some(server_state) = response.state {
                            // Update our local state
                            self.state = to_tomato_state(&server_state);

                            // Convert events
                            self.last_events =
                                response.events.iter().map(to_tomato_event).collect();

                            // Update historical data
                            self.historical_data.add_data_point(&self.state);
                        }
                    } else {
                        eprintln!("Server error: {}", response.message);
                    }
                }
                Err(e) => {
                    eprintln!("Error communicating with server: {}", e);
                    self.connection_error = Some(format!("Lost connection to server: {}", e));
                    self.ipc_client = None;

                    // Fall back to local simulation if we lose connection
                    let result = step(
                        self.state.clone(),
                        self.selected_action,
                        Duration::from_secs(1),
                    );
                    self.state = result.state.clone();
                    self.last_events = result.events;
                    self.historical_data.add_data_point(&self.state);
                }
            }
        } else {
            // Fall back to local simulation if we don't have a client
            let result = step(
                self.state.clone(),
                self.selected_action,
                Duration::from_secs(1),
            );
            self.state = result.state.clone();
            self.last_events = result.events;
            self.historical_data.add_data_point(&self.state);
        }

        // Reset to DoNothing after each step
        self.selected_action = Action::DoNothing;
    }

    fn update(&mut self) -> bool {
        if self.auto_step && self.last_update.elapsed() >= self.auto_step_interval {
            self.step();
            self.last_update = Instant::now();
            return true;
        }
        false
    }

    fn toggle_auto_step(&mut self) {
        self.auto_step = !self.auto_step;
        self.last_update = Instant::now();
    }

    fn is_connected(&self) -> bool {
        self.ipc_client.is_some()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Check for auto-step update
        if app.update() {
            continue;
        }

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => app.step(),
                    KeyCode::Char('a') => app.toggle_auto_step(),
                    KeyCode::Char('w') => {
                        app.selected_action = Action::Water {
                            amount: app.water_amount,
                        };
                        app.step();
                    }
                    KeyCode::Char('l') => {
                        app.selected_action = Action::SetLight {
                            level: app.light_level,
                        };
                        app.step();
                    }
                    KeyCode::Char('t') => {
                        app.selected_action = Action::SetTemp {
                            celsius: app.temperature,
                        };
                        app.step();
                    }
                    KeyCode::Up => app.water_amount = (app.water_amount + 0.1).min(1.0),
                    KeyCode::Down => app.water_amount = (app.water_amount - 0.1).max(0.0),
                    KeyCode::Right => app.light_level = (app.light_level + 0.1).min(1.0),
                    KeyCode::Left => app.light_level = (app.light_level - 0.1).max(0.0),
                    KeyCode::Char('+') => app.temperature += 1.0,
                    KeyCode::Char('-') => app.temperature -= 1.0,
                    _ => {}
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Length(5), // Status
                Constraint::Min(10),   // Charts
                Constraint::Length(5), // Controls
                Constraint::Length(5), // Events
            ]
            .as_ref(),
        )
        .split(f.size());

    // Title
    let title = Paragraph::new("Tomato42 - Deterministic Tomato Plant Simulator")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Status
    render_status(f, app, chunks[1]);

    // Charts area
    let charts_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[2]);

    let top_charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(charts_layout[0]);

    let bottom_charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(charts_layout[1]);

    // Render charts
    render_chart(
        f,
        app,
        "Soil Moisture",
        &app.historical_data.soil_moisture,
        Color::Blue,
        top_charts[0],
    );
    render_chart(
        f,
        app,
        "Stress",
        &app.historical_data.stress,
        Color::Red,
        top_charts[1],
    );
    render_chart(
        f,
        app,
        "Health",
        &app.historical_data.health,
        Color::Green,
        bottom_charts[0],
    );
    render_chart(
        f,
        app,
        "Biomass",
        &app.historical_data.biomass,
        Color::Yellow,
        bottom_charts[1],
    );

    // Controls
    render_controls(f, app, chunks[3]);

    // Events
    render_events(f, app, chunks[4]);
}

fn render_status<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let mut status_text = vec![
        Spans::from(vec![
            Span::raw("Stage: "),
            Span::styled(
                format!("{:?}", app.state.stage),
                Style::default().fg(get_stage_color(&app.state.stage)),
            ),
            Span::raw(" | Time: "),
            Span::raw(format!("{}s", app.state.time.as_secs())),
            Span::raw(" | Auto: "),
            Span::styled(
                if app.auto_step { "ON" } else { "OFF" },
                Style::default().fg(if app.auto_step {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
            Span::raw(" | IPC: "),
            Span::styled(
                if app.is_connected() {
                    "Connected"
                } else {
                    "Standalone"
                },
                Style::default().fg(if app.is_connected() {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ),
        ]),
        Spans::from(vec![
            Span::raw("Temperature: "),
            Span::raw(format!("{:.1}°C", app.state.temperature)),
            Span::raw(" | Light: "),
            Span::raw(format!("{:.2}", app.state.light_level)),
        ]),
    ];

    // Add error message if there is one
    if let Some(error) = &app.connection_error {
        status_text.push(Spans::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(error),
        ]));
    }

    let status = Paragraph::new(status_text)
        .block(Block::default().title("Status").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(status, area);
}

fn render_chart<B: Backend>(
    f: &mut Frame<B>,
    app: &App,
    title: &str,
    data_buffer: &AllocRingBuffer<f64>,
    color: Color,
    area: Rect,
) {
    let data = app.historical_data.get_data_points(data_buffer);

    // Find min/max values for y-axis
    let min_y = data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_bounds = [min_y.max(0.0), max_y.max(1.0)];

    // Find min/max values for x-axis
    let min_x = data.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = data
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_bounds = [min_x.max(0.0), max_x.max(1.0)];

    let datasets = vec![Dataset::default()
        .name(title)
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(color))
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(Block::default().title(title).borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Time (s)")
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(vec![
                    Span::styled(
                        format!("{:.0}", x_bounds[0]),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("{:.0}", x_bounds[1]),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Value")
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(vec![
                    Span::styled(
                        format!("{:.1}", y_bounds[0]),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("{:.1}", y_bounds[1]),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
        );
    f.render_widget(chart, area);
}

fn render_controls<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let controls_text = vec![
        Spans::from(vec![
            Span::styled("Controls: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("q: Quit | Space: Step | a: Toggle Auto"),
        ]),
        Spans::from(vec![
            Span::raw("w: Water ("),
            Span::styled(
                format!("{:.1}", app.water_amount),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(") | l: Light ("),
            Span::styled(
                format!("{:.1}", app.light_level),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(") | t: Temp ("),
            Span::styled(
                format!("{:.1}°C", app.temperature),
                Style::default().fg(Color::Red),
            ),
            Span::raw(")"),
        ]),
        Spans::from(vec![Span::raw(
            "↑/↓: Water amount | ←/→: Light level | +/-: Temperature",
        )]),
    ];

    let controls = Paragraph::new(controls_text)
        .block(Block::default().title("Controls").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(controls, area);
}

fn render_events<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let mut event_text = Vec::new();

    if app.last_events.is_empty() {
        event_text.push(Spans::from(Span::raw("No recent events")));
    } else {
        for event in &app.last_events {
            match event {
                TomatoEvent::StageChange { from, to } => {
                    event_text.push(Spans::from(vec![
                        Span::raw("Stage changed from "),
                        Span::styled(
                            format!("{:?}", from),
                            Style::default().fg(get_stage_color(from)),
                        ),
                        Span::raw(" to "),
                        Span::styled(
                            format!("{:?}", to),
                            Style::default().fg(get_stage_color(to)),
                        ),
                    ]));
                }
                TomatoEvent::WiltRisk => {
                    event_text.push(Spans::from(vec![
                        Span::styled(
                            "WARNING: ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("Plant is at risk of wilting due to high stress!"),
                    ]));
                }
                TomatoEvent::Death => {
                    event_text.push(Spans::from(vec![
                        Span::styled(
                            "ALERT: ",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("Plant has died!"),
                    ]));
                }
            }
        }
    }

    let events = Paragraph::new(event_text)
        .block(Block::default().title("Events").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(events, area);
}

fn get_stage_color(stage: &Stage) -> Color {
    match stage {
        Stage::Seed => Color::White,
        Stage::Seedling => Color::LightGreen,
        Stage::Vegetative => Color::Green,
        Stage::Flowering => Color::Magenta,
        Stage::Fruiting => Color::Red,
        Stage::Dead => Color::DarkGray,
    }
}
